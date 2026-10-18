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
    Occurrence,
    Scope,
    Settings,
    SettingsUpdate,
    SidebarState,
    Signal,
    UnitId,
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
  import { sameRepeat } from './lib/repeat';
  import ScopeDialog, { type ScopeChange } from './lib/ScopeDialog.svelte';
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
   * `editing` is the selected one, which the grid marks. `recurrenceId` says
   * which Occurrence of a Recurring Event was opened, none for the whole.
   */
  type Open =
    | { kind: 'details'; id: EventId; recurrenceId: string | null }
    | {
        kind: 'editing';
        id: EventId;
        recurrenceId: string | null;
        draft: EventDraft;
        customRepeat: boolean;
        repeatable: boolean;
      }
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
  /**
   * The open question which Occurrences of a Recurring Event a change
   * changes, over the editor or, for a drop, the grid.
   */
  let scopePrompt = $state<{ change: ScopeChange; choose: (scope: Scope | null) => void }>();
  /** The message at the bottom of the window, with its Undo if it has one. */
  let toast = $state<{ message: string; undo?: () => void }>();

  const writableCalendars = $derived(
    directory?.calendars.filter((calendar) => !calendar.readOnly) ?? [],
  );
  const selectedEventId = $derived(
    open?.kind === 'details' || open?.kind === 'editing' ? open.id : undefined,
  );
  const selectedRecurrenceId = $derived(
    open?.kind === 'details' || open?.kind === 'editing'
      ? (open.recurrenceId ?? undefined)
      : undefined,
  );
  /** The Occurrence of a Recurring Event whose details are open, as the grid shows it. */
  const openedOccurrence = $derived(
    open?.kind === 'details' && open.recurrenceId !== null
      ? occurrences.find(
          (occurrence) =>
            occurrence.eventId === selectedEventId &&
            occurrence.recurrenceId === selectedRecurrenceId,
        )
      : undefined,
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

  /**
   * Whether the grid holds back a new today for a dropped Occurrence while
   * the user chooses its recurrence scope, as it does during the drag.
   */
  const dropHeld = $derived(scopePrompt?.change === 'move');

  /** Moves the view to a new today if it tracks it and nothing holds it back. */
  function followToday() {
    if (interacting || dropHeld) {
      dateChangeHeld = true;
    } else if (navigation && today) {
      navigation = dateChanged(navigation, today);
    }
  }

  function oninteract(active: boolean) {
    interacting = active;
    followHeldDateChange();
  }

  /** Follows a "Date changed" that was held back, once nothing holds it any more. */
  function followHeldDateChange() {
    if (dateChangeHeld && !interacting && !dropHeld) {
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
      draft: {
        calendarId: target.id,
        title: '',
        when,
        location: '',
        description: '',
        repeat: null,
      },
    };
  }

  function openDetails(id: EventId, recurrenceId: string | null = null) {
    opened = undefined;
    dialogFailure = undefined;
    open = { kind: 'details', id, recurrenceId };
    readOpenedEvent();
  }

  /**
   * Edits the Event; for an Occurrence of a Recurring Event, that
   * Occurrence, and Save asks which Occurrences the change is for.
   */
  function startEditing(event: Event, recurrenceId: string | null, when: When) {
    session++;
    open = {
      kind: 'editing',
      id: event.id,
      recurrenceId,
      draft: {
        calendarId: event.calendarId,
        title: event.title,
        when,
        location: event.location.text,
        description: event.description.text,
        repeat: event.repeat,
      },
      customRepeat: event.customRepeat,
      // An Exception repeats with its Recurring Event, if that is still there.
      repeatable: event.recurrenceId === null || event.repeat !== null,
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

  /**
   * Asks which Occurrences of a Recurring Event a change changes. Resolves
   * to `null` when the user cancels.
   */
  function askScope(change: ScopeChange): Promise<Scope | null> {
    return new Promise((resolve) => {
      scopePrompt = {
        change,
        choose: (scope) => {
          scopePrompt = undefined;
          followHeldDateChange();
          resolve(scope);
        },
      };
    });
  }

  /**
   * Saves an edit and returns to the details of the Occurrence it changed.
   * For an Occurrence of a Recurring Event it asks which Occurrences the
   * change is for first; cancelling that leaves the editor open.
   */
  async function saveEdited(
    editing: { id: EventId; recurrenceId: string | null; draft: EventDraft },
    draft: EventDraft,
  ) {
    const { id, recurrenceId } = editing;
    if (recurrenceId === null) {
      const { value } = await core.editEvent(id, draft);
      openDetails(id, value.recurrenceId);
      return;
    }
    const scope = await askScope(
      sameRepeat(draft.repeat, editing.draft.repeat) ? 'edit' : 'editRepeat',
    );
    if (scope === null) return;
    const { value } = await core.editOccurrence(id, recurrenceId, scope, draft);
    openDetails(value.eventId, value.recurrenceId);
  }

  /**
   * A drag or resize in the grid; for an Occurrence of a Recurring Event it
   * asks which Occurrences move first. The grid puts the Occurrence back if
   * this rejects, as when the user cancels.
   */
  async function reschedule(id: EventId, recurrenceId: string | null, when: When) {
    const scope = recurrenceId === null ? null : await askScope('move');
    if (recurrenceId !== null && scope === null) throw new Error('The move was cancelled.');
    try {
      if (recurrenceId === null) await core.rescheduleEvent(id, when);
      else await core.rescheduleOccurrence(id, recurrenceId, scope!, when);
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

  async function removeOccurrence(id: EventId, recurrenceId: string) {
    try {
      await core.deleteOccurrence(id, recurrenceId);
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
      const { unit } = outcome;
      close();
      toast = {
        message: t('import.done', {
          file,
          imported: String(outcome.imported),
          updated: String(outcome.updated),
          skipped: String(outcome.skipped),
        }),
        // An import that changed nothing has no unit to undo.
        undo: unit === null ? undefined : () => undoImport(file, unit),
      };
    } catch (error) {
      importFailure = t('import.failed', { error: errorText(error) });
    } finally {
      working = undefined;
    }
  }

  async function undoImport(file: string, id: UnitId) {
    toast = undefined;
    working = t('working.undoingImport', { file });
    try {
      await core.undoUnit(id);
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
        {selectedRecurrenceId}
        {onrangechange}
        onselectslot={startCreating}
        onopenevent={openDetails}
        onopenday={(date) => navigate(openDay(date))}
        onreschedule={reschedule}
        {oninteract}
        held={dropHeld}
      />
    </main>
  </div>

  {#if open?.kind === 'details' && opened}
    {@const event = opened}
    {@const recurrenceId = open.recurrenceId}
    <EventDetails
      {event}
      calendar={directory.calendars.find((calendar) => calendar.id === event.calendarId)}
      when={openedOccurrence?.when}
      failure={dialogFailure}
      onedit={() => startEditing(event, recurrenceId, openedOccurrence?.when ?? event.when)}
      ondelete={() => remove(event.id)}
      ondeleteoccurrence={recurrenceId === null
        ? undefined
        : () => removeOccurrence(event.id, recurrenceId)}
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
        customRepeat={editing.kind === 'editing' && editing.customRepeat}
        repeatable={editing.kind === 'creating' || editing.repeatable}
        onsave={(draft) =>
          editing.kind === 'editing' ? saveEdited(editing, draft) : saveNew(draft)}
        oncancel={() =>
          editing.kind === 'editing' ? openDetails(editing.id, editing.recurrenceId) : close()}
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

  {#if scopePrompt}
    <ScopeDialog change={scopePrompt.change} onchoose={scopePrompt.choose} />
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
