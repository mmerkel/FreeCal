use serde::Serialize;

/// A message from the core saying that something changed.
///
/// Always called Signals, never "events" (calendar Events) or "notifications"
/// (desktop notifications). Signals are fire-and-forget. The v1 Signals are
/// added by the tickets that first send them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Signal {
    /// A change Signal: Accounts or Calendars were added, removed, renamed,
    /// recoloured, shown or hidden, or their writability changed. The
    /// frontend reads them again.
    CalendarsChanged,
}

/// Receives the core's Signals, for example the Tauri shell forwarding them to
/// the frontend.
pub trait SignalSink: Send + Sync {
    fn send(&self, signal: Signal);
}
