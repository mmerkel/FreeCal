<script lang="ts">
  import type { CoreApi } from './core/CoreApi';
  import type {
    Calendar,
    CalendarId,
    Event,
    EventDraft,
    EventId,
    ExceptionCounts,
    ExistingEvents,
    JournalTransactionId,
    View,
    When,
  } from './core/types';
  import { errorText, t } from './i18n';
  import AboutDialog from './lib/AboutDialog.svelte';
  import CalendarGrid from './lib/CalendarGrid.svelte';
  import { followAppearance } from './lib/colourScheme';
  import Dialog from './lib/Dialog.svelte';
  import EventDetails from './lib/EventDetails.svelte';
  import EventEditor from './lib/EventEditor.svelte';
  import ImportDialog from './lib/ImportDialog.svelte';
  import {
    goToToday,
    openDay,
    openMonth,
    periodTitle,
    step,
    switchView,
    toggleToday,
  } from './lib/navigation';
  import { sameRepeat } from './lib/repeat';
  import ScopeDialog, { type ScopeChange, type ScopeChoice } from './lib/ScopeDialog.svelte';
  import SettingsDialog from './lib/SettingsDialog.svelte';
  import { shortcutFor, type ShortcutAction } from './lib/shortcuts';
  import Sidebar from './lib/Sidebar.svelte';
  import { overallSyncText } from './lib/syncDisplay.svelte';
  import SidebarPane from './lib/SidebarPane.svelte';
  import StartupError from './lib/StartupError.svelte';
  import Toast from './lib/Toast.svelte';
  import Toolbar from './lib/Toolbar.svelte';
  import { whenForShortcut } from './lib/when';
  import { WindowModel } from './lib/windowModel.svelte';
  import WorkingDialog from './lib/WorkingDialog.svelte';

  interface Props {
    core: CoreApi;
  }

  let { core }: Props = $props();

  // The core stays the same for the window's life.
  // svelte-ignore state_referenced_locally
  const model = new WindowModel(core);

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

  /** FreeCal's version, read when About first opens. */
  let version = $state<string>();
  let quitFailure = $state<string>();
  let rescheduleFailure = $state<string>();
  /**
   * Releases the hold on "Date changed" while the user presses on the grid,
   * as for a drag, so that the grid doesn't move under the pointer.
   */
  let releasePress: (() => void) | undefined;
  /** Now, to the second, so that the time since the last sync counts up. */
  let now = $state(Temporal.Now.instant());
  /** What the status indicator says, once the Sync Statuses are read. */
  const overallText = $derived(
    model.syncStatuses && overallSyncText(model.syncStatuses.overall, now),
  );

  $effect(() => {
    const timer = setInterval(() => (now = Temporal.Now.instant()), 1000);
    return () => clearInterval(timer);
  });
  let open = $state<Open>();
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
   * changes, over the editor or, for a drop, the grid. It is asked once the
   * core has counted the Exceptions it offers to change too.
   */
  let scopePrompt = $state<{
    change: ScopeChange;
    asked?: { counts: ExceptionCounts; choose: (choice: ScopeChoice | null) => void };
  }>();
  /** The message at the bottom of the window, with its Undo if it has one. */
  let toast = $state<{ message: string; undo?: () => void }>();

  const writableCalendars = $derived(
    model.directory?.calendars.filter((calendar) => !calendar.readOnly) ?? [],
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
      ? model.occurrences.find(
          (occurrence) =>
            occurrence.eventId === selectedEventId &&
            occurrence.recurrenceId === selectedRecurrenceId,
        )
      : undefined,
  );

  /**
   * Whether the grid holds back a new today for a dropped Occurrence while
   * the user chooses its recurrence scope, as it does during the drag.
   */
  const dropHeld = $derived(scopePrompt?.change === 'move');

  function oninteract(active: boolean) {
    releasePress?.();
    releasePress = active ? model.hold() : undefined;
  }

  $effect(() => followAppearance(model.settings.appearance));

  $effect(() => model.start());

  /** Opens the editor for a new Event, or says where to create a Calendar first. */
  function startCreating(when: When) {
    const target = defaultCalendar();
    if (!target) {
      show({
        kind: 'notice',
        title: t('event.noCalendarTitle'),
        body: t('event.noCalendarBody', { account: t('account.local') }),
      });
      return;
    }
    session++;
    show({
      kind: 'creating',
      draft: {
        calendarId: target.id,
        title: '',
        when,
        location: '',
        description: '',
        repeat: null,
      },
    });
  }

  function openDetails(id: EventId, recurrenceId: string | null = null) {
    dialogFailure = undefined;
    show({ kind: 'details', id, recurrenceId });
    model.openDetails(id, close);
  }

  /**
   * Edits the Event; for an Occurrence of a Recurring Event, that
   * Occurrence, and Save asks which Occurrences the change is for.
   */
  function startEditing(event: Event, recurrenceId: string | null, when: When) {
    session++;
    show({
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
    });
  }

  /** Opens `next` over the grid in place of what was open, or nothing. */
  function show(next: Open | undefined) {
    // The Window Model reads the Event of the open details only while they are open.
    if (open?.kind === 'details') model.closeDetails();
    open = next;
  }

  function close() {
    show(undefined);
    dialogFailure = undefined;
  }

  async function saveNew(draft: EventDraft) {
    await core.createEvent(draft);
    close();
  }

  /**
   * Asks which Occurrences of a Recurring Event a change to its Occurrence
   * `recurrenceId` changes, and whether its Exceptions with values of their
   * own change too. Resolves to `null` when the user cancels.
   */
  async function askScope(
    change: ScopeChange,
    id: EventId,
    recurrenceId: string,
  ): Promise<ScopeChoice | null> {
    // Open while the counts load too, so that a drop is held from the start.
    scopePrompt = { change };
    const release = change === 'move' ? model.hold() : undefined;
    try {
      const counts = await core.exceptionCounts(id, recurrenceId);
      return await new Promise((choose) => {
        scopePrompt = { change, asked: { counts, choose } };
      });
    } finally {
      scopePrompt = undefined;
      release?.();
    }
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
    const choice = await askScope(
      sameRepeat(draft.repeat, editing.draft.repeat) ? 'edit' : 'editRepeat',
      id,
      recurrenceId,
    );
    if (choice === null) return;
    const { scope, exceptionsToo } = choice;
    const { value } = await core.editOccurrence(id, recurrenceId, scope, exceptionsToo, draft);
    openDetails(value.eventId, value.recurrenceId);
  }

  /**
   * A drag or resize in the grid; for an Occurrence of a Recurring Event it
   * asks which Occurrences move first. The grid puts the Occurrence back if
   * this rejects, as when the user cancels.
   */
  async function reschedule(id: EventId, recurrenceId: string | null, when: When) {
    const choice = recurrenceId === null ? null : await askScope('move', id, recurrenceId);
    if (recurrenceId !== null && choice === null) throw new Error('The move was cancelled.');
    try {
      if (recurrenceId === null) await core.rescheduleEvent(id, when);
      else
        await core.rescheduleOccurrence(
          id,
          recurrenceId,
          choice!.scope,
          choice!.exceptionsToo,
          when,
        );
      rescheduleFailure = undefined;
    } catch (error) {
      rescheduleFailure = t('event.rescheduleFailed', { error: errorText(error) });
      throw error;
    }
  }

  /** A drag the grid refused, because the Occurrence's Calendar is read-only. */
  function refuseDrag(id: EventId, recurrenceId: string | null) {
    const dragged = model.occurrences.find(
      (occurrence) => occurrence.eventId === id && occurrence.recurrenceId === recurrenceId,
    );
    const calendar = model.directory?.calendars.find((found) => found.id === dragged?.calendarId);
    if (!dragged || !calendar) return;
    toast = {
      message: t('event.readOnlyDrag', {
        title: dragged.title || t('event.untitled'),
        calendar: calendar.name,
      }),
    };
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
      show({
        kind: 'notice',
        title: t('import.noCalendarTitle'),
        body: t('event.noCalendarBody', { account: t('account.local') }),
      });
      return;
    }
    let file: string | null;
    try {
      file = await core.pickImportFile(t('import.fileDialogTitle'), t('import.fileFilter'));
    } catch (error) {
      show({
        kind: 'notice',
        title: t('import.pickFailedTitle'),
        body: t('import.pickFailed', { error: errorText(error) }),
      });
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
    show({ kind: 'importing', file, calendarId: target.id });
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
      const { journalTransaction } = outcome;
      close();
      toast = {
        message: t('import.done', {
          file,
          imported: String(outcome.imported),
          updated: String(outcome.updated),
          skipped: String(outcome.skipped),
        }),
        // An import that changed nothing has no Journal Transaction to undo.
        undo: journalTransaction === null ? undefined : () => undoImport(file, journalTransaction),
      };
    } catch (error) {
      importFailure = t('import.failed', { error: errorText(error) });
    } finally {
      working = undefined;
    }
  }

  async function undoImport(file: string, id: JournalTransactionId) {
    toast = undefined;
    working = t('working.undoingImport', { file });
    try {
      await core.undoJournalTransaction(id);
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

  /** Sync now of every server Account; the Sync Status shows how it goes. */
  function syncNow() {
    void core.syncNow(null);
  }

  function openSettings() {
    model.forgetSettingsFailure();
    show({ kind: 'settings' });
  }

  function openAbout() {
    dialogFailure = undefined;
    show({ kind: 'about' });
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
    next: () => model.navigate(step(model.navigation!, 1)),
    previous: () => model.navigate(step(model.navigation!, -1)),
    // Unlike the Today button, `t` doesn't toggle: it always goes to today.
    today: () => model.navigate(goToToday(model.navigation!, model.today!)),
    sync: syncNow,
    settings: openSettings,
    sidebar: () => model.toggleSidebar(),
  };

  function showView(view: View) {
    model.navigate(switchView(model.navigation!, view));
  }

  /**
   * Shortcuts act unless a dialog or menu is open. Those without Ctrl also
   * wait while the user is typing.
   */
  function onkeydown(key: KeyboardEvent) {
    const shortcut = shortcutFor(key);
    if (!shortcut) return;
    if (!model.navigation || open || document.querySelector('dialog[open]')) return;
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

{#if model.startupReason}
  <StartupError reason={model.startupReason} onquit={quit} />
{:else if model.today && model.navigation && model.directory && model.sidebar}
  {@const today = model.today}
  {@const shown = model.navigation}
  {@const directory = model.directory}
  <div class="app">
    <SidebarPane sidebar={model.sidebar} onchange={(next) => model.changeSidebar(next)}>
      <Sidebar
        {core}
        accounts={directory.accounts}
        calendars={directory.calendars}
        syncStatuses={model.syncStatuses}
        {overallText}
      />
    </SidebarPane>
    <main class="grid">
      <Toolbar
        view={shown.view}
        tracking={shown.tracking}
        title={periodTitle(shown)}
        ontoday={() => model.navigate(toggleToday(shown, today))}
        onstep={(by) => model.navigate(step(shown, by))}
        onview={(view) => model.navigate(switchView(shown, view))}
        onmonth={(month) => model.navigate(openMonth(shown, month))}
        syncState={model.syncStatuses?.overall.state}
        {overallText}
        onsyncnow={syncNow}
        onimport={startImport}
        onsettings={openSettings}
        onabout={openAbout}
        onquit={quit}
      />
      {#if model.readFailure}
        <p role="alert">{t('app.readFailed', { error: model.readFailure })}</p>
      {/if}
      {#if model.viewSaveFailure}
        <p role="alert">{model.viewSaveFailure}</p>
      {/if}
      {#if model.sidebarSaveFailure}
        <p role="alert">{model.sidebarSaveFailure}</p>
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
        occurrences={model.occurrences}
        calendars={directory.calendars}
        {selectedEventId}
        {selectedRecurrenceId}
        onrangechange={(range) => model.showRange(range)}
        onselectslot={startCreating}
        onopenevent={openDetails}
        onopenday={(date) => model.navigate(openDay(date))}
        onreschedule={reschedule}
        onrefusedrag={refuseDrag}
        {oninteract}
        held={dropHeld}
      />
    </main>
  </div>

  {#if open?.kind === 'details' && model.opened}
    {@const event = model.opened}
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
      settings={model.settings}
      failure={model.settingsFailure}
      onchange={(update) => model.changeSettings(update)}
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

  {#if scopePrompt?.asked}
    <ScopeDialog
      change={scopePrompt.change}
      counts={scopePrompt.asked.counts}
      onchoose={scopePrompt.asked.choose}
    />
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
