//! The CalDAV Incoming Sync: what changed in a Calendar on its server,
//! brought into its local copy in one Journal Transaction, with
//! its Mirror writes in the same Database Transaction. Server Objects are
//! matched by address, and a UID is kept by one CalDAV Server Object at a
//! time (ADR 0015).

use chrono_tz::Tz;

use super::{
    Ignored, delete_if_unkept, fixed_order, forget_ignored, ignore, log_ignored, mirror, moved,
    remove_object, unreadable,
};
use crate::event::{self, ObjectId, ServerObject};
use crate::import::{Holds, read_caldav_resource};
use crate::journal::{JournalTransaction, TransactionOrigin};
use crate::remote::{Changes, Fetched};
use crate::{CalendarId, ImportedEvent, Result};

/// Brings `changes` into the Calendar `calendar_id` in `journal_transaction`,
/// and records its new sync state with them: the changed Server Objects are
/// mirrored and propagated as [`mirror_and_propagate`] does it. A removed
/// Server Object goes from the Mirror, and the Event Rows of its UID with
/// it unless a resource takes over the UID; a removed Ignored Resource is
/// forgotten.
pub(crate) fn incoming_sync(
    journal_transaction: &mut JournalTransaction,
    calendar_id: CalendarId,
    changes: &Changes,
) -> Result<()> {
    let mut removed = Vec::new();
    for href in &changes.removed {
        forget_ignored(journal_transaction, calendar_id, href)?;
        removed.extend(
            journal_transaction
                .database_transaction()
                .object_at(calendar_id, href)?,
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
        let uid = journal_transaction
            .database_transaction()
            .held_uid(object)?;
        remove_object(journal_transaction, object)?;
        if let Some(uid) = uid {
            delete_if_unkept(journal_transaction, calendar_id, &uid)?;
        }
    }
    journal_transaction.synced(calendar_id, changes.state.token.as_deref())
}

/// A fetched CalDAV resource holding the Events `events` of one UID: a
/// candidate to keep it.
#[derive(Clone, Copy)]
struct Candidate<'a> {
    object: &'a ServerObject,
    events: &'a [ImportedEvent],
}

impl Candidate<'_> {
    fn fixed_order(&self) -> (bool, &str) {
        fixed_order(&self.object.href, self.events)
    }
}

/// Mirrors and propagates the Server Objects `fetched` into the Calendar
/// `calendar_id` in `journal_transaction`, their floating times pinned to
/// `zone`. The resources holding one UID compete to keep it as [`keep`]
/// decides; the one keeping it is mirrored, and propagated to the UID's
/// Event Rows as [`event::propagate_to_event_rows`] writes them, and the
/// others are Ignored Resources. A Server Object that no longer holds the
/// UID it kept goes from the Mirror, and the UID's Event Rows with it
/// unless a resource takes over. A resource holding nothing FreeCal keeps
/// isn't mirrored; one holding several UIDs, which RFC 4791 §4.1 forbids,
/// is also written to the Log and remembered as an Ignored Resource with
/// its ETag (ADR 0015), and an Ignored Resource that now holds nothing is
/// forgotten. One FreeCal can't read keeps its last readable version, UID
/// and Event Rows, recorded as unreadable; an Ignored Resource keeps its
/// ETag, so that it is fetched again. `moved_from` are the Server Objects
/// removed in the same Incoming Sync; one taken over leaves it. A Recurring
/// Event that arrives joins the Calendar's Exceptions that had none without
/// writing them, as they keep their recurrence IDs as obtained (ADR 0012).
pub(crate) fn mirror_and_propagate(
    journal_transaction: &mut JournalTransaction,
    calendar_id: CalendarId,
    fetched: &[Fetched],
    zone: Tz,
    moved_from: &mut Vec<ObjectId>,
) -> Result<()> {
    journal_transaction.writable(calendar_id)?;
    // First every Server Object that stops keeping its UID goes, so that
    // the UID is free for another before any is mirrored.
    let mut lost = Vec::new();
    let mut by_uid: Vec<(&str, Vec<Candidate>)> = Vec::new();
    for one in fetched {
        let href = &one.object.href;
        let database_transaction = journal_transaction.database_transaction();
        let at = database_transaction.object_at(calendar_id, href)?;
        let kept_uid = match at {
            Some(object) => database_transaction.held_uid(object)?,
            None => None,
        };
        let holds_another = match &one.holds {
            Holds::Unreadable => {
                unreadable(journal_transaction, calendar_id, href)?;
                continue;
            }
            Holds::Nothing { .. } => true,
            Holds::OneUid { uid, .. } => kept_uid.as_ref() != Some(uid),
        };
        if let Some(object) = at
            && holds_another
        {
            remove_object(journal_transaction, object)?;
            lost.extend(kept_uid);
        }
        match &one.holds {
            Holds::Unreadable => unreachable!("an unreadable resource is skipped above"),
            Holds::Nothing { uids } if uids.is_empty() => {
                forget_ignored(journal_transaction, calendar_id, href)?;
            }
            Holds::Nothing { uids } => {
                log_ignored(calendar_id, href, Ignored::SeveralUids(uids));
                ignore(journal_transaction, calendar_id, &one.object, None)?;
            }
            Holds::OneUid { uid, events } => {
                let candidate = Candidate {
                    object: &one.object,
                    events,
                };
                match by_uid.iter_mut().find(|(held, _)| held == uid) {
                    Some((_, candidates)) => candidates.push(candidate),
                    None => by_uid.push((uid, vec![candidate])),
                }
            }
        }
    }
    for (uid, candidates) in by_uid {
        keep(
            journal_transaction,
            calendar_id,
            uid,
            &candidates,
            zone,
            moved_from,
        )?;
    }
    for uid in lost {
        delete_if_unkept(journal_transaction, calendar_id, &uid)?;
    }
    Ok(())
}

/// Which resource keeps a UID, as [`keep`] decides it.
enum Keeper {
    /// The candidate at this index, mirrored over the Server Object given,
    /// or as a new one.
    Candidate(usize, Option<ObjectId>),
    /// The Server Object keeping it now, at this address, which stays.
    Stays(String),
}

/// Decides which of `candidates`, the fetched resources holding `uid` in
/// the Calendar `calendar_id`, keeps the UID, mirrors it and propagates it
/// to the UID's Event Rows, and makes the others Ignored Resources holding
/// `uid`, each written to the Log (ADR 0015). The Server Object keeping
/// `uid` keeps it as long as it holds it: when it is a candidate, it keeps
/// it, and when it is unchanged, no candidate does. When it is one of
/// `moved_from`, the first new resource by the [`fixed_order`] is it
/// moved, and takes its place; with no new resource, it goes and the
/// candidates take over. With no Server Object keeping `uid`, the first
/// candidate by the fixed order keeps it. In a First Sync, every Server
/// Object was mirrored in the same Incoming Sync, so the one keeping `uid`
/// competes with the candidates by the fixed order, and becomes an Ignored
/// Resource if one comes first.
fn keep(
    journal_transaction: &mut JournalTransaction,
    calendar_id: CalendarId,
    uid: &str,
    candidates: &[Candidate],
    zone: Tz,
    moved_from: &mut Vec<ObjectId>,
) -> Result<()> {
    let first = |candidates: &mut dyn Iterator<Item = (usize, &Candidate)>| {
        candidates
            .min_by(|(_, a), (_, b)| a.fixed_order().cmp(&b.fixed_order()))
            .map(|(index, _)| index)
    };
    let database_transaction = journal_transaction.database_transaction();
    let kept = match database_transaction.corresponding_server_object(calendar_id, uid)? {
        Some(object) => database_transaction
            .server_object(object)?
            .map(|held| (object, held)),
        None => None,
    };
    let keeper = match kept {
        None => {
            first(&mut candidates.iter().enumerate()).map(|index| Keeper::Candidate(index, None))
        }
        Some((object, held)) => {
            if let Some(index) = candidates
                .iter()
                .position(|candidate| candidate.object.href == held.href)
            {
                Some(Keeper::Candidate(index, Some(object)))
            } else if moved(object, moved_from) {
                let mut new = Vec::new();
                for (index, candidate) in candidates.iter().enumerate() {
                    if !database_transaction.is_ignored(calendar_id, &candidate.object.href)? {
                        new.push((index, candidate));
                    }
                }
                match first(&mut new.into_iter()) {
                    Some(index) => Some(Keeper::Candidate(index, Some(object))),
                    None => {
                        remove_object(journal_transaction, object)?;
                        first(&mut candidates.iter().enumerate())
                            .map(|index| Keeper::Candidate(index, None))
                    }
                }
            } else if journal_transaction.origin() == TransactionOrigin::FirstSync {
                let index = first(&mut candidates.iter().enumerate());
                let comes_first = |events: &[ImportedEvent]| {
                    let kept = fixed_order(&held.href, events);
                    index.filter(|&index| candidates[index].fixed_order() < kept)
                };
                match read_caldav_resource(&held, zone) {
                    Holds::OneUid { events, .. } => match comes_first(&events) {
                        Some(index) => {
                            remove_object(journal_transaction, object)?;
                            let kept_by = &candidates[index].object.href;
                            log_ignored(calendar_id, &held.href, Ignored::UidKept { uid, kept_by });
                            ignore(journal_transaction, calendar_id, &held, Some(uid))?;
                            Some(Keeper::Candidate(index, None))
                        }
                        None => Some(Keeper::Stays(held.href)),
                    },
                    Holds::Unreadable | Holds::Nothing { .. } => Some(Keeper::Stays(held.href)),
                }
            } else {
                Some(Keeper::Stays(held.href))
            }
        }
    };
    let kept_by = match keeper {
        None => return Ok(()),
        Some(Keeper::Stays(href)) => href,
        Some(Keeper::Candidate(index, over)) => {
            let Candidate { object, events } = candidates[index];
            let object_id = mirror(journal_transaction, calendar_id, object, over, uid)?;
            event::propagate_to_event_rows(
                journal_transaction,
                calendar_id,
                object_id,
                uid,
                events,
            )?;
            object.href.clone()
        }
    };
    for candidate in candidates {
        if candidate.object.href != kept_by {
            log_ignored(
                calendar_id,
                &candidate.object.href,
                Ignored::UidKept {
                    uid,
                    kept_by: &kept_by,
                },
            );
            ignore(
                journal_transaction,
                calendar_id,
                candidate.object,
                Some(uid),
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use chrono::NaiveDate;

    use super::*;
    use crate::import::{self, ExistingEvents, ImportFile, read_caldav_resources};
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
        read_caldav_resources(vec![object], Tz::UTC).remove(0)
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
        /// A Local Store with a writable server Calendar whose First Sync
        /// hasn't brought anything yet; `object` and `moved` aren't set.
        fn empty() -> Self {
            let directory = tempfile::TempDir::new().unwrap();
            let store = Store::open(&directory.path().join("freecal.sqlite3")).unwrap();
            let calendar_id = store.writable_server_calendar();
            Self {
                _directory: directory,
                calendar_id,
                object: ObjectId(0),
                moved: EventId(0),
                store,
            }
        }

        fn new() -> Self {
            let alone = Self::empty();
            let calendar_id = alone.calendar_id;
            alone.mirror_and_propagate(
                TransactionOrigin::FirstSync,
                &[fetched("/weekly-moved.ics", MOVED)],
            );
            let object = alone
                .store
                .object_at(calendar_id, "/weekly-moved.ics")
                .unwrap()
                .unwrap();
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
            assert_eq!(
                alone
                    .store
                    .object_at(alone.calendar_id, "/weekly-moved.ics")
                    .unwrap(),
                Some(alone.object)
            );
            let second = Some("2026-10-12T08:00:00".to_owned());
            assert_eq!(alone.shown()[1], ("Moved".to_owned(), alone.moved, second));
            assert_eq!(alone.shown().len(), 3);
        }
    }

    /// An Event of another UID, on 6 October.
    const OTHER: &str = "BEGIN:VEVENT\r\nUID:other\r\nSUMMARY:Other\r\n\
        DTSTART:20261006T080000Z\r\nDTEND:20261006T090000Z\r\nEND:VEVENT\r\n";

    /// The Log lines written on this thread.
    mod captured_log {
        use std::cell::RefCell;
        use std::sync::Once;

        thread_local! {
            static LINES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
        }

        struct Capture;

        impl log::Log for Capture {
            fn enabled(&self, _: &log::Metadata) -> bool {
                true
            }

            fn log(&self, record: &log::Record) {
                LINES.with(|lines| lines.borrow_mut().push(record.args().to_string()));
            }

            fn flush(&self) {}
        }

        /// Starts capturing this thread's Log lines, forgetting earlier ones.
        pub(super) fn start() {
            static INSTALL: Once = Once::new();
            INSTALL.call_once(|| {
                log::set_logger(&Capture).unwrap();
                log::set_max_level(log::LevelFilter::Trace);
            });
            LINES.with(|lines| lines.borrow_mut().clear());
        }

        /// The Log lines this thread wrote since [`start`].
        pub(super) fn lines() -> Vec<String> {
            LINES.with(|lines| lines.borrow().clone())
        }
    }

    /// A CalDAV resource holding two UIDs, which RFC 4791 §4.1 forbids,
    /// isn't mirrored and gives no Event Rows, in a First Sync and an
    /// Incoming Sync, before or after another CalDAV resource holding one of
    /// its UIDs, which is that UID's Corresponding Server Object (ADR 0015).
    /// Each time it is written to the Log.
    #[test]
    fn a_caldav_resource_holding_two_uids_is_ignored_and_logged() {
        let origins = [
            TransactionOrigin::FirstSync,
            TransactionOrigin::IncomingSync,
        ];
        for (origin, two_first) in origins.into_iter().flat_map(|o| [(o, false), (o, true)]) {
            let alone = Alone::new();
            let two = fetched("/two.ics", &format!("{OTHER}{WEEKLY}"));
            let other = fetched("/other.ics", &OTHER.replace("Other", "Other alone"));
            captured_log::start();

            let fetched = if two_first {
                [two, other]
            } else {
                [other, two]
            };
            alone.mirror_and_propagate(origin, &fetched);

            assert_eq!(
                alone
                    .store
                    .object_at(alone.calendar_id, "/two.ics")
                    .unwrap(),
                None
            );
            let other = alone
                .store
                .object_at(alone.calendar_id, "/other.ics")
                .unwrap();
            assert_eq!(
                alone
                    .store
                    .corresponding_server_object(alone.calendar_id, "other")
                    .unwrap(),
                other
            );
            let titles: Vec<String> = alone.shown().into_iter().map(|shown| shown.0).collect();
            assert_eq!(titles, ["Other alone", "Moved"]);
            assert_eq!(
                captured_log::lines(),
                [format!(
                    "Ignored the CalDAV resource \"/two.ics\" of Calendar {}: it holds \
                     several UIDs, which RFC 4791 §4.1 forbids: \"other\", \"weekly\"",
                    alone.calendar_id.0
                )]
            );
        }
    }

    /// [`fetched`], with the ETag `etag`.
    fn fetched_with_etag(href: &str, body: &str, etag: &str) -> Fetched {
        let mut fetched = fetched(href, body);
        fetched.object.etag = Some(etag.to_owned());
        read_caldav_resources(vec![fetched.object], Tz::UTC).remove(0)
    }

    impl Alone {
        /// The Calendar's Ignored Resources, each with its ETag, by address.
        fn ignored(&self) -> Vec<(String, Option<String>)> {
            let mut ignored: Vec<(String, Option<String>)> = self
                .store
                .ignored_resources(self.calendar_id)
                .unwrap()
                .into_iter()
                .map(|(href, held)| (href, held.etag))
                .collect();
            ignored.sort();
            ignored
        }

        /// Brings `changed` and `removed` into the Calendar as one Incoming
        /// Sync.
        fn incoming_sync(&self, changed: Vec<Fetched>, removed: &[&str]) {
            let changes = Changes {
                changed,
                removed: removed.iter().map(|href| (*href).to_owned()).collect(),
                state: crate::remote::SyncPoint { token: None },
                zone: Tz::UTC,
            };
            record(
                &self.store,
                TransactionOrigin::IncomingSync,
                0,
                |journal_transaction| {
                    incoming_sync(journal_transaction, self.calendar_id, &changes)
                },
            )
            .unwrap();
        }
    }

    /// ADR 0015: a CalDAV resource holding several UIDs is remembered with
    /// its ETag, which follows it while it holds several. Once it holds one
    /// UID, it is mirrored instead; coming to hold several again, it leaves
    /// the Mirror for the Ignored Resources. Holding nothing, or gone from
    /// the server, it is forgotten.
    #[test]
    fn a_caldav_resource_holding_two_uids_is_an_ignored_resource_until_it_holds_one() {
        let alone = Alone::new();
        let two = |etag| fetched_with_etag("/two.ics", &format!("{OTHER}{WEEKLY}"), etag);
        let ignored = |etag: &str| [("/two.ics".to_owned(), Some(etag.to_owned()))];

        alone.incoming_sync(vec![two("1")], &[]);
        assert_eq!(alone.ignored(), ignored("1"));

        alone.incoming_sync(vec![two("2")], &[]);
        assert_eq!(alone.ignored(), ignored("2"));
        assert_eq!(
            alone
                .store
                .object_at(alone.calendar_id, "/two.ics")
                .unwrap(),
            None
        );

        alone.incoming_sync(vec![fetched_with_etag("/two.ics", OTHER, "3")], &[]);
        assert_eq!(alone.ignored(), []);
        assert!(
            alone
                .store
                .object_at(alone.calendar_id, "/two.ics")
                .unwrap()
                .is_some()
        );
        let titles: Vec<String> = alone.shown().into_iter().map(|shown| shown.0).collect();
        assert_eq!(titles, ["Other", "Moved"]);

        alone.incoming_sync(vec![two("4")], &[]);
        assert_eq!(alone.ignored(), ignored("4"));
        assert_eq!(
            alone
                .store
                .object_at(alone.calendar_id, "/two.ics")
                .unwrap(),
            None
        );
        assert_eq!(alone.shown().len(), 1);

        alone.incoming_sync(vec![], &["/two.ics"]);
        assert_eq!(alone.ignored(), []);

        alone.incoming_sync(vec![two("5")], &[]);
        let todo = "BEGIN:VTODO\r\nUID:todo\r\nSUMMARY:Todo\r\nEND:VTODO\r\n";
        alone.incoming_sync(vec![fetched_with_etag("/two.ics", todo, "6")], &[]);
        assert_eq!(alone.ignored(), []);
    }

    /// An Ignored Resource that FreeCal can no longer read keeps the ETag
    /// it had, so that it is fetched again.
    #[test]
    fn an_ignored_resource_freecal_cant_read_keeps_its_etag() {
        let alone = Alone::new();
        alone.incoming_sync(
            vec![fetched_with_etag(
                "/two.ics",
                &format!("{OTHER}{WEEKLY}"),
                "1",
            )],
            &[],
        );
        let unreadable = Fetched {
            object: ServerObject {
                href: "/two.ics".to_owned(),
                etag: Some("2".to_owned()),
                data: "not iCalendar".to_owned(),
            },
            holds: Holds::Unreadable,
        };

        alone.incoming_sync(vec![unreadable], &[]);

        assert_eq!(
            alone.ignored(),
            [("/two.ics".to_owned(), Some("1".to_owned()))]
        );
    }

    /// Deleting a Calendar deletes its Ignored Resources.
    #[test]
    fn deleting_its_calendar_deletes_an_ignored_resource() {
        let alone = Alone::new();
        alone.incoming_sync(
            vec![fetched_with_etag(
                "/two.ics",
                &format!("{OTHER}{WEEKLY}"),
                "1",
            )],
            &[],
        );

        journal::delete_calendar(&alone.store, alone.calendar_id).unwrap();

        assert_eq!(alone.ignored(), []);
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

    /// ADR 0015: one CalDAV Server Object keeps a UID in its Calendar; the
    /// other resources holding it are Ignored Resources, until the one
    /// keeping it stops holding it.
    mod one_server_object_per_uid {
        use super::*;

        impl Alone {
            /// The titles of the Occurrences in October 2026.
            fn titles(&self) -> Vec<String> {
                self.shown().into_iter().map(|shown| shown.0).collect()
            }

            /// The Server Object at `href`, if any.
            fn mirrored(&self, href: &str) -> Option<ObjectId> {
                self.store.object_at(self.calendar_id, href).unwrap()
            }

            /// The Calendar's Ignored Resources, each with its ETag and the
            /// UID it holds that a Server Object keeps, by address.
            fn ignored_uids(&self) -> Vec<(String, Option<String>, Option<String>)> {
                let mut ignored: Vec<_> = self
                    .store
                    .ignored_resources(self.calendar_id)
                    .unwrap()
                    .into_iter()
                    .map(|(href, held)| (href, held.etag, held.uid))
                    .collect();
                ignored.sort();
                ignored
            }
        }

        /// The Ignored Resource at `href` with the ETag `etag`, holding the
        /// UID `uid`, as [`Alone::ignored_uids`] lists it.
        fn ignored(
            href: &str,
            etag: Option<&str>,
            uid: &str,
        ) -> (String, Option<String>, Option<String>) {
            (
                href.to_owned(),
                etag.map(str::to_owned),
                Some(uid.to_owned()),
            )
        }

        /// The Log line for the resource at `href` ignored as it holds
        /// `uid`, which the one at `kept_by` keeps.
        fn kept_line(alone: &Alone, href: &str, uid: &str, kept_by: &str) -> String {
            format!(
                "Ignored the CalDAV resource {href:?} of Calendar {}: it holds the UID {uid:?} \
                 that the CalDAV resource {kept_by:?} holds, which RFC 4791 §4.1 forbids",
                alone.calendar_id.0
            )
        }

        /// [`OTHER`] titled `title`.
        fn other(title: &str) -> String {
            OTHER.replace("Other", title)
        }

        /// [`fetched_with_etag`] of [`other`].
        fn other_at(href: &str, title: &str, etag: &str) -> Fetched {
            fetched_with_etag(href, &other(title), etag)
        }

        /// A resource at `href` FreeCal can't read, with the ETag `etag`.
        fn unreadable_at(href: &str, etag: &str) -> Fetched {
            Fetched {
                object: ServerObject {
                    href: href.to_owned(),
                    etag: Some(etag.to_owned()),
                    data: "not iCalendar".to_owned(),
                },
                holds: Holds::Unreadable,
            }
        }

        #[test]
        fn a_first_sync_keeps_the_one_holding_the_recurring_event_else_the_lowest_href() {
            let weekly = ["Weekly", "Weekly", "Weekly"].as_slice();
            let a = ["A"].as_slice();
            // The pages of the First Sync, the kept resource, the ignored one
            // and the UID, and the titles shown.
            let cases = [
                (
                    vec![vec![fetched("/a.ics", MOVED), fetched("/z.ics", WEEKLY)]],
                    ("/z.ics", "/a.ics", "weekly"),
                    weekly,
                ),
                (
                    vec![
                        vec![fetched("/a.ics", MOVED)],
                        vec![fetched("/z.ics", WEEKLY)],
                    ],
                    ("/z.ics", "/a.ics", "weekly"),
                    weekly,
                ),
                (
                    vec![
                        vec![fetched("/z.ics", WEEKLY)],
                        vec![fetched("/a.ics", MOVED)],
                    ],
                    ("/z.ics", "/a.ics", "weekly"),
                    weekly,
                ),
                (
                    vec![vec![
                        fetched("/b.ics", &other("B")),
                        fetched("/a.ics", &other("A")),
                    ]],
                    ("/a.ics", "/b.ics", "other"),
                    a,
                ),
                (
                    vec![
                        vec![fetched("/b.ics", &other("B"))],
                        vec![fetched("/a.ics", &other("A"))],
                    ],
                    ("/a.ics", "/b.ics", "other"),
                    a,
                ),
                (
                    vec![
                        vec![fetched("/a.ics", &other("A"))],
                        vec![fetched("/b.ics", &other("B"))],
                    ],
                    ("/a.ics", "/b.ics", "other"),
                    a,
                ),
            ];
            for (pages, (kept, ignored_href, uid), titles) in cases {
                let alone = Alone::empty();
                captured_log::start();

                for page in &pages {
                    alone.mirror_and_propagate(TransactionOrigin::FirstSync, page);
                }

                assert!(alone.mirrored(kept).is_some(), "{pages:?}");
                assert_eq!(alone.mirrored(ignored_href), None);
                assert_eq!(alone.ignored_uids(), [ignored(ignored_href, None, uid)]);
                assert_eq!(alone.titles(), titles);
                assert_eq!(
                    captured_log::lines(),
                    [kept_line(&alone, ignored_href, uid, kept)]
                );
            }
        }

        /// It never pushes out the one keeping the UID, which keeps it
        /// changed in the same Incoming Sync too.
        #[test]
        fn one_arriving_later_with_a_kept_uid_is_ignored_even_if_it_comes_first() {
            let alone = Alone::new();
            let before = alone.shown();
            captured_log::start();

            let journal_transaction = alone.mirror_and_propagate(
                TransactionOrigin::IncomingSync,
                &[fetched_with_etag("/a.ics", WEEKLY, "1")],
            );

            assert_eq!(journal_transaction, None, "no Event Row changed");
            assert_eq!(alone.shown(), before);
            assert_eq!(alone.mirrored("/weekly-moved.ics"), Some(alone.object));
            assert_eq!(
                alone.ignored_uids(),
                [ignored("/a.ics", Some("1"), "weekly")]
            );
            assert_eq!(
                captured_log::lines(),
                [kept_line(&alone, "/a.ics", "weekly", "/weekly-moved.ics")]
            );

            alone.mirror_and_propagate(
                TransactionOrigin::IncomingSync,
                &[
                    fetched_with_etag("/a.ics", WEEKLY, "2"),
                    fetched("/weekly-moved.ics", &MOVED.replace("Moved", "Moved again")),
                ],
            );

            assert_eq!(alone.titles(), ["Moved again"]);
            assert_eq!(alone.shown()[0].1, alone.moved);
            assert_eq!(
                alone.ignored_uids(),
                [ignored("/a.ics", Some("2"), "weekly")]
            );
        }

        /// [`Alone`] with the Event "A" of [`OTHER`]'s UID kept at `/a.ics`,
        /// and an Ignored Resource holding it at `/b.ics`, titled "B", with
        /// the ETag "1". Returns the Event Row of "A".
        fn kept_and_ignored() -> (Alone, EventId) {
            let alone = Alone::new();
            alone.incoming_sync(vec![other_at("/a.ics", "A", "1")], &[]);
            alone.incoming_sync(vec![other_at("/b.ics", "B", "1")], &[]);
            assert_eq!(
                alone.ignored_uids(),
                [ignored("/b.ics", Some("1"), "other")]
            );
            let id = alone.shown()[0].1;
            (alone, id)
        }

        /// The Ignored Resource, fetched again as the remote layer does
        /// when the one keeping the UID stops, takes it over in the same
        /// Incoming Sync: the Event stays, with its content.
        #[test]
        fn an_ignored_resource_takes_over_once_the_kept_one_is_removed() {
            let (alone, id) = kept_and_ignored();

            alone.incoming_sync(vec![other_at("/b.ics", "B", "1")], &["/a.ics"]);

            assert_eq!(alone.titles(), ["B", "Moved"]);
            assert_eq!(alone.shown()[0].1, id);
            assert_eq!(alone.mirrored("/a.ics"), None);
            assert!(alone.mirrored("/b.ics").is_some());
            assert_eq!(alone.ignored_uids(), []);
        }

        /// The kept one coming to hold another UID, several or nothing
        /// FreeCal keeps.
        #[test]
        fn an_ignored_resource_takes_over_once_the_kept_one_holds_something_else() {
            let review = "BEGIN:VEVENT\r\nUID:review\r\nSUMMARY:Review\r\n\
                DTSTART:20261020T080000Z\r\nDTEND:20261020T090000Z\r\nEND:VEVENT\r\n";
            let todo = "BEGIN:VTODO\r\nUID:todo\r\nSUMMARY:Todo\r\nEND:VTODO\r\n";
            let cases = [
                (review.to_owned(), vec!["B", "Moved", "Review"], vec![]),
                (
                    format!("{}{review}", other("A")),
                    vec!["B", "Moved"],
                    vec![("/a.ics".to_owned(), Some("2".to_owned()), None)],
                ),
                (todo.to_owned(), vec!["B", "Moved"], vec![]),
            ];
            for (holds, titles, ignored_after) in cases {
                let (alone, id) = kept_and_ignored();

                alone.incoming_sync(
                    vec![
                        fetched_with_etag("/a.ics", &holds, "2"),
                        other_at("/b.ics", "B", "1"),
                    ],
                    &[],
                );

                assert_eq!(alone.titles(), titles, "{holds}");
                assert_eq!(alone.shown()[0].1, id);
                assert!(alone.mirrored("/b.ics").is_some());
                assert_eq!(alone.ignored_uids(), ignored_after);
            }
        }

        /// The one holding the Recurring Event first, else the lowest href;
        /// the others stay ignored with their new ETags.
        #[test]
        fn of_several_ignored_resources_taking_over_the_fixed_order_decides() {
            let (alone, _) = kept_and_ignored();
            alone.incoming_sync(vec![other_at("/c.ics", "C", "1")], &[]);

            alone.incoming_sync(
                vec![other_at("/c.ics", "C", "2"), other_at("/b.ics", "B", "2")],
                &["/a.ics"],
            );

            assert_eq!(alone.titles(), ["B", "Moved"]);
            assert_eq!(
                alone.ignored_uids(),
                [ignored("/c.ics", Some("2"), "other")]
            );

            let alone = Alone::new();
            alone.incoming_sync(vec![fetched_with_etag("/z.ics", WEEKLY, "1")], &[]);
            let moved_again = MOVED.replace("Moved", "Moved again");
            alone.incoming_sync(vec![fetched_with_etag("/a.ics", &moved_again, "1")], &[]);

            alone.incoming_sync(
                vec![
                    fetched_with_etag("/a.ics", &moved_again, "1"),
                    fetched_with_etag("/z.ics", WEEKLY, "1"),
                ],
                &["/weekly-moved.ics"],
            );

            assert_eq!(alone.titles(), ["Weekly", "Weekly", "Weekly"]);
            assert_eq!(
                alone.ignored_uids(),
                [ignored("/a.ics", Some("1"), "weekly")]
            );
        }

        /// The Ignored Resource keeps its ETag, so that it is fetched again.
        #[test]
        fn with_no_ignored_resource_readable_on_takeover_the_event_rows_are_deleted() {
            let (alone, _) = kept_and_ignored();

            alone.incoming_sync(vec![unreadable_at("/b.ics", "2")], &["/a.ics"]);

            assert_eq!(alone.titles(), ["Moved"]);
            assert_eq!(alone.mirrored("/a.ics"), None);
            assert_eq!(
                alone.ignored_uids(),
                [ignored("/b.ics", Some("1"), "other")]
            );
        }

        #[test]
        fn a_kept_resource_that_becomes_unreadable_keeps_the_uid_and_its_event_rows() {
            let (alone, id) = kept_and_ignored();
            let kept = alone.mirrored("/a.ics");

            alone.incoming_sync(
                vec![unreadable_at("/a.ics", "2"), other_at("/b.ics", "B", "2")],
                &[],
            );

            assert_eq!(alone.titles(), ["A", "Moved"]);
            assert_eq!(alone.shown()[0].1, id);
            assert_eq!(alone.mirrored("/a.ics"), kept);
            assert_eq!(
                alone.ignored_uids(),
                [ignored("/b.ics", Some("2"), "other")]
            );
        }

        /// A new resource holding its UID in the Incoming Sync that removed
        /// it is it moved, before Ignored Resources by the fixed order.
        #[test]
        fn a_kept_resource_moved_to_another_href_keeps_its_event_rows() {
            let (alone, id) = kept_and_ignored();
            let kept = alone.mirrored("/a.ics");

            alone.incoming_sync(
                vec![
                    other_at("/z-moved.ics", "A moved", "1"),
                    other_at("/b.ics", "B", "1"),
                ],
                &["/a.ics"],
            );

            assert_eq!(alone.titles(), ["A moved", "Moved"]);
            assert_eq!(alone.shown()[0].1, id);
            assert_eq!(alone.mirrored("/z-moved.ics"), kept);
            assert_eq!(
                alone.ignored_uids(),
                [ignored("/b.ics", Some("1"), "other")]
            );
        }
    }
}
