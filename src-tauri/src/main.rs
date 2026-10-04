//! The Tauri shell: exposes the core application interface to the frontend as
//! commands and forwards the core's Signals as Tauri events.

use std::sync::Arc;

use freecal_core::{
    Account, AccountId, Calendar, CalendarId, Colour, Core, CoreError, Signal, SignalSink,
    SystemClock,
};
use tauri::{AppHandle, Emitter, Manager, State};

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

fn main() {
    tauri::Builder::default()
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
        ])
        .run(tauri::generate_context!())
        .expect("FreeCal failed to start");
}
