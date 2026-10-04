<script lang="ts">
  import type { CoreApi } from '../core/CoreApi';
  import type { Account, AccountId, Calendar, CalendarId } from '../core/types';
  import { t } from '../i18n';

  interface Props {
    core: CoreApi;
    accounts: Account[];
    calendars: Calendar[];
  }

  let { core, accounts, calendars }: Props = $props();

  /** New Calendars take these colours in turn; the user can recolour them. */
  const NEW_CALENDAR_COLOURS = [
    '#3366cc',
    '#dc3912',
    '#ff9900',
    '#109618',
    '#990099',
    '#0099c6',
    '#dd4477',
    '#66aa00',
  ];

  /** What the user is in the middle of, if anything. Only one at a time. */
  type Editing =
    | { kind: 'creating'; accountId: AccountId }
    | { kind: 'renaming'; id: CalendarId }
    | { kind: 'confirmingDelete'; id: CalendarId };

  let editing = $state<Editing>();
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
    const colour = NEW_CALENDAR_COLOURS[calendars.length % NEW_CALENDAR_COLOURS.length];
    if (await change(() => core.createCalendar(accountId, name, colour))) editing = undefined;
  }

  async function rename(id: CalendarId, name: string) {
    if (await change(() => core.renameCalendar(id, name))) editing = undefined;
  }

  async function remove(id: CalendarId) {
    if (await change(() => core.deleteCalendar(id))) editing = undefined;
  }

  async function setShown(calendar: Calendar, input: HTMLInputElement) {
    if (!(await change(() => core.setCalendarShown(calendar.id, input.checked)))) {
      input.checked = calendar.shown;
    }
  }

  async function recolour(calendar: Calendar, input: HTMLInputElement) {
    if (!(await change(() => core.recolourCalendar(calendar.id, input.value)))) {
      input.value = calendar.colour;
    }
  }

  /** Stops what the user is in the middle of, with its report if it failed. */
  function cancel() {
    editing = undefined;
    failure = undefined;
  }

  function focus(element: HTMLElement) {
    element.focus();
    if (element instanceof HTMLInputElement) element.select();
  }
</script>

{#snippet nameForm(initialName: string, save: (name: string) => void)}
  <form
    class="name-form"
    onsubmit={(submitted) => {
      submitted.preventDefault();
      const name = new FormData(submitted.currentTarget).get('name');
      save(String(name));
    }}
  >
    <input
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
      <h2 id={headingId}>{accountName(account)}</h2>
      <ul aria-labelledby={headingId}>
        {#each calendarsOf(account) as calendar (calendar.id)}
          <li>
            {#if editing?.kind === 'renaming' && editing.id === calendar.id}
              {@render nameForm(calendar.name, (name) => rename(calendar.id, name))}
            {:else}
              <label class="calendar">
                <input
                  type="checkbox"
                  checked={calendar.shown}
                  style:accent-color={calendar.colour}
                  onchange={(changed) => setShown(calendar, changed.currentTarget)}
                />
                <span class="name">{calendar.name}</span>
              </label>
              <span class="actions">
                <input
                  type="color"
                  aria-label={t('calendar.colourOf', { name: calendar.name })}
                  title={t('calendar.colourOf', { name: calendar.name })}
                  value={calendar.colour}
                  onchange={(changed) => recolour(calendar, changed.currentTarget)}
                />
                <button
                  type="button"
                  aria-label={t('calendar.renameNamed', { name: calendar.name })}
                  title={t('calendar.renameNamed', { name: calendar.name })}
                  onclick={() => (editing = { kind: 'renaming', id: calendar.id })}
                >
                  <span aria-hidden="true">✎</span>
                </button>
                <button
                  type="button"
                  aria-label={t('calendar.deleteNamed', { name: calendar.name })}
                  title={t('calendar.deleteNamed', { name: calendar.name })}
                  onclick={() => (editing = { kind: 'confirmingDelete', id: calendar.id })}
                >
                  <span aria-hidden="true">✕</span>
                </button>
              </span>
            {/if}

            {#if editing?.kind === 'confirmingDelete' && editing.id === calendar.id}
              {@const questionId = `sidebar-delete-${calendar.id}`}
              <div class="confirm" role="alertdialog" aria-labelledby={questionId}>
                <p id={questionId}>{t('calendar.deleteConfirm', { name: calendar.name })}</p>
                <button type="button" onclick={() => remove(calendar.id)}>
                  {t('calendar.delete')}
                </button>
                <button type="button" onclick={cancel} {@attach focus}>
                  {t('common.cancel')}
                </button>
              </div>
            {/if}
          </li>
        {/each}
      </ul>

      {#if account.provider === 'local'}
        {#if editing?.kind === 'creating' && editing.accountId === account.id}
          {@render nameForm('', (name) => create(account.id, name))}
        {:else}
          <button
            type="button"
            class="new-calendar"
            onclick={() => (editing = { kind: 'creating', accountId: account.id })}
          >
            {t('sidebar.newCalendar')}
          </button>
        {/if}
      {/if}
    </section>
  {/each}
</nav>

<style>
  .sidebar {
    width: 14rem;
    padding: 0.75rem;
    overflow-y: auto;
    border-right: 1px solid #ddd;
  }

  h2 {
    margin: 0 0 0.25rem;
    font-size: 0.9rem;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.25rem;
    min-height: 1.75rem;
  }

  .calendar {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 0.4rem;
    min-width: 0;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* The row's actions appear on hover and keyboard focus. Opacity, not
     visibility, so that they stay reachable by keyboard and screen readers. */
  .actions {
    display: flex;
    align-items: center;
    gap: 0.1rem;
    opacity: 0;
  }

  li:hover .actions,
  li:focus-within .actions {
    opacity: 1;
  }

  .actions button {
    border: none;
    background: none;
    padding: 0 0.2rem;
    cursor: pointer;
  }

  input[type='color'] {
    width: 1.25rem;
    height: 1.25rem;
    padding: 0;
    border: none;
    background: none;
    cursor: pointer;
  }

  .name-form {
    flex: 1;
    margin: 0.25rem 0;
  }

  .name-form input {
    box-sizing: border-box;
    width: 100%;
  }

  .confirm {
    flex-basis: 100%;
    margin-bottom: 0.5rem;
  }

  .confirm p {
    margin: 0.25rem 0;
  }

  .new-calendar {
    margin-top: 0.25rem;
  }

  .failure {
    margin: 0 0 0.5rem;
    color: #b00020;
  }
</style>
