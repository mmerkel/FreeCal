<script lang="ts">
  import CircleMinus from '@lucide/svelte/icons/circle-minus';
  import EllipsisVertical from '@lucide/svelte/icons/ellipsis-vertical';
  import Eye from '@lucide/svelte/icons/eye';
  import ListChecks from '@lucide/svelte/icons/list-checks';
  import Pencil from '@lucide/svelte/icons/pencil';
  import Plus from '@lucide/svelte/icons/plus';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import Unplug from '@lucide/svelte/icons/unplug';
  import type { CoreApi } from '../core/CoreApi';
  import type { Account, AccountId, AvailableCalendar, Calendar, CalendarId } from '../core/types';
  import { errorText, t, type MessageKey } from '../i18n';
  import AddAccountDialog from './AddAccountDialog.svelte';
  import Dialog from './Dialog.svelte';
  import ManageCalendarsDialog from './ManageCalendarsDialog.svelte';
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
    | { kind: 'confirmingDelete'; calendar: Calendar }
    | { kind: 'confirmingRemove'; calendar: Calendar }
    | { kind: 'addingAccount' }
    /** `found` holds what adding the Account found, when it was just added. */
    | { kind: 'managing'; account: Account; found?: AvailableCalendar[] }
    | { kind: 'confirmingRemoveAccount'; account: Account };

  let editing = $state<Editing>();
  /** The Calendar whose menu is open. */
  let menuFor = $state<CalendarId>();
  /** The server Account whose menu is open. */
  let accountMenuFor = $state<AccountId>();
  /** Each Calendar's ⋮ button, which its menu opens against however it is opened. */
  const moreButtons: Record<CalendarId, HTMLButtonElement> = {};
  /** Each server Account's ⋮ button. */
  const accountButtons: Record<AccountId, HTMLButtonElement> = {};
  let failure = $state<string>();
  /** Why the last save in "Manage calendars…" failed. */
  let manageFailure = $state<string>();

  function accountName(account: Account): string {
    switch (account.provider) {
      case 'local':
        return t('account.local');
      case 'caldav':
        return t('account.caldav', { username: account.server?.username ?? '' });
    }
  }

  function calendarsOf(account: Account): Calendar[] {
    return calendars.filter((calendar) => calendar.accountId === account.id);
  }

  /** Whether a Calendar comes from a server Account, so that it is removed rather than deleted. */
  function fromServer(calendar: Calendar): boolean {
    const account = accounts.find((account) => account.id === calendar.accountId);
    return account !== undefined && account.provider !== 'local';
  }

  /**
   * Runs a change through the core, reporting a failure with `report`. The
   * sidebar then shows the result when "Calendars changed" arrives, never by
   * changing its own copy.
   */
  async function change(
    request: () => Promise<unknown>,
    report: MessageKey = 'sidebar.changeFailed',
  ): Promise<boolean> {
    failure = undefined;
    try {
      await request();
      return true;
    } catch (error) {
      failure = t(report, { error: errorText(error) });
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

  async function removeFromFreeCal(id: CalendarId) {
    if (await change(() => core.removeCalendar(id))) editing = undefined;
  }

  async function removeAccount(id: AccountId) {
    if (await change(() => core.removeAccount(id))) editing = undefined;
  }

  function startManaging(account: Account, found?: AvailableCalendar[]) {
    manageFailure = undefined;
    editing = { kind: 'managing', account, found };
  }

  /**
   * Saves "Manage calendars…": removes the unchecked Calendars, then closes
   * the dialog while the added ones load for the first time, which the
   * sidebar shows. A failure to add or load them is reported here too.
   */
  async function saveCalendars(account: Account, removed: CalendarId[], added: string[]) {
    manageFailure = undefined;
    try {
      for (const id of removed) await core.removeCalendar(id);
    } catch (error) {
      manageFailure = t('manage.removeFailed', { error: errorText(error) });
      return;
    }
    editing = undefined;
    if (added.length > 0) {
      change(() => core.addCalendars(account.id, added), 'sidebar.addCalendarsFailed');
    }
  }

  /** Stops what the user is in the middle of, with its report if it failed. */
  function cancel() {
    editing = undefined;
    failure = undefined;
  }

  function openMenu(calendar: Calendar) {
    editing = undefined;
    accountMenuFor = undefined;
    menuFor = calendar.id;
  }

  function openAccountMenu(account: Account) {
    editing = undefined;
    menuFor = undefined;
    accountMenuFor = account.id;
  }

  /** Closes an Account's menu, unless another menu has opened since. */
  function closeAccountMenu(id: AccountId) {
    if (accountMenuFor === id) accountMenuFor = undefined;
  }

  function chooseForAccount(id: AccountId, action: () => void) {
    closeAccountMenu(id);
    action();
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
    <span class="calendar-dot" style:--colour={colour} aria-hidden="true"></span>
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
        {:else}
          {@const accountOptions = t('account.optionsNamed', { account: accountName(account) })}
          <button
            type="button"
            class="icon-button"
            aria-label={accountOptions}
            title={accountOptions}
            aria-haspopup="menu"
            aria-expanded={accountMenuFor === account.id}
            bind:this={accountButtons[account.id]}
            onclick={() =>
              accountMenuFor === account.id
                ? closeAccountMenu(account.id)
                : openAccountMenu(account)}
          >
            <EllipsisVertical size={14} aria-hidden="true" />
          </button>
          {#if accountMenuFor === account.id}
            <Menu
              label={accountOptions}
              anchor={accountButtons[account.id]}
              onclose={() => closeAccountMenu(account.id)}
            >
              <button
                type="button"
                class="menu-item"
                role="menuitem"
                onclick={() => chooseForAccount(account.id, () => startManaging(account))}
              >
                <ListChecks size={14} aria-hidden="true" />
                {t('account.manageCalendars')}
              </button>
              <hr class="menu-separator" />
              <button
                type="button"
                class="menu-item danger"
                role="menuitem"
                onclick={() =>
                  chooseForAccount(
                    account.id,
                    () => (editing = { kind: 'confirmingRemoveAccount', account }),
                  )}
              >
                <Unplug size={14} aria-hidden="true" />
                {t('account.remove')}
              </button>
            </Menu>
          {/if}
        {/if}
      </div>

      <ul aria-labelledby={headingId}>
        {#each calendarsOf(account) as calendar (calendar.id)}
          {@const optionsLabel = t('calendar.optionsNamed', { name: calendar.name })}
          {@const loadingId = `sidebar-loading-${calendar.id}`}
          <li class:open={menuFor === calendar.id} class:hidden={!calendar.shown}>
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
                title={t(calendar.shown ? 'calendar.shownTooltip' : 'calendar.hiddenTooltip', {
                  name: calendar.name,
                })}
                aria-describedby={calendar.firstSync ? loadingId : undefined}
                {...menuOpeners(calendar)}
                onclick={() => change(() => core.setCalendarShown(calendar.id, !calendar.shown))}
              >
                <span class="calendar-dot" style:--colour={calendar.colour} aria-hidden="true"
                ></span>
                <span class="label">
                  <span class="name">{calendar.name}</span>
                  {#if calendar.firstSync}
                    <!-- The switch's description, not part of its name. -->
                    <span class="loading" id={loadingId} aria-hidden="true">
                      {t('calendar.firstSync')}
                    </span>
                  {/if}
                </span>
              </button>
              <button
                type="button"
                class="icon-button more"
                aria-label={optionsLabel}
                title={optionsLabel}
                aria-haspopup="menu"
                aria-expanded={menuFor === calendar.id}
                bind:this={moreButtons[calendar.id]}
                {...menuOpeners(calendar)}
                onclick={() =>
                  menuFor === calendar.id ? closeMenu(calendar.id) : openMenu(calendar)}
              >
                <EllipsisVertical size={14} aria-hidden="true" />
              </button>
            {/if}

            {#if menuFor === calendar.id}
              {@const colourLabelId = `sidebar-colour-${calendar.id}`}
              <Menu
                label={optionsLabel}
                anchor={moreButtons[calendar.id]}
                onclose={() => closeMenu(calendar.id)}
              >
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
                    choose(calendar.id, () => change(() => core.showOnlyCalendar(calendar.id)))}
                >
                  <Eye size={14} aria-hidden="true" />
                  {t('calendar.showOnly')}
                </button>
                <hr class="menu-separator" />
                {#if fromServer(calendar)}
                  <button
                    type="button"
                    class="menu-item danger"
                    role="menuitem"
                    onclick={() =>
                      choose(calendar.id, () => (editing = { kind: 'confirmingRemove', calendar }))}
                  >
                    <CircleMinus size={14} aria-hidden="true" />
                    {t('calendar.remove')}
                  </button>
                {:else}
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
                {/if}
              </Menu>
            {/if}
          </li>
        {/each}

        {#if editing?.kind === 'creating' && editing.accountId === account.id}
          <li>
            {@render nameForm('', newCalendarColour(calendars), (name) => create(account.id, name))}
          </li>
        {:else if calendarsOf(account).length === 0}
          <li class="hint">
            {t(account.provider === 'local' ? 'sidebar.noCalendars' : 'sidebar.noServerCalendars')}
          </li>
        {/if}
      </ul>
    </section>
  {/each}

  <button
    type="button"
    class="button outline add-account"
    onclick={() => (editing = { kind: 'addingAccount' })}
  >
    <Plus size={14} aria-hidden="true" />
    {t('sidebar.addAccount')}
  </button>
</nav>

{#snippet confirmation(title: string, body: MessageKey, confirm: MessageKey, onconfirm: () => void)}
  <Dialog role="alertdialog" {title} onclose={cancel}>
    <p>{t(body)}</p>
    {#snippet actions()}
      <button type="button" class="button outline" data-initial-focus onclick={cancel}>
        {t('common.cancel')}
      </button>
      <button type="button" class="button danger" onclick={onconfirm}>{t(confirm)}</button>
    {/snippet}
  </Dialog>
{/snippet}

{#if editing?.kind === 'confirmingDelete'}
  {@const doomed = editing.calendar}
  {@render confirmation(
    t('calendar.deleteTitle', { name: doomed.name }),
    'calendar.deleteBody',
    'calendar.delete',
    () => remove(doomed.id),
  )}
{:else if editing?.kind === 'confirmingRemove'}
  {@const doomed = editing.calendar}
  {@render confirmation(
    t('calendar.removeTitle', { name: doomed.name }),
    'calendar.removeBody',
    'calendar.removeConfirm',
    () => removeFromFreeCal(doomed.id),
  )}
{:else if editing?.kind === 'confirmingRemoveAccount'}
  {@const doomed = editing.account}
  {@render confirmation(
    t('account.removeTitle', { account: accountName(doomed) }),
    'account.removeBody',
    'account.removeConfirm',
    () => removeAccount(doomed.id),
  )}
{:else if editing?.kind === 'addingAccount'}
  <AddAccountDialog
    {core}
    onadded={(added) => startManaging(added.account, added.available)}
    oncancel={cancel}
  />
{:else if editing?.kind === 'managing'}
  {@const managing = editing}
  <ManageCalendarsDialog
    {core}
    account={managing.account}
    accountName={accountName(managing.account)}
    calendars={calendarsOf(managing.account)}
    found={managing.found}
    failure={manageFailure}
    onsave={(removed, added) => saveCalendars(managing.account, removed, added)}
    oncancel={cancel}
  />
{/if}

<style>
  /* Its width is SidebarPane's. */
  .sidebar {
    box-sizing: border-box;
    flex: 1;
    min-width: 0;
    /* Little on the right, next to the scrollbar strip or the splitter. */
    padding: var(--space-6) var(--space-1) var(--space-6) var(--space-4);
    overflow-y: auto;
    border-right: 1px solid var(--border);
    background: var(--surface);
  }

  /* A styled scrollbar takes its own strip, shown only while the sidebar
     overflows, instead of GTK's overlay bar, which lies over the rows' ⋮
     buttons and takes their clicks. WebKitGTK ignores `scrollbar-width` and
     `scrollbar-gutter` for this. */
  .sidebar::-webkit-scrollbar {
    width: 0.5rem;
    background: transparent;
  }

  .sidebar::-webkit-scrollbar-thumb {
    border: 1px solid transparent;
    border-radius: 0.25rem;
    background: var(--border-strong) padding-box;
  }

  .sidebar::-webkit-scrollbar-thumb:hover {
    background-color: var(--foreground-faint);
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

  /* Space between Accounts only, so none is left under the last one. */
  section + section {
    margin-top: var(--space-6);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    min-height: 1.75rem;
    padding: 0 var(--space-1) 0 var(--space-4);
    border-radius: var(--radius);
  }

  li:not(.hint):hover,
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

  /* A ring when hidden. */
  li.hidden .calendar-dot {
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

  .label {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .loading {
    color: var(--foreground-muted);
    font-size: var(--text-xs);
  }

  .name,
  .loading {
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

  .hint {
    color: var(--foreground-muted);
  }

  .add-account {
    margin: var(--space-6) var(--space-4) 0;
  }

  .failure {
    margin: 0 var(--space-4) var(--space-4);
    color: var(--danger);
  }
</style>
