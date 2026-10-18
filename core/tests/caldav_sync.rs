mod support;

use freecal_core::{AccountId, CalDavLogin, CalendarId, Changed, Core, Event, Signal};
use support::radicale::{NewCalendar, Radicale, User};
use support::{Harness, date, event, ics};

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

    fn with_calendar(calendar: NewCalendar, events: &[(&str, Vec<u8>)]) -> Self {
        let radicale = Radicale::shared();
        let user = radicale.user();
        let url = radicale.create_calendar(
            &user,
            NewCalendar {
                name: Some("Work"),
                ..calendar
            },
        );
        for (name, ics) in events {
            radicale.put_event(&user, &url, name, ics.clone());
        }
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

    /// Syncs the Account, which has only this Calendar, and returns the
    /// Calendar's unit.
    fn sync(&self) -> Option<freecal_core::UnitId> {
        let synced = self.core.sync_account(self.account).unwrap();
        assert_eq!(
            synced
                .iter()
                .map(|changed| changed.value)
                .collect::<Vec<_>>(),
            [self.work]
        );
        synced[0].unit
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
        self.core
            .original_server_data(self.event(title).id)
            .unwrap()
            .and_then(|server| server.etag)
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

    let unit = synced.sync();

    assert!(unit.is_some());
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
fn a_sync_that_changes_nothing_records_no_unit_and_sends_nothing() {
    let synced = Synced::with(&[("standup", standup("Standup"))]);

    let unit = synced.sync();

    assert_eq!(unit, None);
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
fn each_calendar_syncs_as_a_unit_of_its_own_with_its_own_signal() {
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
    assert!(changed.iter().all(|changed| changed.unit.is_some()));
    assert_ne!(changed[0].unit, changed[1].unit);
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
    synced.harness.arrange_local_store(&format!(
        "UPDATE calendar SET read_only = 1 WHERE id = {}",
        synced.work.0
    ));
    synced.put("standup", standup("Standup moved"));

    let unit = synced.sync();

    assert!(unit.is_some());
    assert_eq!(synced.titles(), ["Standup moved"]);
}

mod undo {
    use super::*;

    #[test]
    fn restores_the_previous_version_with_its_server_data() {
        let synced = Synced::with(&[("standup", standup("Standup"))]);
        let pulled = synced.held_etag("Standup");
        synced.put("standup", standup("Standup moved"));
        let unit = synced.sync().unwrap();

        let undone = synced.core.undo_unit(unit).unwrap();

        assert!(undone.unit.is_some());
        assert_eq!(synced.titles(), ["Standup"]);
        assert!(pulled.is_some());
        assert_eq!(synced.held_etag("Standup"), pulled);
    }

    #[test]
    fn brings_back_what_the_server_deleted_and_removes_what_it_added() {
        let synced = Synced::with(&[("standup", standup("Standup"))]);
        synced.delete("standup");
        synced.put(
            "review",
            ics(&event(
                "review",
                "DTSTART:20261006T070000Z",
                "DTEND:20261006T080000Z",
            )),
        );
        let unit = synced.sync().unwrap();
        assert_eq!(synced.titles(), ["review"]);

        synced.core.undo_unit(unit).unwrap();

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
         DELETE FROM event WHERE calendar_id = {0} AND uid = 'review';",
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

    let unit = synced.sync();

    assert_eq!(unit, None, "the first sync is the baseline");
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

    let unit = synced.sync();

    assert!(unit.is_some());
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
