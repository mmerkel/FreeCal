//! The Tauri shell: exposes the core application interface to the frontend as
//! commands and forwards the core's Signals as Tauri events.

use std::sync::Arc;

use chrono::NaiveDate;
use freecal_core::{
    Account, AccountId, Calendar, CalendarId, Colour, Core, CoreError, Event, EventDraft, EventId,
    Occurrence, Signal, SignalSink, SystemClock, link_allowed,
};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

/// The Tauri event on which Signals reach the frontend.
const SIGNAL_EVENT: &str = "freecal://signal";

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

/// Opens a link from a description in the system browser. The scheme is
/// checked again here, so that the window can't open anything else even if
/// it asks to.
#[tauri::command]
fn open_link(app: AppHandle, url: String) -> CommandResult<()> {
    if !link_allowed(&url) {
        return Err(format!("FreeCal opens only http, https and mailto links: {url:?}"));
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|error| error.to_string())
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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            today,
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
        ])
        .run(tauri::generate_context!())
        .expect("FreeCal failed to start");
}
