//! The Tauri shell: exposes the core application interface to the frontend as
//! commands and forwards the core's Signals as Tauri events.

use std::sync::Arc;
use std::time::Duration;

use chrono::NaiveDate;
use freecal_core::{
    Account, AccountId, Calendar, CalendarId, Colour, Core, CoreError, Event, EventDraft, EventId,
    Occurrence, Settings, SettingsUpdate, SidebarState, Signal, SignalSink, SystemClock, View,
    link_allowed,
};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

/// The Tauri event on which Signals reach the frontend.
const SIGNAL_EVENT: &str = "freecal://signal";

/// How often the core checks whether the local date changed. A sleep of the
/// thread doesn't count the time the computer was suspended, so a short
/// period also catches waking up on a new day soon.
const DATE_CHECK_PERIOD: Duration = Duration::from_secs(10);

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

type CommandResult<T> = Result<T, String>;

fn to_message(error: CoreError) -> String {
    error.to_string()
}

#[tauri::command]
fn today(core: State<Core>) -> String {
    core.today().to_string()
}

#[tauri::command]
fn last_used_view(core: State<Core>) -> CommandResult<View> {
    core.last_used_view().map_err(to_message)
}

#[tauri::command]
fn set_last_used_view(core: State<Core>, view: View) -> CommandResult<()> {
    core.set_last_used_view(view).map_err(to_message)
}

#[tauri::command]
fn sidebar(core: State<Core>) -> CommandResult<SidebarState> {
    core.sidebar().map_err(to_message)
}

#[tauri::command]
fn set_sidebar(core: State<Core>, sidebar: SidebarState) -> CommandResult<()> {
    core.set_sidebar(sidebar).map_err(to_message)
}

#[tauri::command]
fn settings(core: State<Core>) -> CommandResult<Settings> {
    core.settings().map_err(to_message)
}

#[tauri::command]
fn update_settings(core: State<Core>, update: SettingsUpdate) -> CommandResult<Settings> {
    core.update_settings(update).map_err(to_message)
}

#[tauri::command]
fn list_accounts(core: State<Core>) -> CommandResult<Vec<Account>> {
    core.list_accounts().map_err(to_message)
}

#[tauri::command]
fn list_calendars(core: State<Core>) -> CommandResult<Vec<Calendar>> {
    core.list_calendars().map_err(to_message)
}

#[tauri::command]
fn create_calendar(
    core: State<Core>,
    account_id: i64,
    name: String,
    colour: String,
) -> CommandResult<Calendar> {
    let colour = Colour::parse(&colour).map_err(to_message)?;
    core.create_calendar(AccountId(account_id), &name, colour)
        .map_err(to_message)
}

#[tauri::command]
fn rename_calendar(core: State<Core>, id: i64, name: String) -> CommandResult<()> {
    core.rename_calendar(CalendarId(id), &name)
        .map_err(to_message)
}

#[tauri::command]
fn recolour_calendar(core: State<Core>, id: i64, colour: String) -> CommandResult<()> {
    let colour = Colour::parse(&colour).map_err(to_message)?;
    core.recolour_calendar(CalendarId(id), colour)
        .map_err(to_message)
}

#[tauri::command]
fn delete_calendar(core: State<Core>, id: i64) -> CommandResult<()> {
    core.delete_calendar(CalendarId(id)).map_err(to_message)
}

#[tauri::command]
fn set_calendar_shown(core: State<Core>, id: i64, shown: bool) -> CommandResult<()> {
    core.set_calendar_shown(CalendarId(id), shown)
        .map_err(to_message)
}

#[tauri::command]
fn show_only_calendar(core: State<Core>, id: i64) -> CommandResult<()> {
    core.show_only_calendar(CalendarId(id)).map_err(to_message)
}

#[tauri::command]
fn list_occurrences(
    core: State<Core>,
    from: NaiveDate,
    to: NaiveDate,
) -> CommandResult<Vec<Occurrence>> {
    core.occurrences(from, to).map_err(to_message)
}

#[tauri::command]
fn event(core: State<Core>, id: EventId) -> CommandResult<Event> {
    core.event(id).map_err(to_message)
}

#[tauri::command]
fn create_event(core: State<Core>, draft: EventDraft) -> CommandResult<Event> {
    core.create_event(draft).map_err(to_message)
}

#[tauri::command]
fn edit_event(core: State<Core>, id: EventId, draft: EventDraft) -> CommandResult<Event> {
    core.edit_event(id, draft).map_err(to_message)
}

#[tauri::command]
fn delete_event(core: State<Core>, id: EventId) -> CommandResult<()> {
    core.delete_event(id).map_err(to_message)
}

/// Opens a link from a description or location in the system browser. The scheme
/// is checked again here, so that the window can't open anything else even if
/// it asks to.
#[tauri::command]
fn open_link(app: AppHandle, url: String) -> CommandResult<()> {
    if !link_allowed(&url) {
        return Err(format!(
            "FreeCal opens only http, https and mailto links: {url:?}"
        ));
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|error| error.to_string())
}

/// FreeCal's version, for the About dialog.
#[tauri::command]
fn version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

/// Shows the window, which starts hidden.
#[tauri::command]
fn show_window(app: AppHandle) -> CommandResult<()> {
    show_main_window(&app).map_err(|error| error.to_string())
}

/// Ends FreeCal, also when it would otherwise keep running in the background.
#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // The XDG data directory, e.g. ~/.local/share/io.github.mmerkel.FreeCal (ADR 0004, 0006).
            let data_dir = app.path().app_data_dir()?;
            let signals = Arc::new(ForwardToFrontend(app.handle().clone()));
            let core = Core::open(&data_dir, Arc::new(SystemClock), signals)?;
            app.manage(core);
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(DATE_CHECK_PERIOD);
                    handle.state::<Core>().check_date();
                }
            });
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(SHOW_WINDOW_FALLBACK);
                let _ = show_main_window(&handle);
            });
            Ok(())
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
            list_calendars,
            create_calendar,
            rename_calendar,
            recolour_calendar,
            delete_calendar,
            set_calendar_shown,
            show_only_calendar,
            list_occurrences,
            event,
            create_event,
            edit_event,
            delete_event,
            open_link,
            version,
            show_window,
            quit,
        ])
        .run(tauri::generate_context!())
        .expect("FreeCal failed to start");
}
