<script lang="ts">
  import type { CoreApi } from './core/CoreApi';
  import type {
    Account,
    Calendar,
    Event,
    EventDraft,
    EventId,
    Occurrence,
    Signal,
    View,
    When,
  } from './core/types';
  import { t } from './i18n';
  import CalendarGrid from './lib/CalendarGrid.svelte';
  import Dialog from './lib/Dialog.svelte';
  import EventDetails from './lib/EventDetails.svelte';
  import EventEditor from './lib/EventEditor.svelte';
  import {
    dateChanged,
    goToToday,
    openDay,
    opening,
    periodTitle,
    step,
    switchView,
    toggleToday,
    type Navigation,
  } from './lib/navigation';
  import Sidebar from './lib/Sidebar.svelte';
  import Toolbar from './lib/Toolbar.svelte';
  import { whenForShortcut, type VisibleRange } from './lib/when';

  interface Props {
    core: CoreApi;
  }

  let { core }: Props = $props();

  interface Directory {
    accounts: Account[];
    calendars: Calendar[];
  }

  /**
   * What is open over the grid, if anything. The Event of `details` and
   * `editing` is the selected one, which the grid marks.
   */
  type Open =
    | { kind: 'details'; id: EventId }
    | { kind: 'editing'; id: EventId; draft: EventDraft }
    | { kind: 'creating'; draft: EventDraft }
    | { kind: 'noCalendar' };

  let today = $state<Temporal.PlainDate>();
  let navigation = $state<Navigation>();
  let viewSaveFailure = $state<string>();
  let directory = $state<Directory>();
  let startFailure = $state<string>();
  let readFailure = $state<string>();
  let range = $state<VisibleRange>();
  let occurrences = $state<Occurrence[]>([]);
  let open = $state<Open>();
  /** The Event whose details are open, as last read from the core. */
  let opened = $state<Event>();
  let detailsFailure = $state<string>();
  /** Counts editor sessions, so that each one starts from its own draft. */
  let session = $state(0);

  const writableCalendars = $derived(
    directory?.calendars.filter((calendar) => !calendar.readOnly) ?? [],
  );
  const selectedEventId = $derived(
    open?.kind === 'details' || open?.kind === 'editing' ? open.id : undefined,
  );

  // Reads can overtake each other; only the latest one of each kind may land.
  let latestDirectoryRead = 0;
  let latestOccurrencesRead = 0;
  let latestEventRead = 0;

  async function readDirectory() {
    const read = ++latestDirectoryRead;
    const [accounts, calendars] = await Promise.all([core.listAccounts(), core.listCalendars()]);
    if (read === latestDirectoryRead) {
      directory = { accounts, calendars };
      readFailure = undefined;
    }
  }

  async function readOccurrences() {
    if (!range) return;
    const read = ++latestOccurrencesRead;
    const found = await core.listOccurrences(range.from, range.to);
    if (read === latestOccurrencesRead) occurrences = found;
  }

  /** Reads the Event whose details are open again; closes them if it is gone. */
  async function readOpenedEvent() {
    if (open?.kind !== 'details') return;
    const id = open.id;
    const read = ++latestEventRead;
    try {
      const event = await core.event(id);
      if (read === latestEventRead && open?.kind === 'details' && open.id === id) opened = event;
    } catch {
      if (read === latestEventRead && open?.kind === 'details' && open.id === id) close();
    }
  }

  function reportRead(error: unknown) {
    readFailure = String(error);
  }

  /** Everything shown depends on the Calendars; re-reads after any change. */
  function refreshAll() {
    readDirectory().catch(reportRead);
    readOccurrences().catch(reportRead);
    readOpenedEvent();
  }

  function onSignal(signal: Signal) {
    switch (signal.kind) {
      case 'calendarsChanged':
        refreshAll();
        break;
      case 'occurrencesChanged':
        // The open details and the grid show the new state; an open editor
        // keeps its draft.
        readOccurrences().catch(reportRead);
        readOpenedEvent();
        break;
      case 'dateChanged':
        today = Temporal.PlainDate.from(signal.date);
        if (navigation) navigation = dateChanged(navigation, today);
        break;
    }
  }

  $effect(() => {
    let stopped = false;
    let unsubscribe: (() => void) | undefined;

    async function start() {
      // Subscribe first, then read, so that no change falls in the gap.
      const stop = await core.subscribe((signal) => {
        if (!stopped) onSignal(signal);
      });
      if (stopped) return stop();
      unsubscribe = stop;
      const [currentDate, view] = await Promise.all([
        core.today(),
        core.lastUsedView(),
        readDirectory(),
      ]);
      // A "Date changed" that came in meanwhile is newer.
      today ??= currentDate;
      navigation = opening(view, today);
    }

    start().catch((error) => (startFailure = String(error)));
    return () => {
      stopped = true;
      unsubscribe?.();
    };
  });

  /** Shows what `next` says, remembering a changed view as the last-used one. */
  function navigate(next: Navigation) {
    if (next.view !== navigation?.view) {
      core.setLastUsedView(next.view).then(
        () => (viewSaveFailure = undefined),
        (error) => (viewSaveFailure = t('app.viewSaveFailed', { error: String(error) })),
      );
    }
    navigation = next;
  }

  function onrangechange(visible: VisibleRange) {
    range = visible;
    readOccurrences().catch(reportRead);
  }

  /** Opens the editor for a new Event, or says where to create a Calendar first. */
  function startCreating(when: When) {
    const target =
      writableCalendars.find((calendar) => calendar.shown) ?? writableCalendars[0];
    if (!target) {
      open = { kind: 'noCalendar' };
      return;
    }
    session++;
    open = {
      kind: 'creating',
      draft: { calendarId: target.id, title: '', when, location: '', description: '' },
    };
  }

  function openDetails(id: EventId) {
    opened = undefined;
    detailsFailure = undefined;
    open = { kind: 'details', id };
    readOpenedEvent();
  }

  function startEditing(event: Event) {
    session++;
    open = {
      kind: 'editing',
      id: event.id,
      draft: {
        calendarId: event.calendarId,
        title: event.title,
        when: event.when,
        location: event.location,
        description: event.description.text,
      },
    };
  }

  function close() {
    open = undefined;
    opened = undefined;
    detailsFailure = undefined;
  }

  async function saveNew(draft: EventDraft) {
    await core.createEvent(draft);
    close();
  }

  async function saveEdited(id: EventId, draft: EventDraft) {
    await core.editEvent(id, draft);
    openDetails(id);
  }

  async function remove(id: EventId) {
    try {
      await core.deleteEvent(id);
      close();
    } catch (error) {
      detailsFailure = t('event.deleteFailed', { error: String(error) });
    }
  }

  async function openLink(url: string) {
    try {
      await core.openLink(url);
    } catch (error) {
      detailsFailure = t('link.openFailed', { error: String(error) });
    }
  }

  /** Google Calendar's shortcuts, which act on the grid. */
  const shortcuts: Record<string, () => void> = {
    c: () => startCreating(whenForShortcut(Temporal.Now.plainDateTimeISO())),
    d: () => showView('day'),
    '1': () => showView('day'),
    w: () => showView('week'),
    '2': () => showView('week'),
    m: () => showView('month'),
    '3': () => showView('month'),
    j: () => navigate(step(navigation!, 1)),
    n: () => navigate(step(navigation!, 1)),
    k: () => navigate(step(navigation!, -1)),
    p: () => navigate(step(navigation!, -1)),
    // Unlike the Today button, `t` doesn't toggle: it always goes to today.
    t: () => navigate(goToToday(navigation!, today!)),
  };

  function showView(view: View) {
    navigate(switchView(navigation!, view));
  }

  /** Shortcuts act unless the user is typing or a dialog or menu is open. */
  function onkeydown(key: KeyboardEvent) {
    const shortcut = Object.hasOwn(shortcuts, key.key) ? shortcuts[key.key] : undefined;
    if (!shortcut || key.ctrlKey || key.altKey || key.metaKey || key.repeat) return;
    if (!navigation || open || document.querySelector('dialog[open]')) return;
    const target = key.target;
    if (
      target instanceof Element &&
      target.closest('input, textarea, select, [contenteditable], [role="menu"]')
    ) {
      return;
    }
    key.preventDefault();
    shortcut();
  }
</script>

<svelte:window {onkeydown} />

{#if startFailure}
  <p role="alert">{t('app.loadFailed', { error: startFailure })}</p>
{:else if today && navigation && directory}
  {@const shown = navigation}
  <div class="app">
    <Sidebar {core} accounts={directory.accounts} calendars={directory.calendars} />
    <main class="grid">
      <Toolbar
        view={shown.view}
        tracking={shown.tracking}
        title={periodTitle(shown)}
        ontoday={() => navigate(toggleToday(shown, today!))}
        onstep={(by) => navigate(step(shown, by))}
        onview={(view) => navigate(switchView(shown, view))}
      />
      {#if readFailure}
        <p role="alert">{t('app.readFailed', { error: readFailure })}</p>
      {/if}
      {#if viewSaveFailure}
        <p role="alert">{viewSaveFailure}</p>
      {/if}
      <CalendarGrid
        view={shown.view}
        date={shown.date}
        {today}
        {occurrences}
        calendars={directory.calendars}
        {selectedEventId}
        {onrangechange}
        onselectslot={startCreating}
        onopenevent={openDetails}
        onopenday={(date) => navigate(openDay(date))}
      />
    </main>
  </div>

  {#if open?.kind === 'details' && opened}
    {@const event = opened}
    <EventDetails
      {event}
      calendar={directory.calendars.find((calendar) => calendar.id === event.calendarId)}
      failure={detailsFailure}
      onedit={() => startEditing(event)}
      ondelete={() => remove(event.id)}
      onclose={close}
      onopenlink={openLink}
    />
  {:else if open?.kind === 'creating' || open?.kind === 'editing'}
    {@const editing = open}
    {#key session}
      <EventEditor
        mode={editing.kind === 'creating' ? 'create' : 'edit'}
        initial={editing.draft}
        calendars={writableCalendars}
        onsave={(draft) =>
          editing.kind === 'editing' ? saveEdited(editing.id, draft) : saveNew(draft)}
        oncancel={() => (editing.kind === 'editing' ? openDetails(editing.id) : close())}
      />
    {/key}
  {:else if open?.kind === 'noCalendar'}
    <Dialog title={t('event.noCalendarTitle')} onclose={close}>
      <p>{t('event.noCalendarBody', { account: t('account.local') })}</p>
      {#snippet actions()}
        <button type="button" class="button solid" data-initial-focus onclick={close}>
          {t('common.ok')}
        </button>
      {/snippet}
    </Dialog>
  {/if}
{:else}
  <p>{t('app.loading')}</p>
{/if}

<style>
  .app {
    display: flex;
    height: 100vh;
    /* The window's minimum size from `tauri.conf.json`. KWin on X11 lets a
       tiled window get smaller than that; the page then scrolls rather than
       squeezing the layout (ticket 39). */
    min-width: 880px;
    min-height: 560px;
  }

  .grid {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
    padding: var(--space-6);
  }
</style>
