//! The Sync Engine: it exchanges a Calendar's changes with its server, by
//! Incoming Syncs and Outgoing Syncs, and is the only code that changes the
//! Mirror's server data: it is a Guard, and the Local Store's writes of the
//! Mirror take a [`SyncEngineWritePermission`], which only this module can
//! make. Each Provider's part lives in a module of its own, CalDAV's in
//! [`caldav`] (ADR 0015); this module holds what every Provider and both
//! directions share: the entry that syncs a Calendar both ways and picks the
//! Provider's Sync Engine ([`sync_calendar`]), the [`Recorder`] it opens
//! Journal Transactions through, and the Write Permission. A Calendar's
//! Event Rows with one UID belong to the UID's Corresponding Server Object,
//! and only that Server Object writes them (ADR 0015).

pub(crate) mod caldav;

use std::sync::Mutex;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::journal::{JournalTransaction, JournalTransactionId, TransactionOrigin};
use crate::remote::Remote;
use crate::store::{CalendarToSync, Store};
use crate::sync_status::{StartedBy, about_account};
use crate::{Clock, CoreError, ErrorCode, Provider, Result};

/// Runs a change as one Journal Transaction of a Transaction Origin and then
/// sends its Signals, as `Core::record` does, so that what follows a commit
/// stays in one place, `Core::signal`. Returns the Journal Transaction, none
/// when the change changed nothing. The Sync Engine opens every Journal
/// Transaction through it.
pub(crate) type Recorder<'a> = &'a dyn Fn(
    TransactionOrigin,
    &mut dyn FnMut(&mut JournalTransaction) -> Result<()>,
) -> Result<Option<JournalTransactionId>>;

/// What syncing a Calendar both ways ([`sync_calendar`]) came to, from
/// which the core sends its Signals (new term: Calendar Sync Result).
/// Server read-only may have changed and the sync still failed, as when the
/// server refused a write.
pub(crate) struct CalendarSyncResult {
    /// How the sync ended: the Journal Transaction of the Calendar's
    /// Incoming Sync, none for a First Sync, which the Change Journal
    /// doesn't record (ADR 0009), or for one that changed nothing. When the
    /// Outgoing Sync failed but the Incoming Sync ran, its Journal
    /// Transaction is committed but the failure is returned.
    pub outcome: Result<Option<JournalTransactionId>>,
    /// Whether the Calendar's Server read-only changed, which "Calendars
    /// changed" tells.
    pub server_read_only_changed: bool,
}

/// Syncs `calendar` both ways through its Account's server `remote`, by the
/// Sync Engine of the Account's Provider, in a sync started by `started_by`,
/// the launch sync when `at_launch`. Journal Transactions open through
/// `record`; `clock` gives the time of revision of what the Outgoing Sync
/// writes, and the Display Time Zone, where floating times without a zone
/// of the Calendar's show.
///
/// First its Outgoing Sync sends its Unsent Changes, then its Incoming Sync
/// brings in what changed on the server, deciding from its Known Server
/// Resources between a First Sync, page by page while the Calendar holds
/// nothing from its server yet, its unfinished rest, and an incremental
/// Incoming Sync. Sending first gets the user's changes to the server
/// soonest, and the Incoming Sync right after fetches what the server
/// stored of a write answered without an ETag, and finds what failed a
/// precondition. The price: a server change since the last sync makes a
/// write fail its precondition, a request in vain, where an Incoming Sync
/// first would have read the new version beforehand. A failure of the
/// Outgoing Sync about the whole Account (see [`about_account`]) stops the
/// sync; any other still lets the Incoming Sync run, and is the outcome
/// once it ran, so that the Calendar shows a Sync Error. The Outgoing Sync
/// records nothing in the Change Journal.
///
/// A Server read-only Calendar sends nothing: its Unsent Changes fail with
/// [`CoreError::WritesRefused`] without a request, the Calendar's Sync
/// Error, as the server would refuse them. The user's Read-only mark holds
/// none back: it only stops new changes, so those made before it are sent
/// (ticket 130). Those of a Calendar whose First Sync is unfinished wait,
/// as the Mirror doesn't yet hold every Server Object.
///
/// Whether the server lets the user change the Calendar, Server read-only,
/// is read when it is added, and again only when FreeCal has a reason to
/// think it changed (ticket 130), so that a Poll costs no request for it. A
/// write the server refuses reads it again, to notice a revoked permission;
/// the refusal stays the outcome unless that reading fails about the whole
/// Account. A Server read-only Calendar sends no writes, so nothing tells of
/// a permission granted again: a sync the user starts and the launch sync
/// read it again before sending, once its First Sync is finished, as only
/// then could it send. Trade-off, named: this rule exists because
/// reading a CalDAV Calendar's privileges costs one request per Calendar,
/// so a CalDAV cost rule lives in Provider-neutral code; with one Provider,
/// a rule per Provider would be a seam for a hypothetical (ticket 26 may
/// move it into the Provider).
pub(crate) fn sync_calendar(
    remote: &dyn Remote,
    store: &Mutex<Store>,
    calendar: &CalendarToSync,
    started_by: StartedBy,
    at_launch: bool,
    clock: &dyn Clock,
    record: Recorder,
) -> CalendarSyncResult {
    let mut server_read_only_changed = false;
    let outcome = sync_both_ways(
        remote,
        store,
        calendar,
        at_launch || started_by == StartedBy::User,
        clock,
        record,
        &mut server_read_only_changed,
    );
    CalendarSyncResult {
        outcome,
        server_read_only_changed,
    }
}

/// The body of [`sync_calendar`], which looks for grants when
/// `look_for_grants` and sets `server_read_only_changed` when a reading
/// changed Server read-only, even when the sync then fails.
fn sync_both_ways(
    remote: &dyn Remote,
    store: &Mutex<Store>,
    calendar: &CalendarToSync,
    look_for_grants: bool,
    clock: &dyn Clock,
    record: Recorder,
    server_read_only_changed: &mut bool,
) -> Result<Option<JournalTransactionId>> {
    let (calendar_id, _, ref state) = *calendar;
    let mut server_read_only = store
        .lock()
        .unwrap()
        .read_only(calendar_id)?
        .server_read_only();
    // A Calendar in its First Sync sends nothing, so a grant changes
    // nothing yet, and adding it read its privileges just now.
    if server_read_only && look_for_grants && state.is_some() {
        server_read_only =
            read_server_read_only(remote, store, calendar, server_read_only_changed)?;
    }
    let sent = if state.is_none() {
        // Its First Sync is unfinished: its Unsent Changes wait, as the
        // Mirror doesn't yet hold every Server Object, so a create could
        // meet a UID the server holds.
        Ok(())
    } else if server_read_only {
        if store.lock().unwrap().unsent_uids(calendar_id)?.is_empty() {
            Ok(())
        } else {
            Err(CoreError::WritesRefused)
        }
    } else {
        let now = clock.now().with_timezone(&Utc);
        let sent = match remote.provider() {
            Provider::Local => unreachable!("the Local Provider has no server"),
            Provider::CalDav => caldav::sender::send(remote, store, calendar, now),
        };
        match sent {
            // Refused by the server: has a permission been revoked?
            Err(CoreError::WritesRefused) => {
                if let Err(error) =
                    read_server_read_only(remote, store, calendar, server_read_only_changed)
                {
                    if about_account(&error) {
                        return Err(error);
                    }
                    log::warn!(
                        "Reading the privileges of Calendar {} failed: {error}",
                        calendar_id.0
                    );
                }
                Err(CoreError::WritesRefused)
            }
            sent => sent,
        }
    };
    if let Err(error) = &sent
        && about_account(error)
    {
        return sent.map(|()| None);
    }
    let journal_transaction = match remote.provider() {
        Provider::Local => unreachable!("the Local Provider has no server"),
        Provider::CalDav => {
            caldav::incoming::sync(remote, store, calendar, clock.time_zone(), record)?
        }
    };
    sent.map(|()| journal_transaction)
}

/// Reads from the server `remote` whether it lets the user change
/// `calendar`, keeps it as its Server read-only, sets `changed` when that
/// changed, and returns it.
fn read_server_read_only(
    remote: &dyn Remote,
    store: &Mutex<Store>,
    &(calendar_id, ref url, _): &CalendarToSync,
    changed: &mut bool,
) -> Result<bool> {
    let server_read_only = remote.server_read_only(url)?;
    *changed |= store.lock().unwrap().write(|database_transaction| {
        database_transaction.set_server_read_only(calendar_id, server_read_only)
    })?;
    Ok(server_read_only)
}

/// Why the Sender's last try to send the Unsent Changes of a UID didn't
/// send them (new term: Send Failure), kept with them until a send
/// succeeds, for "Not yet synced: <reason>" (spec: "Event details"). It
/// covers only what is about the UID itself. What is about
/// its Account or Calendar is that one's Sync Status instead, and stores
/// none: the Account Offline, the Calendar's Sync Error (the server
/// refusing writes, a paused Calendar) or its First Sync unfinished. An
/// Unsent Change never tried has none either. The Local Store keeps it as
/// its JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SendFailure {
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
