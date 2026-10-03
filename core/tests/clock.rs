mod support;

use chrono::NaiveDate;
use support::Harness;

#[test]
fn today_is_the_local_date_of_the_clock() {
    let harness = Harness::new();
    let core = harness.open();

    // 23:30 in UTC+2 is already the next day in UTC; today is the local date.
    harness.clock.set("2026-10-03T23:30:00+02:00");

    assert_eq!(core.today(), NaiveDate::from_ymd_opt(2026, 10, 3).unwrap());
}
