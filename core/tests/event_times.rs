//! How the Local Store keeps Event times: wall time plus Event Time Zone,
//! with UTC instants derived for range queries (ADR 0007).

mod support;

use chrono::DateTime;
use freecal_core::{CalendarId, Colour, Core, When};
use support::{Harness, all_day, date, draft, timed};

/// The title and time of each Occurrence on the dates `from` up to `to`
/// (exclusive).
fn shown(core: &Core, from: &str, to: &str) -> Vec<(String, When)> {
    core.occurrences(date(from), date(to))
        .unwrap()
        .into_iter()
        .map(|occurrence| (occurrence.title, occurrence.when))
        .collect()
}

/// The schema of ticket 03, where a timed Event's UTC instant was the truth.
const TICKET_03_SCHEMA: &str = "
    CREATE TABLE account (id INTEGER PRIMARY KEY, provider TEXT NOT NULL);
    INSERT INTO account (provider) VALUES ('local');
    CREATE TABLE calendar (
        id INTEGER PRIMARY KEY,
        account_id INTEGER NOT NULL REFERENCES account (id) ON DELETE CASCADE,
        name TEXT NOT NULL,
        colour TEXT NOT NULL,
        shown INTEGER NOT NULL DEFAULT 1,
        read_only INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX calendar_account ON calendar (account_id);
    CREATE TABLE event (
        id INTEGER PRIMARY KEY,
        calendar_id INTEGER NOT NULL REFERENCES calendar (id) ON DELETE CASCADE,
        uid TEXT NOT NULL,
        title TEXT NOT NULL,
        all_day INTEGER NOT NULL,
        start_at TEXT NOT NULL,
        end_at TEXT NOT NULL,
        time_zone TEXT,
        location TEXT NOT NULL,
        description TEXT NOT NULL,
        description_html INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX event_calendar_start ON event (calendar_id, start_at);
    CREATE TABLE app_state (
        id INTEGER PRIMARY KEY CHECK (id = 1),
        view TEXT NOT NULL DEFAULT 'month',
        appearance TEXT NOT NULL DEFAULT 'system'
    );
    INSERT INTO app_state (id) VALUES (1);
    PRAGMA user_version = 5;
";

#[test]
fn a_local_store_from_ticket_03_opens_with_the_same_occurrences() {
    let harness = Harness::new();
    harness.arrange_local_store(TICKET_03_SCHEMA);
    harness.arrange_local_store(
        "
        INSERT INTO calendar (account_id, name, colour) VALUES (1, 'Home', '#3366cc');
        INSERT INTO event (calendar_id, uid, title, all_day, start_at, end_at, time_zone,
                           location, description)
        VALUES
            (1, 'a', 'Berlin call', 0, '2026-10-05T07:00:00Z', '2026-10-05T08:30:00Z',
             'Europe/Berlin', '', ''),
            (1, 'b', 'New York call', 0, '2026-10-05T13:00:00Z', '2026-10-05T14:00:00Z',
             'America/New_York', '', ''),
            (1, 'c', 'Holiday', 1, '2026-10-05', '2026-10-06', NULL, '', ''),
            (1, 'd', 'Late', 0, '2026-10-05T21:30:00Z', '2026-10-05T21:45:00Z',
             'Europe/Berlin', 'Room 1', 'Notes');
        ",
    );

    let core = harness.open();

    let expected = [
        ("Holiday", all_day("2026-10-05", "2026-10-06")),
        (
            "Berlin call",
            timed("2026-10-05T09:00:00", "2026-10-05T10:30:00"),
        ),
        (
            "New York call",
            timed("2026-10-05T15:00:00", "2026-10-05T16:00:00"),
        ),
        ("Late", timed("2026-10-05T23:30:00", "2026-10-05T23:45:00")),
    ]
    .map(|(title, when)| (title.to_owned(), when));
    assert_eq!(shown(&core, "2026-10-05", "2026-10-06"), expected);
    assert!(shown(&core, "2026-10-06", "2026-10-07").is_empty());
    let late = core
        .occurrences(date("2026-10-05"), date("2026-10-06"))
        .unwrap()[3]
        .event_id;
    let late = core.event(late).unwrap();
    assert_eq!(
        (late.location.as_str(), late.description.text.as_str()),
        ("Room 1", "Notes")
    );
}

fn home(core: &Core) -> CalendarId {
    let local = core.list_accounts().unwrap()[0].id;
    core.create_calendar(local, "Home", Colour::parse("#3366cc").unwrap())
        .unwrap()
        .id
}

/// Unix seconds of an RFC 3339 time.
fn seconds(rfc3339: &str) -> i64 {
    DateTime::parse_from_rfc3339(rfc3339).unwrap().timestamp()
}

#[test]
fn utc_instants_from_other_time_zone_rules_are_recomputed_on_opening() {
    let harness = Harness::new();
    let core = harness.open();
    let home = home(&core);
    let call = core
        .create_event(draft(
            home,
            "Call",
            timed("2026-10-05T09:00:00", "2026-10-05T10:00:00"),
        ))
        .unwrap();
    drop(core);
    harness.arrange_local_store(&format!(
        "UPDATE app_state SET time_zone_rules = '2020a';
         UPDATE event SET start_utc = 0, end_utc = 60 WHERE id = {};",
        call.id.0
    ));

    let core = harness.open();

    assert_eq!(
        shown(&core, "2026-10-05", "2026-10-06"),
        [("Call".to_owned(), call.when)]
    );
    assert!(shown(&core, "1970-01-01", "1970-01-02").is_empty());
}

#[test]
fn a_time_the_clocks_skip_happens_shifted_by_the_gap() {
    let harness = Harness::new();
    let core = harness.open();
    let home = home(&core);

    // On 29 March 2026, Berlin's clocks jump from 02:00 to 03:00.
    let gap = core
        .create_event(draft(
            home,
            "Gap",
            timed("2026-03-29T02:30:00", "2026-03-29T04:00:00"),
        ))
        .unwrap();
    harness.clock.set_time_zone("UTC");

    assert_eq!(
        core.event(gap.id).unwrap().when,
        timed("2026-03-29T01:30:00", "2026-03-29T02:00:00")
    );
}

#[test]
fn a_time_the_clocks_repeat_is_the_first_one_and_round_trips() {
    let harness = Harness::new();
    let core = harness.open();
    let home = home(&core);
    // On 25 October 2026, Berlin's clocks go back from 03:00 to 02:00.
    let when = timed("2026-10-25T02:30:00", "2026-10-25T03:30:00");

    let repeated = core.create_event(draft(home, "Twice", when)).unwrap();
    core.edit_event(repeated.id, draft(home, "Twice, renamed", when))
        .unwrap();

    assert_eq!(core.event(repeated.id).unwrap().when, when);
    harness.clock.set_time_zone("UTC");
    assert_eq!(
        core.event(repeated.id).unwrap().when,
        timed("2026-10-25T00:30:00", "2026-10-25T02:30:00")
    );
}

#[test]
fn ranges_across_a_change_of_the_clocks_find_the_right_events() {
    let harness = Harness::new();
    let core = harness.open();
    let home = home(&core);
    for (title, when) in [
        (
            "Saturday",
            timed("2026-10-24T23:30:00", "2026-10-25T00:00:00"),
        ),
        (
            "Before",
            timed("2026-10-25T01:00:00", "2026-10-25T01:30:00"),
        ),
        ("After", timed("2026-10-25T23:30:00", "2026-10-26T00:00:00")),
        (
            "Monday",
            timed("2026-10-26T00:00:00", "2026-10-26T00:30:00"),
        ),
    ] {
        core.create_event(draft(home, title, when)).unwrap();
    }

    let titles: Vec<String> = shown(&core, "2026-10-25", "2026-10-26")
        .into_iter()
        .map(|(title, _)| title)
        .collect();
    assert_eq!(titles, ["Before", "After"]);
}

#[test]
fn start_and_end_in_different_time_zones_are_shown_in_the_display_time_zone() {
    let harness = Harness::new();
    let core = harness.open();
    let home = home(&core);
    // Nothing creates such an Event before tickets 06 and 10: 23:00 in
    // Berlin until 00:30 in London, which is 01:30 in Berlin.
    harness.arrange_local_store(&format!(
        "INSERT INTO event (calendar_id, uid, title, all_day, start_at, end_at, start_zone,
             end_zone, start_utc, end_utc, location, description)
         VALUES ({}, 'x', 'Flight', 0, '2026-10-05T23:00:00', '2026-10-06T00:30:00',
             'Europe/Berlin', 'Europe/London', {}, {}, '', '');",
        home.0,
        seconds("2026-10-05T21:00:00Z"),
        seconds("2026-10-05T23:30:00Z"),
    ));
    let flight = timed("2026-10-05T23:00:00", "2026-10-06T01:30:00");

    assert_eq!(
        shown(&core, "2026-10-05", "2026-10-06"),
        [("Flight".to_owned(), flight)]
    );
    assert_eq!(
        shown(&core, "2026-10-06", "2026-10-07"),
        [("Flight".to_owned(), flight)]
    );
    assert!(shown(&core, "2026-10-07", "2026-10-08").is_empty());

    let id = core
        .occurrences(date("2026-10-05"), date("2026-10-06"))
        .unwrap()[0]
        .event_id;
    let later = timed("2026-10-05T23:00:00", "2026-10-06T02:30:00");
    core.edit_event(id, draft(home, "Flight", later)).unwrap();
    harness.clock.set_time_zone("Europe/London");

    assert_eq!(
        core.event(id).unwrap().when,
        timed("2026-10-05T22:00:00", "2026-10-06T01:30:00")
    );
}

#[test]
fn occurrences_in_the_repeated_hour_are_ordered_by_when_they_happen() {
    let harness = Harness::new();
    let core = harness.open();
    let home = home(&core);
    // On 25 October 2026, Berlin's clocks go back from 03:00 to 02:00. 02:45
    // entered in Berlin is the first 02:45 (00:45 UTC); 01:15 UTC is the
    // second 02:15.
    core.create_event(draft(
        home,
        "First",
        timed("2026-10-25T02:45:00", "2026-10-25T02:50:00"),
    ))
    .unwrap();
    harness.clock.set_time_zone("UTC");
    core.create_event(draft(
        home,
        "Second",
        timed("2026-10-25T01:15:00", "2026-10-25T01:20:00"),
    ))
    .unwrap();
    harness.clock.set_time_zone("Europe/Berlin");

    assert_eq!(
        shown(&core, "2026-10-25", "2026-10-26"),
        [
            (
                "First".to_owned(),
                timed("2026-10-25T02:45:00", "2026-10-25T02:50:00")
            ),
            (
                "Second".to_owned(),
                timed("2026-10-25T02:15:00", "2026-10-25T02:20:00")
            ),
        ]
    );
}

#[test]
fn all_day_occurrences_come_before_timed_ones_just_after_midnight() {
    let harness = Harness::new();
    let core = harness.open();
    let home = home(&core);
    // 00:30 in Berlin is 22:30 UTC the day before.
    core.create_event(draft(
        home,
        "Early",
        timed("2026-10-05T00:30:00", "2026-10-05T01:00:00"),
    ))
    .unwrap();
    core.create_event(draft(home, "Holiday", all_day("2026-10-05", "2026-10-06")))
        .unwrap();

    let titles: Vec<String> = shown(&core, "2026-10-05", "2026-10-06")
        .into_iter()
        .map(|(title, _)| title)
        .collect();
    assert_eq!(titles, ["Holiday", "Early"]);
}

#[test]
fn a_local_store_from_ticket_03_with_hostile_times_still_opens() {
    let harness = Harness::new();
    harness.arrange_local_store(TICKET_03_SCHEMA);
    harness.arrange_local_store(
        "
        INSERT INTO calendar (account_id, name, colour) VALUES (1, 'Home', '#3366cc');
        INSERT INTO event (calendar_id, uid, title, all_day, start_at, end_at, time_zone,
                           location, description)
        VALUES
            (1, 'a', 'Unknown zone', 0, '2026-10-05T07:00:00Z', '2026-10-05T08:00:00Z',
             'Mars/Olympus_Mons', '', ''),
            (1, 'b', 'No zone', 0, '2026-10-05T10:00:00Z', '2026-10-05T11:00:00Z',
             NULL, '', ''),
            (1, 'c', 'Blob', 0, X'00FF', '2026-10-05T11:00:00Z', 'Europe/Berlin', '', ''),
            (1, 'd', 'Garbage', 0, 'soon', 'later', 'Europe/Berlin', '', '');
        ",
    );

    let core = harness.open();

    // A zone FreeCal doesn't know becomes UTC, keeping the instant.
    assert_eq!(
        shown(&core, "2026-10-05", "2026-10-06"),
        [
            (
                "Unknown zone".to_owned(),
                timed("2026-10-05T09:00:00", "2026-10-05T10:00:00")
            ),
            (
                "No zone".to_owned(),
                timed("2026-10-05T12:00:00", "2026-10-05T13:00:00")
            ),
        ]
    );
}
