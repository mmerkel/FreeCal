<script lang="ts">
  import type { CoreApi } from './core/CoreApi';
  import type { CalendarId, ExistingEvents, JournalTransactionId, View } from './core/types';
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
  import ScopeDialog from './lib/ScopeDialog.svelte';
  import SettingsDialog from './lib/SettingsDialog.svelte';
  import { shortcutFor, type ShortcutAction } from './lib/shortcuts';
  import Sidebar from './lib/Sidebar.svelte';
  import { overallSyncText } from './lib/syncDisplay.svelte';
  import SidebarPane from './lib/SidebarPane.svelte';
  import StartupError from './lib/StartupError.svelte';
  import Toast from './lib/Toast.svelte';
  import Toolbar from './lib/Toolbar.svelte';
  import { whenForShortcut } from './lib/when';
  import { WindowDialogs } from './lib/windowDialogs.svelte';
  import { WindowModel } from './lib/windowModel.svelte';
  import WorkingDialog from './lib/WorkingDialog.svelte';

  interface Props {
    core: CoreApi;
  }

  let { core }: Props = $props();

  // The core stays the same for the window's life.
  // svelte-ignore state_referenced_locally
  const model = new WindowModel(core);
  // svelte-ignore state_referenced_locally
  const dialogs = new WindowDialogs(core, model);

  let quitFailure = $state<string>();
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
  /** Why the last import from the open import dialog failed. */
  let importFailure = $state<string>();
  /** What the modal "Working…" dialog says while a long command runs. */
  let working = $state<string>();

  function oninteract(active: boolean) {
    releasePress?.();
    releasePress = active ? model.hold() : undefined;
  }

  $effect(() => followAppearance(model.settings.appearance));

  $effect(() => model.start());

  /** Asks for a file to import, then opens the import dialog for it. */
  async function startImport() {
    if (!dialogs.defaultCalendar()) {
      dialogs.showNotice(
        t('import.noCalendarTitle'),
        t('event.noCalendarBody', { account: t('account.local') }),
      );
      return;
    }
    let file: string | null;
    try {
      file = await core.pickImportFile(t('import.fileDialogTitle'), t('import.fileFilter'));
    } catch (error) {
      dialogs.showNotice(
        t('import.pickFailedTitle'),
        t('import.pickFailed', { error: errorText(error) }),
      );
      return;
    }
    if (file === null) return;
    const target = dialogs.defaultCalendar();
    importFailure = undefined;
    // The user may have opened something else while the file dialog was showing.
    if (!target || !dialogs.openImport(file, target.id)) cancelImport();
  }

  function cancelImport() {
    // Nothing closes the dialog while its import runs, not even an Escape
    // that reaches it under "Working…".
    if (working) return;
    core.discardImport().catch(() => {});
    if (dialogs.open?.kind === 'importing') dialogs.close();
  }

  async function runImport(file: string, calendarId: CalendarId, existing: ExistingEvents) {
    working = t('working.importing', { file });
    try {
      const outcome = await core.runImport(calendarId, existing);
      const { journalTransaction } = outcome;
      dialogs.close();
      dialogs.showToast({
        message: t('import.done', {
          file,
          imported: String(outcome.imported),
          updated: String(outcome.updated),
          skipped: String(outcome.skipped),
        }),
        // An import that changed nothing has no Journal Transaction to undo.
        undo: journalTransaction === null ? undefined : () => undoImport(file, journalTransaction),
      });
    } catch (error) {
      importFailure = t('import.failed', { error: errorText(error) });
    } finally {
      working = undefined;
    }
  }

  async function undoImport(file: string, id: JournalTransactionId) {
    dialogs.dismissToast();
    working = t('working.undoingImport', { file });
    try {
      await core.undoJournalTransaction(id);
      dialogs.showToast({ message: t('import.undone', { file }) });
    } catch (error) {
      // Undo can succeed later, such as once the Calendar is writable again.
      dialogs.showToast({
        message: t('import.undoFailed', { file, error: errorText(error) }),
        undo: () => undoImport(file, id),
      });
    } finally {
      working = undefined;
    }
  }

  /** Sync now of every server Account; the Sync Status shows how it goes. */
  function syncNow() {
    void core.syncNow(null);
  }

  function quit() {
    core.quit().catch((error) => (quitFailure = t('app.quitFailed', { error: errorText(error) })));
  }

  /** What each keyboard shortcut does (see `shortcuts.ts` for the keys). */
  const shortcutActions: Record<ShortcutAction, () => void> = {
    create: () => dialogs.startCreating(whenForShortcut(Temporal.Now.plainDateTimeISO())),
    day: () => showView('day'),
    week: () => showView('week'),
    month: () => showView('month'),
    next: () => model.navigate(step(model.navigation!, 1)),
    previous: () => model.navigate(step(model.navigation!, -1)),
    // Unlike the Today button, `t` doesn't toggle: it always goes to today.
    today: () => model.navigate(goToToday(model.navigation!, model.today!)),
    sync: syncNow,
    settings: () => dialogs.openSettings(),
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
    // The Sidebar's own dialogs are not Window Dialogs' yet.
    if (!model.navigation || dialogs.isOpen || document.querySelector('dialog[open]')) return;
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
        onsettings={() => dialogs.openSettings()}
        onabout={() => dialogs.openAbout()}
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
      {#if dialogs.rescheduleFailure}
        <p role="alert">{dialogs.rescheduleFailure}</p>
      {/if}
      <CalendarGrid
        view={shown.view}
        date={shown.date}
        {today}
        occurrences={model.occurrences}
        calendars={directory.calendars}
        selectedEventId={dialogs.selectedEventId}
        selectedRecurrenceId={dialogs.selectedRecurrenceId}
        onrangechange={(range) => model.showRange(range)}
        onselectslot={(when) => dialogs.startCreating(when)}
        onopenevent={(id, recurrenceId) => dialogs.openDetails(id, recurrenceId)}
        onopenday={(date) => model.navigate(openDay(date))}
        onreschedule={(id, recurrenceId, when) => dialogs.reschedule(id, recurrenceId, when)}
        onrefusedrag={(id, recurrenceId) => dialogs.refuseDrag(id, recurrenceId)}
        {oninteract}
        held={dialogs.dropHeld}
      />
    </main>
  </div>

  {#if dialogs.open?.kind === 'details' && model.opened}
    {@const event = model.opened}
    <EventDetails
      {event}
      calendar={directory.calendars.find((calendar) => calendar.id === event.calendarId)}
      when={dialogs.openedOccurrence?.when}
      failure={dialogs.failure}
      onedit={() => dialogs.startEditing()}
      ondelete={() => dialogs.remove()}
      ondeleteoccurrence={dialogs.open.recurrenceId === null
        ? undefined
        : () => dialogs.removeOccurrence()}
      onclose={() => dialogs.close()}
      onopenlink={(url) => dialogs.openLink(url)}
    />
  {:else if dialogs.open?.kind === 'creating' || dialogs.open?.kind === 'editing'}
    {@const editing = dialogs.open}
    {#key dialogs.editorSession}
      <EventEditor
        mode={editing.kind === 'creating' ? 'create' : 'edit'}
        initial={editing.draft}
        calendars={dialogs.writableCalendars}
        customRepeat={editing.kind === 'editing' && editing.customRepeat}
        repeatable={editing.kind === 'creating' || editing.repeatable}
        onsave={(draft) => dialogs.save(draft)}
        oncancel={() => dialogs.cancelEditing()}
      />
    {/key}
  {:else if dialogs.open?.kind === 'settings'}
    <SettingsDialog
      settings={model.settings}
      failure={model.settingsFailure}
      onchange={(update) => model.changeSettings(update)}
      onclose={() => dialogs.close()}
    />
  {:else if dialogs.open?.kind === 'about'}
    <AboutDialog
      version={dialogs.version}
      failure={dialogs.failure}
      onopenlink={(url) => dialogs.openLink(url)}
      onclose={() => dialogs.close()}
    />
  {:else if dialogs.open?.kind === 'importing'}
    {@const importing = dialogs.open}
    <ImportDialog
      {core}
      file={importing.file}
      calendars={dialogs.writableCalendars}
      initialCalendarId={importing.calendarId}
      failure={importFailure}
      onimport={(calendarId, existing) => runImport(importing.file, calendarId, existing)}
      oncancel={cancelImport}
    />
  {:else if dialogs.open?.kind === 'notice'}
    <Dialog title={dialogs.open.title} onclose={() => dialogs.close()}>
      <p>{dialogs.open.body}</p>
      {#snippet actions()}
        <button
          type="button"
          class="button solid"
          data-initial-focus
          onclick={() => dialogs.close()}
        >
          {t('common.ok')}
        </button>
      {/snippet}
    </Dialog>
  {/if}

  {#if dialogs.scopePrompt?.counts}
    <ScopeDialog
      change={dialogs.scopePrompt.change}
      counts={dialogs.scopePrompt.counts}
      onchoose={(choice) => dialogs.chooseScope(choice)}
    />
  {/if}

  {#if dialogs.toast}
    {@const shown = dialogs.toast}
    <Toast
      message={shown.message}
      action={shown.undo && { label: t('import.undo'), run: shown.undo }}
      ondismiss={() => dialogs.dismissToast()}
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
