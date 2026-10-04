mod support;

use freecal_core::{AccountId, CalendarId, Colour, Core, CoreError, Signal};
use support::Harness;

fn local_account(core: &Core) -> AccountId {
    core.list_accounts().unwrap()[0].id
}

fn colour(hex: &str) -> Colour {
    Colour::parse(hex).unwrap()
}

#[test]
fn a_new_local_store_has_no_calendars() {
    let harness = Harness::new();
    let core = harness.open();

    assert!(core.list_calendars().unwrap().is_empty());
}

#[test]
fn a_created_calendar_is_listed_under_its_account_and_shown() {
    let harness = Harness::new();
    let core = harness.open();
    let local = local_account(&core);

    let created = core
        .create_calendar(local, "Home", colour("#3366cc"))
        .unwrap();

    assert_eq!(created.account_id, local);
    assert_eq!(created.name, "Home");
    assert_eq!(created.colour, colour("#3366cc"));
    assert!(created.shown);
    assert_eq!(core.list_calendars().unwrap(), vec![created]);
    assert_eq!(harness.signals.received(), vec![Signal::CalendarsChanged]);
}

#[test]
fn calendars_are_listed_in_the_order_they_were_created() {
    let harness = Harness::new();
    let core = harness.open();
    let local = local_account(&core);

    core.create_calendar(local, "Work", colour("#aa0000"))
        .unwrap();
    core.create_calendar(local, "Home", colour("#00aa00"))
        .unwrap();

    let names: Vec<_> = core
        .list_calendars()
        .unwrap()
        .into_iter()
        .map(|calendar| calendar.name)
        .collect();
    assert_eq!(names, ["Work", "Home"]);
}

#[test]
fn calendars_survive_a_relaunch() {
    let harness = Harness::new();
    let first_launch = harness.open();
    let local = local_account(&first_launch);
    let created = first_launch
        .create_calendar(local, "Home", colour("#3366cc"))
        .unwrap();
    first_launch.set_calendar_shown(created.id, false).unwrap();
    drop(first_launch);

    let relaunch = harness.open();

    let listed = relaunch.list_calendars().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "Home");
    assert!(!listed[0].shown);
}

#[test]
fn a_calendar_name_is_trimmed_and_must_not_be_empty() {
    let harness = Harness::new();
    let core = harness.open();
    let local = local_account(&core);

    let created = core
        .create_calendar(local, "  Home \n", colour("#3366cc"))
        .unwrap();
    assert_eq!(created.name, "Home");

    for blank in ["", "   ", "\t\n"] {
        assert!(matches!(
            core.create_calendar(local, blank, colour("#3366cc")),
            Err(CoreError::EmptyCalendarName)
        ));
        assert!(matches!(
            core.rename_calendar(created.id, blank),
            Err(CoreError::EmptyCalendarName)
        ));
    }
    assert_eq!(core.list_calendars().unwrap(), vec![created]);
}

#[test]
fn creating_a_calendar_in_an_unknown_account_is_refused() {
    let harness = Harness::new();
    let core = harness.open();

    let result = core.create_calendar(AccountId(999), "Home", colour("#3366cc"));

    assert!(matches!(
        result,
        Err(CoreError::AccountNotFound(AccountId(999)))
    ));
    assert!(harness.signals.received().is_empty());
}

#[test]
fn renaming_a_calendar_changes_only_its_name() {
    let harness = Harness::new();
    let core = harness.open();
    let local = local_account(&core);
    let created = core
        .create_calendar(local, "Home", colour("#3366cc"))
        .unwrap();

    core.rename_calendar(created.id, "Family").unwrap();

    let listed = core.list_calendars().unwrap();
    assert_eq!(listed[0].name, "Family");
    assert_eq!(listed[0].colour, created.colour);
    assert_eq!(
        harness.signals.received(),
        vec![Signal::CalendarsChanged, Signal::CalendarsChanged]
    );
}

#[test]
fn recolouring_a_calendar_changes_only_its_colour() {
    let harness = Harness::new();
    let core = harness.open();
    let local = local_account(&core);
    let created = core
        .create_calendar(local, "Home", colour("#3366cc"))
        .unwrap();

    core.recolour_calendar(created.id, colour("#ff8800"))
        .unwrap();

    let listed = core.list_calendars().unwrap();
    assert_eq!(listed[0].colour, colour("#ff8800"));
    assert_eq!(listed[0].name, "Home");
    assert_eq!(harness.signals.received().len(), 2);
}

#[test]
fn deleting_a_calendar_removes_it() {
    let harness = Harness::new();
    let core = harness.open();
    let local = local_account(&core);
    let doomed = core
        .create_calendar(local, "Old", colour("#3366cc"))
        .unwrap();
    let kept = core
        .create_calendar(local, "Kept", colour("#3366cc"))
        .unwrap();

    core.delete_calendar(doomed.id).unwrap();

    assert_eq!(core.list_calendars().unwrap(), vec![kept]);
    assert_eq!(harness.signals.received().len(), 3);
}

#[test]
fn hiding_and_showing_a_calendar_is_recorded_and_signalled() {
    let harness = Harness::new();
    let core = harness.open();
    let local = local_account(&core);
    let created = core
        .create_calendar(local, "Home", colour("#3366cc"))
        .unwrap();

    core.set_calendar_shown(created.id, false).unwrap();
    assert!(!core.list_calendars().unwrap()[0].shown);

    core.set_calendar_shown(created.id, true).unwrap();
    assert!(core.list_calendars().unwrap()[0].shown);

    assert_eq!(harness.signals.received().len(), 3);
}

#[test]
fn showing_only_one_calendar_hides_all_others_with_one_signal() {
    let harness = Harness::new();
    let core = harness.open();
    let local = local_account(&core);
    let home = core
        .create_calendar(local, "Home", colour("#3366cc"))
        .unwrap();
    let work = core
        .create_calendar(local, "Work", colour("#dc3912"))
        .unwrap();
    let birthdays = core
        .create_calendar(local, "Birthdays", colour("#ff9900"))
        .unwrap();
    core.set_calendar_shown(work.id, false).unwrap();
    let before = harness.signals.received().len();

    core.show_only_calendar(work.id).unwrap();

    let shown: Vec<_> = core
        .list_calendars()
        .unwrap()
        .into_iter()
        .map(|calendar| (calendar.id, calendar.shown))
        .collect();
    assert_eq!(
        shown,
        vec![(home.id, false), (work.id, true), (birthdays.id, false)]
    );
    assert_eq!(
        harness.signals.received()[before..],
        [Signal::CalendarsChanged]
    );
}

#[test]
fn changing_an_unknown_calendar_is_refused_without_a_signal() {
    let harness = Harness::new();
    let core = harness.open();
    let missing = CalendarId(999);

    let results = [
        core.rename_calendar(missing, "Home"),
        core.recolour_calendar(missing, colour("#3366cc")),
        core.delete_calendar(missing),
        core.set_calendar_shown(missing, false),
        core.show_only_calendar(missing),
    ];

    for result in results {
        assert!(matches!(result, Err(CoreError::CalendarNotFound(id)) if id == missing));
    }
    assert!(harness.signals.received().is_empty());
}

#[test]
fn a_colour_is_six_digit_hex_and_kept_in_lower_case() {
    assert_eq!(Colour::parse("#3366CC").unwrap().as_str(), "#3366cc");

    for invalid in [
        "",
        "red",
        "#36c",
        "3366cc",
        "#3366cc00",
        "#3366cg",
        "#fff;background:url(x)",
        "#３３６６ｃｃ",
    ] {
        assert!(
            matches!(Colour::parse(invalid), Err(CoreError::InvalidColour(text)) if text == invalid),
            "{invalid:?} should be refused"
        );
    }
}

#[test]
fn a_hostile_calendar_name_is_stored_and_returned_as_plain_text() {
    let harness = Harness::new();
    let core = harness.open();
    let local = local_account(&core);
    let hostile = "<img src=x onerror=alert(1)>'); DROP TABLE calendar; --";

    core.create_calendar(local, hostile, colour("#3366cc"))
        .unwrap();

    assert_eq!(core.list_calendars().unwrap()[0].name, hostile);
}

#[test]
fn an_invalid_stored_colour_is_read_as_the_default_colour() {
    let harness = Harness::new();
    let core = harness.open();
    let local = local_account(&core);
    let created = core
        .create_calendar(local, "Home", colour("#3366cc"))
        .unwrap();
    harness.arrange_local_store(&format!(
        "UPDATE calendar SET colour = 'red;background:url(x)' WHERE id = {}",
        created.id.0
    ));

    assert_eq!(core.list_calendars().unwrap()[0].colour, Colour::default());
}
