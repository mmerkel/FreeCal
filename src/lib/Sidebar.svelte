<script lang="ts">
  import EllipsisVertical from '@lucide/svelte/icons/ellipsis-vertical';
  import Eye from '@lucide/svelte/icons/eye';
  import Pencil from '@lucide/svelte/icons/pencil';
  import Plus from '@lucide/svelte/icons/plus';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import type { CoreApi } from '../core/CoreApi';
  import type { Account, AccountId, Calendar, CalendarId } from '../core/types';
  import { t } from '../i18n';
  import Dialog from './Dialog.svelte';
  import Menu from './Menu.svelte';
  import { newCalendarColour, SWATCHES } from './palette';

  interface Props {
    core: CoreApi;
    accounts: Account[];
    calendars: Calendar[];
  }

  let { core, accounts, calendars }: Props = $props();

  /** What the user is in the middle of, if anything. Only one at a time. */
  type Editing =
    | { kind: 'creating'; accountId: AccountId }
    | { kind: 'renaming'; id: CalendarId }
    | { kind: 'confirmingDelete'; calendar: Calendar };

  let editing = $state<Editing>();
  /** The Calendar whose menu is open. */
  let menuFor = $state<CalendarId>();
  let failure = $state<string>();

  function accountName(account: Account): string {
    switch (account.provider) {
      case 'local':
        return t('account.local');
    }
  }

  function calendarsOf(account: Account): Calendar[] {
    return calendars.filter((calendar) => calendar.accountId === account.id);
  }

  /**
   * Runs a change through the core. The sidebar then shows the result when
   * "Calendars changed" arrives, never by changing its own copy.
   */
  async function change(request: () => Promise<unknown>): Promise<boolean> {
    failure = undefined;
    try {
      await request();
      return true;
    } catch (error) {
      failure = t('sidebar.changeFailed', { error: String(error) });
      return false;
    }
  }

  async function create(accountId: AccountId, name: string) {
    const colour = newCalendarColour(calendars);
    if (await change(() => core.createCalendar(accountId, name, colour))) editing = undefined;
  }

  async function rename(id: CalendarId, name: string) {
    if (await change(() => core.renameCalendar(id, name))) editing = undefined;
  }

  async function remove(id: CalendarId) {
    if (await change(() => core.deleteCalendar(id))) editing = undefined;
  }

  /** Stops what the user is in the middle of, with its report if it failed. */
  function cancel() {
    editing = undefined;
    failure = undefined;
  }

  function openMenu(calendar: Calendar) {
    editing = undefined;
    menuFor = calendar.id;
  }

  /** Closes a Calendar's menu, unless another Calendar's menu has opened since. */
  function closeMenu(id: CalendarId) {
    if (menuFor === id) menuFor = undefined;
  }

  /** Runs a menu item's action: the menu closes first, so focus returns to its button. */
  function choose(id: CalendarId, action: () => void) {
    closeMenu(id);
    action();
  }

  /** Right-click, the context-menu key and Shift+F10 on a row's buttons open its menu. */
  function menuOpeners(calendar: Calendar) {
    return {
      oncontextmenu(clicked: MouseEvent) {
        clicked.preventDefault();
        openMenu(calendar);
      },
      onkeydown(key: KeyboardEvent) {
        if (key.key === 'ContextMenu' || (key.key === 'F10' && key.shiftKey)) {
          key.preventDefault();
          openMenu(calendar);
        }
      },
    };
  }

  function focus(element: HTMLElement) {
    element.focus();
    if (element instanceof HTMLInputElement) element.select();
  }
</script>

{#snippet nameForm(initialName: string, colour: string, save: (name: string) => void)}
  <form
    class="row name-form"
    onsubmit={(submitted) => {
      submitted.preventDefault();
      const name = new FormData(submitted.currentTarget).get('name');
      save(String(name));
    }}
  >
    <span class="dot" style:--colour={colour} aria-hidden="true"></span>
    <input
      class="input"
      name="name"
      aria-label={t('calendar.name')}
      value={initialName}
      required
      onkeydown={(key) => {
        if (key.key === 'Escape') cancel();
      }}
      {@attach focus}
    />
  </form>
{/snippet}

<nav class="sidebar" aria-label={t('sidebar.label')}>
  {#if failure}
    <p class="failure" role="alert">{failure}</p>
  {/if}

  {#each accounts as account (account.id)}
    {@const headingId = `sidebar-account-${account.id}`}
    <section>
      <div class="account">
        <h2 id={headingId} title={accountName(account)}>{accountName(account)}</h2>
        {#if account.provider === 'local'}
          <button
            type="button"
            class="icon-button"
            aria-label={t('sidebar.newCalendar')}
            title={t('sidebar.newCalendar')}
            onclick={() => (editing = { kind: 'creating', accountId: account.id })}
          >
            <Plus size={14} aria-hidden="true" />
          </button>
        {/if}
      </div>

      <ul aria-labelledby={headingId}>
        {#each calendarsOf(account) as calendar (calendar.id)}
          {@const optionsLabel = t('calendar.optionsNamed', { name: calendar.name })}
          <li
            class:open={menuFor === calendar.id}
            class:hidden={!calendar.shown}
          >
            {#if editing?.kind === 'renaming' && editing.id === calendar.id}
              {@render nameForm(calendar.name, calendar.colour, (name) =>
                rename(calendar.id, name),
              )}
            {:else}
              <button
                type="button"
                class="row toggle"
                role="switch"
                aria-checked={calendar.shown}
                {...menuOpeners(calendar)}
                onclick={() => change(() => core.setCalendarShown(calendar.id, !calendar.shown))}
              >
                <span class="dot" style:--colour={calendar.colour} aria-hidden="true"></span>
                <span class="name" title={calendar.name}>{calendar.name}</span>
              </button>
              <button
                type="button"
                class="icon-button more"
                aria-label={optionsLabel}
                title={optionsLabel}
                aria-haspopup="menu"
                aria-expanded={menuFor === calendar.id}
                {...menuOpeners(calendar)}
                onclick={() =>
                  menuFor === calendar.id ? closeMenu(calendar.id) : openMenu(calendar)}
              >
                <EllipsisVertical size={14} aria-hidden="true" />
              </button>
            {/if}

            {#if menuFor === calendar.id}
              {@const colourLabelId = `sidebar-colour-${calendar.id}`}
              <div class="menu-anchor">
                <Menu label={optionsLabel} onclose={() => closeMenu(calendar.id)}>
                  <div class="menu-label" id={colourLabelId}>{t('calendar.colour')}</div>
                  <div class="swatches" role="group" aria-labelledby={colourLabelId}>
                    {#each SWATCHES as swatch (swatch.colour)}
                      <button
                        type="button"
                        class="swatch"
                        role="menuitemradio"
                        aria-checked={swatch.colour === calendar.colour}
                        aria-label={t(swatch.name)}
                        title={t(swatch.name)}
                        style:--colour={swatch.colour}
                        onclick={() =>
                          choose(calendar.id, () =>
                            change(() => core.recolourCalendar(calendar.id, swatch.colour)),
                          )}
                      ></button>
                    {/each}
                  </div>
                  <hr class="menu-separator" />
                  <button
                    type="button"
                    class="menu-item"
                    role="menuitem"
                    onclick={() =>
                      choose(calendar.id, () => (editing = { kind: 'renaming', id: calendar.id }))}
                  >
                    <Pencil size={14} aria-hidden="true" />
                    {t('calendar.rename')}
                  </button>
                  <button
                    type="button"
                    class="menu-item"
                    role="menuitem"
                    onclick={() =>
                      choose(calendar.id, () =>
                        change(() => core.showOnlyCalendar(calendar.id)),
                      )}
                  >
                    <Eye size={14} aria-hidden="true" />
                    {t('calendar.showOnly')}
                  </button>
                  <hr class="menu-separator" />
                  <button
                    type="button"
                    class="menu-item danger"
                    role="menuitem"
                    onclick={() =>
                      choose(calendar.id, () => (editing = { kind: 'confirmingDelete', calendar }))}
                  >
                    <Trash2 size={14} aria-hidden="true" />
                    {t('calendar.delete')}
                  </button>
                </Menu>
              </div>
            {/if}
          </li>
        {/each}

        {#if editing?.kind === 'creating' && editing.accountId === account.id}
          <li>
            {@render nameForm('', newCalendarColour(calendars), (name) => create(account.id, name))}
          </li>
        {/if}
      </ul>
    </section>
  {/each}
</nav>

{#if editing?.kind === 'confirmingDelete'}
  {@const doomed = editing.calendar}
  <Dialog
    role="alertdialog"
    title={t('calendar.deleteTitle', { name: doomed.name })}
    onclose={cancel}
  >
    <p>{t('calendar.deleteBody')}</p>
    {#snippet actions()}
      <button type="button" class="button outline" data-initial-focus onclick={cancel}>
        {t('common.cancel')}
      </button>
      <button type="button" class="button danger" onclick={() => remove(doomed.id)}>
        {t('calendar.delete')}
      </button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .sidebar {
    box-sizing: border-box;
    flex: none;
    width: 14rem;
    padding: var(--space-6) var(--space-4);
    overflow-y: auto;
    border-right: 1px solid var(--border);
    background: var(--surface);
  }

  .account {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: var(--space-2) var(--space-2) var(--space-2) var(--space-4);
  }

  h2 {
    margin: 0;
    overflow: hidden;
    color: var(--foreground-muted);
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: 0.04em;
    text-overflow: ellipsis;
    text-transform: uppercase;
    white-space: nowrap;
  }

  ul {
    list-style: none;
    margin: 0 0 var(--space-6);
    padding: 0;
  }

  li {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-4);
    min-height: 1.75rem;
    padding: 0 var(--space-1) 0 var(--space-4);
    border-radius: var(--radius);
  }

  li:hover,
  li.open {
    background: var(--hover);
  }

  .row {
    display: flex;
    flex: 1;
    align-items: center;
    gap: var(--space-5);
    min-width: 0;
  }

  .toggle {
    align-self: stretch;
    padding: 0;
    border: none;
    background: none;
    color: inherit;
    font: inherit;
    text-align: start;
    cursor: pointer;
  }

  /* Filled when shown, a ring when hidden. Drawn as radial gradients with a
     soft edge, which look rounder at this size than a rounded box does. */
  .dot {
    flex: none;
    width: 0.625rem;
    height: 0.625rem;
    background: radial-gradient(
      circle closest-side,
      var(--colour) calc(100% - 1px),
      transparent 100%
    );
  }

  li.hidden .dot {
    background: radial-gradient(
      circle closest-side,
      transparent calc(100% - 2.5px),
      var(--colour) calc(100% - 1.75px),
      var(--colour) calc(100% - 0.75px),
      transparent 100%
    );
  }

  li.hidden .name {
    color: var(--foreground-faint);
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Opacity, not visibility, so that the button stays reachable by keyboard
     and screen readers. */
  .more {
    opacity: 0;
  }

  li:hover .more,
  li.open .more,
  li:has(:focus-visible) .more {
    opacity: 1;
  }

  .name-form .input {
    flex: 1;
    margin: var(--space-1) 0;
  }

  /* The menu opens below its row, inside the sidebar, so it isn't clipped. */
  .menu-anchor {
    position: absolute;
    top: 100%;
    right: 0;
    z-index: 10;
  }

  .swatches {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    gap: var(--space-2);
    padding: var(--space-1) var(--space-4) var(--space-3);
  }

  .swatch {
    width: 0.875rem;
    height: 0.875rem;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: var(--colour);
    cursor: pointer;
  }

  .swatch[aria-checked='true'],
  .swatch:hover,
  .swatch:focus-visible {
    outline: none;
    box-shadow:
      0 0 0 2px var(--surface-raised),
      0 0 0 3px var(--foreground);
  }

  .failure {
    margin: 0 var(--space-4) var(--space-4);
    color: var(--danger);
  }
</style>
