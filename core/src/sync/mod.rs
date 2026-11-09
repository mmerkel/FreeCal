//! The Sync Engine: it exchanges a Calendar's changes with its server, by
//! Incoming Syncs and Outgoing Syncs, and is the only code that changes the
//! Mirror's server data: it is a Guard, and the Local Store's writes of the
//! Mirror take a [`SyncEngineWritePermission`], which only this module can
//! make. Each Provider's part lives in a module of its own, CalDAV's in
//! [`caldav`] (ADR 0015); this module holds what every Provider and both
//! directions share: the entries that pick the Provider's Sync Engine
//! ([`incoming`], [`outgoing`]), the [`Recorder`] it opens Journal
//! Transactions through, and the Write Permission. A Calendar's
//! Event Rows with one UID belong to the UID's Corresponding Server Object,
//! and only that Server Object writes them (ADR 0015).

pub(crate) mod caldav;

use std::sync::Mutex;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::journal::{JournalTransaction, JournalTransactionId, TransactionOrigin};
use crate::remote::Remote;
use crate::store::{CalendarToSync, Store};
use crate::{ErrorCode, Provider, Result};

/// Runs a change as one Journal Transaction of a Transaction Origin and then
/// sends its Signals, as `Core::record` does, so that what follows a commit
/// stays in one place, `Core::signal`. Returns the Journal Transaction, none
/// when the change changed nothing. The Sync Engine opens every Journal
/// Transaction through it.
pub(crate) type Recorder<'a> = &'a dyn Fn(
    TransactionOrigin,
    &mut dyn FnMut(&mut JournalTransaction) -> Result<()>,
) -> Result<Option<JournalTransactionId>>;

/// Runs the Incoming Sync of `calendar` through its Account's server
/// `remote`, by the Sync Engine of the Account's Provider,
/// with Journal Transactions opened through `record`. The Sync Engine reads
/// its Known Server Resources from `store` and decides between a First Sync,
/// page by page while the Calendar holds nothing from its server yet, its
/// unfinished rest, and an incremental Incoming Sync. Floating times without
/// a zone of the Calendar's show in `display`. Returns the Calendar's
/// Journal Transaction, none for a First Sync, which the Change Journal
/// doesn't record (ADR 0009), or for one that changed nothing.
pub(crate) fn incoming(
    remote: &dyn Remote,
    store: &Mutex<Store>,
    calendar: &CalendarToSync,
    display: Tz,
    record: Recorder,
) -> Result<Option<JournalTransactionId>> {
    match remote.provider() {
        Provider::Local => unreachable!("the Local Provider has no server"),
        Provider::CalDav => caldav::incoming::sync(remote, store, calendar, display, record),
    }
}

/// Runs the Outgoing Sync of `calendar` through its Account's server
/// `remote`, by the Sender of the Account's Provider, which sends the
/// Calendar's Unsent Changes, with `now` as the time of revision of what
/// it writes, and records nothing in the Change Journal.
pub(crate) fn outgoing(
    remote: &dyn Remote,
    store: &Mutex<Store>,
    calendar: &CalendarToSync,
    server_read_only: bool,
    now: DateTime<Utc>,
) -> Result<()> {
    match remote.provider() {
        Provider::Local => unreachable!("the Local Provider has no server"),
        Provider::CalDav => caldav::sender::send(remote, store, calendar, server_read_only, now),
    }
}

/// Why the Sender's last try to send the Unsent Changes of a UID didn't
/// send them (new term: Send Failure), kept with them until a send
/// succeeds, for "Not yet synced: <reason>" (spec: "Event details", shown by
/// ticket 18). It covers only what is about the UID itself. What is about
/// its Account or Calendar is that one's Sync Status instead, and stores
/// none: the Account Offline, the Calendar's Sync Error (the server
/// refusing writes, a paused Calendar) or its First Sync unfinished. An
/// Unsent Change never tried has none either. The Local Store keeps it as
/// its JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum SendFailure {
    /// An update or a delete failed its precondition: the server holds a
    /// version FreeCal hasn't read yet, which the Incoming Sync brings in
    /// (a Conflict once tickets 20 and 21 raise them).
    ServerChanged,
    /// A create failed its precondition: something already is at the
    /// address of the new resource, maybe holding another UID.
    AddressTaken,
    /// An update or a delete waits until the server's version reads, as
    /// the Mirror holds no ETag to make it conditional on (ADR 0015's
    /// amendment).
    WaitingForServerVersion,
    /// The resource to send can't be built from the UID's Event Rows, as
    /// one has a time FreeCal can't read.
    Unbuildable,
    /// The request failed with `error`, as when the server refused it.
    Failed { error: ErrorCode },
}

impl SendFailure {
    /// The Send Failure as the Local Store keeps it, as JSON.
    pub(crate) fn stored(&self) -> String {
        serde_json::to_string(self).expect("a Send Failure always serializes")
    }

    /// Reads a stored Send Failure, untrusted: none when it can't be read,
    /// and an error code checked as a Sync Record's is
    /// ([`ErrorCode::checked`]).
    #[cfg(test)]
    pub(crate) fn from_stored(json: &str) -> Option<Self> {
        match serde_json::from_str(json).ok()? {
            SendFailure::Failed { error } => Some(SendFailure::Failed {
                error: error.checked()?,
            }),
            failure => Some(failure),
        }
    }
}

/// The Sync Engine's Write Permission (ADR 0015), which the writes of the
/// Mirror's server data and of the Ignored Resources take. Its field is
/// private to this module, so that nothing else can make one.
pub(crate) struct SyncEngineWritePermission(());

/// The [`SyncEngineWritePermission`] the Sync Engine's writes pass.
const WRITE_PERMISSION: &SyncEngineWritePermission = &SyncEngineWritePermission(());
