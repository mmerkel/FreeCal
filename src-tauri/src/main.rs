//! The Tauri shell: exposes the core application interface to the frontend as
//! commands and forwards the core's Signals as Tauri events.

#[cfg(target_os = "linux")]
mod autostart;
#[cfg(target_os = "linux")]
mod desktop_watch;
#[cfg(target_os = "linux")]
mod handoff;
mod keyring;
#[cfg(all(test, target_os = "linux"))]
mod test_bus;
mod tray;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::NaiveDate;
use freecal_core::{
    Account, AccountId, AddedAccount, AvailableCalendar, CalDavLogin, Calendar, CalendarId,
    Changed, Colour, Core, ErrorCode, Event, EventDraft, EventId, ExceptionCounts, ExistingEvents,
    ImportFile, ImportOutcome, ImportPreview, ImportSummary, JournalTransactionId, Occurrence,
    Scope, Settings, SettingsUpdate, SidebarState, Signal, SignalSink, SyncStatuses, SyncTrigger,
    SystemClock, View, When, link_allowed,
};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

/// The Tauri event on which Signals reach the frontend.
const SIGNAL_EVENT: &str = "freecal://signal";

/// The Tauri event that asks the frontend to quit as Quit in the main menu
/// does, when closing the window would quit with Unsent Changes waiting.
const QUIT_ASKED_EVENT: &str = "freecal://quit-asked";

/// How often the core checks whether the local date changed. A sleep of the
/// thread doesn't count the time the computer was suspended, so a short
/// period also catches waking up on a new day soon.
const DATE_CHECK_PERIOD: Duration = Duration::from_secs(10);

/// How long the thread that runs the scheduled syncs waits at most before
/// it checks for a due poll again. Like [`DATE_CHECK_PERIOD`], short enough
/// to notice soon after a sleep that a poll is due.
const SYNC_CHECK_PERIOD: Duration = Duration::from_secs(10);

/// The window starts hidden and the frontend shows it once it knows the
/// appearance, so that it never flashes the wrong theme. Should the frontend
/// never get that far, the window is shown after this time anyway.
const SHOW_WINDOW_FALLBACK: Duration = Duration::from_secs(5);

/// The label of FreeCal's one window in `tauri.conf.json`.
const MAIN_WINDOW: &str = "main";

struct ForwardToFrontend(AppHandle);

impl SignalSink for ForwardToFrontend {
    fn send(&self, signal: Signal) {
        // Signals are fire-and-forget: with no window open they are dropped.
        let _ = self.0.emit(SIGNAL_EVENT, signal);
    }
}

/// Whether the desktop session ends, by logout or shutdown, as the Desktop
/// Watch reports it. Then closing the window quits at once, without trying
/// to send the Unsent Changes, which stay in the Local Store (spec:
/// "Quitting").
#[derive(Default)]
struct SessionEnding(AtomicBool);

/// Why the core couldn't open its Local Store. Managed only then, in place
/// of the [`Core`], and the window shows it on the startup error screen.
struct StartupFailure(ErrorCode);

/// Whether the window shows the startup error screen: because the core
/// couldn't open its Local Store, or because the window's own start failed.
/// Then closing the window quits FreeCal, which has nothing to keep running
/// for.
#[derive(Default)]
struct StartupFailed(AtomicBool);

/// The file the user picked to import, with its latest preview. It is held
/// here, so that the window never handles paths or the parsed Events: it
/// can't make the shell read any other file.
#[derive(Default)]
struct PickedImport(Mutex<Option<PickedFile>>);

struct PickedFile {
    file: ImportFile,
    preview: Option<ImportPreview>,
}

/// Failures reach the frontend as codes it translates, never as English text.
type CommandResult<T> = Result<T, ErrorCode>;

#[tauri::command]
fn today(core: State<Core>) -> String {
    core.today().to_string()
}

#[tauri::command]
fn last_used_view(core: State<Core>) -> CommandResult<View> {
    core.last_used_view().map_err(ErrorCode::from)
}

#[tauri::command]
fn set_last_used_view(core: State<Core>, view: View) -> CommandResult<()> {
    core.set_last_used_view(view).map_err(ErrorCode::from)
}

#[tauri::command]
fn sidebar(core: State<Core>) -> CommandResult<SidebarState> {
    core.sidebar().map_err(ErrorCode::from)
}

#[tauri::command]
fn set_sidebar(core: State<Core>, sidebar: SidebarState) -> CommandResult<()> {
    core.set_sidebar(sidebar).map_err(ErrorCode::from)
}

#[tauri::command]
fn settings(core: State<Core>) -> CommandResult<Settings> {
    core.settings().map_err(ErrorCode::from)
}

/// Changes the settings. Start at login is turned on or off on the desktop
/// first, and kept only when that worked; the portal may ask the user, so
/// this runs off the UI thread.
#[tauri::command]
async fn update_settings(app: AppHandle, update: SettingsUpdate) -> CommandResult<Settings> {
    tauri::async_runtime::spawn_blocking(move || {
        if let Some(on) = update.start_at_login {
            set_start_at_login(&app, on)?;
        }
        app.state::<Core>()
            .update_settings(update)
            .map_err(ErrorCode::from)
    })
    .await
    // A panic there is a bug, as it would be in a command of its own.
    .expect("updating the settings panicked")
}

#[cfg(target_os = "linux")]
fn set_start_at_login(app: &AppHandle, on: bool) -> CommandResult<()> {
    let config_dir = app
        .path()
        .config_dir()
        .map_err(|_| ErrorCode::StartAtLoginFailed)?;
    autostart::StartAtLogin::current(
        &app.config().identifier,
        &app.package_info().name,
        &config_dir,
    )
    .ok_or(ErrorCode::StartAtLoginFailed)?
    .set(on)
}

#[cfg(not(target_os = "linux"))]
fn set_start_at_login(_app: &AppHandle, _on: bool) -> CommandResult<()> {
    Err(ErrorCode::StartAtLoginFailed)
}

#[tauri::command]
fn list_accounts(core: State<Core>) -> CommandResult<Vec<Account>> {
    core.list_accounts().map_err(ErrorCode::from)
}

/// Runs a core request that talks to a server or the keyring, which can take
/// long, on a thread of its own, so that the window and other commands go on
/// meanwhile.
async fn off_the_ui_thread<T: Send + 'static>(
    app: AppHandle,
    request: impl FnOnce(&Core) -> freecal_core::Result<T> + Send + 'static,
) -> CommandResult<T> {
    tauri::async_runtime::spawn_blocking(move || request(&app.state::<Core>()))
        .await
        // A panic there is a bug, as it would be in a command of its own.
        .expect("a core request panicked")
        .map_err(ErrorCode::from)
}

#[tauri::command]
async fn add_caldav_account(app: AppHandle, login: CalDavLogin) -> CommandResult<AddedAccount> {
    off_the_ui_thread(app, move |core| core.add_caldav_account(&login)).await
}

#[tauri::command]
async fn available_calendars(
    app: AppHandle,
    account_id: i64,
) -> CommandResult<Vec<AvailableCalendar>> {
    off_the_ui_thread(app, move |core| {
        core.available_calendars(AccountId(account_id))
    })
    .await
}

/// Returns once every added Calendar's first sync has ended, which the
/// sidebar shows meanwhile.
#[tauri::command]
async fn add_calendars(
    app: AppHandle,
    account_id: i64,
    urls: Vec<String>,
) -> CommandResult<Vec<Calendar>> {
    off_the_ui_thread(app, move |core| {
        core.add_calendars(AccountId(account_id), &urls)
    })
    .await
}

#[tauri::command]
async fn remove_account(app: AppHandle, id: i64) -> CommandResult<()> {
    off_the_ui_thread(app, move |core| core.remove_account(AccountId(id))).await
}

#[tauri::command]
fn list_calendars(core: State<Core>) -> CommandResult<Vec<Calendar>> {
    core.list_calendars().map_err(ErrorCode::from)
}

#[tauri::command]
fn sync_statuses(core: State<Core>) -> CommandResult<SyncStatuses> {
    core.sync_statuses().map_err(ErrorCode::from)
}

/// Tries to send every Unsent Change before quitting, for a few seconds,
/// and returns the Sync Statuses then (see [`Core::send_before_quit`]).
#[tauri::command]
async fn send_before_quit(app: AppHandle) -> CommandResult<SyncStatuses> {
    off_the_ui_thread(app, |core| core.send_before_quit()).await
}

/// Sync now, of the Account `account_id`, or of every server Account.
/// Returns at once; "Sync status changed" tells how it goes.
#[tauri::command]
fn sync_now(core: State<Core>, account_id: Option<i64>) {
    core.sync_now(account_id.map(AccountId));
}

#[tauri::command]
fn create_calendar(
    core: State<Core>,
    account_id: i64,
    name: String,
    colour: String,
) -> CommandResult<Calendar> {
    let colour = Colour::parse(&colour).map_err(ErrorCode::from)?;
    core.create_calendar(AccountId(account_id), &name, colour)
        .map_err(ErrorCode::from)
}

#[tauri::command]
fn rename_calendar(core: State<Core>, id: i64, name: String) -> CommandResult<()> {
    core.rename_calendar(CalendarId(id), &name)
        .map_err(ErrorCode::from)
}

#[tauri::command]
fn recolour_calendar(core: State<Core>, id: i64, colour: String) -> CommandResult<()> {
    let colour = Colour::parse(&colour).map_err(ErrorCode::from)?;
    core.recolour_calendar(CalendarId(id), colour)
        .map_err(ErrorCode::from)
}

#[tauri::command]
fn delete_calendar(core: State<Core>, id: i64) -> CommandResult<()> {
    core.delete_calendar(CalendarId(id))
        .map_err(ErrorCode::from)
}

#[tauri::command]
fn remove_calendar(core: State<Core>, id: i64) -> CommandResult<()> {
    core.remove_calendar(CalendarId(id))
        .map_err(ErrorCode::from)
}

#[tauri::command]
fn set_calendar_shown(core: State<Core>, id: i64, shown: bool) -> CommandResult<()> {
    core.set_calendar_shown(CalendarId(id), shown)
        .map_err(ErrorCode::from)
}

#[tauri::command]
fn set_calendar_read_only(core: State<Core>, id: i64, read_only: bool) -> CommandResult<()> {
    core.set_calendar_read_only(CalendarId(id), read_only)
        .map_err(ErrorCode::from)
}

#[tauri::command]
fn show_only_calendar(core: State<Core>, id: i64) -> CommandResult<()> {
    core.show_only_calendar(CalendarId(id))
        .map_err(ErrorCode::from)
}

#[tauri::command]
fn list_occurrences(
    core: State<Core>,
    from: NaiveDate,
    to: NaiveDate,
) -> CommandResult<Vec<Occurrence>> {
    core.occurrences(from, to).map_err(ErrorCode::from)
}

#[tauri::command]
fn event(core: State<Core>, id: EventId) -> CommandResult<Event> {
    core.event(id).map_err(ErrorCode::from)
}

#[tauri::command]
fn create_event(core: State<Core>, draft: EventDraft) -> CommandResult<Changed<Event>> {
    core.create_event(draft).map_err(ErrorCode::from)
}

#[tauri::command]
fn edit_event(core: State<Core>, id: EventId, draft: EventDraft) -> CommandResult<Changed<Event>> {
    core.edit_event(id, draft).map_err(ErrorCode::from)
}

#[tauri::command]
fn reschedule_event(core: State<Core>, id: EventId, when: When) -> CommandResult<Changed<Event>> {
    core.reschedule_event(id, when).map_err(ErrorCode::from)
}

#[tauri::command]
fn edit_occurrence(
    core: State<Core>,
    id: EventId,
    recurrence_id: String,
    scope: Scope,
    exceptions_too: bool,
    draft: EventDraft,
) -> CommandResult<Changed<Occurrence>> {
    core.edit_occurrence(id, &recurrence_id, scope, exceptions_too, draft)
        .map_err(ErrorCode::from)
}

#[tauri::command]
fn reschedule_occurrence(
    core: State<Core>,
    id: EventId,
    recurrence_id: String,
    scope: Scope,
    exceptions_too: bool,
    when: When,
) -> CommandResult<Changed<Occurrence>> {
    core.reschedule_occurrence(id, &recurrence_id, scope, exceptions_too, when)
        .map_err(ErrorCode::from)
}

#[tauri::command]
fn exception_counts(
    core: State<Core>,
    id: EventId,
    recurrence_id: String,
) -> CommandResult<ExceptionCounts> {
    core.exception_counts(id, &recurrence_id)
        .map_err(ErrorCode::from)
}

#[tauri::command]
fn delete_event(core: State<Core>, id: EventId) -> CommandResult<Changed<()>> {
    core.delete_event(id).map_err(ErrorCode::from)
}

#[tauri::command]
fn delete_occurrence(
    core: State<Core>,
    id: EventId,
    recurrence_id: String,
) -> CommandResult<Changed<()>> {
    core.delete_occurrence(id, &recurrence_id)
        .map_err(ErrorCode::from)
}

/// Asks the user for an `.ics` or `.zip` file to import through the system's
/// file dialog (the portal's, where there is one) and reads it. Returns its
/// name, or `None` when the user cancels. `title` and `filter` are the
/// dialog's translated words.
#[tauri::command]
async fn pick_import_file(
    app: AppHandle,
    picked: State<'_, PickedImport>,
    title: String,
    filter: String,
) -> CommandResult<Option<String>> {
    let Some(path) = import_path(&app, title, filter) else {
        return Ok(None);
    };
    let file = ImportFile::read(&path).map_err(ErrorCode::from)?;
    let name = file.name.clone();
    *picked.0.lock().unwrap() = Some(PickedFile {
        file,
        preview: None,
    });
    Ok(Some(name))
}

fn import_path(app: &AppHandle, title: String, filter: String) -> Option<PathBuf> {
    // The agent's Xvfb session has no portal; a debug build takes the file
    // from the environment instead (see the run-freecal skill).
    #[cfg(debug_assertions)]
    if let Some(path) = std::env::var_os("FREECAL_IMPORT_FILE") {
        return Some(path.into());
    }
    let mut dialog = app
        .dialog()
        .file()
        .set_title(title)
        .add_filter(filter, &["ics", "zip"]);
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        dialog = dialog.set_parent(&window);
    }
    dialog.blocking_pick_file()?.into_path().ok()
}

/// Previews importing the picked file into a Calendar, and keeps the preview
/// for [`run_import`].
#[tauri::command]
async fn preview_import(
    core: State<'_, Core>,
    picked: State<'_, PickedImport>,
    calendar_id: i64,
) -> CommandResult<ImportSummary> {
    let mut held = picked.0.lock().unwrap();
    let current = held.as_mut().ok_or(ErrorCode::NoFileToImport)?;
    let preview = core
        .preview_import(&current.file, CalendarId(calendar_id))
        .map_err(ErrorCode::from)?;
    let summary = preview.summary();
    current.preview = Some(preview);
    Ok(summary)
}

/// Imports the picked file into a Calendar, doing with the Events it already
/// has what `existing` says, and lets go of the file. The kept preview is
/// used when it is of that Calendar.
#[tauri::command]
async fn run_import(
    core: State<'_, Core>,
    picked: State<'_, PickedImport>,
    calendar_id: i64,
    existing: ExistingEvents,
) -> CommandResult<ImportOutcome> {
    let mut held = picked.0.lock().unwrap();
    let current = held.as_mut().ok_or(ErrorCode::NoFileToImport)?;
    let calendar_id = CalendarId(calendar_id);
    let preview = match current.preview.take() {
        Some(preview) if preview.calendar_id == calendar_id => preview,
        _ => core
            .preview_import(&current.file, calendar_id)
            .map_err(ErrorCode::from)?,
    };
    let outcome = core.run_import(&preview, existing);
    match outcome {
        Ok(_) => *held = None,
        Err(_) => current.preview = Some(preview),
    }
    outcome.map_err(ErrorCode::from)
}

/// Lets go of the picked file without importing it.
#[tauri::command]
fn discard_import(picked: State<PickedImport>) {
    *picked.0.lock().unwrap() = None;
}

#[tauri::command]
async fn undo_journal_transaction(
    core: State<'_, Core>,
    id: JournalTransactionId,
) -> CommandResult<Changed<()>> {
    core.undo_journal_transaction(id).map_err(ErrorCode::from)
}

/// Opens a link from a description or location in the system browser. The scheme
/// is checked again here, so that the window can't open anything else even if
/// it asks to.
#[tauri::command]
fn open_link(app: AppHandle, url: String) -> CommandResult<()> {
    if !link_allowed(&url) {
        return Err(ErrorCode::LinkNotAllowed);
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|_| ErrorCode::LinkNotOpened)
}

/// FreeCal's version, for the About dialog.
#[tauri::command]
fn version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

/// Shows the window, which starts hidden.
#[tauri::command]
fn show_window(app: AppHandle) -> CommandResult<()> {
    show_main_window(&app).map_err(|_| ErrorCode::WindowNotShown)
}

/// Why FreeCal couldn't start, if it couldn't. The frontend asks before
/// anything else and then shows the startup error screen instead.
#[tauri::command]
fn startup_failure(app: AppHandle) -> Option<ErrorCode> {
    app.try_state::<StartupFailure>()
        .map(|failure| failure.0.clone())
}

/// The window shows the startup error screen although the core opened, as
/// when its first reads failed: closing it then quits FreeCal.
#[tauri::command]
fn window_failed_to_start(app: AppHandle) {
    app.state::<StartupFailed>().0.store(true, Ordering::SeqCst);
}

/// Ends FreeCal, also when it would otherwise keep running in the background.
#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

/// What closing the window does.
#[derive(Debug, PartialEq)]
enum Closing {
    /// The window goes and FreeCal keeps running in the background, in the
    /// tray (US-103).
    ToBackground,
    /// The frontend quits as Quit in the main menu does, trying to send the
    /// Unsent Changes first.
    AskToQuit,
    Quit,
}

/// What closing the window does: with background running on and a tray
/// there, FreeCal keeps running (US-103, US-106); otherwise it quits (US-104),
/// trying to send the Unsent Changes first when some wait. Logout and
/// shutdown quit at once and never wait for that (spec: "Quitting").
fn closing(
    session_ending: bool,
    run_in_background: bool,
    tray_present: impl FnOnce() -> bool,
    unsent_changes_wait: impl FnOnce() -> bool,
) -> Closing {
    if session_ending {
        Closing::Quit
    } else if run_in_background && tray_present() {
        Closing::ToBackground
    } else if unsent_changes_wait() {
        Closing::AskToQuit
    } else {
        Closing::Quit
    }
}

/// Does what closing the window does. Runs on a thread of its own, since
/// the tray check asks the desktop.
fn close_main_window(app: &AppHandle) {
    let Some(core) = app.try_state::<Core>() else {
        return app.exit(0);
    };
    let closed = closing(
        app.state::<SessionEnding>().0.load(Ordering::SeqCst),
        // When the settings can't be read, closing quits rather than risk
        // FreeCal running on unseen.
        core.settings()
            .is_ok_and(|settings| settings.run_in_background),
        || tray_present(app),
        || unsent_changes_wait(&core),
    );
    match closed {
        // The window is gone rather than hidden, so that it holds no memory
        // in the background; Open FreeCal creates it again.
        Closing::ToBackground => {
            if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
                let _ = window.destroy();
            }
        }
        Closing::AskToQuit => {
            let _ = app.emit_to(MAIN_WINDOW, QUIT_ASKED_EVENT, ());
        }
        Closing::Quit => app.exit(0),
    }
}

/// Whether there is a tray with FreeCal's icon in it.
fn tray_present(app: &AppHandle) -> bool {
    #[cfg(target_os = "linux")]
    let desktop_tray = desktop_watch::tray_present();
    // Other systems are for later.
    #[cfg(not(target_os = "linux"))]
    let desktop_tray = false;
    desktop_tray && app.tray_by_id(tray::TRAY).is_some()
}

/// Whether some Unsent Changes wait. When they can't be counted, none do.
fn unsent_changes_wait(core: &Core) -> bool {
    core.sync_statuses().is_ok_and(|statuses| {
        statuses
            .accounts
            .iter()
            .any(|account| account.report.unsent_changes > 0)
    })
}

/// Does what the user picked in the tray menu.
fn on_tray_choice(app: &AppHandle, choice: tray::TrayChoice) {
    match choice {
        tray::TrayChoice::OpenFreeCal => open_main_window(app),
        // As Quit in the main menu, in the window, which comes to the front
        // for it; with no window there to ask, FreeCal quits at once and
        // the Unsent Changes stay in the Local Store.
        tray::TrayChoice::Quit => {
            let window_there = app.get_webview_window(MAIN_WINDOW).is_some();
            if window_there && !app.state::<StartupFailed>().0.load(Ordering::SeqCst) {
                open_main_window(app);
                let _ = app.emit_to(MAIN_WINDOW, QUIT_ASKED_EVENT, ());
            } else {
                app.exit(0);
            }
        }
    }
}

/// Brings the window to the front, creating it again when FreeCal runs in
/// the background.
fn open_main_window(app: &AppHandle) {
    #[cfg(target_os = "linux")]
    bring_to_front(app, None);
    #[cfg(not(target_os = "linux"))]
    if app.get_webview_window(MAIN_WINDOW).is_some() {
        let _ = show_main_window(app);
    } else {
        reopen_main_window(app);
    }
}

/// Creates the window again after it closed for background running. Like
/// the first, it starts hidden and the frontend shows it, after subscribing
/// to the Signals and reading the current state.
fn reopen_main_window(app: &AppHandle) {
    let Some(config) = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == MAIN_WINDOW)
    else {
        return;
    };
    let created =
        tauri::WebviewWindowBuilder::from_config(app, config).and_then(|window| window.build());
    match created {
        Ok(_) => {
            if let Some(core) = app.try_state::<Core>() {
                core.set_window_open(true);
            }
            show_main_window_later(app);
        }
        Err(error) => eprintln!("FreeCal can't open its window again: {error}"),
    }
}

/// Shows the window after [`SHOW_WINDOW_FALLBACK`], should the frontend
/// never get that far.
fn show_main_window_later(app: &AppHandle) {
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(SHOW_WINDOW_FALLBACK);
        let _ = show_main_window(&handle);
    });
}

fn show_main_window(app: &AppHandle) -> tauri::Result<()> {
    match app.get_webview_window(MAIN_WINDOW) {
        Some(window) if !window.is_visible()? => {
            window.show()?;
            window.set_focus()
        }
        _ => Ok(()),
    }
}

/// Brings the window to the front, for a second launch or the tray, and
/// creates it again when FreeCal runs in the background. A second launch's
/// activation token is what lets the compositor raise the window; without
/// it KWin on Wayland only marks FreeCal in the taskbar. A window created
/// again doesn't use it.
#[cfg(target_os = "linux")]
fn bring_to_front(app: &AppHandle, activation_token: Option<String>) {
    use gtk::prelude::*;

    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return reopen_main_window(app);
    };
    // GTK only works on the main thread.
    let _ = app.run_on_main_thread(move || {
        let Ok(window) = window.gtk_window() else {
            return;
        };
        if let Some(token) = activation_token {
            set_activation_token(&window, &token);
        }
        window.deiconify();
        // Shows the window if needed, and on Wayland activates it with the token.
        window.present_with_time(gtk::gdk::ffi::GDK_CURRENT_TIME as u32);
    });
}

/// Makes the window's next activation use `token`: through xdg-activation on
/// Wayland, as a startup notification ID on X11.
#[cfg(target_os = "linux")]
fn set_activation_token(window: &gtk::ApplicationWindow, token: &str) {
    use gtk::glib::translate::ToGlibPtr;
    use gtk::prelude::*;

    let display = WidgetExt::display(window);
    if display.type_().name() == "GdkWaylandDisplay" {
        let Ok(token) = std::ffi::CString::new(token) else {
            return;
        };
        // SAFETY: `display` is a live GdkWaylandDisplay, checked above, and
        // GDK copies the token.
        unsafe {
            gdk_wayland_sys::gdk_wayland_display_set_startup_notification_id(
                ToGlibPtr::<*mut gtk::gdk::ffi::GdkDisplay>::to_glib_none(&display)
                    .0
                    .cast(),
                token.as_ptr(),
            );
        }
    } else {
        window.set_startup_id(token);
    }
}

/// Hands this launch to the running FreeCal on the same Local Store and exits,
/// or becomes the one later launches hand off to. Without a session bus
/// FreeCal still starts: the core's lock keeps a second launch from opening
/// the Local Store anyway.
#[cfg(target_os = "linux")]
fn hand_off_or_serve(app: &tauri::App, data_dir: &std::path::Path) {
    let handle = app.handle().clone();
    let name = handoff::local_store_name(&app.config().identifier, data_dir);
    // Files to open among the arguments are ticket 30's.
    let claimed = handoff::claim(&name, &handoff::Launch::current(), move |launch| {
        bring_to_front(&handle, launch.activation_token)
    });
    match claimed {
        Ok(handoff::Claim::Running(connection)) => {
            app.manage(connection);
        }
        Ok(handoff::Claim::HandedOff) => {
            app.cleanup_before_exit();
            std::process::exit(0);
        }
        Err(error) => eprintln!("FreeCal can't take over from a second launch: {error}"),
    }
}

/// Runs the syncs the core's Sync Scheduler finds due, on a thread of its
/// own, and tells it about the network and waking from sleep.
fn run_scheduled_syncs(app: &AppHandle) {
    let handle = app.clone();
    std::thread::spawn(move || {
        let core = handle.state::<Core>();
        loop {
            core.run_scheduled_syncs();
            core.wait_for_sync_request(SYNC_CHECK_PERIOD);
        }
    });
    #[cfg(target_os = "linux")]
    {
        let handle = app.clone();
        desktop_watch::watch_session_end(move |ending| {
            handle
                .state::<SessionEnding>()
                .0
                .store(ending, Ordering::SeqCst)
        });
        let handle = app.clone();
        desktop_watch::watch_network(move |available| {
            handle.state::<Core>().set_network_available(available)
        });
        let handle = app.clone();
        desktop_watch::watch_wake(move || handle.state::<Core>().trigger_sync(SyncTrigger::Woke));
    }
}

/// The most the Log file grows to before it starts again, the old one gone.
const MAX_LOG_SIZE: u128 = 1_000_000;

/// Writes the Log, the core's `log` lines of warnings and errors, to a file in
/// the operating system's log directory, e.g.
/// ~/.local/share/io.github.mmerkel.FreeCal/logs/FreeCal.log on Linux or
/// ~/Library/Logs/io.github.mmerkel.FreeCal on macOS. The window gets no
/// permission to write it.
fn log_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    use tauri_plugin_log::{Builder, RotationStrategy, Target, TargetKind, log::LevelFilter};
    Builder::new()
        .targets([Target::new(TargetKind::LogDir { file_name: None })])
        .level(LevelFilter::Off)
        .level_for("freecal_core", LevelFilter::Warn)
        .max_file_size(MAX_LOG_SIZE)
        .rotation_strategy(RotationStrategy::KeepOne)
        .build()
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(log_plugin())
        .manage(PickedImport::default())
        .manage(SessionEnding::default())
        .manage(StartupFailed::default())
        .setup(|app| {
            // The XDG data directory, e.g. ~/.local/share/io.github.mmerkel.FreeCal (ADR 0004, 0006).
            let data_dir = app.path().app_data_dir()?;
            // Before the core opens, so that a second launch hands off
            // rather than failing with `LocalStoreInUse`.
            #[cfg(target_os = "linux")]
            hand_off_or_serve(app, &data_dir);
            let signals = Arc::new(ForwardToFrontend(app.handle().clone()));
            match Core::open(&data_dir, Arc::new(SystemClock), signals, keyring::system()) {
                Ok(core) => {
                    app.manage(core);
                    let handle = app.handle().clone();
                    std::thread::spawn(move || {
                        loop {
                            std::thread::sleep(DATE_CHECK_PERIOD);
                            handle.state::<Core>().check_date();
                        }
                    });
                    run_scheduled_syncs(app.handle());
                    // Without its icon, closing the window quits.
                    if let Err(error) = tray::show(app.handle(), on_tray_choice) {
                        eprintln!("FreeCal can't show its tray icon: {error}");
                    }
                }
                Err(error) => {
                    eprintln!("FreeCal could not open its Local Store: {error}");
                    app.manage(StartupFailure(error.code()));
                    app.state::<StartupFailed>().0.store(true, Ordering::SeqCst);
                }
            }
            show_main_window_later(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // Closing the startup error screen quits FreeCal, which has
            // nothing to keep running for.
            WindowEvent::CloseRequested { .. }
                if window.state::<StartupFailed>().0.load(Ordering::SeqCst) =>
            {
                window.app_handle().exit(0);
            }
            // The close decides between running on in the background and
            // quitting (`closing`), off the UI thread.
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let app = window.app_handle().clone();
                std::thread::spawn(move || close_main_window(&app));
            }
            // The Sync Scheduler polls less often in the background.
            WindowEvent::Destroyed => {
                if let Some(core) = window.try_state::<Core>() {
                    core.set_window_open(false);
                }
                // Closing the window cancels whatever dialog it showed, the
                // import dialog too, so the picked file goes. An import still
                // running holds it; this thread mustn't wait for that, and a
                // file left over is replaced by the next pick anyway.
                if let Ok(mut picked) = window.state::<PickedImport>().0.try_lock() {
                    *picked = None;
                }
            }
            WindowEvent::Focused(true) => {
                if let Some(core) = window.try_state::<Core>() {
                    core.trigger_sync(SyncTrigger::WindowFocused);
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            today,
            last_used_view,
            set_last_used_view,
            sidebar,
            set_sidebar,
            settings,
            update_settings,
            list_accounts,
            add_caldav_account,
            available_calendars,
            add_calendars,
            remove_account,
            list_calendars,
            sync_statuses,
            send_before_quit,
            sync_now,
            create_calendar,
            rename_calendar,
            recolour_calendar,
            delete_calendar,
            remove_calendar,
            set_calendar_shown,
            set_calendar_read_only,
            show_only_calendar,
            list_occurrences,
            event,
            create_event,
            edit_event,
            reschedule_event,
            edit_occurrence,
            reschedule_occurrence,
            exception_counts,
            delete_event,
            delete_occurrence,
            pick_import_file,
            preview_import,
            run_import,
            discard_import,
            undo_journal_transaction,
            open_link,
            version,
            show_window,
            startup_failure,
            window_failed_to_start,
            quit,
        ])
        .build(tauri::generate_context!())
        .expect("FreeCal failed to start")
        .run(|_, event| {
            // The window closing never ends FreeCal by itself: the close
            // decides (`close_main_window`), and quitting goes through
            // `exit`, which gives a code.
            if let tauri::RunEvent::ExitRequested {
                code: None, api, ..
            } = event
            {
                api.prevent_exit();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What closing does with background running `on`, and whether a tray
    /// exists and Unsent Changes wait.
    fn closing_with(on: bool, tray: bool, unsent: bool) -> Closing {
        closing(false, on, || tray, || unsent)
    }

    #[test]
    fn with_a_tray_closing_keeps_freecal_running_in_the_background() {
        assert_eq!(closing_with(true, true, false), Closing::ToBackground);
        // The Unsent Changes are sent from the background.
        assert_eq!(closing_with(true, true, true), Closing::ToBackground);
    }

    #[test]
    fn without_a_tray_closing_quits() {
        assert_eq!(closing_with(true, false, false), Closing::Quit);
        assert_eq!(closing_with(true, false, true), Closing::AskToQuit);
    }

    #[test]
    fn with_background_running_off_closing_quits_without_asking_for_a_tray() {
        let closed = closing(false, false, || panic!("asked for a tray"), || false);

        assert_eq!(closed, Closing::Quit);
        assert_eq!(closing_with(false, true, true), Closing::AskToQuit);
    }

    #[test]
    fn as_the_session_ends_closing_quits_at_once() {
        let closed = closing(
            true,
            true,
            || panic!("asked for a tray"),
            || panic!("counted the Unsent Changes"),
        );

        assert_eq!(closed, Closing::Quit);
    }
}
