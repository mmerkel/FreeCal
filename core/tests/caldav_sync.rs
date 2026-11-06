mod support;

use freecal_core::{
    AccountId, CalDavLogin, CalendarId, Changed, Core, Event, ServerObject, Signal,
};
use support::radicale::{NewCalendar, Radicale, User};
use support::{Harness, date, event, ics, make_writable};

/// A user on Radicale with a Calendar "Work", and a new Core with the
/// user's Account added and that Calendar's first sync done.
struct Synced {
    radicale: &'static Radicale,
    user: User,
    /// The Calendar's URL on the server.
    url: String,
    harness: Harness,
    core: Core,
    account: AccountId,
    work: CalendarId,
}

impl Synced {
    /// Puts `events` (name, iCalendar object) on the server, then adds the
    /// Calendar.
    fn with(events: &[(&str, Vec<u8>)]) -> Self {
        Self::with_calendar(NewCalendar::default(), events)
    }

    /// Like [`Synced::with`], with the Calendar unmarked read-only, so that
    /// its changes can be undone.
    fn writable(events: &[(&str, Vec<u8>)]) -> Self {
        let synced = Self::with(events);
        make_writable(&synced.harness, synced.work);
        synced
    }

    fn with_calendar(calendar: NewCalendar, events: &[(&str, Vec<u8>)]) -> Self {
        Self::arranged(calendar, |radicale, user, url| {
            for (name, ics) in events {
                radicale.put_event(user, url, name, ics.clone());
            }
        })
    }

    /// Like [`Synced::with`], with `files` written straight into the
    /// server's storage (see [`Radicale::write_event_file`]).
    fn with_files(files: &[(&str, Vec<u8>)]) -> Self {
        Self::arranged(NewCalendar::default(), |radicale, _, url| {
            for (name, ics) in files {
                radicale.write_event_file(url, name, ics.clone());
            }
        })
    }

    /// A new Core with the Calendar `calendar` added once `arrange` put
    /// what it holds on the server.
    fn arranged(calendar: NewCalendar, arrange: impl FnOnce(&Radicale, &User, &str)) -> Self {
        let radicale = Radicale::shared();
        let user = radicale.user();
        let url = radicale.create_calendar(
            &user,
            NewCalendar {
                name: Some("Work"),
                ..calendar
            },
        );
        arrange(radicale, &user, &url);
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let account = core
            .add_caldav_account(&CalDavLogin {
                url: radicale.url(),
                username: user.name.clone(),
                password: user.password.clone(),
            })
            .unwrap()
            .account
            .id;
        let work = core
            .add_calendars(account, std::slice::from_ref(&url))
            .unwrap()[0]
            .id;
        harness.signals.clear();
        Self {
            radicale,
            user,
            url,
            harness,
            core,
            account,
            work,
        }
    }

    fn put(&self, name: &str, ics: Vec<u8>) {
        self.radicale.put_event(&self.user, &self.url, name, ics);
    }

    fn delete(&self, name: &str) {
        self.radicale.delete_event(&self.user, &self.url, name);
    }

    /// Writes the object `name` straight into the server's storage.
    fn write(&self, name: &str, ics: Vec<u8>) {
        self.radicale.write_event_file(&self.url, name, ics);
    }

    /// Makes the Calendar sync as on a server without sync tokens, listing
    /// the whole Calendar each time.
    fn without_sync_tokens(&self) {
        self.harness.arrange_local_store(&format!(
            "UPDATE calendar SET sync_token = NULL WHERE id = {}",
            self.work.0
        ));
    }

    /// The object the Calendar's Event titled `title` is in, as FreeCal
    /// holds it.
    fn held(&self, title: &str) -> Option<ServerObject> {
        self.core
            .original_server_data(self.event(title).id)
            .unwrap()
    }

    /// The address of the object the Event titled `title` is in.
    fn href(&self, title: &str) -> String {
        self.held(title).expect("an object").href
    }

    /// Syncs the Account, which has only this Calendar, and returns the
    /// Calendar's Journal Transaction.
    fn sync(&self) -> Option<freecal_core::JournalTransactionId> {
        let synced = self.core.sync_account(self.account).unwrap();
        assert_eq!(
            synced
                .iter()
                .map(|changed| changed.value)
                .collect::<Vec<_>>(),
            [self.work]
        );
        synced[0].journal_transaction
    }

    /// The Calendar's Event titled `title`, in October 2026.
    fn event(&self, title: &str) -> Event {
        let occurrence = self
            .core
            .occurrences(date("2026-10-01"), date("2026-11-01"))
            .unwrap()
            .into_iter()
            .find(|occurrence| occurrence.calendar_id == self.work && occurrence.title == title)
            .unwrap_or_else(|| panic!("an Event titled {title:?}"));
        self.core.event(occurrence.event_id).unwrap()
    }

    /// The ETag the server serves the object `name` with.
    fn etag(&self, name: &str) -> String {
        self.radicale.get_event(&self.user, &self.url, name).0
    }

    fn held_etag(&self, title: &str) -> Option<String> {
        self.held(title).and_then(|server| server.etag)
    }

    fn changed_signals(&self) -> Vec<Signal> {
        vec![Signal::OccurrencesChanged {
            calendar_ids: vec![self.work],
        }]
    }

    /// The titles of the Calendar's Occurrences in October 2026, in order.
    fn titles(&self) -> Vec<String> {
        self.core
            .occurrences(date("2026-10-01"), date("2026-11-01"))
            .unwrap()
            .into_iter()
            .filter(|occurrence| occurrence.calendar_id == self.work)
            .map(|occurrence| occurrence.title)
            .collect()
    }
}

/// The object of an Event on `day` (`YYYYMMDD`) from 07:00 to 08:00 UTC.
fn titled(uid: &str, title: &str, day: &str) -> Vec<u8> {
    ics(&format!(
        "BEGIN:VEVENT\nUID:{uid}\nSUMMARY:{title}\nDTSTART:{day}T070000Z\n\
         DTEND:{day}T080000Z\nEND:VEVENT\n"
    ))
}

fn standup(title: &str) -> Vec<u8> {
    ics(&format!(
        "BEGIN:VEVENT\nUID:standup\nSUMMARY:{title}\nDTSTART:20261005T070000Z\n\
         DTEND:20261005T073000Z\nEND:VEVENT\n"
    ))
}

#[test]
fn a_sync_pulls_an_event_created_on_the_server() {
    let synced = Synced::with(&[("standup", standup("Standup"))]);
    synced.put(
        "review",
        ics(&event(
            "review",
            "DTSTART:20261006T070000Z",
            "DTEND:20261006T080000Z",
        )),
    );

    synced.core.sync_account(synced.account).unwrap();

    assert_eq!(synced.titles(), ["Standup", "review"]);
    assert_eq!(
        synced.harness.signals.received(),
        [Signal::OccurrencesChanged {
            calendar_ids: vec![synced.work]
        }]
    );
}

#[test]
fn a_sync_pulls_an_event_changed_on_the_server_with_its_new_server_data() {
    let synced = Synced::with(&[("standup", standup("Standup"))]);
    synced.put("standup", standup("Standup moved"));

    let journal_transaction = synced.sync();

    assert!(journal_transaction.is_some());
    assert_eq!(synced.titles(), ["Standup moved"]);
    assert_eq!(
        synced.held_etag("Standup moved"),
        Some(synced.etag("standup"))
    );
    assert_eq!(synced.harness.signals.received(), synced.changed_signals());
}

#[test]
fn a_sync_deletes_an_event_deleted_on_the_server_with_its_exceptions() {
    let synced = Synced::with(&[
        ("standup", standup("Standup")),
        (
            "weekly",
            ics(
                "BEGIN:VEVENT\nUID:weekly\nSUMMARY:Weekly\nDTSTART:20261005T080000Z\n\
                 DTEND:20261005T090000Z\nRRULE:FREQ=WEEKLY;COUNT=3\nEND:VEVENT\n\
                 BEGIN:VEVENT\nUID:weekly\nSUMMARY:Moved\nRECURRENCE-ID:20261012T080000Z\n\
                 DTSTART:20261012T090000Z\nDTEND:20261012T100000Z\nEND:VEVENT\n",
            ),
        ),
    ]);
    assert_eq!(synced.titles(), ["Standup", "Weekly", "Moved", "Weekly"]);
    synced.delete("weekly");

    synced.sync();

    assert_eq!(synced.titles(), ["Standup"]);
}

#[test]
fn a_sync_replaces_a_changed_occurrence_whose_recurrence_id_is_written_differently() {
    let weekly = |moved: &str| {
        ics(&format!(
            "BEGIN:VEVENT\nUID:weekly\nSUMMARY:Weekly\nDTSTART:20261005T080000Z\n\
             DTEND:20261005T090000Z\nRRULE:FREQ=WEEKLY;COUNT=3\nEND:VEVENT\n\
             BEGIN:VEVENT\nUID:weekly\nSUMMARY:{moved}\nRECURRENCE-ID:20261012T080000Z\n\
             DTSTART:20261012T090000Z\nDTEND:20261012T100000Z\nEND:VEVENT\n"
        ))
    };
    let synced = Synced::with(&[("weekly", weekly("Moved"))]);
    let id = synced.event("Moved").id;
    // Another form of the same time, as a stored field may have it.
    synced.harness.arrange_local_store(&format!(
        "UPDATE event SET recurrence_id = '2026-10-12T8:0:0' WHERE id = {}",
        id.0
    ));
    synced.put("weekly", weekly("Moved again"));

    synced.sync();

    assert_eq!(synced.titles(), ["Weekly", "Moved again", "Weekly"]);
    assert_eq!(synced.event("Moved again").id, id);
}

#[test]
fn a_sync_that_changes_nothing_records_no_journal_transaction_and_sends_nothing() {
    let synced = Synced::with(&[("standup", standup("Standup"))]);

    let journal_transaction = synced.sync();

    assert_eq!(journal_transaction, None);
    assert_eq!(synced.harness.signals.received(), []);
}

#[test]
fn many_changes_send_one_occurrences_changed() {
    let synced = Synced::with(&[("standup", standup("Standup"))]);
    synced.put("standup", standup("Standup moved"));
    for (uid, day) in [("a", "20261006"), ("b", "20261007"), ("c", "20261008")] {
        synced.put(uid, titled(uid, uid, day));
    }

    synced.sync();

    assert_eq!(synced.titles(), ["Standup moved", "a", "b", "c"]);
    assert_eq!(synced.harness.signals.received(), synced.changed_signals());
}

#[test]
fn each_calendar_syncs_as_a_journal_transaction_of_its_own_with_its_own_signal() {
    let synced = Synced::with(&[("standup", standup("Standup"))]);
    let home_url = synced.radicale.create_calendar(
        &synced.user,
        NewCalendar {
            name: Some("Home"),
            ..NewCalendar::default()
        },
    );
    let home = synced
        .core
        .add_calendars(synced.account, std::slice::from_ref(&home_url))
        .unwrap()[0]
        .id;
    synced.put("standup", standup("Standup moved"));
    synced
        .radicale
        .put_event(&synced.user, &home_url, "standup", standup("Dentist"));
    synced.harness.signals.clear();

    let changed = synced.core.sync_account(synced.account).unwrap();

    assert_eq!(
        changed
            .iter()
            .map(|changed| changed.value)
            .collect::<Vec<_>>(),
        [synced.work, home]
    );
    assert!(
        changed
            .iter()
            .all(|changed| changed.journal_transaction.is_some())
    );
    assert_ne!(
        changed[0].journal_transaction,
        changed[1].journal_transaction
    );
    assert_eq!(
        synced.harness.signals.received(),
        [
            Signal::OccurrencesChanged {
                calendar_ids: vec![synced.work]
            },
            Signal::OccurrencesChanged {
                calendar_ids: vec![home]
            },
        ]
    );
}

#[test]
fn a_sync_writes_into_a_read_only_calendar_and_records_it() {
    let synced = Synced::with(&[("standup", standup("Standup"))]);
    // As every newly added server Calendar is, until 1.0.
    assert!(synced.core.list_calendars().unwrap()[0].read_only);
    synced.put("standup", standup("Standup moved"));

    let journal_transaction = synced.sync();

    assert!(journal_transaction.is_some());
    assert_eq!(synced.titles(), ["Standup moved"]);
}

mod undo {
    use super::*;

    /// ADR 0010: the next write is conditional on what the server has now.
    #[test]
    fn restores_the_previous_event_version_and_leaves_the_object_as_the_server_has_it() {
        let synced = Synced::writable(&[("standup", standup("Standup"))]);
        synced.put("standup", standup("Standup moved"));
        let journal_transaction = synced.sync().unwrap();
        let served = synced
            .radicale
            .get_event(&synced.user, &synced.url, "standup");

        let undone = synced
            .core
            .undo_journal_transaction(journal_transaction)
            .unwrap();

        assert!(undone.journal_transaction.is_some());
        assert_eq!(synced.titles(), ["Standup"]);
        let held = synced.held("Standup").unwrap();
        assert_eq!((held.etag, held.data), (Some(served.0), served.1));
    }

    #[test]
    fn of_a_journal_transaction_is_not_blocked_by_a_later_sync_that_changed_only_the_object() {
        let synced = Synced::writable(&[("standup", standup("Standup"))]);
        synced.put("standup", standup("Standup moved"));
        let journal_transaction = synced.sync().unwrap();
        // A property FreeCal doesn't read: a new ETag and data, the same Event.
        synced.put(
            "standup",
            ics(
                "BEGIN:VEVENT\nUID:standup\nSUMMARY:Standup moved\nDTSTART:20261005T070000Z\n\
                 DTEND:20261005T073000Z\nX-UNKNOWN:new\nEND:VEVENT\n",
            ),
        );

        assert_eq!(synced.sync(), None);
        assert_eq!(
            synced.held_etag("Standup moved"),
            Some(synced.etag("standup"))
        );
        synced
            .core
            .undo_journal_transaction(journal_transaction)
            .unwrap();

        assert_eq!(synced.titles(), ["Standup"]);
    }

    /// As a delete already sent would be: the Event counts as not yet on
    /// the server.
    #[test]
    fn of_a_delete_brings_back_an_event_without_the_object_the_server_removed_since() {
        let synced = Synced::writable(&[("standup", standup("Standup"))]);
        let deleted = synced
            .core
            .delete_event(synced.event("Standup").id)
            .unwrap();
        synced.delete("standup");
        synced.sync();

        synced
            .core
            .undo_journal_transaction(deleted.journal_transaction.unwrap())
            .unwrap();

        let id = synced.event("Standup").id;
        assert_eq!(synced.core.original_server_data(id).unwrap(), None);
    }

    #[test]
    fn brings_back_what_the_server_deleted_and_removes_what_it_added() {
        let synced = Synced::writable(&[("standup", standup("Standup"))]);
        synced.delete("standup");
        synced.put(
            "review",
            ics(&event(
                "review",
                "DTSTART:20261006T070000Z",
                "DTEND:20261006T080000Z",
            )),
        );
        let journal_transaction = synced.sync().unwrap();
        assert_eq!(synced.titles(), ["review"]);

        synced
            .core
            .undo_journal_transaction(journal_transaction)
            .unwrap();

        assert_eq!(synced.titles(), ["Standup"]);
    }
}

mod floating_times {
    use super::*;

    fn floating_standup(title: &str) -> Vec<u8> {
        ics(&format!(
            "BEGIN:VEVENT\nUID:standup\nSUMMARY:{title}\nDTSTART:20261005T090000\n\
             DTEND:20261005T093000\nEND:VEVENT\n"
        ))
    }

    /// ADR 0007, rule 2.
    #[test]
    fn a_new_version_floating_at_both_ends_keeps_the_zone_pinned_before() {
        let synced = Synced::with(&[("standup", floating_standup("Standup"))]);
        assert_eq!(
            synced.event("Standup").pinned_zone.as_deref(),
            Some("Europe/Berlin")
        );
        synced.harness.clock.set_time_zone("Asia/Tokyo");
        synced.put("standup", floating_standup("Standup renamed"));

        synced.sync();

        let standup = synced.event("Standup renamed");
        assert_eq!(standup.pinned_zone.as_deref(), Some("Europe/Berlin"));
        // 09:00 in Berlin, shown in Tokyo.
        assert_eq!(
            standup.when,
            support::timed("2026-10-05T16:00:00", "2026-10-05T16:30:00")
        );
    }

    /// ADR 0007, rule 3.
    #[test]
    fn one_floating_end_of_a_new_version_gets_the_zone_of_the_other() {
        let synced = Synced::with(&[("standup", floating_standup("Standup"))]);
        synced.put(
            "standup",
            ics("BEGIN:VEVENT\nUID:standup\nSUMMARY:Standup\n\
                 DTSTART;TZID=America/New_York:20261005T090000\n\
                 DTEND:20261005T093000\nEND:VEVENT\n"),
        );

        synced.sync();

        let standup = synced.event("Standup");
        assert_eq!(standup.pinned_zone.as_deref(), Some("America/New_York"));
        assert_eq!(
            standup.when,
            support::timed("2026-10-05T15:00:00", "2026-10-05T15:30:00")
        );
    }
}

#[test]
fn a_calendar_whose_first_sync_didnt_finish_gets_the_rest_of_it_unrecorded() {
    let synced = Synced::with(&[
        ("standup", standup("Standup")),
        (
            "review",
            ics(&event(
                "review",
                "DTSTART:20261006T070000Z",
                "DTEND:20261006T080000Z",
            )),
        ),
    ]);
    // As a first sync that stopped before the review.
    synced.harness.arrange_local_store(&format!(
        "UPDATE calendar SET synced = 0, sync_token = NULL WHERE id = {0};
         DELETE FROM event WHERE calendar_id = {0} AND uid = 'review';
         DELETE FROM server_object WHERE calendar_id = {0} AND href LIKE '%/review.ics';",
        synced.work.0
    ));
    assert_eq!(synced.titles(), ["Standup"]);
    synced.put(
        "late",
        ics(&event(
            "late",
            "DTSTART:20261007T070000Z",
            "DTEND:20261007T080000Z",
        )),
    );
    synced.delete("standup");

    let journal_transaction = synced.sync();

    assert_eq!(journal_transaction, None, "the First Sync isn't recorded");
    assert_eq!(synced.titles(), ["review", "late"]);
    assert_eq!(
        synced.harness.signals.received(),
        [
            Signal::CalendarsChanged,
            synced.changed_signals()[0].clone(),
            Signal::CalendarsChanged,
        ]
    );
    // It is finished now: the next sync is recorded.
    synced.put("late", titled("late", "late moved", "20261007"));
    assert!(synced.sync().is_some());
}

#[test]
fn a_sync_token_the_server_refuses_lists_the_whole_calendar_instead() {
    let synced = Synced::with(&[
        ("standup", standup("Standup")),
        (
            "review",
            ics(&event(
                "review",
                "DTSTART:20261006T070000Z",
                "DTEND:20261006T080000Z",
            )),
        ),
    ]);
    synced.harness.arrange_local_store(&format!(
        "UPDATE calendar SET sync_token = 'http://radicale.org/ns/sync/unknown' WHERE id = {}",
        synced.work.0
    ));
    synced.delete("standup");
    synced.put("review", titled("review", "review moved", "20261006"));

    let journal_transaction = synced.sync();

    assert!(journal_transaction.is_some());
    assert_eq!(synced.titles(), ["review moved"]);
}

#[test]
fn the_local_account_has_nothing_to_sync() {
    let harness = Harness::new();
    let core = harness.open();
    let local = core.list_accounts().unwrap()[0].id;

    assert_eq!(
        core.sync_account(local).unwrap(),
        Vec::<Changed<CalendarId>>::new()
    );
}

#[test]
fn an_event_whose_object_moved_to_another_address_stays_the_same_event() {
    let synced = Synced::with(&[("standup", standup("Standup"))]);
    let id = synced.event("Standup").id;
    synced.delete("standup");
    synced.put("standup-moved", standup("Standup moved"));

    synced.sync();

    let moved = synced.event("Standup moved");
    assert_eq!(moved.id, id);
    let server = synced.core.original_server_data(id).unwrap().unwrap();
    assert!(
        server.href.ends_with("/standup-moved.ics"),
        "{}",
        server.href
    );
}

/// ADR 0010: sync matches Server Objects by address, and each one changes
/// only its own Event Rows.
mod server_objects {
    use super::*;

    fn weekly(title: &str) -> Vec<u8> {
        ics(&recurring(title))
    }

    /// A weekly Recurring Event with three Occurrences, from 5 October.
    fn recurring(title: &str) -> String {
        format!(
            "BEGIN:VEVENT\nUID:weekly\nSUMMARY:{title}\nDTSTART:20261005T080000Z\n\
             DTEND:20261005T090000Z\nRRULE:FREQ=WEEKLY;COUNT=3\nEND:VEVENT\n"
        )
    }

    /// An Exception of `weekly`'s second Occurrence.
    fn moved(title: &str) -> String {
        format!(
            "BEGIN:VEVENT\nUID:weekly\nSUMMARY:{title}\nRECURRENCE-ID:20261012T080000Z\n\
             DTSTART:20261012T090000Z\nDTEND:20261012T100000Z\nEND:VEVENT\n"
        )
    }

    /// As Google keeps a Recurring Event: each Exception is a Server Object
    /// of its own.
    fn google_like() -> Synced {
        Synced::with_files(&[
            ("weekly", weekly("Weekly")),
            ("weekly-moved", ics(&moved("Moved"))),
        ])
    }

    /// ADR 0011: of two Server Objects alike, the first mirrored
    /// corresponds, which depends on the order the server listed them.
    #[test]
    fn of_two_objects_with_one_uid_the_first_corresponds_on_a_server_without_sync_tokens() {
        let synced = Synced::with_files(&[
            ("standup", standup("Standup")),
            ("standup-copy", standup("Standup copy")),
        ]);
        synced.without_sync_tokens();
        let shown = synced.titles();
        assert_eq!(shown.len(), 1, "{shown:?}");
        let (corresponding, other) = if shown[0] == "Standup" {
            ("standup", "standup-copy")
        } else {
            ("standup-copy", "standup")
        };
        assert!(
            synced
                .href(&shown[0])
                .ends_with(&format!("/{corresponding}.ics"))
        );

        assert_eq!(synced.sync(), None);
        assert_eq!(synced.sync(), None);

        synced.write(other, standup("Standup elsewhere"));
        assert_eq!(
            synced.sync(),
            None,
            "the other Server Object has no Event Rows"
        );
        assert_eq!(synced.titles(), shown);
        synced.write(corresponding, standup("Standup moved"));
        assert!(synced.sync().is_some());
        assert_eq!(synced.titles(), ["Standup moved"]);
    }

    #[test]
    fn the_object_holding_the_recurring_event_corresponds_whichever_came_first() {
        let together = google_like();
        let exception_first = Synced::with_files(&[("weekly-moved", ics(&moved("Moved")))]);
        assert_eq!(exception_first.titles(), ["Moved"]);
        exception_first.write("weekly", weekly("Weekly"));
        exception_first.sync();
        let recurring_first = Synced::with_files(&[("weekly", weekly("Weekly"))]);
        recurring_first.write("weekly-moved", ics(&moved("Moved")));
        assert_eq!(recurring_first.sync(), None);

        for synced in [together, exception_first, recurring_first] {
            assert_eq!(synced.titles(), ["Weekly", "Weekly", "Weekly"]);
            assert!(synced.href("Weekly").ends_with("/weekly.ics"));
        }
    }

    #[test]
    fn an_object_that_corresponds_to_no_uid_changes_no_event_row() {
        let synced = google_like();
        synced.write("weekly-moved", ics(&moved("Moved again")));

        assert_eq!(synced.sync(), None);

        assert_eq!(synced.titles(), ["Weekly", "Weekly", "Weekly"]);
    }

    /// A removed Server Object goes from the Mirror; another holding its
    /// UID becomes its Corresponding Server Object, its Event Rows read from
    /// what FreeCal holds of it.
    #[test]
    fn the_next_object_corresponds_when_the_corresponding_one_goes() {
        for tokens in [true, false] {
            let synced = Synced::writable(&[]);
            synced.write("weekly", weekly("Weekly"));
            synced.write("weekly-moved", ics(&moved("Moved")));
            synced.sync();
            if !tokens {
                synced.without_sync_tokens();
            }
            synced.delete("weekly");

            let journal_transaction = synced.sync().unwrap();

            assert_eq!(synced.titles(), ["Moved"], "tokens: {tokens}");
            assert!(synced.href("Moved").ends_with("/weekly-moved.ics"));
            synced
                .core
                .undo_journal_transaction(journal_transaction)
                .unwrap();
            assert_eq!(synced.titles(), ["Weekly", "Weekly", "Weekly"]);
        }
    }

    /// It keeps its place in the order objects were mirrored.
    #[test]
    fn an_object_that_corresponds_to_no_uid_moved_to_another_address_stays_the_same_object() {
        let synced = Synced::with_files(&[("standup", standup("Standup"))]);
        synced.write("standup-b", standup("Standup B"));
        synced.sync();
        synced.write("standup-c", standup("Standup C"));
        synced.sync();
        synced.delete("standup-b");
        synced.write("standup-b-moved", standup("Standup B"));
        synced.sync();

        synced.delete("standup");
        synced.sync();

        assert_eq!(synced.titles(), ["Standup B"]);
        assert!(synced.href("Standup B").ends_with("/standup-b-moved.ics"));
    }

    #[test]
    fn a_removed_object_that_corresponds_to_no_uid_changes_no_event_row() {
        let synced = google_like();
        synced.delete("weekly-moved");

        assert_eq!(synced.sync(), None);

        assert_eq!(synced.titles(), ["Weekly", "Weekly", "Weekly"]);
    }

    #[test]
    fn of_two_exceptions_for_one_occurrence_in_an_object_the_highest_sequence_is_kept() {
        let with_sequence = |sequence: u32, title: &str| {
            moved(title).replace("END:VEVENT", &format!("SEQUENCE:{sequence}\nEND:VEVENT"))
        };
        let synced = Synced::with_files(&[(
            "weekly",
            ics(&format!(
                "{}{}{}",
                recurring("Weekly"),
                with_sequence(1, "Older"),
                with_sequence(2, "Latest"),
            )),
        )]);

        assert_eq!(synced.titles(), ["Weekly", "Latest", "Weekly"]);
    }

    /// When the Corresponding Server Object goes, the other's Exception
    /// stays, an Event of its own as if it had come alone (ticket 88).
    #[test]
    fn of_two_objects_with_an_exception_for_one_occurrence_the_recurring_events_shows() {
        let synced = Synced::with_files(&[
            (
                "weekly",
                ics(&format!("{}{}", recurring("Weekly"), moved("Moved"))),
            ),
            ("weekly-moved", ics(&moved("Moved elsewhere"))),
        ]);
        assert_eq!(synced.titles(), ["Weekly", "Moved", "Weekly"]);

        synced.delete("weekly");
        synced.sync();

        assert_eq!(synced.titles(), ["Moved elsewhere"]);
    }

    /// ADR 0011: an Event Row the server doesn't have yet belongs to the
    /// Server Object holding its UID.
    #[test]
    fn a_new_exception_not_yet_sent_belongs_to_the_object_of_its_recurring_event() {
        let synced = Synced::writable(&[("weekly", weekly("Weekly"))]);
        let second = support::on(&synced.core, "2026-10-12");

        let moved = synced
            .core
            .reschedule_occurrence(
                second.event_id,
                second.recurrence_id.as_deref().unwrap(),
                freecal_core::Scope::This,
                false,
                support::timed("2026-10-12T12:00:00", "2026-10-12T13:00:00"),
            )
            .unwrap()
            .value;

        assert_ne!(moved.event_id, second.event_id, "an Exception");
        let held = synced.core.original_server_data(moved.event_id).unwrap();
        assert!(held.is_some());
        assert_eq!(held, synced.held("Weekly"));
    }

    /// Whichever of the two was the UID's Corresponding Server Object, its
    /// Event stays the same.
    #[test]
    fn an_object_whose_uid_changed_leaves_its_old_uid_to_the_next_object_holding_it() {
        let synced = Synced::with_files(&[
            ("standup", standup("Standup")),
            ("standup-copy", standup("Standup copy")),
        ]);
        let id = synced.event(&synced.titles()[0]).id;
        synced.write("standup", titled("review", "Review", "20261006"));

        synced.sync();

        assert_eq!(synced.titles(), ["Standup copy", "Review"]);
        assert_eq!(synced.event("Standup copy").id, id);
        assert!(synced.href("Standup copy").ends_with("/standup-copy.ics"));
    }

    #[test]
    fn an_object_freecal_can_no_longer_read_keeps_its_last_readable_version() {
        let synced = Synced::with(&[("standup", standup("Standup"))]);
        let held = synced.held("Standup");
        // Radicale takes a UTC time with a zone; FreeCal can't read it.
        synced.put(
            "standup",
            ics("BEGIN:VEVENT\nUID:standup\nSUMMARY:Standup moved\n\
                 DTSTART;TZID=Europe/Berlin:20261005T070000Z\n\
                 DTEND;TZID=Europe/Berlin:20261005T073000Z\nEND:VEVENT\n"),
        );

        assert_eq!(synced.sync(), None);

        assert_eq!(synced.titles(), ["Standup"]);
        let id = synced.event("Standup").id;
        assert_eq!(synced.core.original_server_data(id).unwrap(), held);
        synced.put("standup", standup("Standup moved"));
        synced.sync();
        assert_eq!(synced.titles(), ["Standup moved"]);
        assert_eq!(synced.event("Standup moved").id, id);
    }

    /// Exceptions without their Recurring Event, such as an invitation to
    /// one Occurrence, are Events of their own until it arrives (ticket 88).
    mod exceptions_alone {
        use super::*;

        /// `moved`, its recurrence ID given in New York's time: 04:00 there
        /// is 08:00 UTC, the Occurrence's start.
        fn moved_in_new_york(title: &str) -> String {
            moved(title).replace(
                "RECURRENCE-ID:20261012T080000Z",
                "RECURRENCE-ID;TZID=America/New_York:20261012T040000",
            )
        }

        /// The Calendar's Occurrences in October 2026, each with its title
        /// and whether it is one of a Recurring Event.
        fn shown(synced: &Synced) -> Vec<(String, bool)> {
            synced
                .core
                .occurrences(date("2026-10-01"), date("2026-11-01"))
                .unwrap()
                .into_iter()
                .map(|occurrence| (occurrence.title, occurrence.recurrence_id.is_some()))
                .collect()
        }

        fn alone(title: &str) -> Vec<(String, bool)> {
            vec![(title.to_owned(), false)]
        }

        fn weekly_with(title: &str) -> Vec<(String, bool)> {
            ["Weekly", title, "Weekly"]
                .into_iter()
                .map(|title| (title.to_owned(), true))
                .collect()
        }

        #[test]
        fn shows_at_its_own_time_in_its_object() {
            let synced = Synced::with_files(&[("weekly-moved", ics(&moved_in_new_york("Moved")))]);

            assert_eq!(shown(&synced), alone("Moved"));
            let moved = synced.event("Moved");
            assert_eq!(
                moved.when,
                support::timed("2026-10-12T11:00:00", "2026-10-12T12:00:00")
            );
            // With no Recurring Event to take the zone from, it keeps its own.
            assert_eq!(
                moved.recurrence_id.as_deref(),
                Some("2026-10-12T04:00:00[America/New_York]")
            );
            assert!(synced.href("Moved").ends_with("/weekly-moved.ics"));
        }

        /// ADR 0011: the Server Object holding the Recurring Event is the
        /// UID's Corresponding Server Object; the other keeps its Combined
        /// Event Keys, with recurrence IDs as obtained, and becomes it when
        /// that goes.
        #[test]
        fn hides_once_its_recurring_event_arrives_in_another_object_until_that_goes() {
            let synced = Synced::with_files(&[("weekly-moved", ics(&moved_in_new_york("Moved")))]);
            synced.write("weekly", weekly("Weekly"));
            synced.sync();
            assert_eq!(shown(&synced), weekly_with("Weekly"));

            synced.delete("weekly");
            synced.sync();

            assert_eq!(shown(&synced), alone("Moved"));
            assert!(synced.href("Moved").ends_with("/weekly-moved.ics"));
            // As if it had come alone, so that its next version pairs with it.
            let moved = synced.event("Moved");
            assert_eq!(
                moved.recurrence_id.as_deref(),
                Some("2026-10-12T04:00:00[America/New_York]")
            );
            synced.write("weekly-moved", ics(&moved_in_new_york("Moved again")));
            synced.sync();
            assert_eq!(synced.event("Moved again").id, moved.id);
        }

        #[test]
        fn joins_its_recurring_event_when_its_object_comes_to_hold_it() {
            let synced = Synced::with_files(&[("weekly", ics(&moved_in_new_york("Moved")))]);
            let id = synced.event("Moved").id;
            synced.write(
                "weekly",
                ics(&format!("{}{}", recurring("Weekly"), moved("Moved again"))),
            );

            synced.sync();

            assert_eq!(shown(&synced), weekly_with("Moved again"));
            assert_eq!(synced.event("Moved again").id, id);
        }

        #[test]
        fn a_known_object_that_comes_to_hold_only_it_keeps_it() {
            let synced = Synced::with_files(&[(
                "weekly",
                ics(&format!("{}{}", recurring("Weekly"), moved("Moved"))),
            )]);
            let id = synced.event("Moved").id;
            synced.write("weekly", ics(&moved("Moved again")));

            synced.sync();

            assert_eq!(shown(&synced), alone("Moved again"));
            assert_eq!(synced.event("Moved again").id, id);
        }

        #[test]
        fn one_that_matches_no_occurrence_stays_after_its_recurring_event_arrives() {
            let odd = moved("Odd").replace("20261012T080000Z", "20261013T080000Z");
            let synced = Synced::with_files(&[("weekly", ics(&odd))]);
            synced.write("weekly", ics(&format!("{}{odd}", recurring("Weekly"))));

            synced.sync();

            let shown = shown(&synced);
            assert_eq!(shown[2], ("Odd".to_owned(), false));
            assert_eq!(shown.len(), 4);
        }
    }

    /// With a sync token, which lists only what changed since.
    #[test]
    fn an_object_freecal_couldnt_read_is_fetched_again_on_each_sync_until_it_reads() {
        let synced = Synced::with(&[("standup", standup("Standup"))]);
        // As if FreeCal couldn't read the version the server has, and held
        // an earlier one.
        synced.harness.arrange_local_store(&format!(
            "UPDATE server_object SET unreadable = 1 WHERE calendar_id = {0};
             UPDATE event SET title = 'Earlier' WHERE calendar_id = {0};",
            synced.work.0
        ));

        assert!(synced.sync().is_some());

        assert_eq!(synced.titles(), ["Standup"]);
        assert_eq!(synced.sync(), None, "it read, so it isn't fetched again");
    }
}
