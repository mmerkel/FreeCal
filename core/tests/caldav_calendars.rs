mod support;

use freecal_core::{AccountId, CalDavLogin, Calendar, Colour, Core, CoreError, ErrorCode, Signal};
use support::radicale::{NewCalendar, Radicale, User};
use support::{Harness, calendar, draft, timed};

fn user_login(radicale: &Radicale, user: &User) -> CalDavLogin {
    CalDavLogin {
        url: radicale.url(),
        username: user.name.clone(),
        password: user.password.clone(),
    }
}

fn server_calendar(radicale: &Radicale, user: &User, name: &str) -> String {
    radicale.create_calendar(
        user,
        NewCalendar {
            name: Some(name),
            ..NewCalendar::default()
        },
    )
}

fn add_account(core: &Core, radicale: &Radicale, user: &User) -> AccountId {
    core.add_caldav_account(&user_login(radicale, user))
        .unwrap()
        .account
        .id
}

fn available_urls(core: &Core, account: AccountId) -> Vec<String> {
    core.available_calendars(account)
        .unwrap()
        .into_iter()
        .map(|calendar| calendar.url)
        .collect()
}

#[test]
fn adding_an_available_calendar_makes_it_a_calendar_of_the_account() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let work = radicale.create_calendar(
        &user,
        NewCalendar {
            name: Some("Work"),
            colour: Some("#3366CCFF"),
            ..NewCalendar::default()
        },
    );
    let home = server_calendar(radicale, &user, "Home");
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let account = add_account(&core, radicale, &user);
    harness.signals.clear();

    let added = core
        .add_calendars(account, std::slice::from_ref(&work))
        .unwrap();

    let expected = vec![Calendar {
        id: added[0].id,
        account_id: account,
        name: "Work".to_owned(),
        colour: Colour::parse("#3366cc").unwrap(),
        shown: true,
        read_only: false,
        first_sync: false,
    }];
    assert_eq!(added, expected);
    assert_eq!(core.list_calendars().unwrap(), expected);
    assert_eq!(available_urls(&core, account), vec![home]);
    // As its first sync starts, and as it ends.
    assert_eq!(
        harness.signals.received(),
        vec![Signal::CalendarsChanged, Signal::CalendarsChanged]
    );
}

#[test]
fn several_available_calendars_are_added_at_once() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let work = server_calendar(radicale, &user, "Work");
    let home = server_calendar(radicale, &user, "Home");
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let account = add_account(&core, radicale, &user);

    let added = core.add_calendars(account, &[work, home]).unwrap();

    let names: Vec<_> = added
        .iter()
        .map(|calendar| calendar.name.as_str())
        .collect();
    assert_eq!(names, vec!["Work", "Home"]);
    assert_eq!(core.list_calendars().unwrap(), added);
    assert_eq!(available_urls(&core, account), Vec::<String>::new());
}

#[test]
fn the_hostile_calendar_name_is_stored_unchanged_as_text() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/hostile-event.json")).unwrap();
    let hostile = fixture["calendarName"].as_str().unwrap();
    let radicale = Radicale::shared();
    let user = radicale.user();
    let url = server_calendar(radicale, &user, hostile);
    let harness = Harness::new();
    let core = harness.open();
    let account = add_account(&core, radicale, &user);

    let added = core.add_calendars(account, &[url]).unwrap();
    drop(core);

    assert_eq!(added[0].name, hostile);
    let relaunched = harness.open().list_calendars().unwrap();
    assert!(relaunched.contains(&added[0]));
}

#[test]
fn a_calendar_without_a_colour_on_the_server_gets_the_default_colour() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let plain = server_calendar(radicale, &user, "Plain");
    let harness = Harness::new();
    let core = harness.open();
    let account = add_account(&core, radicale, &user);

    let added = core.add_calendars(account, &[plain]).unwrap();

    assert_eq!(added[0].colour, Colour::default());
}

#[test]
fn added_calendars_stay_after_a_relaunch() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let work = server_calendar(radicale, &user, "Work");
    let harness = Harness::new();
    let core = harness.open();
    let account = add_account(&core, radicale, &user);
    let added = core.add_calendars(account, &[work]).unwrap();
    drop(core);

    let core = harness.open();

    assert!(core.list_calendars().unwrap().contains(&added[0]));
    assert_eq!(available_urls(&core, account), Vec::<String>::new());
}

#[test]
fn a_calendar_the_server_doesnt_offer_cant_be_added() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let other = radicale.user();
    let theirs = server_calendar(radicale, &other, "Theirs");
    let harness = Harness::new();
    let core = harness.open();
    let account = add_account(&core, radicale, &user);
    let calendars = core.list_calendars().unwrap();
    harness.signals.clear();

    for url in [theirs, "https://elsewhere.example/cal/".to_owned()] {
        let refused = core.add_calendars(account, &[url]).unwrap_err();

        assert!(matches!(refused, CoreError::CalendarNotAvailable(_)));
        assert_eq!(refused.code(), ErrorCode::CalendarNotAvailable);
    }
    assert_eq!(core.list_calendars().unwrap(), calendars);
    assert!(harness.signals.received().is_empty());
}

#[test]
fn adding_stores_nothing_when_one_calendar_isnt_available() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let work = server_calendar(radicale, &user, "Work");
    let harness = Harness::new();
    let core = harness.open();
    let account = add_account(&core, radicale, &user);
    let calendars = core.list_calendars().unwrap();

    let refused = core
        .add_calendars(
            account,
            &[work, "https://elsewhere.example/cal/".to_owned()],
        )
        .unwrap_err();

    assert!(matches!(refused, CoreError::CalendarNotAvailable(_)));
    assert_eq!(core.list_calendars().unwrap(), calendars);
}

#[test]
fn a_calendar_already_added_cant_be_added_again() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let work = server_calendar(radicale, &user, "Work");
    let harness = Harness::new();
    let core = harness.open();
    let account = add_account(&core, radicale, &user);
    core.add_calendars(account, std::slice::from_ref(&work))
        .unwrap();
    let calendars = core.list_calendars().unwrap();

    let again = core
        .add_calendars(account, std::slice::from_ref(&work))
        .unwrap_err();
    let twice = core
        .add_calendars(account, &[work.clone(), work])
        .unwrap_err();

    assert!(matches!(again, CoreError::CalendarNotAvailable(_)));
    assert!(matches!(twice, CoreError::CalendarNotAvailable(_)));
    assert_eq!(core.list_calendars().unwrap(), calendars);
}

#[test]
fn the_local_account_has_no_calendars_to_add() {
    let harness = Harness::new();
    let core = harness.open();
    let local = core.list_accounts().unwrap()[0].id;

    let refused = core
        .add_calendars(local, &["https://elsewhere.example/cal/".to_owned()])
        .unwrap_err();

    assert!(matches!(refused, CoreError::CalendarNotAvailable(_)));
}

#[test]
fn adding_to_an_unknown_account_is_refused() {
    let harness = Harness::new();
    let core = harness.open();

    let refused = core
        .add_calendars(
            AccountId(999),
            &["https://elsewhere.example/cal/".to_owned()],
        )
        .unwrap_err();

    assert!(matches!(
        refused,
        CoreError::AccountNotFound(AccountId(999))
    ));
}

#[test]
fn removing_a_calendar_deletes_only_its_local_copy_and_makes_it_available_again() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let work = server_calendar(radicale, &user, "Work");
    let harness = Harness::new();
    let core = harness.open();
    let account = add_account(&core, radicale, &user);
    let calendars = core.list_calendars().unwrap();
    let added = core
        .add_calendars(account, std::slice::from_ref(&work))
        .unwrap()[0]
        .clone();
    core.create_event(draft(
        added.id,
        "Stand-up",
        timed("2026-10-05T09:00:00", "2026-10-05T09:15:00"),
    ))
    .unwrap();
    harness.signals.clear();

    core.remove_calendar(added.id).unwrap();

    assert_eq!(core.list_calendars().unwrap(), calendars);
    assert_eq!(
        core.occurrences(support::date("2026-10-05"), support::date("2026-10-06"))
            .unwrap(),
        vec![]
    );
    assert_eq!(available_urls(&core, account), vec![work.clone()]);
    assert_eq!(harness.signals.received(), vec![Signal::CalendarsChanged]);
    // The server still has it, so it can be added again.
    core.add_calendars(account, &[work]).unwrap();
}

#[test]
fn a_local_calendar_cant_be_removed_only_deleted() {
    let harness = Harness::new();
    let core = harness.open();
    let local = calendar(&core, "Mine");
    let calendars = core.list_calendars().unwrap();

    let refused = core.remove_calendar(local.id).unwrap_err();

    assert!(matches!(refused, CoreError::NotServerCalendar(id) if id == local.id));
    assert_eq!(refused.code(), ErrorCode::NotServerCalendar);
    assert_eq!(core.list_calendars().unwrap(), calendars);
}

#[test]
fn a_server_calendar_cant_be_deleted_only_removed() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let work = server_calendar(radicale, &user, "Work");
    let harness = Harness::new();
    let core = harness.open();
    let account = add_account(&core, radicale, &user);
    let added = core.add_calendars(account, &[work]).unwrap()[0].clone();
    let calendars = core.list_calendars().unwrap();
    harness.signals.clear();

    let refused = core.delete_calendar(added.id).unwrap_err();

    assert!(matches!(refused, CoreError::NotLocalCalendar(id) if id == added.id));
    assert_eq!(refused.code(), ErrorCode::NotLocalCalendar);
    assert_eq!(core.list_calendars().unwrap(), calendars);
    assert!(harness.signals.received().is_empty());
}

#[test]
fn removing_or_deleting_an_unknown_calendar_is_refused() {
    let harness = Harness::new();
    let core = harness.open();
    let unknown = freecal_core::CalendarId(999);

    assert!(matches!(
        core.remove_calendar(unknown),
        Err(CoreError::CalendarNotFound(id)) if id == unknown
    ));
    assert!(matches!(
        core.delete_calendar(unknown),
        Err(CoreError::CalendarNotFound(id)) if id == unknown
    ));
}

#[test]
fn removing_an_account_removes_its_calendars_their_events_and_their_history() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let work = server_calendar(radicale, &user, "Work");
    let harness = Harness::new();
    let core = harness.open();
    let calendars = core.list_calendars().unwrap();
    let dentist = core
        .create_event(draft(
            calendars[0].id,
            "Dentist",
            timed("2026-10-05T11:00:00", "2026-10-05T12:00:00"),
        ))
        .unwrap();
    let account = add_account(&core, radicale, &user);
    let added = core.add_calendars(account, &[work]).unwrap()[0].clone();
    let created = core
        .create_event(draft(
            added.id,
            "Stand-up",
            timed("2026-10-05T09:00:00", "2026-10-05T09:15:00"),
        ))
        .unwrap();
    harness.signals.clear();

    core.remove_account(account).unwrap();

    assert_eq!(core.list_calendars().unwrap(), calendars);
    let titles: Vec<_> = core
        .occurrences(support::date("2026-10-05"), support::date("2026-10-06"))
        .unwrap()
        .into_iter()
        .map(|occurrence| occurrence.title)
        .collect();
    assert_eq!(titles, vec!["Dentist"]);
    assert!(harness.keyring.secrets().is_empty());
    assert_eq!(harness.signals.received(), vec![Signal::CalendarsChanged]);
    let unit = created.unit.unwrap();
    let refused = core.undo_unit(unit);
    assert!(matches!(refused, Err(CoreError::UnitNotFound(id)) if id == unit));
    // The Local Calendar's history stays.
    core.undo_unit(dentist.unit.unwrap()).unwrap();
}

#[test]
fn several_accounts_side_by_side_each_keep_their_own_password_and_calendars() {
    let radicale = Radicale::shared();
    let (alice, bob) = (radicale.user(), radicale.user());
    let alices_work = server_calendar(radicale, &alice, "Work");
    let bobs_work = server_calendar(radicale, &bob, "Work");
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let alices = add_account(&core, radicale, &alice);
    let bobs = add_account(&core, radicale, &bob);

    assert_eq!(core.list_accounts().unwrap().len(), 3);
    let mut secrets = harness.keyring.secrets();
    secrets.sort();
    assert_eq!(secrets, vec![alice.password.clone(), bob.password.clone()]);
    assert_eq!(available_urls(&core, alices), vec![alices_work.clone()]);
    assert_eq!(available_urls(&core, bobs), vec![bobs_work.clone()]);
    core.add_calendars(alices, &[alices_work]).unwrap();
    let bobs_calendar = core.add_calendars(bobs, &[bobs_work]).unwrap()[0].clone();
    let stand_up = core
        .create_event(draft(
            bobs_calendar.id,
            "Stand-up",
            timed("2026-10-05T09:00:00", "2026-10-05T09:15:00"),
        ))
        .unwrap();

    core.remove_account(alices).unwrap();

    let accounts: Vec<_> = core
        .list_accounts()
        .unwrap()
        .into_iter()
        .map(|account| account.id)
        .collect();
    assert!(accounts.contains(&bobs) && !accounts.contains(&alices));
    assert_eq!(harness.keyring.secrets(), vec![bob.password.clone()]);
    assert_eq!(core.list_calendars().unwrap(), vec![bobs_calendar]);
    assert_eq!(core.event(stand_up.value.id).unwrap(), stand_up.value);
    core.undo_unit(stand_up.unit.unwrap()).unwrap();
    // Bob's password still logs in.
    server_calendar(radicale, &bob, "Home");
    assert_eq!(available_urls(&core, bobs).len(), 1);
}
