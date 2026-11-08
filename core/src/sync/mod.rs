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

use crate::journal::{JournalTransaction, JournalTransactionId, TransactionOrigin};
use crate::remote::Remote;
use crate::store::{CalendarToSync, Store};
use crate::{Provider, Result};

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
    now: DateTime<Utc>,
) -> Result<()> {
    match remote.provider() {
        Provider::Local => unreachable!("the Local Provider has no server"),
        Provider::CalDav => caldav::sender::send(remote, store, calendar, now),
    }
}

/// The Sync Engine's Write Permission (ADR 0015), which the writes of the
/// Mirror's server data and of the Ignored Resources take. Its field is
/// private to this module, so that nothing else can make one.
pub(crate) struct SyncEngineWritePermission(());

/// The [`SyncEngineWritePermission`] the Sync Engine's writes pass.
const WRITE_PERMISSION: &SyncEngineWritePermission = &SyncEngineWritePermission(());
