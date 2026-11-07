//! Sync Status and the overall status (ticket 15): "Sync status changed"
//! for server Accounts and Calendars, Offline against Sync Errors, where
//! each lives, Unsent Changes, and the overall status.

mod support;

use std::sync::{Arc, Mutex};

use freecal_core::{
    AccountId, AccountSyncStatus, CalDavLogin, CalendarId, CalendarSyncStatus, Core, ErrorCode,
    FinishedSync, OverallState, OverallStatus, Signal, StartedBy, SyncOutcome, SyncStatus,
    SyncStatusReport,
};
use support::radicale::{NewCalendar, Radicale, User, closed_port_url};
use support::{Harness, calendar, draft, make_writable, time, timed};

/// A user on Radicale with two Calendars, "Work" and "Home", both added
/// to a new Core, with no Signals recorded yet.
struct Server {
    radicale: &'static Radicale,
    user: User,
    harness: Harness,
    core: Core,
    account: AccountId,
    work: CalendarId,
    home: CalendarId,
    home_url: String,
}

impl Server {
    fn new() -> Self {
        let radicale = Radicale::shared();
        let user = radicale.user();
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let account = add_account(&core, radicale, &user);
        let mut urls = ["Work", "Home"].map(|name| {
            radicale.create_calendar(
                &user,
                NewCalendar {
                    name: Some(name),
                    ..NewCalendar::default()
                },
            )
        });
        let added = core.add_calendars(account, &urls).unwrap();
        harness.signals.clear();
        Self {
            radicale,
            user,
            harness,
            core,
            account,
            work: added[0].id,
            home: added[1].id,
            home_url: std::mem::take(&mut urls[1]),
        }
    }

    /// Points the Account and its Calendars at a server that isn't there,
    /// as when offline.
    fn go_offline(&self) {
        move_account(
            &self.harness,
            self.account,
            &self.radicale.url(),
            &closed_port_url(),
        );
    }

    fn go_online(&self) {
        let offline = self.core.list_accounts().unwrap()[1]
            .server
            .clone()
            .unwrap()
            .url;
        move_account(&self.harness, self.account, &offline, &self.radicale.url());
    }

    /// Quits FreeCal and launches it again on the same Local Store.
    fn relaunch(self) -> Self {
        let Server {
            radicale,
            user,
            harness,
            core,
            account,
            work,
            home,
            home_url,
        } = self;
        drop(core);
        let core = harness.open();
        Server {
            radicale,
            user,
            harness,
            core,
            account,
            work,
            home,
            home_url,
        }
    }

    fn account_status(&self) -> SyncStatusReport {
        let statuses = self.core.sync_statuses().unwrap();
        statuses
            .accounts
            .into_iter()
            .find(|status| status.account_id == self.account)
            .unwrap()
            .report
    }

    fn calendar_status(&self, id: CalendarId) -> SyncStatusReport {
        let statuses = self.core.sync_statuses().unwrap();
        statuses
            .calendars
            .into_iter()
            .find(|status| status.calendar_id == id)
            .unwrap()
            .report
    }
}

/// Moves the Account `id` and its Calendars from the server at `from` to
/// the one at `to`.
fn move_account(harness: &Harness, id: AccountId, from: &str, to: &str) {
    harness.arrange_local_store(&format!(
        "UPDATE account SET url = replace(url, '{from}', '{to}') WHERE id = {id};
         UPDATE calendar SET url = replace(url, '{from}', '{to}') WHERE account_id = {id}",
        id = id.0
    ));
}

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

/// The harness clock's time, 12:00 in Berlin, in the Display Time Zone.
fn noon() -> FinishedSync {
    finished(SyncOutcome::Synced)
}

fn finished(outcome: SyncOutcome) -> FinishedSync {
    FinishedSync {
        at: time("2026-10-03T12:00:00"),
        outcome,
    }
}

fn report(status: SyncStatus, last_sync: Option<FinishedSync>) -> SyncStatusReport {
    SyncStatusReport {
        status,
        last_sync,
        unsent_changes: 0,
    }
}

fn syncing(by_user: bool, first_sync: bool) -> SyncStatus {
    SyncStatus::Syncing {
        by_user,
        first_sync,
    }
}

#[test]
fn the_local_account_has_no_sync_status_and_the_overall_status_is_synced() {
    let harness = Harness::new();
    let core = harness.open();
    let local = core.list_accounts().unwrap()[0].id;
    let mine = calendar(&core, "Mine");
    harness.signals.clear();

    core.create_event(draft(
        mine.id,
        "Dentist",
        timed("2026-10-05T09:00:00", "2026-10-05T10:00:00"),
    ))
    .unwrap();
    assert!(
        core.sync_account(local, StartedBy::User)
            .unwrap()
            .is_empty()
    );

    let statuses = core.sync_statuses().unwrap();
    assert_eq!(
        statuses.overall,
        OverallStatus {
            state: OverallState::Synced,
            last_sync: None,
        }
    );
    assert!(statuses.accounts.is_empty());
    assert!(statuses.calendars.is_empty());
    assert!(harness.signals.received().iter().all(|signal| !matches!(
        signal,
        Signal::AccountSyncStatusChanged(_) | Signal::CalendarSyncStatusChanged(_)
    )));
}

#[test]
fn adding_calendars_reports_their_first_sync_as_started_by_the_user() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let url = radicale.create_calendar(&user, NewCalendar::default());
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let account = add_account(&core, radicale, &user);
    harness.signals.clear();

    let work = core.add_calendars(account, &[url]).unwrap()[0].id;

    let calendar = |status, last_sync| {
        Signal::CalendarSyncStatusChanged(CalendarSyncStatus {
            calendar_id: work,
            report: report(status, last_sync),
        })
    };
    let account_signal = |status, last_sync| {
        Signal::AccountSyncStatusChanged(AccountSyncStatus {
            account_id: account,
            report: report(status, last_sync),
        })
    };
    assert_eq!(
        harness.signals.received(),
        [
            Signal::CalendarsChanged,
            calendar(syncing(true, true), None),
            account_signal(syncing(true, false), None),
            calendar(SyncStatus::Synced, Some(noon())),
            account_signal(SyncStatus::Synced, Some(noon())),
        ]
    );
    assert_eq!(
        core.sync_statuses().unwrap().overall,
        OverallStatus {
            state: OverallState::Synced,
            last_sync: Some(time("2026-10-03T12:00:00")),
        }
    );
}

#[test]
fn a_sync_reports_syncing_then_synced_for_the_account_and_each_calendar() {
    let server = Server::new();
    server.harness.clock.set("2026-10-03T12:05:00+02:00");

    server
        .core
        .sync_account(server.account, StartedBy::FreeCal)
        .unwrap();

    let at_five_past = Some(FinishedSync {
        at: time("2026-10-03T12:05:00"),
        outcome: SyncOutcome::Synced,
    });
    let calendar = |id, status, last_sync| {
        Signal::CalendarSyncStatusChanged(CalendarSyncStatus {
            calendar_id: id,
            report: report(status, last_sync),
        })
    };
    let account = |status, last_sync| {
        Signal::AccountSyncStatusChanged(AccountSyncStatus {
            account_id: server.account,
            report: report(status, last_sync),
        })
    };
    assert_eq!(
        server.harness.signals.received(),
        [
            account(syncing(false, false), Some(noon())),
            calendar(server.work, syncing(false, false), Some(noon())),
            calendar(server.work, SyncStatus::Synced, at_five_past.clone()),
            calendar(server.home, syncing(false, false), Some(noon())),
            calendar(server.home, SyncStatus::Synced, at_five_past.clone()),
            account(SyncStatus::Synced, at_five_past),
        ]
    );
}

#[test]
fn a_network_failure_makes_the_account_offline_and_leaves_its_calendars_as_they_were() {
    let server = Server::new();
    server.go_offline();

    let failed = server.core.sync_account(server.account, StartedBy::FreeCal);

    assert!(failed.is_err());
    assert_eq!(
        server.account_status(),
        report(SyncStatus::Offline, Some(finished(SyncOutcome::Offline)))
    );
    // Shown once on the Account, not on every Calendar.
    for calendar in [server.work, server.home] {
        assert_eq!(
            server.calendar_status(calendar),
            report(SyncStatus::Synced, Some(noon()))
        );
    }
    // The sync stops at the first Calendar instead of waiting on each.
    assert!(
        !server
            .harness
            .signals
            .received()
            .iter()
            .any(|signal| matches!(signal, Signal::CalendarSyncStatusChanged(status) if status.calendar_id == server.home))
    );
    assert_eq!(
        server.core.sync_statuses().unwrap().overall.state,
        OverallState::Offline
    );

    server.go_online();
    server
        .core
        .sync_account(server.account, StartedBy::FreeCal)
        .unwrap();

    assert_eq!(server.account_status().status, SyncStatus::Synced);
    assert_eq!(
        server.core.sync_statuses().unwrap().overall.state,
        OverallState::Synced
    );
}

#[test]
fn an_offline_account_that_freecal_tries_again_stays_offline_overall_while_it_tries() {
    let server = Arc::new(Server::new());
    server.go_offline();
    let _ = server.core.sync_account(server.account, StartedBy::FreeCal);
    // The overall status each Signal of the next try finds.
    let seen = Arc::new(Mutex::new(Vec::new()));
    let reader = Arc::downgrade(&server);
    let record = seen.clone();
    server.harness.signals.on_receive(move |_| {
        let server = reader.upgrade().unwrap();
        let state = server.core.sync_statuses().unwrap().overall.state;
        record.lock().unwrap().push(state);
    });

    let _ = server.core.sync_account(server.account, StartedBy::FreeCal);

    let seen = seen.lock().unwrap();
    assert!(!seen.is_empty());
    assert!(
        seen.iter().all(|state| *state == OverallState::Offline),
        "{seen:?}"
    );
}

#[test]
fn an_error_response_is_a_sync_error_of_the_calendar_where_it_happened() {
    let server = Server::new();
    server
        .radicale
        .delete_calendar(&server.user, &server.home_url);

    let failed = server.core.sync_account(server.account, StartedBy::FreeCal);

    assert!(failed.is_err());
    let home = server.calendar_status(server.home);
    assert!(
        matches!(
            &home.status,
            SyncStatus::SyncError {
                error: ErrorCode::ServerRefused { status: 404, .. }
            }
        ),
        "{home:?}"
    );
    assert_eq!(
        home.last_sync.map(|last| last.outcome),
        Some(SyncOutcome::SyncError {
            error: match home.status {
                SyncStatus::SyncError { error } => error,
                _ => unreachable!(),
            }
        })
    );
    assert_eq!(
        server.calendar_status(server.work).status,
        SyncStatus::Synced
    );
    assert_eq!(server.account_status().status, SyncStatus::Synced);
    assert_eq!(
        server.core.sync_statuses().unwrap().overall.state,
        OverallState::SyncError
    );
}

#[test]
fn a_login_the_keyring_doesnt_give_is_a_sync_error_of_the_account() {
    let server = Server::new();
    server.harness.keyring.forget_all();

    let failed = server.core.sync_account(server.account, StartedBy::User);

    assert!(failed.is_err());
    assert_eq!(
        server.account_status().status,
        SyncStatus::SyncError {
            error: ErrorCode::PasswordNotInKeyring
        }
    );
    assert_eq!(
        server.calendar_status(server.work).status,
        SyncStatus::Synced
    );
    assert_eq!(
        server.core.sync_statuses().unwrap().overall.state,
        OverallState::SyncError
    );
}

#[test]
fn the_overall_status_is_offline_only_when_every_server_account_is() {
    let offline = Server::new();
    let core = &offline.core;
    let other = Radicale::shared().user();
    let second = add_account(core, offline.radicale, &other);
    let url = offline
        .radicale
        .create_calendar(&other, NewCalendar::default());
    core.add_calendars(second, &[url]).unwrap();
    offline.go_offline();
    let _ = core.sync_account(offline.account, StartedBy::FreeCal);

    assert_eq!(
        core.sync_statuses().unwrap().overall.state,
        OverallState::Synced,
        "only the sidebar marks some Accounts Offline"
    );

    move_account(
        &offline.harness,
        second,
        &offline.radicale.url(),
        &closed_port_url(),
    );
    let _ = core.sync_account(second, StartedBy::FreeCal);

    assert_eq!(
        core.sync_statuses().unwrap().overall.state,
        OverallState::Offline
    );
}

#[test]
fn the_overall_last_sync_is_the_oldest_last_successful_sync() {
    let server = Server::new();
    let core = &server.core;
    let other = Radicale::shared().user();
    let second = add_account(core, server.radicale, &other);
    let url = server
        .radicale
        .create_calendar(&other, NewCalendar::default());
    core.add_calendars(second, &[url]).unwrap();
    let last_sync = || core.sync_statuses().unwrap().overall.last_sync;

    server.harness.clock.set("2026-10-03T12:30:00+02:00");
    core.sync_account(second, StartedBy::FreeCal).unwrap();

    assert_eq!(
        last_sync(),
        Some(time("2026-10-03T12:00:00")),
        "the first Account last synced at noon"
    );

    server.harness.clock.set("2026-10-03T12:40:00+02:00");
    core.sync_account(server.account, StartedBy::FreeCal)
        .unwrap();

    assert_eq!(last_sync(), Some(time("2026-10-03T12:30:00")));

    server.harness.clock.set("2026-10-03T12:50:00+02:00");
    server.go_offline();
    let _ = core.sync_account(server.account, StartedBy::FreeCal);

    assert_eq!(
        last_sync(),
        Some(time("2026-10-03T12:30:00")),
        "a failed sync doesn't count"
    );
}

#[test]
fn after_a_relaunch_each_last_sync_shows_until_the_next_sync() {
    let server = Server::new();
    server.harness.clock.set("2026-10-03T12:30:00+02:00");
    server.go_offline();
    let _ = server.core.sync_account(server.account, StartedBy::FreeCal);

    let server = server.relaunch();

    let offline_at = FinishedSync {
        at: time("2026-10-03T12:30:00"),
        outcome: SyncOutcome::Offline,
    };
    assert_eq!(
        server.account_status(),
        report(SyncStatus::Offline, Some(offline_at))
    );
    assert_eq!(
        server.calendar_status(server.work),
        report(SyncStatus::Synced, Some(noon()))
    );
    assert_eq!(
        server.core.sync_statuses().unwrap().overall,
        OverallStatus {
            state: OverallState::Offline,
            last_sync: Some(time("2026-10-03T12:00:00")),
        }
    );
}

#[test]
fn a_stored_last_sync_is_untrusted() {
    let server = Server::new();
    let long = "x".repeat(300);
    server.harness.arrange_local_store(&format!(
        "UPDATE calendar SET last_sync_outcome = 'not an outcome' WHERE id = {work};
         UPDATE calendar SET last_sync_outcome =
             '{{\"kind\":\"syncError\",\"error\":{{\"code\":\"serverRefused\",\"status\":500,\"message\":\"a\\nb{long}\"}}}}'
             WHERE id = {home};
         UPDATE account SET last_sync_at = 1e30",
        work = server.work.0,
        home = server.home.0,
    ));

    let server = server.relaunch();

    assert_eq!(server.calendar_status(server.work).last_sync, None);
    let Some(FinishedSync {
        outcome:
            SyncOutcome::SyncError {
                error: ErrorCode::ServerRefused { message, .. },
            },
        ..
    }) = server.calendar_status(server.home).last_sync
    else {
        panic!("the stored Sync Error is read");
    };
    assert!(message.starts_with("a b"), "{message}");
    assert_eq!(message.chars().count(), 200);
    assert_eq!(server.account_status().last_sync, None);
}

#[test]
fn a_change_in_a_server_calendar_reports_its_unsent_changes() {
    let server = Server::new();
    make_writable(&server.harness, server.work);

    server
        .core
        .create_event(draft(
            server.work,
            "Standup",
            timed("2026-10-05T09:00:00", "2026-10-05T09:15:00"),
        ))
        .unwrap();

    let received = server.harness.signals.received();
    assert!(
        received.contains(&Signal::CalendarSyncStatusChanged(CalendarSyncStatus {
            calendar_id: server.work,
            report: SyncStatusReport {
                unsent_changes: 1,
                ..report(SyncStatus::Synced, Some(noon()))
            },
        }))
    );
    assert!(
        received.contains(&Signal::AccountSyncStatusChanged(AccountSyncStatus {
            account_id: server.account,
            report: SyncStatusReport {
                unsent_changes: 1,
                ..report(SyncStatus::Synced, Some(noon()))
            },
        }))
    );
    assert_eq!(server.calendar_status(server.home).unsent_changes, 0);
}

#[test]
fn a_removed_calendar_has_no_sync_status() {
    let server = Server::new();

    server.core.remove_calendar(server.home).unwrap();

    let statuses = server.core.sync_statuses().unwrap();
    assert_eq!(
        statuses
            .calendars
            .iter()
            .map(|status| status.calendar_id)
            .collect::<Vec<_>>(),
        [server.work]
    );
}
