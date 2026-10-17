<script lang="ts">
  import { tick } from 'svelte';
  import type { CoreApi } from './core/CoreApi';
  import type {
    Account,
    Calendar,
    CalendarId,
    Event,
    EventDraft,
    EventId,
    ExistingEvents,
    ImportId,
    Occurrence,
    Settings,
    SettingsUpdate,
    SidebarState,
    Signal,
    View,
    When,
  } from './core/types';
  import { errorText, startupText, t } from './i18n';
  import AboutDialog from './lib/AboutDialog.svelte';
  import CalendarGrid from './lib/CalendarGrid.svelte';
  import { followAppearance } from './lib/colourScheme';
  import Dialog from './lib/Dialog.svelte';
  import EventDetails from './lib/EventDetails.svelte';
  import EventEditor from './lib/EventEditor.svelte';
  import ImportDialog from './lib/ImportDialog.svelte';
  import {
    dateChanged,
    goToToday,
    openDay,
    opening,
    openMonth,
    periodTitle,
    step,
    switchView,
    toggleToday,
    type Navigation,
  } from './lib/navigation';
  import { DEFAULT_SETTINGS } from './lib/settings';
  import SettingsDialog from './lib/SettingsDialog.svelte';
  import { shortcutFor, type ShortcutAction } from './lib/shortcuts';
  import Sidebar from './lib/Sidebar.svelte';
  import SidebarPane from './lib/SidebarPane.svelte';
  import StartupError from './lib/StartupError.svelte';
  import Toast from './lib/Toast.svelte';
  import Toolbar from './lib/Toolbar.svelte';
  import { whenForShortcut, type VisibleRange } from './lib/when';
  import WorkingDialog from './lib/WorkingDialog.svelte';

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
    | { kind: 'importing'; file: string; calendarId: CalendarId }
    | { kind: 'notice'; title: string; body: string }
    | { kind: 'settings' }
    | { kind: 'about' };

  let settings = $state<Settings>({ ...DEFAULT_SETTINGS });
  let settingsFailure = $state<string>();
  /** FreeCal's version, read when About first opens. */
  let version = $state<string>();
  let today = $state<Temporal.PlainDate>();
  let navigation = $state<Navigation>();
  let viewSaveFailure = $state<string>();
  let sidebar = $state<SidebarState>();
  let sidebarSaveFailure = $state<string>();
  let quitFailure = $state<string>();
  let rescheduleFailure = $state<string>();
  /**
   * Whether the user is pressing on the grid, as for a drag. A "Date
   * changed" meanwhile moves the view only once the press ends, so that the
   * grid doesn't move under the pointer.
   */
  let interacting = false;
  let dateChangeHeld = false;
  let directory = $state<Directory>();
  /** Why FreeCal couldn't start, in words, for the startup error screen. */
  let startupReason = $state<string>();
  let readFailure = $state<string>();
  let range = $state<VisibleRange>();
  let occurrences = $state<Occurrence[]>([]);
  let open = $state<Open>();
  /** The Event whose details are open, as last read from the core. */
  let opened = $state<Event>();
  /** Why an action in the open details or About dialog failed. */
  let dialogFailure = $state<string>();
  /** Counts editor sessions, so that each one starts from its own draft. */
  let session = $state(0);
  /** Why the last import from the open import dialog failed. */
  let importFailure = $state<string>();
  /** What the modal "Working…" dialog says while a long command runs. */
  let working = $state<string>();
  /** The message at the bottom of the window, with its Undo if it has one. */
  let toast = $state<{ message: string; undo?: () => void }>();

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
    readFailure = errorText(error);
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
        followToday();
        break;
    }
  }

  /** Moves the view to a new today if it tracks it and nothing holds it back. */
  function followToday() {
    if (interacting) {
      dateChangeHeld = true;
    } else if (navigation && today) {
      navigation = dateChanged(navigation, today);
    }
  }

  function oninteract(active: boolean) {
    interacting = active;
    if (!active && dateChangeHeld) {
      dateChangeHeld = false;
      followToday();
    }
  }

  $effect(() => followAppearance(settings.appearance));

  /**
   * Shows the window, which starts hidden. Should that fail, the shell shows
   * it a few seconds later anyway.
   */
  function showWindow() {
    core.showWindow().catch(() => {});
  }

  $effect(() => {
    let stopped = false;
    let unsubscribe: (() => void) | undefined;

    async function start() {
      const failure = await core.startupFailure();
      if (failure) {
        startupReason = startupText(failure);
        showWindow();
        return;
      }
      // Subscribe first, then read, so that no change falls in the gap.
      const stop = await core.subscribe((signal) => {
        if (!stopped) onSignal(signal);
      });
      if (stopped) return stop();
      unsubscribe = stop;
      // The window shows once its appearance is known, so that it never
      // flashes the wrong theme.
      settings = await core.settings();
      await tick();
      showWindow();
      const [currentDate, view, sidebarState] = await Promise.all([
        core.today(),
        core.lastUsedView(),
        core.sidebar(),
        readDirectory(),
      ]);
      // A "Date changed" that came in meanwhile is newer.
      today ??= currentDate;
      sidebar = sidebarState;
      navigation = opening(view, today);
    }

    start().catch((error) => {
      // Drawn with the default settings.
      startupReason = t('app.loadFailed', { error: errorText(error) });
      showWindow();
    });
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
        (error) => (viewSaveFailure = t('app.viewSaveFailed', { error: errorText(error) })),
      );
    }
    navigation = next;
  }

  /** Shows the sidebar as `next` says and remembers it straight away. */
  function changeSidebar(next: SidebarState) {
    sidebar = next;
    core.setSidebar(next).then(
      () => (sidebarSaveFailure = undefined),
      (error) => (sidebarSaveFailure = t('app.sidebarSaveFailed', { error: errorText(error) })),
    );
  }

  function toggleSidebar() {
    changeSidebar({ ...sidebar!, shown: !sidebar!.shown });
  }

  function onrangechange(visible: VisibleRange) {
    range = visible;
    readOccurrences().catch(reportRead);
  }

  /** Opens the editor for a new Event, or says where to create a Calendar first. */
  function startCreating(when: When) {
    const target = defaultCalendar();
    if (!target) {
      open = {
        kind: 'notice',
        title: t('event.noCalendarTitle'),
        body: t('event.noCalendarBody', { account: t('account.local') }),
      };
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
    dialogFailure = undefined;
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
        location: event.location.text,
        description: event.description.text,
      },
    };
  }

  function close() {
    open = undefined;
    opened = undefined;
    dialogFailure = undefined;
  }

  async function saveNew(draft: EventDraft) {
    await core.createEvent(draft);
    close();
  }

  async function saveEdited(id: EventId, draft: EventDraft) {
    await core.editEvent(id, draft);
    openDetails(id);
  }

  /** A drag or resize in the grid; the grid puts the Event back if this rejects. */
  async function reschedule(id: EventId, when: When) {
    try {
      await core.rescheduleEvent(id, when);
      rescheduleFailure = undefined;
    } catch (error) {
      rescheduleFailure = t('event.rescheduleFailed', { error: errorText(error) });
      throw error;
    }
  }

  async function remove(id: EventId) {
    try {
      await core.deleteEvent(id);
      close();
    } catch (error) {
      dialogFailure = t('event.deleteFailed', { error: errorText(error) });
    }
  }

  async function openLink(url: string) {
    try {
      await core.openLink(url);
    } catch (error) {
      dialogFailure = t('link.openFailed', { error: errorText(error) });
    }
  }

  /** The Calendar new things go into: the first shown writable one. */
  function defaultCalendar(): Calendar | undefined {
    return writableCalendars.find((calendar) => calendar.shown) ?? writableCalendars[0];
  }

  /** Asks for a file to import, then opens the import dialog for it. */
  async function startImport() {
    if (!defaultCalendar()) {
      open = {
        kind: 'notice',
        title: t('import.noCalendarTitle'),
        body: t('event.noCalendarBody', { account: t('account.local') }),
      };
      return;
    }
    let file: string | null;
    try {
      file = await core.pickImportFile(t('import.fileDialogTitle'), t('import.fileFilter'));
    } catch (error) {
      open = {
        kind: 'notice',
        title: t('import.pickFailedTitle'),
        body: t('import.pickFailed', { error: errorText(error) }),
      };
      return;
    }
    if (file === null) return;
    const target = defaultCalendar();
    // The user may have opened something else while the file dialog was showing.
    if (open || !target) {
      cancelImport();
      return;
    }
    importFailure = undefined;
    open = { kind: 'importing', file, calendarId: target.id };
  }

  function cancelImport() {
    // Nothing closes the dialog while its import runs, not even an Escape
    // that reaches it under "Working…".
    if (working) return;
    core.discardImport().catch(() => {});
    if (open?.kind === 'importing') close();
  }

  async function runImport(file: string, calendarId: CalendarId, existing: ExistingEvents) {
    working = t('working.importing', { file });
    try {
      const outcome = await core.runImport(calendarId, existing);
      close();
      toast = {
        message: t('import.done', {
          file,
          imported: String(outcome.imported),
          updated: String(outcome.updated),
          skipped: String(outcome.skipped),
        }),
        // An import that changed nothing has nothing to undo.
        undo:
          outcome.imported + outcome.updated > 0 ? () => undoImport(file, outcome.id) : undefined,
      };
    } catch (error) {
      importFailure = t('import.failed', { error: errorText(error) });
    } finally {
      working = undefined;
    }
  }

  async function undoImport(file: string, id: ImportId) {
    toast = undefined;
    working = t('working.undoingImport', { file });
    try {
      await core.undoImport(id);
      toast = { message: t('import.undone', { file }) };
    } catch (error) {
      // Undo can succeed later, such as once the Calendar is writable again.
      toast = {
        message: t('import.undoFailed', { file, error: errorText(error) }),
        undo: () => undoImport(file, id),
      };
    } finally {
      working = undefined;
    }
  }

  /** Writes one changed setting; it has taken effect before the core answers. */
  function changeSettings(update: SettingsUpdate) {
    settings = { ...settings, ...update };
    core.updateSettings(update).then(
      () => (settingsFailure = undefined),
      (error) => {
        settingsFailure = t('settings.saveFailed', { error: errorText(error) });
        // Back to what the Local Store kept, so that the window doesn't show
        // a setting it won't open with next time.
        core.settings().then(
          (stored) => (settings = stored),
          () => {},
        );
      },
    );
  }

  function openSettings() {
    settingsFailure = undefined;
    open = { kind: 'settings' };
  }

  function openAbout() {
    dialogFailure = undefined;
    open = { kind: 'about' };
    // Without the version About still says everything else.
    if (version === undefined)
      core.version().then(
        (read) => (version = read),
        () => {},
      );
  }

  function quit() {
    core.quit().catch((error) => (quitFailure = t('app.quitFailed', { error: errorText(error) })));
  }

  /** What each keyboard shortcut does (see `shortcuts.ts` for the keys). */
  const shortcutActions: Record<ShortcutAction, () => void> = {
    create: () => startCreating(whenForShortcut(Temporal.Now.plainDateTimeISO())),
    day: () => showView('day'),
    week: () => showView('week'),
    month: () => showView('month'),
    next: () => navigate(step(navigation!, 1)),
    previous: () => navigate(step(navigation!, -1)),
    // Unlike the Today button, `t` doesn't toggle: it always goes to today.
    today: () => navigate(goToToday(navigation!, today!)),
    settings: openSettings,
    sidebar: toggleSidebar,
  };

  function showView(view: View) {
    navigate(switchView(navigation!, view));
  }

  /**
   * Shortcuts act unless a dialog or menu is open. Those without Ctrl also
   * wait while the user is typing.
   */
  function onkeydown(key: KeyboardEvent) {
    const shortcut = shortcutFor(key);
    if (!shortcut) return;
    if (!navigation || open || document.querySelector('dialog[open]')) return;
    const target = key.target;
    const typing = 'input, textarea, select, [contenteditable]';
    if (
      target instanceof Element &&
      target.closest(shortcut.ctrl ? '[role="menu"]' : `${typing}, [role="menu"]`)
    ) {
      return;
    }
    key.preventDefault();
    shortcutActions[shortcut.action]();
  }
</script>

<svelte:window {onkeydown} />

{#if startupReason}
  <StartupError reason={startupReason} onquit={quit} />
{:else if today && navigation && directory && sidebar}
  {@const shown = navigation}
  {@const sidebarState = sidebar}
  <div class="app">
    <SidebarPane sidebar={sidebarState} onchange={changeSidebar}>
      <Sidebar {core} accounts={directory.accounts} calendars={directory.calendars} />
    </SidebarPane>
    <main class="grid">
      <Toolbar
        view={shown.view}
        tracking={shown.tracking}
        title={periodTitle(shown)}
        ontoday={() => navigate(toggleToday(shown, today!))}
        onstep={(by) => navigate(step(shown, by))}
        onview={(view) => navigate(switchView(shown, view))}
        onmonth={(month) => navigate(openMonth(shown, month))}
        onimport={startImport}
        onsettings={openSettings}
        onabout={openAbout}
        onquit={quit}
      />
      {#if readFailure}
        <p role="alert">{t('app.readFailed', { error: readFailure })}</p>
      {/if}
      {#if viewSaveFailure}
        <p role="alert">{viewSaveFailure}</p>
      {/if}
      {#if sidebarSaveFailure}
        <p role="alert">{sidebarSaveFailure}</p>
      {/if}
      {#if quitFailure}
        <p role="alert">{quitFailure}</p>
      {/if}
      {#if rescheduleFailure}
        <p role="alert">{rescheduleFailure}</p>
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
        onreschedule={reschedule}
        {oninteract}
      />
    </main>
  </div>

  {#if open?.kind === 'details' && opened}
    {@const event = opened}
    <EventDetails
      {event}
      calendar={directory.calendars.find((calendar) => calendar.id === event.calendarId)}
      failure={dialogFailure}
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
  {:else if open?.kind === 'settings'}
    <SettingsDialog
      {settings}
      failure={settingsFailure}
      onchange={changeSettings}
      onclose={close}
    />
  {:else if open?.kind === 'about'}
    <AboutDialog {version} failure={dialogFailure} onopenlink={openLink} onclose={close} />
  {:else if open?.kind === 'importing'}
    {@const importing = open}
    <ImportDialog
      {core}
      file={importing.file}
      calendars={writableCalendars}
      initialCalendarId={importing.calendarId}
      failure={importFailure}
      onimport={(calendarId, existing) => runImport(importing.file, calendarId, existing)}
      oncancel={cancelImport}
    />
  {:else if open?.kind === 'notice'}
    <Dialog title={open.title} onclose={close}>
      <p>{open.body}</p>
      {#snippet actions()}
        <button type="button" class="button solid" data-initial-focus onclick={close}>
          {t('common.ok')}
        </button>
      {/snippet}
    </Dialog>
  {/if}

  {#if toast}
    {@const shown = toast}
    <Toast
      message={shown.message}
      action={shown.undo && { label: t('import.undo'), run: shown.undo }}
      ondismiss={() => (toast = undefined)}
    />
  {/if}
  {#if working}
    <WorkingDialog message={working} />
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
