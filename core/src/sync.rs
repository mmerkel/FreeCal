//! The sync engine's incoming half: what changed in a Calendar on its
//! server, whatever its Provider, applied to its local copy in one Change
//! Journal unit. Server objects are matched by address. A Calendar's rows
//! with one UID belong to the object owning it, and only that object
//! writes them (ADR 0011).

use std::collections::HashMap;

use chrono_tz::Tz;

use crate::event::{self, Loaded, ObjectEvent, ObjectId};
use crate::import::read_server_objects;
use crate::remote::{Changes, Fetched};
use crate::store::Unit;
use crate::{CalendarId, ImportedEvent, ImportedWhen, Result};

/// Applies `changes` to the Calendar `calendar_id` in `unit`, and records
/// its new sync state with them: the changed objects are taken as
/// [`take`] takes them, and a removed object goes as [`remove`] removes it.
pub(crate) fn apply(unit: &mut Unit, calendar_id: CalendarId, changes: &Changes) -> Result<()> {
    unit.writable(calendar_id)?;
    let mut removed = Vec::new();
    for href in &changes.removed {
        removed.extend(unit.objects_at(calendar_id, href)?);
    }
    take(
        unit,
        calendar_id,
        &changes.changed,
        changes.zone,
        &mut removed,
    )?;
    for object in removed {
        remove(unit, calendar_id, object, changes.zone)?;
    }
    unit.synced(calendar_id, changes.state.token.as_deref())
}

/// Takes the server objects `fetched` into the Calendar `calendar_id` in
/// `unit`, their floating times pinned to `zone`. Each object FreeCal can
/// read is mirrored with its keys, and its Events become the rows of the
/// UIDs it owns, as [`event::take_object`] writes them; a UID it owned
/// before and no longer does goes as [`settle`] gives it. An object
/// holding nothing FreeCal keeps goes from the mirror, as [`remove`]
/// removes it. One it can't read keeps its last readable version, its
/// keys and its rows, recorded as unreadable. A new object that holds a
/// UID that one of `moved_from`, the objects removed in the same sync,
/// holds is that object moved to another address: it takes the object's
/// place, which leaves `moved_from`. A Recurring Event joins the
/// Calendar's Exceptions that had none ([`Loaded::join`]).
pub(crate) fn take(
    unit: &mut Unit,
    calendar_id: CalendarId,
    fetched: &[Fetched],
    zone: Tz,
    moved_from: &mut Vec<ObjectId>,
) -> Result<()> {
    let recurring = recurring_of(fetched);
    for one in fetched {
        let Some(events) = &one.events else {
            unit.unreadable(calendar_id, &one.object.href)?;
            continue;
        };
        let events = object_events(events, |uid| match recurring.get(uid) {
            Some(&when) => Ok(Some(when)),
            None => Ok(Loaded::of_uid(unit, calendar_id, uid)?.recurring_when()),
        })?;
        let object = match unit.objects_at(calendar_id, &one.object.href)?.first() {
            Some(&object) => Some(object),
            None => moved(unit, &events, moved_from)?,
        };
        for event in &events {
            if event.rid.is_none() && event.incoming.rule.is_some() {
                Loaded::of_uid(unit, calendar_id, event.uid)?.join(unit, &event.incoming.when)?;
            }
        }
        if events.is_empty() {
            if let Some(object) = object {
                remove(unit, calendar_id, object, zone)?;
            }
            continue;
        }
        let owned = match object {
            Some(object) => unit.owned_uids(object)?,
            None => Vec::new(),
        };
        let keys: Vec<_> = events
            .iter()
            .map(|event| (event.uid, event.rid.as_deref()))
            .collect();
        let object = unit.mirror(calendar_id, &one.object, object, &keys)?;
        event::take_object(unit, calendar_id, object, &events)?;
        let now = unit.owned_uids(object)?;
        let lost: Vec<String> = owned.into_iter().filter(|uid| !now.contains(uid)).collect();
        settle(unit, calendar_id, &lost, zone)?;
    }
    Ok(())
}

/// Removes the server object `object` of the Calendar `calendar_id` from
/// the mirror, as gone from the server; each UID it owned goes as
/// [`settle`] gives it.
fn remove(unit: &mut Unit, calendar_id: CalendarId, object: ObjectId, zone: Tz) -> Result<()> {
    let owned = unit.owned_uids(object)?;
    unit.remove_object(object)?;
    settle(unit, calendar_id, &owned, zone)
}

/// Gives the rows of each of `uids`, UIDs of the Calendar `calendar_id`
/// whose owner went, to the next object owning it: its Events of the UID,
/// read from its mirrored data with floating times pinned to `zone`, take
/// the rows' place as [`event::take_object`] writes them. Their recurrence
/// IDs are normalised against the Recurring Event it holds, or are in the
/// zone they were given in if it holds none, as if it had come alone: the
/// rows of the object that went don't count, and its keys of the UID are
/// written again to match. A UID no object holds any more loses its rows.
/// Should the mirrored data no longer read, the rows stay as they are, as
/// for an object FreeCal can't read.
fn settle(unit: &mut Unit, calendar_id: CalendarId, uids: &[String], zone: Tz) -> Result<()> {
    for uid in uids {
        let Some(owner) = unit.owner(calendar_id, uid)? else {
            Loaded::of_uid(unit, calendar_id, uid)?.delete_all(unit)?;
            continue;
        };
        let Some(object) = unit.server_object(owner)? else {
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
        let recurring = recurring_of(&read);
        let events = object_events(&events, |uid| Ok(recurring.get(uid).copied()))?;
        let keys: Vec<_> = events
            .iter()
            .map(|event| (event.uid, event.rid.as_deref()))
            .collect();
        unit.rekey(owner, uid, &keys)?;
        event::take_object(unit, calendar_id, owner, &events)?;
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

/// `events`, those of an object, as the object holds them: an Exception
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

/// The object of `moved_from` that holds a UID of `events`, by its keys,
/// taken out of it; none if there is none.
fn moved(
    unit: &Unit,
    events: &[ObjectEvent],
    moved_from: &mut Vec<ObjectId>,
) -> Result<Option<ObjectId>> {
    for (index, &object) in moved_from.iter().enumerate() {
        let held = unit.held_uids(object)?;
        if events
            .iter()
            .any(|event| held.iter().any(|uid| uid == event.uid))
        {
            return Ok(Some(moved_from.remove(index)));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ServerObject;
    use crate::store::{Origin, Store, Uid, record};

    /// A weekly Recurring Event with three Occurrences, from 5 October.
    const WEEKLY: &str = "BEGIN:VEVENT\r\nUID:weekly\r\nSUMMARY:Weekly\r\n\
        DTSTART:20261005T080000Z\r\nDTEND:20261005T090000Z\r\nRRULE:FREQ=WEEKLY;COUNT=3\r\n\
        END:VEVENT\r\n";

    /// An Exception of [`WEEKLY`]'s second Occurrence without it, its
    /// recurrence ID given in New York's time: 04:00 there is 08:00 UTC.
    const MOVED: &str = "BEGIN:VEVENT\r\nUID:weekly\r\nSUMMARY:Moved\r\n\
        RECURRENCE-ID;TZID=America/New_York:20261012T040000\r\n\
        DTSTART:20261012T090000Z\r\nDTEND:20261012T100000Z\r\nEND:VEVENT\r\n";

    /// The object at `href` holding `body`, as a sync fetches it.
    fn fetched(href: &str, body: &str) -> Fetched {
        let data = format!(
            "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Test//EN\r\n{body}END:VCALENDAR\r\n"
        );
        let object = ServerObject {
            href: href.to_owned(),
            etag: None,
            data,
        };
        read_server_objects(vec![object], Tz::UTC).remove(0)
    }

    /// A Local Store whose first Calendar holds [`MOVED`] in its object.
    struct Alone {
        _directory: tempfile::TempDir,
        store: Store,
        calendar_id: CalendarId,
        object: ObjectId,
    }

    impl Alone {
        fn new() -> Self {
            let directory = tempfile::TempDir::new().unwrap();
            let store = Store::open(&directory.path().join("freecal.sqlite3")).unwrap();
            let calendar_id = store.list_calendars().unwrap()[0].id;
            let alone = Self {
                _directory: directory,
                calendar_id,
                object: ObjectId(0),
                store,
            };
            alone.take(fetched("/weekly-moved.ics", MOVED));
            let object = alone
                .store
                .objects_at(calendar_id, "/weekly-moved.ics")
                .unwrap()[0];
            Self { object, ..alone }
        }

        fn take(&self, one: Fetched) {
            record(&self.store, Origin::Sync, 0, |unit| {
                take(unit, self.calendar_id, &[one], Tz::UTC, &mut Vec::new())
            })
            .unwrap();
        }

        /// The recurrence IDs of the keys of the object holding [`MOVED`].
        fn keys(&self) -> Vec<Option<String>> {
            let keys = self.store.keys(self.object).unwrap();
            keys.into_iter().map(|(_, rid)| rid).collect()
        }

        /// The recurrence IDs of the Calendar's rows with the UID.
        fn rows(&self) -> Vec<Option<String>> {
            let rows = self
                .store
                .events_with_uid(Uid::In(self.calendar_id, "weekly"))
                .unwrap();
            rows.into_iter()
                .map(|row| row.fields.recurrence_id)
                .collect()
        }
    }

    #[test]
    fn an_exception_without_its_recurring_event_keeps_matching_its_keys_after_joining_it() {
        let alone = Alone::new();
        let in_new_york = Some("2026-10-12T04:00:00[America/New_York]".to_owned());
        assert_eq!(alone.rows(), [in_new_york]);
        assert_eq!(alone.keys(), alone.rows());
        let weekly = fetched("/weekly.ics", WEEKLY).events.unwrap().remove(0);

        // As an import of its Recurring Event joins it.
        record(&alone.store, Origin::Import, 0, |unit| {
            Loaded::of_uid(unit, alone.calendar_id, "weekly")?.join(unit, &weekly.when)
        })
        .unwrap();

        assert_eq!(alone.rows(), [Some("2026-10-12T08:00:00".to_owned())]);
        assert_eq!(alone.keys(), alone.rows());
    }

    /// The object without rows keeps keys normalised as rows would hold
    /// them, for when it takes over.
    #[test]
    fn its_keys_join_its_recurring_event_arriving_in_another_object() {
        let alone = Alone::new();

        alone.take(fetched("/weekly.ics", WEEKLY));

        assert_eq!(alone.rows(), [None]);
        assert_eq!(alone.keys(), [Some("2026-10-12T08:00:00".to_owned())]);
    }
}
