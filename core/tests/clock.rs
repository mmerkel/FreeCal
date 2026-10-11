mod support;

use chrono::NaiveDate;
use freecal_core::Signal;
use support::Harness;

fn date(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
}

#[test]
fn today_is_the_local_date_of_the_clock() {
    let harness = Harness::new();
    let core = harness.open();

    // 23:30 in UTC+2 is already the next day in UTC; today is the local date.
    harness.clock.set("2026-10-03T23:30:00+02:00");

    assert_eq!(core.today(), NaiveDate::from_ymd_opt(2026, 10, 3).unwrap());
}

#[test]
fn the_date_is_unchanged_during_the_day() {
    let harness = Harness::new();
    let core = harness.open();

    harness.clock.set("2026-10-03T23:59:59+02:00");
    core.check_date();

    assert!(harness.signals.received().is_empty());
}

#[test]
fn date_changed_is_sent_at_midnight_once() {
    let harness = Harness::new();
    let core = harness.open();

    harness.clock.set("2026-10-04T00:00:00+02:00");
    core.check_date();
    core.check_date();

    assert_eq!(
        harness.signals.received(),
        vec![Signal::DateChanged { date: date(4) }]
    );
}

#[test]
fn waking_up_days_later_sends_one_date_changed_with_the_new_date() {
    let harness = Harness::new();
    let core = harness.open();

    harness.clock.set("2026-10-07T08:15:00+02:00");
    core.check_date();

    assert_eq!(
        harness.signals.received(),
        vec![Signal::DateChanged { date: date(7) }]
    );
}

#[test]
fn the_local_date_is_what_counts_not_utc() {
    let harness = Harness::new();
    let core = harness.open();

    // Already 4 October in UTC, still 3 October in Berlin.
    harness.clock.set("2026-10-03T23:30:00+02:00");
    core.check_date();
    assert!(harness.signals.received().is_empty());

    // After travelling west it is still 3 October, though already the 4th
    // in Berlin.
    harness.clock.set("2026-10-04T01:00:00+02:00");
    harness.clock.set_time_zone("America/New_York");
    core.check_date();
    assert!(harness.signals.received().is_empty());
}
