//! The Incoming Sync: what changed in a Calendar on its server, whatever
//! its Provider, applied to its local copy in one Journal Transaction, with
//! its Mirror writes in the same Database Transaction. Server Objects are
//! matched by address.

use chrono_tz::Tz;

use super::{
    event_keys_in_calendar, mirror, object_events, recurring_of, remove_object, settle, unreadable,
};
use crate::event::{self, Loaded, ObjectEvent, ObjectId};
use crate::journal::JournalTransaction;
use crate::remote::{Changes, Fetched};
use crate::{CalendarId, Result};

/// Applies `changes` to the Calendar `calendar_id` in
/// `journal_transaction`, and records its new sync state with them: the
/// changed Server Objects are taken as [`take`] takes them, and a removed
/// one goes as [`remove`] removes it.
pub(crate) fn apply(
    journal_transaction: &mut JournalTransaction,
    calendar_id: CalendarId,
    changes: &Changes,
) -> Result<()> {
    journal_transaction.writable(calendar_id)?;
    let mut removed = Vec::new();
    for href in &changes.removed {
        removed.extend(journal_transaction.objects_at(calendar_id, href)?);
    }
    take(
        journal_transaction,
        calendar_id,
        &changes.changed,
        changes.zone,
        &mut removed,
    )?;
    for object in removed {
        remove(journal_transaction, calendar_id, object, changes.zone)?;
    }
    journal_transaction.synced(calendar_id, changes.state.token.as_deref())
}

/// Takes the Server Objects `fetched` into the Calendar `calendar_id` in
/// `journal_transaction`, their floating times pinned to `zone`. Each one
/// FreeCal can read is mirrored with its Combined Event Keys, and its Events
/// become the Event Rows of the UIDs it is the Corresponding Server Object
/// of, as [`event::take_object`] writes them; a UID it was that of before
/// and no longer is goes as [`settle`] gives it. A Server Object holding
/// nothing FreeCal keeps goes from the Mirror, as [`remove`] removes it. One
/// it can't read keeps its last readable version, its Combined Event Keys
/// and its Event Rows, recorded as unreadable. A new Server Object that
/// holds a UID that one of `moved_from`, the Server Objects removed in the
/// same Incoming Sync, holds is that one moved to another address: it takes
/// its place, which leaves `moved_from`. A Recurring Event joins the
/// Calendar's Exceptions that had none ([`Loaded::join`]).
pub(crate) fn take(
    journal_transaction: &mut JournalTransaction,
    calendar_id: CalendarId,
    fetched: &[Fetched],
    zone: Tz,
    moved_from: &mut Vec<ObjectId>,
) -> Result<()> {
    let recurring = recurring_of(fetched);
    for one in fetched {
        let Some(events) = &one.events else {
            unreadable(journal_transaction, calendar_id, &one.object.href)?;
            continue;
        };
        let events = object_events(events, |uid| match recurring.get(uid) {
            Some(&when) => Ok(Some(when)),
            None => Ok(Loaded::of_uid(journal_transaction, calendar_id, uid)?.recurring_when()),
        })?;
        let object = match journal_transaction
            .objects_at(calendar_id, &one.object.href)?
            .first()
        {
            Some(&object) => Some(object),
            None => moved(journal_transaction, &events, moved_from)?,
        };
        for event in &events {
            if event.rid.is_none() && event.incoming.rule.is_some() {
                Loaded::of_uid(journal_transaction, calendar_id, event.uid)?
                    .join(journal_transaction, &event.incoming.when)?;
            }
        }
        if events.is_empty() {
            if let Some(object) = object {
                remove(journal_transaction, calendar_id, object, zone)?;
            }
            continue;
        }
        let corresponding_uids = match object {
            Some(object) => journal_transaction.corresponding_uids(object)?,
            None => Vec::new(),
        };
        let object = mirror(
            journal_transaction,
            calendar_id,
            &one.object,
            object,
            &event_keys_in_calendar(&events),
        )?;
        event::take_object(journal_transaction, calendar_id, object, &events)?;
        if !corresponding_uids.is_empty() {
            let now = journal_transaction.corresponding_uids(object)?;
            let lost: Vec<String> = corresponding_uids
                .into_iter()
                .filter(|uid| !now.contains(uid))
                .collect();
            settle(journal_transaction, calendar_id, &lost, zone)?;
        }
    }
    Ok(())
}

/// Removes the Server Object `object` of the Calendar `calendar_id` from
/// the Mirror, as gone from the server; each UID it was the Corresponding
/// Server Object of goes as [`settle`] gives it.
fn remove(
    journal_transaction: &mut JournalTransaction,
    calendar_id: CalendarId,
    object: ObjectId,
    zone: Tz,
) -> Result<()> {
    let corresponding_uids = journal_transaction.corresponding_uids(object)?;
    remove_object(journal_transaction, object)?;
    settle(journal_transaction, calendar_id, &corresponding_uids, zone)
}

/// The Server Object of `moved_from` that holds a UID of `events`, by its
/// Combined Event Keys, taken out of it; none if there is none.
fn moved(
    journal_transaction: &JournalTransaction,
    events: &[ObjectEvent],
    moved_from: &mut Vec<ObjectId>,
) -> Result<Option<ObjectId>> {
    for (index, &object) in moved_from.iter().enumerate() {
        let held = journal_transaction.held_uids(object)?;
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
    use crate::import::read_server_objects;
    use crate::journal::{TransactionOrigin, record};
    use crate::store::{Store, Uid};

    /// A weekly Recurring Event with three Occurrences, from 5 October.
    const WEEKLY: &str = "BEGIN:VEVENT\r\nUID:weekly\r\nSUMMARY:Weekly\r\n\
        DTSTART:20261005T080000Z\r\nDTEND:20261005T090000Z\r\nRRULE:FREQ=WEEKLY;COUNT=3\r\n\
        END:VEVENT\r\n";

    /// An Exception of [`WEEKLY`]'s second Occurrence without it, its
    /// recurrence ID given in New York's time: 04:00 there is 08:00 UTC.
    const MOVED: &str = "BEGIN:VEVENT\r\nUID:weekly\r\nSUMMARY:Moved\r\n\
        RECURRENCE-ID;TZID=America/New_York:20261012T040000\r\n\
        DTSTART:20261012T090000Z\r\nDTEND:20261012T100000Z\r\nEND:VEVENT\r\n";

    /// The Server Object at `href` holding `body`, as an Incoming Sync
    /// fetches it.
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

    /// A Local Store whose first Calendar holds [`MOVED`] in its Server
    /// Object.
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
            record(
                &self.store,
                TransactionOrigin::IncomingSync,
                0,
                |journal_transaction| {
                    take(
                        journal_transaction,
                        self.calendar_id,
                        &[one],
                        Tz::UTC,
                        &mut Vec::new(),
                    )
                },
            )
            .unwrap();
        }

        /// The recurrence IDs of the Combined Event Keys of the Server
        /// Object holding [`MOVED`].
        fn combined_event_keys(&self) -> Vec<Option<String>> {
            let combined_event_keys = self.store.combined_event_keys(self.object).unwrap();
            combined_event_keys
                .into_iter()
                .map(|event_key| event_key.recurrence_id)
                .collect()
        }

        /// The recurrence IDs of the Calendar's Event Rows with the UID.
        fn event_rows(&self) -> Vec<Option<String>> {
            let event_rows = self
                .store
                .events_with_uid(Uid::In(self.calendar_id, "weekly"))
                .unwrap();
            event_rows
                .into_iter()
                .map(|event_row| event_row.fields.recurrence_id)
                .collect()
        }
    }

    #[test]
    fn an_exception_without_its_recurring_event_keeps_matching_its_combined_event_keys_after_joining_it()
     {
        let alone = Alone::new();
        let in_new_york = Some("2026-10-12T04:00:00[America/New_York]".to_owned());
        assert_eq!(alone.event_rows(), [in_new_york]);
        assert_eq!(alone.combined_event_keys(), alone.event_rows());
        let weekly = fetched("/weekly.ics", WEEKLY).events.unwrap().remove(0);

        // As an import of its Recurring Event joins it.
        record(
            &alone.store,
            TransactionOrigin::Import,
            0,
            |journal_transaction| {
                Loaded::of_uid(journal_transaction, alone.calendar_id, "weekly")?
                    .join(journal_transaction, &weekly.when)
            },
        )
        .unwrap();

        assert_eq!(alone.event_rows(), [Some("2026-10-12T08:00:00".to_owned())]);
        assert_eq!(alone.combined_event_keys(), alone.event_rows());
    }

    /// The Server Object without Event Rows keeps Combined Event Keys
    /// normalised as Event Rows would hold them, for when it becomes the
    /// Corresponding Server Object.
    #[test]
    fn its_combined_event_keys_join_its_recurring_event_arriving_in_another_server_object() {
        let alone = Alone::new();

        alone.take(fetched("/weekly.ics", WEEKLY));

        assert_eq!(alone.event_rows(), [None]);
        assert_eq!(
            alone.combined_event_keys(),
            [Some("2026-10-12T08:00:00".to_owned())]
        );
    }
}
