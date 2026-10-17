//! The Tauri shell: exposes the core application interface to the frontend as
//! commands and forwards the core's Signals as Tauri events.

#[cfg(target_os = "linux")]
mod handoff;

use std::sync::Arc;
use std::time::Duration;

use chrono::NaiveDate;
use freecal_core::{
    Account, AccountId, Calendar, CalendarId, Colour, Core, ErrorCode, Event, EventDraft, EventId,
    Occurrence, Settings, SettingsUpdate, SidebarState, Signal, SignalSink, SystemClock, View,
    When, link_allowed,
};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
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

/// Why the core couldn't open its Local Store. Managed only then, in place
/// of the [`Core`], and the window shows it on the startup error screen.
struct StartupFailure(ErrorCode);

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

#[tauri::command]
fn update_settings(core: State<Core>, update: SettingsUpdate) -> CommandResult<Settings> {
    core.update_settings(update).map_err(ErrorCode::from)
}

#[tauri::command]
fn list_accounts(core: State<Core>) -> CommandResult<Vec<Account>> {
    core.list_accounts().map_err(ErrorCode::from)
}

#[tauri::command]
fn list_calendars(core: State<Core>) -> CommandResult<Vec<Calendar>> {
    core.list_calendars().map_err(ErrorCode::from)
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
fn set_calendar_shown(core: State<Core>, id: i64, shown: bool) -> CommandResult<()> {
    core.set_calendar_shown(CalendarId(id), shown)
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
fn create_event(core: State<Core>, draft: EventDraft) -> CommandResult<Event> {
    core.create_event(draft).map_err(ErrorCode::from)
}

#[tauri::command]
fn edit_event(core: State<Core>, id: EventId, draft: EventDraft) -> CommandResult<Event> {
    core.edit_event(id, draft).map_err(ErrorCode::from)
}

#[tauri::command]
fn reschedule_event(core: State<Core>, id: EventId, when: When) -> CommandResult<Event> {
    core.reschedule_event(id, when).map_err(ErrorCode::from)
}

#[tauri::command]
fn delete_event(core: State<Core>, id: EventId) -> CommandResult<()> {
    core.delete_event(id).map_err(ErrorCode::from)
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
    app.try_state::<StartupFailure>().map(|failure| failure.0)
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

/// Brings the window to the front for a second launch. Its activation
/// token is what lets the compositor raise the window; without it KWin on
/// Wayland only marks FreeCal in the taskbar.
#[cfg(target_os = "linux")]
fn bring_to_front(app: &AppHandle, activation_token: Option<String>) {
    use gtk::prelude::*;

    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // The XDG data directory, e.g. ~/.local/share/io.github.mmerkel.FreeCal (ADR 0004, 0006).
            let data_dir = app.path().app_data_dir()?;
            // Before the core opens, so that a second launch hands off
            // rather than failing with `LocalStoreInUse`.
            #[cfg(target_os = "linux")]
            hand_off_or_serve(app, &data_dir);
            let signals = Arc::new(ForwardToFrontend(app.handle().clone()));
            match Core::open(&data_dir, Arc::new(SystemClock), signals) {
                Ok(core) => {
                    app.manage(core);
                    let handle = app.handle().clone();
                    std::thread::spawn(move || {
                        loop {
                            std::thread::sleep(DATE_CHECK_PERIOD);
                            handle.state::<Core>().check_date();
                        }
                    });
                }
                Err(error) => {
                    eprintln!("FreeCal could not open its Local Store: {error}");
                    app.manage(StartupFailure(error.code()));
                }
            }
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(SHOW_WINDOW_FALLBACK);
                let _ = show_main_window(&handle);
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the startup error screen quits FreeCal, which has
            // nothing to keep running for.
            if let WindowEvent::CloseRequested { .. } = event
                && window.try_state::<StartupFailure>().is_some()
            {
                window.app_handle().exit(0);
            }
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
            reschedule_event,
            delete_event,
            open_link,
            version,
            show_window,
            startup_failure,
            quit,
        ])
        .run(tauri::generate_context!())
        .expect("FreeCal failed to start");
}
