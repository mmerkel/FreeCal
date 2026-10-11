use chrono::NaiveDate;
use serde::Serialize;

use crate::CalendarId;

/// A message from the core saying that something changed.
///
/// Always called Signals, never "events" (calendar Events) or "notifications"
/// (desktop notifications). Signals are fire-and-forget. The v1 Signals are
/// added by the tickets that first send them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Signal {
    /// A change Signal: Accounts or Calendars were added, removed, renamed,
    /// recoloured, shown or hidden, or their writability changed. The
    /// frontend reads them again.
    CalendarsChanged,
    /// A change Signal: the Events of these Calendars changed, for any
    /// reason. The frontend reads the visible range again.
    #[serde(rename_all = "camelCase")]
    OccurrencesChanged { calendar_ids: Vec<CalendarId> },
    /// A status Signal: the local date changed, at midnight or on waking up
    /// on a new day. The frontend moves the today highlight, and jumps to
    /// the new today while it is tracking today.
    DateChanged { date: NaiveDate },
}

/// Receives the core's Signals, for example the Tauri shell forwarding them to
/// the frontend.
pub trait SignalSink: Send + Sync {
    fn send(&self, signal: Signal);
}
