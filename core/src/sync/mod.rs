//! The Sync Engine: it exchanges a Calendar's changes with its server, by
//! Incoming Syncs ([`incoming`]) and, with ticket 17, Outgoing Syncs, and is
//! the only code that changes the Mirror's server data: it is a Guard, and
//! the Local Store's writes of the Mirror take a
//! [`SyncEngineWritePermission`], which only this module can make. This
//! module holds what both directions share. A Calendar's Event Rows with one
//! UID belong to the UID's Corresponding Server Object, and only that Server
//! Object writes them (ADR 0011).

pub(crate) mod incoming;

use chrono_tz::Tz;

use crate::event::{self, Loaded, ObjectId, ServerObject};
use crate::import::read_server_objects;
use crate::journal::JournalTransaction;
use crate::store::EventKeyInCalendar;
use crate::{CalendarId, ImportedEvent, Result};

/// The Sync Engine's Write Permission (ADR 0011), which the writes of the
/// Mirror's server data take. Its field is private to this module, so that
/// nothing else can make one.
pub(crate) struct SyncEngineWritePermission(());

/// The [`SyncEngineWritePermission`] this module's writes pass.
const WRITE_PERMISSION: &SyncEngineWritePermission = &SyncEngineWritePermission(());

/// Makes `object`, as the server holds it now, the Mirror's version of the
/// Server Object `over` in a Calendar, address included, or of a new one,
/// and returns its ID. Its Combined Event Keys become `event_keys`, one per
/// component, in the Calendar the Server Object is in. Written in the
/// Database Transaction of the Incoming Sync's `journal_transaction`, as only
/// an Incoming Sync knows what the server holds.
fn mirror(
    journal_transaction: &JournalTransaction,
    calendar_id: CalendarId,
    object: &ServerObject,
    over: Option<ObjectId>,
    event_keys: &[EventKeyInCalendar],
) -> Result<ObjectId> {
    journal_transaction.writable(calendar_id)?;
    journal_transaction.database_transaction().write_object(
        WRITE_PERMISSION,
        calendar_id,
        object,
        over,
        event_keys,
    )
}

/// Records that FreeCal couldn't read what the server holds now at `href` in
/// a Calendar: the Server Objects there keep the last version it could with
/// their Combined Event Keys, and their Event Rows stay as they are.
fn unreadable(
    journal_transaction: &JournalTransaction,
    calendar_id: CalendarId,
    href: &str,
) -> Result<()> {
    journal_transaction
        .database_transaction()
        .mark_unreadable(WRITE_PERMISSION, calendar_id, href)
}

/// Removes the Server Object `id` from the Mirror with its Combined Event
/// Keys, as an Incoming Sync finds it gone from the server. The Event Rows
/// of the UIDs it was the Corresponding Server Object of stay, for the
/// Incoming Sync to give to their next Corresponding Server Object or
/// delete.
fn remove_object(journal_transaction: &JournalTransaction, id: ObjectId) -> Result<()> {
    journal_transaction
        .database_transaction()
        .delete_object(WRITE_PERMISSION, id)
}

/// Gives the Event Rows of each of `uids`, UIDs of the Calendar
/// `calendar_id` whose Corresponding Server Object went, to their next
/// Corresponding Server Object: its Events of the UID, read from its
/// mirrored data with floating times pinned to `zone`, take the Event Rows'
/// place as [`event::take_object`] writes them. A UID no Server Object holds
/// any more loses its Event Rows. Should the mirrored data no longer read, the
/// Event Rows stay as they are, as for a Server Object FreeCal can't
/// read.
fn settle(
    journal_transaction: &mut JournalTransaction,
    calendar_id: CalendarId,
    uids: &[String],
    zone: Tz,
) -> Result<()> {
    for uid in uids {
        let Some(corresponding_server_object) =
            journal_transaction.corresponding_server_object(calendar_id, uid)?
        else {
            Loaded::of_uid(journal_transaction, calendar_id, uid)?
                .delete_all(journal_transaction)?;
            continue;
        };
        let Some(object) = journal_transaction.server_object(corresponding_server_object)? else {
            continue;
        };
        let read = read_server_objects(vec![object], zone);
        let Some(events) = &read[0].events else {
            continue;
        };
        let events: Vec<ImportedEvent> = events
            .iter()
            .filter(|event| event.uid == *uid)
            .cloned()
            .collect();
        event::take_object(
            journal_transaction,
            calendar_id,
            corresponding_server_object,
            &events,
        )?;
    }
    Ok(())
}

/// The Combined Event Key of each of `events`, in the Calendar of the Server
/// Object holding them (ADR 0011), its recurrence ID as obtained (ADR 0012).
fn event_keys_in_calendar(events: &[ImportedEvent]) -> Vec<EventKeyInCalendar> {
    events
        .iter()
        .map(|event| EventKeyInCalendar::new(&event.uid, event.recurrence_id().as_ref()))
        .collect()
}

/// The Sync Engine's Mirror writes, for other modules' tests to set the
/// Mirror up with.
#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    /// [`mirror`](super::mirror).
    pub(crate) fn mirror(
        journal_transaction: &JournalTransaction,
        calendar_id: CalendarId,
        object: &ServerObject,
        over: Option<ObjectId>,
        event_keys: &[EventKeyInCalendar],
    ) -> Result<ObjectId> {
        super::mirror(journal_transaction, calendar_id, object, over, event_keys)
    }

    /// [`remove_object`](super::remove_object).
    pub(crate) fn remove_object(
        journal_transaction: &JournalTransaction,
        id: ObjectId,
    ) -> Result<()> {
        super::remove_object(journal_transaction, id)
    }
}
