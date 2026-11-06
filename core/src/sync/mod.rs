//! The Sync Engine: it exchanges a Calendar's changes with its server, by
//! Incoming Syncs ([`incoming`]) and, with ticket 17, Outgoing Syncs, and is
//! the only code that changes the Mirror's server data. This module holds
//! what both directions share. A Calendar's Event Rows with one UID belong
//! to the UID's Corresponding Server Object, and only that Server Object
//! writes them (ADR 0011).

pub(crate) mod incoming;

use std::collections::HashMap;

use chrono_tz::Tz;

use crate::event::{self, Loaded, ObjectEvent};
use crate::import::read_server_objects;
use crate::journal::JournalTransaction;
use crate::remote::Fetched;
use crate::store::EventKeyInCalendar;
use crate::{CalendarId, ImportedEvent, ImportedWhen, Result};

/// Gives the Event Rows of each of `uids`, UIDs of the Calendar
/// `calendar_id` whose Corresponding Server Object went, to their next
/// Corresponding Server Object: its Events of the UID, read from its
/// mirrored data with floating times pinned to `zone`, take the Event Rows'
/// place as [`event::take_object`] writes them. Their recurrence IDs are
/// normalised against the Recurring Event it holds, or are in the zone they
/// were given in if it holds none, as if it had come alone: the Event Rows
/// of the Server Object that went don't count, and its Combined Event Keys
/// of the UID are written again to match. A UID no Server Object holds any
/// more loses its Event Rows. Should the mirrored data no longer read, the
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
        let recurring = recurring_of(&read);
        let mut events = object_events(events, |uid| Ok(recurring.get(uid).copied()))?;
        events.retain(|event| event.uid == uid);
        journal_transaction.rewrite_combined_event_keys(
            corresponding_server_object,
            uid,
            &event_keys_in_calendar(&events),
        )?;
        event::take_object(
            journal_transaction,
            calendar_id,
            corresponding_server_object,
            &events,
        )?;
    }
    Ok(())
}

/// The time of the first Recurring Event in `fetched` with each UID.
fn recurring_of(fetched: &[Fetched]) -> HashMap<&str, ImportedWhen> {
    let mut recurring = HashMap::new();
    for event in fetched
        .iter()
        .filter_map(|one| one.events.as_deref())
        .flatten()
    {
        if event.original.is_none() && event.rule.is_some() {
            recurring.entry(event.uid.as_str()).or_insert(event.when);
        }
    }
    recurring
}

/// `events`, those of a Server Object, as it holds them: an Exception
/// with the recurrence ID of the Occurrence it changed of its Recurring
/// Event, the one `recurring` gives for its UID. One without its Recurring
/// Event is an Event of its own, its recurrence ID in the zone it was given
/// in (ticket 88).
fn object_events<'a>(
    events: &'a [ImportedEvent],
    recurring: impl Fn(&str) -> Result<Option<ImportedWhen>>,
) -> Result<Vec<ObjectEvent<'a>>> {
    let mut kept = Vec::new();
    for event in events {
        let rid = match event.original {
            None => None,
            Some(_) => event.recurrence_id_in(recurring(&event.uid)?.as_ref()),
        };
        kept.push(ObjectEvent {
            uid: &event.uid,
            rid,
            incoming: event,
        });
    }
    Ok(kept)
}

/// The Combined Event Key of each of `events`, in the Calendar of the Server
/// Object holding them (ADR 0011).
fn event_keys_in_calendar(events: &[ObjectEvent]) -> Vec<EventKeyInCalendar> {
    events
        .iter()
        .map(|event| EventKeyInCalendar::new(event.uid, event.rid.as_deref()))
        .collect()
}
