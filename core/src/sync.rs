//! The sync engine's incoming half: what changed in a Calendar on its
//! server, whatever its Provider, applied to its local copy in one Change
//! Journal unit. Server objects are matched by address, and each changes
//! only its own rows (ADR 0010).

use crate::event::{self, Loaded, ObjectEvent, ObjectId};
use crate::journal::Unit;
use crate::remote::{Changes, Fetched};
use crate::{CalendarId, ImportedEvent, Result};

/// Applies `changes` to the Calendar `calendar_id` in `unit`, and records
/// its new sync state with them: the changed objects are taken as
/// [`take`] takes them, and a removed object goes with its rows.
pub(crate) fn apply(unit: &mut Unit, calendar_id: CalendarId, changes: &Changes) -> Result<()> {
    unit.writable(calendar_id)?;
    let mut removed = Vec::new();
    for href in &changes.removed {
        removed.extend(unit.objects_at(calendar_id, href)?);
    }
    take(unit, calendar_id, &changes.changed, &mut removed)?;
    for object in removed {
        unit.remove_object(object)?;
    }
    unit.synced(calendar_id, changes.state.token.as_deref())
}

/// Takes the server objects `fetched` into the Calendar `calendar_id` in
/// `unit`. Each object FreeCal can read is mirrored and its Events become
/// its rows, as [`event::take_object`] writes them, so that an object
/// holding nothing FreeCal keeps goes from the mirror with its rows. One it
/// can't read keeps its last readable version and its rows, recorded as
/// unreadable. A new object that holds a UID of rows of one of
/// `moved_from`, the objects removed in the same sync, is that object moved
/// to another address: it takes the object's place, which leaves
/// `moved_from`. A Recurring Event joins the Calendar's Exceptions that had
/// none ([`Loaded::join`]).
pub(crate) fn take(
    unit: &mut Unit,
    calendar_id: CalendarId,
    fetched: &[Fetched],
    moved_from: &mut Vec<ObjectId>,
) -> Result<()> {
    for one in fetched {
        let Some(events) = &one.events else {
            unit.unreadable(calendar_id, &one.object.href)?;
            continue;
        };
        let events = object_events(unit, calendar_id, events, fetched)?;
        let object = match unit.objects_at(calendar_id, &one.object.href)?.first() {
            Some(&object) => Some(object),
            None => moved(unit, &events, moved_from)?,
        };
        for event in &events {
            if event.rid.is_none() && event.incoming.rule.is_some() {
                Loaded::of_uid(unit, calendar_id, event.uid)?.join(unit, &event.incoming.when)?;
            }
        }
        if !events.is_empty() {
            let object = unit.mirror(calendar_id, &one.object, object)?;
            event::take_object(unit, calendar_id, object, &events)?;
        } else if let Some(object) = object {
            unit.remove_object(object)?;
        }
    }
    Ok(())
}

/// `events`, those of an object fetched with `batch`, as the object holds
/// them: an Exception with the recurrence ID of the Occurrence it changed
/// of its Recurring Event, the first in `batch`, otherwise the Calendar's.
/// One without its Recurring Event is an Event of its own, its recurrence
/// ID in the zone it was given in (ticket 88).
fn object_events<'a>(
    unit: &Unit,
    calendar_id: CalendarId,
    events: &'a [ImportedEvent],
    batch: &[Fetched],
) -> Result<Vec<ObjectEvent<'a>>> {
    let mut kept = Vec::new();
    for event in events {
        let rid = match event.original {
            None => None,
            Some(_) => {
                let fetched = batch
                    .iter()
                    .filter_map(|fetched| fetched.events.as_deref())
                    .flatten()
                    .find(|recurring| {
                        recurring.uid == event.uid
                            && recurring.original.is_none()
                            && recurring.rule.is_some()
                    })
                    .map(|recurring| recurring.when);
                let recurring = match fetched {
                    Some(when) => Some(when),
                    None => Loaded::of_uid(unit, calendar_id, &event.uid)?.recurring_when(),
                };
                event.recurrence_id_in(recurring.as_ref())
            }
        };
        kept.push(ObjectEvent {
            uid: &event.uid,
            rid,
            incoming: event,
        });
    }
    Ok(kept)
}

/// The object of `moved_from` that rows of a UID of `events` are in, taken
/// out of it; none if there is none.
fn moved(
    unit: &Unit,
    events: &[ObjectEvent],
    moved_from: &mut Vec<ObjectId>,
) -> Result<Option<ObjectId>> {
    for (index, &object) in moved_from.iter().enumerate() {
        let rows = unit.events_in_object(object)?;
        if rows
            .iter()
            .any(|row| events.iter().any(|event| event.uid == row.uid))
        {
            return Ok(Some(moved_from.remove(index)));
        }
    }
    Ok(None)
}
