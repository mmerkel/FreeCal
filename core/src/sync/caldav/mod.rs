//! The CalDAV part of the Sync Engine (ADR 0013): CalDAV's Incoming Sync
//! ([`incoming`]), its writes of the Mirror, and the rules that read the
//! Mirror for it: handing a UID's Event Rows to its next Corresponding
//! Server Object ([`settle`]) and recognising a Server Object that moved
//! ([`moved`]).

pub(crate) mod incoming;

use chrono_tz::Tz;

use super::WRITE_PERMISSION;
use crate::event::{self, Loaded, ObjectId, ServerObject};
use crate::import::read_server_objects;
use crate::journal::JournalTransaction;
use crate::store::EventKeyInCalendar;
use crate::{CalendarId, ImportedEvent, Result};

/// Gives the Event Rows of each of `uids`, UIDs of the Calendar
/// `calendar_id` whose Corresponding Server Object went, to their next
/// Corresponding Server Object, which is propagated to them as
/// [`event::propagate_to_event_rows`] writes them: its Events of the UID,
/// read from its mirrored data with floating times pinned to `zone`. A UID
/// no Server Object holds any more loses its Event Rows. Should the
/// mirrored data no longer read, the Event Rows stay as they are, as for a
/// Server Object FreeCal can't read.
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
        event::propagate_to_event_rows(
            journal_transaction,
            calendar_id,
            corresponding_server_object,
            &events,
        )?;
    }
    Ok(())
}

/// The Server Object of `moved_from` that holds a UID of `events`, by its
/// Combined Event Keys, taken out of it; none if there is none.
fn moved(
    journal_transaction: &JournalTransaction,
    events: &[ImportedEvent],
    moved_from: &mut Vec<ObjectId>,
) -> Result<Option<ObjectId>> {
    for (index, &object) in moved_from.iter().enumerate() {
        let held = journal_transaction.held_uids(object)?;
        if events.iter().any(|event| held.contains(&event.uid)) {
            return Ok(Some(moved_from.remove(index)));
        }
    }
    Ok(None)
}

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
