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
use crate::{CalendarId, ImportedEvent, Result};

/// What a CalDAV Server Object holding `events`, which are not empty, holds
/// of its UID (ADR 0013): the UID, and its Events, which are none when the
/// Server Object holds several UIDs, as RFC 4791 §4.1 forbids. Such a Server
/// Object is mirrored with the UID of its first Event as
/// [`read_server_objects`] lists them, Events that aren't Exceptions first,
/// and its Events aren't shown.
fn of_its_uid(events: &[ImportedEvent]) -> (&str, &[ImportedEvent]) {
    let uid = events[0].uid.as_str();
    if events.iter().all(|event| event.uid == uid) {
        (uid, events)
    } else {
        (uid, &[])
    }
}

/// Whether `events`, a CalDAV Server Object's Events of its UID
/// ([`of_its_uid`]), hold the Event that isn't an Exception: the single
/// Event or the Recurring Event.
fn holds_non_exception(events: &[ImportedEvent]) -> bool {
    events.iter().any(|event| event.recurrence_id().is_none())
}

/// Gives the Event Rows of `uid`, a UID of the Calendar `calendar_id` whose
/// Corresponding Server Object went, to its next Corresponding Server
/// Object, which is propagated to them as [`event::propagate_to_event_rows`]
/// writes them: its Events of the UID ([`of_its_uid`]), read from its
/// mirrored data with floating times pinned to `zone`. A UID no Server
/// Object holds any more loses its Event Rows. Should the mirrored data no
/// longer read, the Event Rows stay as they are, as for a Server Object
/// FreeCal can't read.
fn settle(
    journal_transaction: &mut JournalTransaction,
    calendar_id: CalendarId,
    uid: &str,
    zone: Tz,
) -> Result<()> {
    let Some(corresponding_server_object) =
        journal_transaction.corresponding_server_object(calendar_id, uid)?
    else {
        return Loaded::of_uid(journal_transaction, calendar_id, uid)?
            .delete_all(journal_transaction);
    };
    let Some(object) = journal_transaction.server_object(corresponding_server_object)? else {
        return Ok(());
    };
    let read = read_server_objects(vec![object], zone);
    let Some(events) = read[0]
        .events
        .as_deref()
        .filter(|events| !events.is_empty())
    else {
        return Ok(());
    };
    let (_, events) = of_its_uid(events);
    event::propagate_to_event_rows(
        journal_transaction,
        calendar_id,
        corresponding_server_object,
        uid,
        events,
    )
}

/// The CalDAV Server Object of `moved_from` that holds `uid`, taken out of
/// it; none if there is none.
fn moved(
    journal_transaction: &JournalTransaction,
    uid: &str,
    moved_from: &mut Vec<ObjectId>,
) -> Result<Option<ObjectId>> {
    for (index, &object) in moved_from.iter().enumerate() {
        if journal_transaction.held_uid(object)?.as_deref() == Some(uid) {
            return Ok(Some(moved_from.remove(index)));
        }
    }
    Ok(None)
}

/// Makes `object`, as the server holds it now, the Mirror's version of the
/// CalDAV Server Object `over` in a Calendar, address included, or of a new
/// one, and returns its ID. It holds the UID `uid`, and the Event of it
/// that isn't an Exception when `holds_non_exception`. Written in the
/// Database Transaction of the Incoming Sync's `journal_transaction`, as
/// only an Incoming Sync knows what the server holds.
fn mirror(
    journal_transaction: &JournalTransaction,
    calendar_id: CalendarId,
    object: &ServerObject,
    over: Option<ObjectId>,
    uid: &str,
    holds_non_exception: bool,
) -> Result<ObjectId> {
    journal_transaction.writable(calendar_id)?;
    journal_transaction.database_transaction().write_object(
        WRITE_PERMISSION,
        calendar_id,
        object,
        over,
        uid,
        holds_non_exception,
    )
}

/// Records that FreeCal couldn't read what the server holds now at `href` in
/// a Calendar: the Server Objects there keep the last version it could, and
/// their Event Rows stay as they are.
fn unreadable(
    journal_transaction: &JournalTransaction,
    calendar_id: CalendarId,
    href: &str,
) -> Result<()> {
    journal_transaction
        .database_transaction()
        .mark_unreadable(WRITE_PERMISSION, calendar_id, href)
}

/// Removes the Server Object `id` from the Mirror, as an Incoming Sync
/// finds it gone from the server. The Event Rows of the UID it was the
/// Corresponding Server Object of stay, for the Incoming Sync to give to
/// their next Corresponding Server Object or delete.
fn remove_object(journal_transaction: &JournalTransaction, id: ObjectId) -> Result<()> {
    journal_transaction
        .database_transaction()
        .delete_object(WRITE_PERMISSION, id)
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
        uid: &str,
        holds_non_exception: bool,
    ) -> Result<ObjectId> {
        super::mirror(
            journal_transaction,
            calendar_id,
            object,
            over,
            uid,
            holds_non_exception,
        )
    }

    /// [`remove_object`](super::remove_object).
    pub(crate) fn remove_object(
        journal_transaction: &JournalTransaction,
        id: ObjectId,
    ) -> Result<()> {
        super::remove_object(journal_transaction, id)
    }
}
