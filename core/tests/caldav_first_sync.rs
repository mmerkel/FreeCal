mod support;

use std::sync::{Arc, Mutex, Weak};

use freecal_core::{
    AccountId, CalDavLogin, CalendarId, Colour, Core, EventId, FIRST_SYNC_PAGE, Piece, Signal,
    link_allowed,
};
use support::radicale::{NewCalendar, Radicale, User};
use support::{Harness, all_day, date, draft, event, ics, make_writable, timed};

fn add_account(core: &Core, radicale: &Radicale, user: &User) -> AccountId {
    core.add_caldav_account(&CalDavLogin {
        url: radicale.url(),
        username: user.name.clone(),
        password: user.password.clone(),
    })
    .unwrap()
    .account
    .id
}

fn server_calendar(radicale: &Radicale, user: &User) -> String {
    radicale.create_calendar(
        user,
        NewCalendar {
            name: Some("Work"),
            ..NewCalendar::default()
        },
    )
}

/// The titles of a Calendar's Occurrences in October 2026, in order.
fn titles(core: &Core, calendar_id: CalendarId) -> Vec<String> {
    core.occurrences(date("2026-10-01"), date("2026-11-01"))
        .unwrap()
        .into_iter()
        .filter(|occurrence| occurrence.calendar_id == calendar_id)
        .map(|occurrence| occurrence.title)
        .collect()
}

fn occurrences_changed(harness: &Harness, calendar_id: CalendarId) -> usize {
    harness
        .signals
        .received()
        .into_iter()
        .filter(|signal| {
            *signal
                == Signal::OccurrencesChanged {
                    calendar_ids: vec![calendar_id],
                }
        })
        .count()
}

#[test]
fn adding_a_calendar_pulls_its_events() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let work = server_calendar(radicale, &user);
    radicale.put_event(
        &user,
        &work,
        "standup",
        ics(&event(
            "standup",
            "DTSTART:20261005T070000Z",
            "DTEND:20261005T073000Z",
        )),
    );
    radicale.put_event(
        &user,
        &work,
        "offsite",
        ics(&event(
            "offsite",
            "DTSTART;VALUE=DATE:20261007",
            "DTEND;VALUE=DATE:20261009",
        )),
    );
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let account = add_account(&core, radicale, &user);

    let added = core.add_calendars(account, &[work]).unwrap();

    let occurrences = core
        .occurrences(date("2026-10-01"), date("2026-11-01"))
        .unwrap();
    assert_eq!(occurrences.len(), 2);
    let standup = core.event(occurrences[0].event_id).unwrap();
    assert_eq!(standup.calendar_id, added[0].id);
    assert_eq!(standup.title, "standup");
    assert_eq!(
        standup.when,
        timed("2026-10-05T09:00:00", "2026-10-05T09:30:00")
    );
    let offsite = core.event(occurrences[1].event_id).unwrap();
    assert_eq!(offsite.when, all_day("2026-10-07", "2026-10-09"));
}

#[test]
fn a_recurring_event_comes_with_its_exceptions() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let work = server_calendar(radicale, &user);
    radicale.put_event(
        &user,
        &work,
        "weekly",
        ics(
            "BEGIN:VEVENT\nUID:weekly\nSUMMARY:Weekly\nDTSTART:20261005T070000Z\n\
             DTEND:20261005T080000Z\nRRULE:FREQ=WEEKLY;COUNT=3\nEND:VEVENT\n\
             BEGIN:VEVENT\nUID:weekly\nSUMMARY:Moved\nRECURRENCE-ID:20261012T070000Z\n\
             DTSTART:20261012T090000Z\nDTEND:20261012T100000Z\nEND:VEVENT\n",
        ),
    );
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let account = add_account(&core, radicale, &user);

    let added = core
        .add_calendars(account, std::slice::from_ref(&work))
        .unwrap();

    assert_eq!(titles(&core, added[0].id), ["Weekly", "Moved", "Weekly"]);
    // The Recurring Event and its Exception refer to the object holding them.
    let occurrences = core
        .occurrences(date("2026-10-01"), date("2026-11-01"))
        .unwrap();
    let (_, served) = radicale.get_event(&user, &work, "weekly");
    let held = |occurrence: usize| {
        core.original_server_data(occurrences[occurrence].event_id)
            .unwrap()
            .map(|server| server.data)
    };
    assert_eq!(held(0), Some(served.clone()));
    assert_eq!(held(1), Some(served));
}

/// A pulled Event, in a Calendar of a new Core, and the Core's Local
/// Calendar "Home".
fn pulled_standup() -> (Harness, Core, CalendarId, CalendarId) {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let work = server_calendar(radicale, &user);
    radicale.put_event(
        &user,
        &work,
        "standup",
        ics(&event(
            "standup",
            "DTSTART:20261005T070000Z",
            "DTEND:20261005T073000Z",
        )),
    );
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let account = add_account(&core, radicale, &user);
    let work = core.add_calendars(account, &[work]).unwrap()[0].id;
    make_writable(&harness, work);
    let local = core.list_accounts().unwrap()[0].id;
    let home = core
        .create_calendar(local, "Home", Colour::parse("#3366cc").unwrap())
        .unwrap()
        .id;
    (harness, core, work, home)
}

mod original_server_data {
    use super::*;

    fn standup(core: &Core) -> EventId {
        core.occurrences(date("2026-10-05"), date("2026-10-06"))
            .unwrap()[0]
            .event_id
    }

    #[test]
    fn an_event_freecal_makes_has_none() {
        let (_harness, core, work, home) = pulled_standup();

        for calendar_id in [work, home] {
            let made = core
                .create_event(draft(
                    calendar_id,
                    "Lunch",
                    timed("2026-10-06T12:00:00", "2026-10-06T13:00:00"),
                ))
                .unwrap()
                .value;
            assert_eq!(core.original_server_data(made.id).unwrap(), None);
        }
    }

    #[test]
    fn stays_through_an_edit() {
        let (_harness, core, work, _) = pulled_standup();
        let id = standup(&core);
        let pulled = core.original_server_data(id).unwrap();

        core.edit_event(
            id,
            draft(
                work,
                "Standup, longer",
                timed("2026-10-05T09:00:00", "2026-10-05T10:00:00"),
            ),
        )
        .unwrap();

        assert!(pulled.is_some());
        assert_eq!(core.original_server_data(id).unwrap(), pulled);
    }

    #[test]
    fn goes_with_a_move_to_another_calendar_and_comes_back_with_its_undo() {
        let (_harness, core, _, home) = pulled_standup();
        let id = standup(&core);
        let pulled = core.original_server_data(id).unwrap();

        let moved = core
            .edit_event(
                id,
                draft(
                    home,
                    "standup",
                    timed("2026-10-05T09:00:00", "2026-10-05T09:30:00"),
                ),
            )
            .unwrap();
        assert_eq!(core.original_server_data(moved.value.id).unwrap(), None);
        core.undo_unit(moved.unit.unwrap()).unwrap();

        assert!(pulled.is_some());
        assert_eq!(core.original_server_data(standup(&core)).unwrap(), pulled);
    }

    #[test]
    fn comes_back_with_the_undo_of_a_delete() {
        let (_harness, core, _, _) = pulled_standup();
        let id = standup(&core);
        let pulled = core.original_server_data(id).unwrap();

        let deleted = core.delete_event(id).unwrap();
        core.undo_unit(deleted.unit.unwrap()).unwrap();

        assert!(pulled.is_some());
        assert_eq!(core.original_server_data(standup(&core)).unwrap(), pulled);
    }
}

#[test]
fn the_first_sync_commits_page_by_page_and_reports_that_it_runs() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let work = server_calendar(radicale, &user);
    for index in 0..=FIRST_SYNC_PAGE {
        let uid = format!("event-{index:03}");
        radicale.put_event(
            &user,
            &work,
            &uid,
            ics(&event(
                &uid,
                "DTSTART:20261005T070000Z",
                "DTEND:20261005T073000Z",
            )),
        );
    }
    let harness = Harness::new();
    let core = Arc::new(harness.open_without_starting_calendars());
    let account = add_account(&core, radicale, &user);
    // What a subscriber reading the core on each Signal sees: whether the
    // Calendar is on its first sync, and how many of its Events are there.
    let seen = Arc::new(Mutex::new(Vec::new()));
    let reader: Weak<Core> = Arc::downgrade(&core);
    let record = seen.clone();
    harness.signals.on_receive(move |signal| {
        let core = reader.upgrade().unwrap();
        let calendar = core.list_calendars().unwrap().pop();
        let events = core
            .occurrences(date("2026-10-05"), date("2026-10-06"))
            .unwrap()
            .len();
        record.lock().unwrap().push((
            signal.clone(),
            calendar.map(|calendar| calendar.first_sync),
            events,
        ));
    });
    harness.signals.clear();

    let added = core.add_calendars(account, &[work]).unwrap();

    let id = added[0].id;
    let page = Signal::OccurrencesChanged {
        calendar_ids: vec![id],
    };
    assert_eq!(
        *seen.lock().unwrap(),
        [
            (Signal::CalendarsChanged, Some(true), 0),
            (page.clone(), Some(true), FIRST_SYNC_PAGE),
            (page, Some(true), FIRST_SYNC_PAGE + 1),
            (Signal::CalendarsChanged, Some(false), FIRST_SYNC_PAGE + 1),
        ]
    );
    assert!(!added[0].first_sync);
}

#[test]
fn a_calendar_removed_during_its_first_sync_stops_it() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let work = server_calendar(radicale, &user);
    for index in 0..=FIRST_SYNC_PAGE {
        let uid = format!("event-{index:03}");
        radicale.put_event(
            &user,
            &work,
            &uid,
            ics(&event(
                &uid,
                "DTSTART:20261005T070000Z",
                "DTEND:20261005T073000Z",
            )),
        );
    }
    let harness = Harness::new();
    let core = Arc::new(harness.open_without_starting_calendars());
    let account = add_account(&core, radicale, &user);
    let reader: Weak<Core> = Arc::downgrade(&core);
    harness.signals.on_receive(move |signal| {
        if let Signal::OccurrencesChanged { calendar_ids } = signal {
            let core = reader.upgrade().unwrap();
            let _ = core.remove_calendar(calendar_ids[0]);
        }
    });

    core.add_calendars(account, std::slice::from_ref(&work))
        .unwrap();

    assert_eq!(core.list_calendars().unwrap(), []);
    assert_eq!(
        core.occurrences(date("2026-10-05"), date("2026-10-06"))
            .unwrap(),
        []
    );
    assert_eq!(
        core.available_calendars(account).unwrap()[0].url,
        work,
        "it is available again"
    );
}

#[test]
fn a_calendar_with_no_events_sends_no_occurrences_changed() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let work = server_calendar(radicale, &user);
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let account = add_account(&core, radicale, &user);
    harness.signals.clear();

    core.add_calendars(account, &[work]).unwrap();

    assert_eq!(
        harness.signals.received(),
        [Signal::CalendarsChanged, Signal::CalendarsChanged]
    );
}

#[test]
fn an_event_the_server_holds_that_freecal_cant_read_is_left_out() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let work = radicale.create_calendar(
        &user,
        NewCalendar {
            name: Some("Work"),
            components: &["VEVENT", "VTODO"],
            ..NewCalendar::default()
        },
    );
    // Valid on the server, but not an Event.
    radicale.put_event(
        &user,
        &work,
        "todo",
        ics("BEGIN:VTODO\nUID:todo\nSUMMARY:Todo\nEND:VTODO\n"),
    );
    radicale.put_event(
        &user,
        &work,
        "standup",
        ics(&event(
            "standup",
            "DTSTART:20261005T070000Z",
            "DTEND:20261005T073000Z",
        )),
    );
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let account = add_account(&core, radicale, &user);

    let added = core.add_calendars(account, &[work]).unwrap();

    assert_eq!(titles(&core, added[0].id), ["standup"]);
    assert_eq!(occurrences_changed(&harness, added[0].id), 1);
}

mod floating_times {
    use super::*;

    const NEW_YORK: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Test//EN\r\n\
        BEGIN:VTIMEZONE\r\nTZID:America/New_York\r\nBEGIN:STANDARD\r\n\
        DTSTART:19701101T020000\r\nRRULE:FREQ=YEARLY;BYMONTH=11;BYDAY=1SU\r\n\
        TZOFFSETFROM:-0400\r\nTZOFFSETTO:-0500\r\nEND:STANDARD\r\nBEGIN:DAYLIGHT\r\n\
        DTSTART:19700308T020000\r\nRRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=2SU\r\n\
        TZOFFSETFROM:-0500\r\nTZOFFSETTO:-0400\r\nEND:DAYLIGHT\r\nEND:VTIMEZONE\r\n\
        END:VCALENDAR\r\n";

    fn floating_standup(radicale: &Radicale, user: &User, calendar: &str) {
        radicale.put_event(
            user,
            calendar,
            "standup",
            ics(&event(
                "standup",
                "DTSTART:20261005T090000",
                "DTEND:20261005T093000",
            )),
        );
    }

    #[test]
    fn are_pinned_to_the_calendars_own_time_zone() {
        let radicale = Radicale::shared();
        let user = radicale.user();
        let work = radicale.create_calendar(
            &user,
            NewCalendar {
                name: Some("Work"),
                timezone: Some(NEW_YORK),
                ..NewCalendar::default()
            },
        );
        floating_standup(radicale, &user, &work);
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let account = add_account(&core, radicale, &user);

        core.add_calendars(account, &[work]).unwrap();

        let occurrences = core
            .occurrences(date("2026-10-05"), date("2026-10-06"))
            .unwrap();
        let standup = core.event(occurrences[0].event_id).unwrap();
        assert_eq!(standup.pinned_zone.as_deref(), Some("America/New_York"));
        assert_eq!(
            standup.when,
            timed("2026-10-05T15:00:00", "2026-10-05T15:30:00")
        );
    }

    #[test]
    fn are_pinned_to_the_display_time_zone_without_one() {
        let radicale = Radicale::shared();
        let user = radicale.user();
        let work = server_calendar(radicale, &user);
        floating_standup(radicale, &user, &work);
        let harness = Harness::new();
        harness.clock.set_time_zone("Asia/Tokyo");
        let core = harness.open_without_starting_calendars();
        let account = add_account(&core, radicale, &user);

        core.add_calendars(account, &[work]).unwrap();

        let occurrences = core
            .occurrences(date("2026-10-05"), date("2026-10-06"))
            .unwrap();
        let standup = core.event(occurrences[0].event_id).unwrap();
        assert_eq!(standup.pinned_zone.as_deref(), Some("Asia/Tokyo"));
        assert_eq!(
            standup.when,
            timed("2026-10-05T09:00:00", "2026-10-05T09:30:00")
        );
    }
}

mod hostile {
    use super::*;
    use icalendar::{Component, EventLike};

    fn fixture() -> serde_json::Value {
        serde_json::from_str(include_str!("../../fixtures/hostile-event.json")).unwrap()
    }

    fn field(name: &str) -> String {
        fixture()[name].as_str().unwrap().to_owned()
    }

    /// Radicale holding the hostile Event in the Calendar at the URL it
    /// returns, as the object "hostile", for the user it returns.
    fn on_radicale() -> (User, String) {
        let radicale = Radicale::shared();
        let user = radicale.user();
        let work = server_calendar(radicale, &user);
        let file = icalendar::Calendar::new()
            .push(
                icalendar::Event::new()
                    .uid("hostile")
                    .summary(&field("title"))
                    .location(&field("location"))
                    .description(&field("description"))
                    .starts(support::time("2026-10-05T09:00:00"))
                    .ends(support::time("2026-10-05T10:00:00"))
                    .done(),
            )
            .done()
            .to_string()
            .into_bytes();
        radicale.put_event(&user, &work, "hostile", file);
        (user, work)
    }

    #[test]
    fn it_is_pulled_and_stored_as_text_and_allowed_links() {
        let radicale = Radicale::shared();
        let (user, work) = on_radicale();
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let account = add_account(&core, radicale, &user);

        core.add_calendars(account, &[work]).unwrap();

        let occurrences = core
            .occurrences(date("2026-10-05"), date("2026-10-06"))
            .unwrap();
        let event = core.event(occurrences[0].event_id).unwrap();
        // CalDAV's XML turns line breaks into `\n` (XML 1.0, 2.11).
        assert_eq!(event.title, field("title").replace("\r\n", "\n"));
        assert_eq!(event.location.text, field("location"));
        assert_eq!(
            serde_json::to_value(&event.description).unwrap(),
            fixture()["expected"]["html"]
        );
        assert_eq!(
            serde_json::to_value(&event.location).unwrap(),
            fixture()["expected"]["location"]
        );
        for piece in event
            .description
            .paragraphs
            .iter()
            .chain(&event.location.paragraphs)
            .flatten()
        {
            if let Piece::Link { url, .. } = piece {
                assert!(link_allowed(url), "{url:?} must not be a link");
            }
        }
    }

    #[test]
    fn its_server_data_is_stored_as_the_server_serves_it() {
        let radicale = Radicale::shared();
        let (user, work) = on_radicale();
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let account = add_account(&core, radicale, &user);

        core.add_calendars(account, std::slice::from_ref(&work))
            .unwrap();

        let occurrences = core
            .occurrences(date("2026-10-05"), date("2026-10-06"))
            .unwrap();
        let stored = core
            .original_server_data(occurrences[0].event_id)
            .unwrap()
            .unwrap();
        let (etag, served) = radicale.get_event(&user, &work, "hostile");
        assert_eq!(stored.data, served);
        assert_eq!(stored.etag, Some(etag));
        assert!(stored.href.ends_with("/hostile.ics"), "{}", stored.href);
    }
}
