mod support;

use freecal_core::{CalDavLogin, CoreError, ErrorCode, Provider, ServerLogin, Signal};
use support::Harness;
use support::radicale::{Radicale, closed_port_url, http_stub};

fn login(url: &str, username: &str, password: &str) -> CalDavLogin {
    CalDavLogin {
        url: url.to_owned(),
        username: username.to_owned(),
        password: password.to_owned(),
    }
}

#[test]
fn adding_an_account_checks_its_credentials_and_keeps_its_password_in_the_keyring() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let harness = Harness::new();
    let core = harness.open();

    let account = core
        .add_caldav_account(&login(&radicale.url(), &user.name, &user.password))
        .unwrap()
        .account;

    assert_eq!(account.provider, Provider::CalDav);
    assert_eq!(
        account.server,
        Some(ServerLogin {
            url: radicale.url(),
            username: user.name.clone(),
        })
    );
    assert_eq!(core.list_accounts().unwrap()[1], account);
    assert_eq!(harness.keyring.secrets(), vec![user.password.clone()]);
    assert_eq!(harness.signals.received(), vec![Signal::CalendarsChanged]);
}

#[test]
fn an_added_account_is_still_there_after_a_relaunch() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let harness = Harness::new();
    let account = harness
        .open()
        .add_caldav_account(&login(&radicale.url(), &user.name, &user.password))
        .unwrap()
        .account;

    let accounts = harness.open().list_accounts().unwrap();

    assert_eq!(accounts[1], account);
}

#[test]
fn the_password_is_never_written_to_the_local_store() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let harness = Harness::new();
    harness
        .open()
        .add_caldav_account(&login(&radicale.url(), &user.name, &user.password))
        .unwrap();

    assert!(!harness.data_dir_contains(user.password.as_bytes()));
}

#[test]
fn a_wrong_password_stores_nothing() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let harness = Harness::new();
    let core = harness.open();

    let result = core.add_caldav_account(&login(&radicale.url(), &user.name, "wrong"));

    assert!(matches!(result, Err(CoreError::WrongCredentials)));
    assert_nothing_stored(&harness, &core);
}

#[test]
fn an_unknown_user_stores_nothing() {
    let radicale = Radicale::shared();
    let harness = Harness::new();
    let core = harness.open();

    let result = core.add_caldav_account(&login(&radicale.url(), "nobody", "secret"));

    assert!(matches!(result, Err(CoreError::WrongCredentials)));
    assert_nothing_stored(&harness, &core);
}

#[test]
fn an_unreachable_server_stores_nothing() {
    let harness = Harness::new();
    let core = harness.open();

    let result = core.add_caldav_account(&login(&closed_port_url(), "alice", "secret"));

    assert!(matches!(result, Err(CoreError::ServerUnreachable(_))));
    assert_nothing_stored(&harness, &core);
}

#[test]
fn a_failed_secure_connection_stores_nothing() {
    // The stub answers a TLS handshake with plain HTTP.
    let url = http_stub("HTTP/1.1 200 OK\r\n\r\n").replace("http://", "https://");
    let harness = Harness::new();
    let core = harness.open();

    let result = core.add_caldav_account(&login(&url, "alice", "secret"));

    assert!(
        matches!(result, Err(CoreError::SecureConnectionFailed(_))),
        "{result:?}"
    );
    assert_nothing_stored(&harness, &core);
}

#[test]
fn without_a_working_keyring_adding_an_account_fails_and_stores_nothing() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let harness = Harness::new();
    harness.keyring.break_it();
    let core = harness.open();

    let result = core.add_caldav_account(&login(&radicale.url(), &user.name, &user.password));

    assert!(matches!(result, Err(CoreError::KeyringUnavailable(_))));
    assert_nothing_stored(&harness, &core);
}

#[test]
fn a_server_refusing_the_request_stores_nothing() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let other = radicale.user();
    let harness = Harness::new();
    let core = harness.open();

    // Radicale lets each user see only their own collections.
    let url = format!("{}{}/", radicale.url(), other.name);
    let result = core.add_caldav_account(&login(&url, &user.name, &user.password));

    assert!(matches!(
        result,
        Err(CoreError::ServerRefused { status: 403, .. })
    ));
    assert_nothing_stored(&harness, &core);
}

#[test]
fn a_server_that_isnt_a_caldav_server_stores_nothing() {
    let url = http_stub("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html>Welcome</html>");
    let harness = Harness::new();
    let core = harness.open();

    let result = core.add_caldav_account(&login(&url, "alice", "secret"));

    assert!(matches!(result, Err(CoreError::NotCalDavServer)));
    assert_nothing_stored(&harness, &core);
}

#[test]
fn only_http_and_https_urls_are_servers() {
    let harness = Harness::new();
    let core = harness.open();

    for url in [
        "ftp://example.org/dav/",
        "example.org",
        "",
        "file:///etc/passwd",
    ] {
        let result = core.add_caldav_account(&login(url, "alice", "secret"));

        assert!(
            matches!(result, Err(CoreError::InvalidServerUrl)),
            "{url:?} gave {result:?}"
        );
    }
    assert_nothing_stored(&harness, &core);
}

#[test]
fn a_servers_error_text_reaches_the_frontend_only_as_plain_text() {
    let hostile = "<img src=x onerror=alert(1)>\r\nSet-Cookie: x\u{1b}[31m\u{202e}";
    let url = http_stub(&format!(
        "HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/plain\r\n\r\n{hostile}{}",
        "x".repeat(10_000)
    ));
    let harness = Harness::new();
    let core = harness.open();

    let error = core
        .add_caldav_account(&login(&url, "alice", "secret"))
        .unwrap_err();

    let ErrorCode::ServerRefused { status, message } = error.code() else {
        panic!("expected a refusal, got {error:?}");
    };
    assert_eq!(status, 500);
    assert!(message.starts_with("<img src=x onerror=alert(1)> Set-Cookie: x [31m "));
    assert!(message.chars().count() <= 200);
    assert_nothing_stored(&harness, &core);
}

#[test]
fn the_hostile_server_text_reaches_the_frontend_unchanged_as_text() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/hostile-event.json")).unwrap();
    let hostile = fixture["serverMessage"].as_str().unwrap();
    let url = http_stub(&format!(
        "HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/plain\r\n\r\n{hostile}"
    ));
    let harness = Harness::new();
    let core = harness.open();

    let error = core
        .add_caldav_account(&login(&url, "alice", "secret"))
        .unwrap_err();

    assert_eq!(
        error.code(),
        ErrorCode::ServerRefused {
            status: 500,
            message: hostile.to_owned()
        }
    );
}

#[test]
fn a_username_with_a_colon_cant_log_in_and_stores_nothing() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let harness = Harness::new();
    let core = harness.open();

    // Basic authentication can't tell such a username from the password.
    let result = core.add_caldav_account(&login(
        &radicale.url(),
        &format!("{}:x", user.name),
        &user.password,
    ));

    assert!(matches!(result, Err(CoreError::WrongCredentials)));
    assert_nothing_stored(&harness, &core);
}

#[test]
fn an_html_error_page_is_not_passed_on() {
    let url = http_stub(
        "HTTP/1.1 503 Service Unavailable\r\nContent-Type: text/html\r\n\r\n<h1>Down</h1>",
    );
    let harness = Harness::new();
    let core = harness.open();

    let error = core
        .add_caldav_account(&login(&url, "alice", "secret"))
        .unwrap_err();

    assert_eq!(
        error.code(),
        ErrorCode::ServerRefused {
            status: 503,
            message: String::new()
        }
    );
}

#[test]
fn removing_an_account_deletes_its_password_from_the_keyring() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let harness = Harness::new();
    let core = harness.open();
    let account = core
        .add_caldav_account(&login(&radicale.url(), &user.name, &user.password))
        .unwrap()
        .account;
    harness.signals.clear();

    core.remove_account(account.id).unwrap();

    assert_eq!(core.list_accounts().unwrap().len(), 1);
    assert!(harness.keyring.secrets().is_empty());
    assert_eq!(harness.signals.received(), vec![Signal::CalendarsChanged]);
}

#[test]
fn a_server_account_gets_no_calendars_created_in_freecal() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let harness = Harness::new();
    let core = harness.open();
    let account = core
        .add_caldav_account(&login(&radicale.url(), &user.name, &user.password))
        .unwrap()
        .account;

    let result = core.create_calendar(
        account.id,
        "Work",
        freecal_core::Colour::parse("#3366cc").unwrap(),
    );

    assert!(matches!(result, Err(CoreError::NotLocalAccount(_))));
}

fn assert_nothing_stored(harness: &Harness, core: &freecal_core::Core) {
    assert_eq!(
        core.list_accounts().unwrap().len(),
        1,
        "only the Local Account"
    );
    assert!(
        harness.keyring.secrets().is_empty(),
        "nothing in the keyring"
    );
    assert!(harness.signals.received().is_empty(), "no Signals");
}
