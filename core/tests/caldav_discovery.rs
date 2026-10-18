mod support;

use freecal_core::{AccountId, AvailableCalendar, CalDavLogin, Colour, CoreError, ErrorCode};
use support::Harness;
use support::radicale::{NewCalendar, Radicale, RoutingStub, User, multistatus, redirect};

fn login(url: &str, username: &str, password: &str) -> CalDavLogin {
    CalDavLogin {
        url: url.to_owned(),
        username: username.to_owned(),
        password: password.to_owned(),
    }
}

fn user_login(radicale: &Radicale, user: &User) -> CalDavLogin {
    login(&radicale.url(), &user.name, &user.password)
}

fn names(calendars: &[AvailableCalendar]) -> Vec<&str> {
    calendars
        .iter()
        .map(|calendar| calendar.name.as_str())
        .collect()
}

#[test]
fn adding_an_account_discovers_its_calendars_as_available_calendars() {
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
    let home = radicale.create_calendar(
        &user,
        NewCalendar {
            name: Some("home"),
            colour: Some("#ff8800"),
            ..NewCalendar::default()
        },
    );
    let harness = Harness::new();
    let core = harness.open();

    let added = core
        .add_caldav_account(&user_login(radicale, &user))
        .unwrap();

    assert_eq!(
        added.available,
        vec![
            AvailableCalendar {
                url: home,
                name: "home".to_owned(),
                colour: Some(Colour::parse("#ff8800").unwrap()),
                read_only: false,
            },
            AvailableCalendar {
                url: work,
                name: "Work".to_owned(),
                colour: Some(Colour::parse("#3366cc").unwrap()),
                read_only: false,
            },
        ]
    );
}

#[test]
fn a_calendar_the_server_doesnt_let_the_user_change_is_available_as_read_only() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let holidays = radicale.create_read_only_calendar(
        &user,
        NewCalendar {
            name: Some("Holidays"),
            ..NewCalendar::default()
        },
    );
    let work = radicale.create_calendar(
        &user,
        NewCalendar {
            name: Some("Work"),
            ..NewCalendar::default()
        },
    );
    let harness = Harness::new();
    let core = harness.open();

    let added = core
        .add_caldav_account(&user_login(radicale, &user))
        .unwrap();

    assert_eq!(
        added.available,
        vec![
            AvailableCalendar {
                url: holidays,
                name: "Holidays".to_owned(),
                colour: None,
                read_only: true,
            },
            AvailableCalendar {
                url: work,
                name: "Work".to_owned(),
                colour: None,
                read_only: false,
            },
        ]
    );
}

#[test]
fn freecal_stores_nothing_for_available_calendars() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let name = format!("Available {}", user.name);
    radicale.create_calendar(
        &user,
        NewCalendar {
            name: Some(&name),
            ..NewCalendar::default()
        },
    );
    let harness = Harness::new();
    let core = harness.open();
    let calendars = core.list_calendars().unwrap();

    let added = core
        .add_caldav_account(&user_login(radicale, &user))
        .unwrap();
    core.available_calendars(added.account.id).unwrap();

    assert_eq!(names(&added.available), vec![name.as_str()]);
    assert_eq!(core.list_calendars().unwrap(), calendars);
    drop(core);
    assert!(!harness.data_dir_contains(name.as_bytes()));
}

#[test]
fn an_account_without_calendars_has_no_available_calendars() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let harness = Harness::new();
    let core = harness.open();

    let added = core
        .add_caldav_account(&user_login(radicale, &user))
        .unwrap();

    assert_eq!(added.available, vec![]);
    assert_eq!(core.available_calendars(added.account.id).unwrap(), vec![]);
}

#[test]
fn listing_fetches_the_servers_current_calendars_each_time() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let old = radicale.create_calendar(
        &user,
        NewCalendar {
            name: Some("Old"),
            ..NewCalendar::default()
        },
    );
    let harness = Harness::new();
    let core = harness.open();
    let account = core
        .add_caldav_account(&user_login(radicale, &user))
        .unwrap()
        .account;

    radicale.create_calendar(
        &user,
        NewCalendar {
            name: Some("New"),
            ..NewCalendar::default()
        },
    );
    radicale.delete_calendar(&user, &old);

    assert_eq!(
        names(&core.available_calendars(account.id).unwrap()),
        vec!["New"]
    );
}

#[test]
fn listing_works_again_after_a_relaunch() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    radicale.create_calendar(
        &user,
        NewCalendar {
            name: Some("Work"),
            ..NewCalendar::default()
        },
    );
    let harness = Harness::new();
    let account = harness
        .open()
        .add_caldav_account(&user_login(radicale, &user))
        .unwrap()
        .account;

    let available = harness.open().available_calendars(account.id).unwrap();

    assert_eq!(names(&available), vec!["Work"]);
}

#[test]
fn listing_sends_no_signal() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let harness = Harness::new();
    let core = harness.open();
    let account = core
        .add_caldav_account(&user_login(radicale, &user))
        .unwrap()
        .account;
    harness.signals.clear();

    core.available_calendars(account.id).unwrap();

    assert!(harness.signals.received().is_empty());
}

#[test]
fn task_lists_arent_offered() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    radicale.create_calendar(
        &user,
        NewCalendar {
            name: Some("Tasks"),
            components: &["VTODO"],
            ..NewCalendar::default()
        },
    );
    radicale.create_calendar(
        &user,
        NewCalendar {
            name: Some("Both"),
            components: &["VTODO", "VEVENT"],
            ..NewCalendar::default()
        },
    );
    let harness = Harness::new();
    let core = harness.open();

    let added = core
        .add_caldav_account(&user_login(radicale, &user))
        .unwrap();

    assert_eq!(names(&added.available), vec!["Both"]);
}

#[test]
fn a_calendar_without_a_usable_colour_has_none() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let url = radicale.create_calendar(
        &user,
        NewCalendar {
            name: Some("Red"),
            colour: Some("red"),
            ..NewCalendar::default()
        },
    );
    let harness = Harness::new();
    let core = harness.open();

    let added = core
        .add_caldav_account(&user_login(radicale, &user))
        .unwrap();

    assert_eq!(
        added.available,
        vec![AvailableCalendar {
            url,
            name: "Red".to_owned(),
            colour: None,
            read_only: false,
        }]
    );
}

#[test]
fn the_hostile_calendar_name_reaches_the_frontend_unchanged_as_text() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/hostile-event.json")).unwrap();
    let hostile = fixture["calendarName"].as_str().unwrap();
    let radicale = Radicale::shared();
    let user = radicale.user();
    radicale.create_calendar(
        &user,
        NewCalendar {
            name: Some(hostile),
            ..NewCalendar::default()
        },
    );
    let harness = Harness::new();
    let core = harness.open();

    let added = core
        .add_caldav_account(&user_login(radicale, &user))
        .unwrap();

    assert_eq!(names(&added.available), vec![hostile]);
}

#[test]
fn the_local_account_offers_no_available_calendars() {
    let harness = Harness::new();
    let core = harness.open();
    let local = core.list_accounts().unwrap()[0].id;

    assert_eq!(core.available_calendars(local).unwrap(), vec![]);
}

#[test]
fn listing_an_unknown_account_fails() {
    let harness = Harness::new();
    let core = harness.open();

    let result = core.available_calendars(AccountId(99));

    assert!(matches!(result, Err(CoreError::AccountNotFound(_))));
}

#[test]
fn listing_without_the_password_in_the_keyring_fails() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let harness = Harness::new();
    let core = harness.open();
    let account = core
        .add_caldav_account(&user_login(radicale, &user))
        .unwrap()
        .account;
    harness.keyring.forget_all();

    let error = core.available_calendars(account.id).unwrap_err();

    assert!(matches!(error, CoreError::PasswordNotInKeyring));
    assert_eq!(error.code(), ErrorCode::PasswordNotInKeyring);
}

#[test]
fn listing_without_a_working_keyring_fails() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let harness = Harness::new();
    let core = harness.open();
    let account = core
        .add_caldav_account(&user_login(radicale, &user))
        .unwrap()
        .account;
    harness.keyring.break_it();

    let result = core.available_calendars(account.id);

    assert!(matches!(result, Err(CoreError::KeyringUnavailable(_))));
}

/// A CalDAV server whose discovery starts at `/.well-known/caldav`, as on
/// Nextcloud, whose root is a web page.
fn well_known_server() -> RoutingStub {
    let web_page = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 6\r\n\r\n<html>";
    RoutingStub::new(&[
        ("/", web_page.to_owned()),
        ("/.well-known/caldav", redirect("/dav/")),
        (
            "/dav/",
            multistatus(
                r#"<d:multistatus xmlns:d="DAV:"><d:response><d:href>/dav/</d:href><d:propstat>
                   <d:prop><d:current-user-principal><d:href>/dav/principals/alice/</d:href></d:current-user-principal></d:prop>
                   <d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response></d:multistatus>"#,
            ),
        ),
        (
            "/dav/principals/alice/",
            multistatus(
                r#"<d:multistatus xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav"><d:response>
                   <d:href>/dav/principals/alice/</d:href><d:propstat><d:prop>
                   <c:calendar-home-set><d:href>/dav/calendars/alice/</d:href></c:calendar-home-set>
                   </d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response></d:multistatus>"#,
            ),
        ),
        (
            "/dav/calendars/alice/",
            multistatus(
                r#"<d:multistatus xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
                   <d:response><d:href>/dav/calendars/alice/</d:href><d:propstat><d:prop>
                   <d:resourcetype><d:collection/></d:resourcetype>
                   </d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>
                   <d:response><d:href>/dav/calendars/alice/my%20work/</d:href><d:propstat><d:prop>
                   <d:resourcetype><d:collection/><c:calendar/></d:resourcetype>
                   </d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat>
                   <d:propstat><d:prop><d:displayname/></d:prop>
                   <d:status>HTTP/1.1 404 Not Found</d:status></d:propstat></d:response>
                   </d:multistatus>"#,
            ),
        ),
    ])
}

#[test]
fn discovery_falls_back_to_the_well_known_address_and_follows_its_redirect() {
    let server = well_known_server();
    let harness = Harness::new();
    let core = harness.open();

    let added = core
        .add_caldav_account(&login(&server.url, "alice", "secret"))
        .unwrap();

    assert_eq!(
        added.available,
        vec![AvailableCalendar {
            url: format!("{}dav/calendars/alice/my%20work/", server.url),
            name: "my work".to_owned(),
            colour: None,
            read_only: false,
        }]
    );
}

#[test]
fn discovery_asks_each_collection_with_the_right_depth() {
    let server = well_known_server();
    let harness = Harness::new();
    let core = harness.open();

    core.add_caldav_account(&login(&server.url, "alice", "secret"))
        .unwrap();

    assert_eq!(
        server.requests(),
        vec![
            "PROPFIND / HTTP/1.1",
            "PROPFIND /.well-known/caldav HTTP/1.1",
            "PROPFIND /dav/ HTTP/1.1",
            "PROPFIND /dav/principals/alice/ HTTP/1.1",
            "PROPFIND /dav/calendars/alice/ HTTP/1.1",
        ]
    );
}

#[test]
fn a_redirect_to_another_server_isnt_followed_and_stores_nothing() {
    let elsewhere = RoutingStub::new(&[]);
    let server = RoutingStub::new(&[("/", redirect(&format!("{}dav/", elsewhere.url)))]);
    let harness = Harness::new();
    let core = harness.open();

    let error = core
        .add_caldav_account(&login(&server.url, "alice", "secret"))
        .unwrap_err();

    assert_eq!(
        error.code(),
        ErrorCode::RedirectedElsewhere {
            url: format!("{}dav/", elsewhere.url)
        }
    );
    assert!(elsewhere.requests().is_empty(), "the password stays home");
    assert!(harness.keyring.secrets().is_empty());
    assert_eq!(core.list_accounts().unwrap().len(), 1);
}

#[test]
fn a_server_without_calendars_for_the_user_stores_nothing() {
    // A WebDAV server that knows the user but has no calendar home, such as
    // a CardDAV-only one.
    let server = RoutingStub::new(&[
        (
            "/",
            multistatus(
                r#"<multistatus xmlns="DAV:"><response><href>/</href><propstat>
                   <prop><current-user-principal><href>/p/</href></current-user-principal></prop>
                   <status>HTTP/1.1 200 OK</status></propstat></response></multistatus>"#,
            ),
        ),
        (
            "/p/",
            multistatus(
                r#"<multistatus xmlns="DAV:"><response><href>/p/</href><propstat>
                   <prop><calendar-home-set xmlns="urn:ietf:params:xml:ns:caldav"/></prop>
                   <status>HTTP/1.1 404 Not Found</status></propstat></response></multistatus>"#,
            ),
        ),
    ]);
    let harness = Harness::new();
    let core = harness.open();

    let result = core.add_caldav_account(&login(&server.url, "alice", "secret"));

    assert!(
        matches!(result, Err(CoreError::NotCalDavServer)),
        "{result:?}"
    );
    assert!(harness.keyring.secrets().is_empty());
    assert_eq!(core.list_accounts().unwrap().len(), 1);
}
