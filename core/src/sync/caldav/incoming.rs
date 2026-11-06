//! The CalDAV Incoming Sync: what changed in a Calendar on its server,
//! brought into its local copy in one Journal Transaction, with
//! its Mirror writes in the same Database Transaction. Server Objects are
//! matched by address.

use chrono_tz::Tz;

use super::{holds_non_exception, mirror, moved, of_its_uid, remove_object, settle, unreadable};
use crate::event::{self, ObjectId};
use crate::journal::JournalTransaction;
use crate::remote::{Changes, Fetched};
use crate::{CalendarId, Result};

/// Brings `changes` into the Calendar `calendar_id` in `journal_transaction`,
/// and records its new sync state with them: the changed Server Objects are
/// mirrored and propagated as [`mirror_and_propagate`] does it, and a removed
/// one goes as [`remove`] removes it.
pub(crate) fn incoming_sync(
    journal_transaction: &mut JournalTransaction,
    calendar_id: CalendarId,
    changes: &Changes,
) -> Result<()> {
    let mut removed = Vec::new();
    for href in &changes.removed {
        removed.extend(
            journal_transaction
                .database_transaction()
                .objects_at(calendar_id, href)?,
        );
    }
    mirror_and_propagate(
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

/// Mirrors and propagates the Server Objects `fetched` into the Calendar
/// `calendar_id` in `journal_transaction`, their floating times pinned to
/// `zone`. Each one FreeCal can read is mirrored with the UID it holds, and
/// its Events of it ([`of_its_uid`]) become the UID's Event Rows if it is
/// the UID's Corresponding Server Object, as
/// [`event::propagate_to_event_rows`] writes them; a UID it was that of
/// before and no longer is goes as [`settle`] gives it. A Server Object
/// holding nothing FreeCal keeps goes from the Mirror, as [`remove`]
/// removes it. One it can't read keeps its last readable version and its
/// Event Rows, recorded as unreadable. A new Server Object that holds the
/// UID that one of `moved_from`, the Server Objects removed in the same
/// Incoming Sync, holds is that one moved to another address: it takes its
/// place, which leaves `moved_from`. A Recurring Event that arrives joins
/// the Calendar's Exceptions that had none without writing them, as they
/// keep their recurrence IDs as obtained (ADR 0012).
pub(crate) fn mirror_and_propagate(
    journal_transaction: &mut JournalTransaction,
    calendar_id: CalendarId,
    fetched: &[Fetched],
    zone: Tz,
    moved_from: &mut Vec<ObjectId>,
) -> Result<()> {
    journal_transaction.writable(calendar_id)?;
    for one in fetched {
        let Some(events) = &one.events else {
            unreadable(journal_transaction, calendar_id, &one.object.href)?;
            continue;
        };
        let at = journal_transaction
            .database_transaction()
            .objects_at(calendar_id, &one.object.href)?
            .first()
            .copied();
        if events.is_empty() {
            if let Some(object) = at {
                remove(journal_transaction, calendar_id, object, zone)?;
            }
            continue;
        }
        let (uid, events) = of_its_uid(events);
        let object = match at {
            Some(object) => Some(object),
            None => moved(journal_transaction, uid, moved_from)?,
        };
        let corresponding_uid = match object {
            Some(object) => journal_transaction
                .database_transaction()
                .corresponding_uid(object)?,
            None => None,
        };
        let object = mirror(
            journal_transaction,
            calendar_id,
            &one.object,
            object,
            uid,
            holds_non_exception(events),
        )?;
        event::propagate_to_event_rows(journal_transaction, calendar_id, object, uid, events)?;
        if let Some(lost) = corresponding_uid
            && journal_transaction
                .database_transaction()
                .corresponding_uid(object)?
                .as_ref()
                != Some(&lost)
        {
            settle(journal_transaction, calendar_id, &lost, zone)?;
        }
    }
    Ok(())
}

/// Removes the Server Object `object` of the Calendar `calendar_id` from
/// the Mirror, as gone from the server; the UID it was the Corresponding
/// Server Object of goes as [`settle`] gives it.
fn remove(
    journal_transaction: &mut JournalTransaction,
    calendar_id: CalendarId,
    object: ObjectId,
    zone: Tz,
) -> Result<()> {
    let corresponding_uid = journal_transaction
        .database_transaction()
        .corresponding_uid(object)?;
    remove_object(journal_transaction, object)?;
    match corresponding_uid {
        Some(uid) => settle(journal_transaction, calendar_id, &uid, zone),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use chrono::NaiveDate;

    use super::*;
    use crate::import::{self, ExistingEvents, ImportFile, read_server_objects};
    use crate::journal::{self, JournalTransactionId, TransactionOrigin, record};
    use crate::store::{Store, Uid};
    use crate::{EventId, ServerObject};

    /// A weekly Recurring Event with three Occurrences, from 5 October.
    const WEEKLY: &str = "BEGIN:VEVENT\r\nUID:weekly\r\nSUMMARY:Weekly\r\n\
        DTSTART:20261005T080000Z\r\nDTEND:20261005T090000Z\r\nRRULE:FREQ=WEEKLY;COUNT=3\r\n\
        END:VEVENT\r\n";

    /// An Exception of [`WEEKLY`]'s second Occurrence without it, its
    /// recurrence ID given in New York's time: 04:00 there is 08:00 UTC.
    const MOVED: &str = "BEGIN:VEVENT\r\nUID:weekly\r\nSUMMARY:Moved\r\n\
        RECURRENCE-ID;TZID=America/New_York:20261012T040000\r\n\
        DTSTART:20261012T090000Z\r\nDTEND:20261012T100000Z\r\nEND:VEVENT\r\n";

    /// [`MOVED`]'s recurrence ID as obtained, as the Local Store keeps it.
    const IN_NEW_YORK: &str = "2026-10-12T04:00:00[America/New_York]";

    /// [`MOVED`] with its recurrence ID given in UTC, as [`WEEKLY`]'s own
    /// Exception would be.
    fn moved_in_utc() -> String {
        MOVED.replace(
            "RECURRENCE-ID;TZID=America/New_York:20261012T040000",
            "RECURRENCE-ID:20261012T080000Z",
        )
    }

    fn calendar(body: &str) -> String {
        format!("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Test//EN\r\n{body}END:VCALENDAR\r\n")
    }

    /// The Server Object at `href` holding `body`, as an Incoming Sync
    /// fetches it.
    fn fetched(href: &str, body: &str) -> Fetched {
        let object = ServerObject {
            href: href.to_owned(),
            etag: None,
            data: calendar(body),
        };
        read_server_objects(vec![object], Tz::UTC).remove(0)
    }

    /// A Local Store with a writable server Calendar whose First Sync
    /// brought [`MOVED`] alone, in its Server Object.
    struct Alone {
        _directory: tempfile::TempDir,
        store: Store,
        calendar_id: CalendarId,
        object: ObjectId,
        /// The Event Row of [`MOVED`].
        moved: EventId,
    }

    impl Alone {
        fn new() -> Self {
            let directory = tempfile::TempDir::new().unwrap();
            let store = Store::open(&directory.path().join("freecal.sqlite3")).unwrap();
            let calendar_id = store.writable_server_calendar();
            let alone = Self {
                _directory: directory,
                calendar_id,
                object: ObjectId(0),
                moved: EventId(0),
                store,
            };
            alone.mirror_and_propagate(
                TransactionOrigin::FirstSync,
                &[fetched("/weekly-moved.ics", MOVED)],
            );
            let object = alone
                .store
                .objects_at(calendar_id, "/weekly-moved.ics")
                .unwrap()[0];
            let moved = alone.event_rows()[0].0;
            Self {
                object,
                moved,
                ..alone
            }
        }

        /// Mirrors and propagates `fetched` as one Incoming Sync of `origin`.
        fn mirror_and_propagate(
            &self,
            origin: TransactionOrigin,
            fetched: &[Fetched],
        ) -> Option<JournalTransactionId> {
            record(&self.store, origin, 0, |journal_transaction| {
                mirror_and_propagate(
                    journal_transaction,
                    self.calendar_id,
                    fetched,
                    Tz::UTC,
                    &mut Vec::new(),
                )
            })
            .unwrap()
            .journal_transaction
        }

        /// Whether the Server Object holding [`MOVED`] holds the Event that
        /// isn't an Exception, none if it is gone.
        fn holds_non_exception(&self) -> Option<bool> {
            self.store.holds_non_exception(self.object).unwrap()
        }

        /// The Calendar's Event Rows with the UID, each with its recurrence
        /// ID as stored.
        fn event_rows(&self) -> Vec<(EventId, Option<String>)> {
            let event_rows = self
                .store
                .events_with_uid(Uid::In(self.calendar_id, "weekly"))
                .unwrap();
            event_rows
                .into_iter()
                .map(|event_row| {
                    let recurrence_id = event_row.fields.recurrence_id;
                    let text = recurrence_id.map(|obtained| obtained.as_obtained().to_owned());
                    (event_row.id, text)
                })
                .collect()
        }

        /// The Occurrences in October 2026, each with its title, Event and
        /// recurrence ID.
        fn shown(&self) -> Vec<(String, EventId, Option<String>)> {
            let date = |day| NaiveDate::from_ymd_opt(2026, 10, day).unwrap();
            event::occurrences(&self.store, date(1), date(31), Tz::UTC)
                .unwrap()
                .into_iter()
                .map(|occurrence| {
                    (
                        occurrence.title,
                        occurrence.event_id,
                        occurrence.recurrence_id,
                    )
                })
                .collect()
        }

        /// The recurrence IDs of the Unsent Changes, as stored.
        fn unsent(&self) -> Vec<Option<String>> {
            let unsent = self.store.unsent_changes().unwrap();
            unsent
                .into_iter()
                .map(|unsent| unsent.recurrence_id)
                .collect()
        }

        /// Asserts that every Unsent Change holds a recurrence ID as an
        /// Event Row holds it, or none.
        fn assert_unsent_as_the_event_rows_hold_them(&self) {
            let held: HashSet<Option<String>> = self
                .event_rows()
                .into_iter()
                .map(|(_, recurrence_id)| recurrence_id)
                .chain([None])
                .collect();
            for recurrence_id in self.unsent() {
                assert!(held.contains(&recurrence_id), "{recurrence_id:?} unsent");
            }
        }

        /// Imports the `.ics` file holding `body` into the Calendar,
        /// updating the Events it has.
        fn import(&self, body: &str) {
            let file = ImportFile::new("weekly.ics", calendar(body).into_bytes());
            let existing = self.store.event_uids(self.calendar_id).unwrap();
            let preview = import::preview(&file, self.calendar_id, &existing, Tz::UTC).unwrap();
            record(
                &self.store,
                TransactionOrigin::Import,
                0,
                |journal_transaction| {
                    import::run(journal_transaction, &preview, ExistingEvents::Update)
                },
            )
            .unwrap();
        }
    }

    #[test]
    fn an_exception_alone_keeps_its_recurrence_id_as_given() {
        let alone = Alone::new();

        assert_eq!(
            alone.event_rows(),
            [(alone.moved, Some(IN_NEW_YORK.to_owned()))]
        );
        assert_eq!(alone.holds_non_exception(), Some(false));
        // An Event of its own, without a recurrence ID on its Occurrence.
        assert_eq!(alone.shown(), [("Moved".to_owned(), alone.moved, None)]);
    }

    /// ADR 0012: its Recurring Event arriving changes no stored recurrence
    /// ID; the Exception changes the Occurrence it names in another zone.
    /// Should the server give it in another form, the Event Row takes that.
    #[test]
    fn it_changes_its_occurrence_once_its_recurring_event_arrives_in_its_server_object() {
        for (moved, obtained) in [
            (MOVED.to_owned(), IN_NEW_YORK),
            (moved_in_utc(), "2026-10-12T08:00:00[UTC]"),
        ] {
            let alone = Alone::new();

            alone.mirror_and_propagate(
                TransactionOrigin::IncomingSync,
                &[fetched("/weekly-moved.ics", &format!("{WEEKLY}{moved}"))],
            );

            let weekly = alone.event_rows()[1].0;
            assert_eq!(
                alone.event_rows(),
                [(alone.moved, Some(obtained.to_owned())), (weekly, None)]
            );
            assert_eq!(alone.holds_non_exception(), Some(true));
            let second = Some("2026-10-12T08:00:00".to_owned());
            assert_eq!(alone.shown()[1], ("Moved".to_owned(), alone.moved, second));
            assert_eq!(alone.shown().len(), 3);
        }
    }

    /// ADR 0011: the Server Object holding the Recurring Event corresponds,
    /// whether it arrives in the same Incoming Sync or a later one; the
    /// other is mirrored without Event Rows.
    #[test]
    fn in_another_server_object_its_recurring_event_corresponds() {
        for same_incoming_sync in [false, true] {
            let alone = Alone::new();
            let weekly = fetched("/weekly.ics", WEEKLY);
            if same_incoming_sync {
                let moved = fetched(
                    "/weekly-moved.ics",
                    &moved_in_utc().replace("Moved", "Moved again"),
                );
                alone.mirror_and_propagate(TransactionOrigin::IncomingSync, &[moved, weekly]);
            } else {
                alone.mirror_and_propagate(TransactionOrigin::IncomingSync, &[weekly]);
            }

            assert_eq!(alone.event_rows().len(), 1);
            assert_eq!(alone.event_rows()[0].1, None);
            assert_eq!(alone.holds_non_exception(), Some(false));
            assert_eq!(alone.store.corresponding_uid(alone.object).unwrap(), None);
        }
    }

    /// An Event of another UID, on 6 October.
    const OTHER: &str = "BEGIN:VEVENT\r\nUID:other\r\nSUMMARY:Other\r\n\
        DTSTART:20261006T080000Z\r\nDTEND:20261006T090000Z\r\nEND:VEVENT\r\n";

    /// ADR 0013: a resource holding two UIDs, which RFC 4791 §4.1 forbids,
    /// is mirrored with its first UID and without Event Rows, new or known.
    #[test]
    fn a_server_object_holding_two_uids_is_mirrored_without_event_rows() {
        let alone = Alone::new();

        alone.mirror_and_propagate(
            TransactionOrigin::IncomingSync,
            &[fetched("/two.ics", &format!("{OTHER}{WEEKLY}"))],
        );

        let two = alone
            .store
            .objects_at(alone.calendar_id, "/two.ics")
            .unwrap();
        assert_eq!(two.len(), 1);
        assert_eq!(
            alone.store.held_uid(two[0]).unwrap().as_deref(),
            Some("other")
        );
        assert_eq!(
            alone.store.holds_non_exception(two[0]).unwrap(),
            Some(false)
        );
        assert_eq!(alone.shown(), [("Moved".to_owned(), alone.moved, None)]);

        alone.mirror_and_propagate(
            TransactionOrigin::IncomingSync,
            &[fetched("/weekly-moved.ics", &format!("{WEEKLY}{OTHER}"))],
        );

        assert_eq!(
            alone.store.held_uid(alone.object).unwrap().as_deref(),
            Some("weekly")
        );
        assert_eq!(alone.holds_non_exception(), Some(false));
        assert_eq!(alone.event_rows(), []);
        assert_eq!(alone.shown(), []);
    }

    #[test]
    fn undoing_the_incoming_sync_that_brought_its_recurring_event_leaves_it_an_event_of_its_own() {
        let alone = Alone::new();
        let journal_transaction = alone
            .mirror_and_propagate(
                TransactionOrigin::IncomingSync,
                &[fetched("/weekly-moved.ics", &format!("{WEEKLY}{MOVED}"))],
            )
            .unwrap();

        journal::undo(&alone.store, journal_transaction, 1).unwrap();

        assert_eq!(
            alone.event_rows(),
            [(alone.moved, Some(IN_NEW_YORK.to_owned()))]
        );
        assert_eq!(alone.shown(), [("Moved".to_owned(), alone.moved, None)]);
        assert_eq!(alone.unsent(), [None]);
        alone.assert_unsent_as_the_event_rows_hold_them();
    }

    /// ADR 0012: changing the whole Recurring Event without moving it
    /// leaves its Exceptions' recurrence IDs as obtained; moving it rewrites
    /// them.
    #[test]
    fn changing_its_recurring_event_rewrites_its_recurrence_id_only_when_it_moves() {
        use crate::When;
        use crate::event::{Edit, EventDraft, Opened, Scope, edit_occurrence};
        for (shift, obtained) in [(0, IN_NEW_YORK), (1, "2026-10-12T09:00:00")] {
            let alone = Alone::new();
            alone.import(WEEKLY);
            let weekly = alone.event_rows()[1].0;
            let event = Opened::load(&alone.store, weekly)
                .unwrap()
                .to_event(Tz::UTC)
                .unwrap();
            let When::Timed { start, end } = event.when else {
                panic!("a timed Event");
            };
            let hour = chrono::TimeDelta::hours(shift);
            let draft = EventDraft {
                calendar_id: alone.calendar_id,
                title: "Renamed".to_owned(),
                when: When::Timed {
                    start: start + hour,
                    end: end + hour,
                },
                location: String::new(),
                description: String::new(),
                repeat: event.repeat,
            };

            record(
                &alone.store,
                TransactionOrigin::UserAction,
                1,
                |journal_transaction| {
                    edit_occurrence(
                        journal_transaction,
                        Tz::UTC,
                        weekly,
                        "2026-10-05T08:00:00",
                        Scope::All,
                        false,
                        Edit::Draft(draft),
                    )
                },
            )
            .unwrap();

            assert_eq!(
                alone.event_rows()[0],
                (alone.moved, Some(obtained.to_owned()))
            );
            assert_eq!(alone.shown()[1].1, alone.moved);
            if shift == 0 {
                alone.assert_unsent_as_the_event_rows_hold_them();
            }
        }
    }

    /// The import pairs the Exception that came alone by Occurrence, without
    /// writing it, or keeping its recurrence ID where its own replaces it.
    #[test]
    fn importing_its_recurring_event_leaves_nothing_unsent_in_another_form() {
        for body in [WEEKLY.to_owned(), format!("{WEEKLY}{}", moved_in_utc())] {
            let alone = Alone::new();

            alone.import(&body);

            assert_eq!(alone.shown().len(), 3);
            assert_eq!(alone.shown()[1].1, alone.moved);
            assert_eq!(
                alone.event_rows()[0],
                (alone.moved, Some(IN_NEW_YORK.to_owned()))
            );
            alone.assert_unsent_as_the_event_rows_hold_them();
        }
    }
}
