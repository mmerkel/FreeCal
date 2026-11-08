//! Sync Status and the overall status (ticket 15): "Sync status changed"
//! for server Accounts and Calendars, Offline against Sync Errors, where
//! each lives, Unsent Changes, and the overall status.

mod support;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use freecal_core::{
    AccountId, AccountSyncStatus, CalDavLogin, CalendarId, CalendarSyncStatus, Core, CoreError,
    ErrorCode, FinishedSync, OverallState, OverallStatus, Signal, StartedBy, SyncOutcome,
    SyncStatus, SyncStatusReport,
};
use support::radicale::{NewCalendar, Radicale, RewritingProxy, User, closed_port_url};
use support::{
    Harness, account_sync_status, add_account, add_account_with_calendar, calendar,
    calendar_sync_status, draft, event, ics, timed, titles, user_login, utc,
};

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

    /// Adds the Account of another Radicale user, with one Calendar.
    fn add_second_account(&self) -> AccountId {
        add_account_with_calendar(&self.core, self.radicale).0
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
        account_sync_status(&self.core, self.account).report
    }

    fn calendar_status(&self, id: CalendarId) -> SyncStatusReport {
        calendar_sync_status(&self.core, id).report
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

/// The harness clock's time, 12:00 in Berlin, in the Display Time Zone.
fn noon() -> FinishedSync {
    finished(SyncOutcome::Synced)
}

fn finished(outcome: SyncOutcome) -> FinishedSync {
    FinishedSync {
        at: utc("2026-10-03T10:00:00Z"),
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

fn calendar_signal(id: CalendarId, status: SyncStatus, last_sync: Option<FinishedSync>) -> Signal {
    Signal::CalendarSyncStatusChanged(CalendarSyncStatus {
        calendar_id: id,
        report: report(status, last_sync),
        first_sync_unfinished: false,
    })
}

/// Like [`calendar_signal`], for a Calendar whose First Sync is unfinished.
fn unfinished_signal(
    id: CalendarId,
    status: SyncStatus,
    last_sync: Option<FinishedSync>,
) -> Signal {
    Signal::CalendarSyncStatusChanged(CalendarSyncStatus {
        calendar_id: id,
        report: report(status, last_sync),
        first_sync_unfinished: true,
    })
}

fn account_signal(id: AccountId, status: SyncStatus, last_sync: Option<FinishedSync>) -> Signal {
    Signal::AccountSyncStatusChanged(AccountSyncStatus {
        account_id: id,
        report: report(status, last_sync),
    })
}

fn syncing(by_user: bool) -> SyncStatus {
    SyncStatus::Syncing { by_user }
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

    assert_eq!(
        harness.signals.received(),
        [
            Signal::CalendarsChanged,
            unfinished_signal(work, syncing(true), None),
            account_signal(account, syncing(true), None),
            calendar_signal(work, SyncStatus::Synced, Some(noon())),
            account_signal(account, SyncStatus::Synced, Some(noon())),
        ]
    );
    assert_eq!(
        core.sync_statuses().unwrap().overall,
        OverallStatus {
            state: OverallState::Synced,
            last_sync: Some(utc("2026-10-03T10:00:00Z")),
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
        at: utc("2026-10-03T10:05:00Z"),
        outcome: SyncOutcome::Synced,
    });
    let (account, work, home) = (server.account, server.work, server.home);
    assert_eq!(
        server.harness.signals.received(),
        [
            account_signal(account, syncing(false), Some(noon())),
            calendar_signal(work, syncing(false), Some(noon())),
            calendar_signal(work, SyncStatus::Synced, at_five_past.clone()),
            calendar_signal(home, syncing(false), Some(noon())),
            calendar_signal(home, SyncStatus::Synced, at_five_past.clone()),
            account_signal(account, SyncStatus::Synced, at_five_past),
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
    let second = offline.add_second_account();
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
    let second = server.add_second_account();
    let last_sync = || core.sync_statuses().unwrap().overall.last_sync;

    server.harness.clock.set("2026-10-03T12:30:00+02:00");
    core.sync_account(second, StartedBy::FreeCal).unwrap();

    assert_eq!(
        last_sync(),
        Some(utc("2026-10-03T10:00:00Z")),
        "the first Account last synced at noon, 10:00 UTC"
    );

    server.harness.clock.set("2026-10-03T12:40:00+02:00");
    core.sync_account(server.account, StartedBy::FreeCal)
        .unwrap();

    assert_eq!(last_sync(), Some(utc("2026-10-03T10:30:00Z")));

    server.harness.clock.set("2026-10-03T12:50:00+02:00");
    server.go_offline();
    let _ = core.sync_account(server.account, StartedBy::FreeCal);

    assert_eq!(
        last_sync(),
        Some(utc("2026-10-03T10:30:00Z")),
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
        at: utc("2026-10-03T10:30:00Z"),
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
            last_sync: Some(utc("2026-10-03T10:00:00Z")),
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
    server
        .core
        .set_calendar_read_only(server.work, false)
        .unwrap();

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
            first_sync_unfinished: false,
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

/// Three Calendars on Radicale, "A", "B" and "C", with one Event each, and
/// a new Core with the user's Account added through `proxy`, none of the
/// Calendars added yet. Returns the Core's harness, the Core, the Account
/// and the Calendars' URLs through the proxy.
fn three_calendars(
    radicale: &Radicale,
    user: &User,
    proxy: &RewritingProxy,
) -> (Harness, Arc<Core>, AccountId, Vec<String>) {
    let urls: Vec<String> = ["A", "B", "C"]
        .into_iter()
        .map(|name| {
            let url = radicale.create_calendar(
                user,
                NewCalendar {
                    name: Some(name),
                    ..NewCalendar::default()
                },
            );
            radicale.put_event(
                user,
                &url,
                name,
                ics(&event(
                    name,
                    "DTSTART:20261005T070000Z",
                    "DTEND:20261005T080000Z",
                )),
            );
            url.replace(&radicale.url(), &proxy.url)
        })
        .collect();
    let harness = Harness::new();
    let core = Arc::new(harness.open_without_starting_calendars());
    let account = core
        .add_caldav_account(&CalDavLogin {
            url: proxy.url.clone(),
            ..user_login(radicale, user)
        })
        .unwrap()
        .account
        .id;
    harness.signals.clear();
    (harness, core, account, urls)
}

/// Runs `react` once, on the first Signal saying that the Calendar listed
/// first in Sync Statuses is Synced.
fn once_first_synced(
    harness: &Harness,
    core: &Arc<Core>,
    react: impl Fn() + Send + Sync + 'static,
) {
    let reader = Arc::downgrade(core);
    let done = AtomicBool::new(false);
    harness.signals.on_receive(move |signal| {
        let Signal::CalendarSyncStatusChanged(status) = signal else {
            return;
        };
        let core = reader.upgrade().unwrap();
        let first = core.sync_statuses().unwrap().calendars[0].calendar_id;
        if status.calendar_id == first
            && status.report.status == SyncStatus::Synced
            && !done.swap(true, Ordering::SeqCst)
        {
            react();
        }
    });
}

#[test]
fn adding_calendars_stopped_by_a_failure_about_the_account_leaves_the_rest_for_the_next_sync() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let proxy = Arc::new(RewritingProxy::passthrough(radicale));
    let (harness, core, account, urls) = three_calendars(radicale, &user, &proxy);
    let away = proxy.clone();
    once_first_synced(&harness, &core, move || away.go_away());

    let failed = core.add_calendars(account, &urls);

    assert!(
        matches!(failed, Err(CoreError::ServerUnreachable(_))),
        "{failed:?}"
    );
    let ids: Vec<CalendarId> = core
        .list_calendars()
        .unwrap()
        .into_iter()
        .filter(|calendar| calendar.account_id == account)
        .map(|calendar| calendar.id)
        .collect();
    // Each added Calendar is syncing its First Sync from when it is added.
    assert_eq!(
        harness.signals.received()[..5],
        [
            Signal::CalendarsChanged,
            unfinished_signal(ids[0], syncing(true), None),
            unfinished_signal(ids[1], syncing(true), None),
            unfinished_signal(ids[2], syncing(true), None),
            account_signal(account, syncing(true), None),
        ]
    );
    let statuses = core.sync_statuses().unwrap();
    assert_eq!(
        statuses.calendars,
        [
            CalendarSyncStatus {
                calendar_id: ids[0],
                report: report(SyncStatus::Synced, Some(noon())),
                first_sync_unfinished: false,
            },
            // As before the sync: never synced.
            CalendarSyncStatus {
                calendar_id: ids[1],
                report: report(SyncStatus::Synced, None),
                first_sync_unfinished: true,
            },
            CalendarSyncStatus {
                calendar_id: ids[2],
                report: report(SyncStatus::Synced, None),
                first_sync_unfinished: true,
            },
        ]
    );
    assert_eq!(
        account_sync_status(&core, account).report,
        report(SyncStatus::Offline, Some(finished(SyncOutcome::Offline)))
    );
    assert_eq!(
        statuses.overall,
        OverallStatus {
            state: OverallState::Offline,
            last_sync: None,
        }
    );

    proxy.come_back();
    harness.clock.set("2026-10-03T12:05:00+02:00");
    core.sync_account(account, StartedBy::FreeCal).unwrap();

    for id in &ids {
        let status = calendar_sync_status(&core, *id);
        assert_eq!(status.report.status, SyncStatus::Synced);
        assert!(!status.first_sync_unfinished);
    }
    assert_eq!(titles(&core, ids[1]), ["B"]);
    assert_eq!(titles(&core, ids[2]), ["C"]);
}

#[test]
fn a_failure_about_one_calendar_while_adding_calendars_is_that_calendars_sync_error() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let proxy = RewritingProxy::passthrough(radicale);
    let (harness, core, account, urls) = three_calendars(radicale, &user, &proxy);
    let b = urls[1].replace(&proxy.url, &radicale.url());
    once_first_synced(&harness, &core, move || radicale.delete_calendar(&user, &b));

    let failed = core.add_calendars(account, &urls);

    assert!(
        matches!(failed, Err(CoreError::ServerRefused { status: 404, .. })),
        "{failed:?}"
    );
    let statuses = core.sync_statuses().unwrap();
    let [a, b, c] = statuses.calendars.as_slice() else {
        panic!("{statuses:?}");
    };
    assert_eq!(a.report.status, SyncStatus::Synced);
    assert!(
        matches!(
            &b.report.status,
            SyncStatus::SyncError {
                error: ErrorCode::ServerRefused { status: 404, .. }
            }
        ),
        "{b:?}"
    );
    assert!(b.first_sync_unfinished);
    // The other Calendars still sync, as in any sync of the Account.
    assert_eq!(c.report.status, SyncStatus::Synced);
    assert!(!c.first_sync_unfinished);
    assert_eq!(titles(&core, c.calendar_id), ["C"]);
    assert_eq!(
        account_sync_status(&core, account).report,
        report(SyncStatus::Synced, Some(noon()))
    );
    assert_eq!(statuses.overall.state, OverallState::SyncError);
}

#[test]
fn a_calendar_quit_during_its_first_sync_says_so_after_a_relaunch_until_it_finishes() {
    let server = Server::new();
    // As if FreeCal quit during the Work Calendar's First Sync.
    server.harness.arrange_local_store(&format!(
        "UPDATE calendar SET synced = 0, sync_token = NULL, last_sync_at = NULL, \
             last_sync_outcome = NULL, last_synced_at = NULL WHERE id = {}",
        server.work.0
    ));
    let server = server.relaunch();

    let statuses = server.core.sync_statuses().unwrap();

    assert_eq!(
        statuses.calendars[0],
        CalendarSyncStatus {
            calendar_id: server.work,
            report: report(SyncStatus::Synced, None),
            first_sync_unfinished: true,
        }
    );
    assert!(!statuses.calendars[1].first_sync_unfinished);
    // The overall status doesn't show it; never having synced does.
    assert_eq!(
        statuses.overall,
        OverallStatus {
            state: OverallState::Synced,
            last_sync: None,
        }
    );

    server.harness.signals.clear();
    server
        .core
        .sync_account(server.account, StartedBy::FreeCal)
        .unwrap();

    let at_noon = Some(noon());
    let received = server.harness.signals.received();
    assert_eq!(
        received[1..3],
        [
            unfinished_signal(server.work, syncing(false), None),
            calendar_signal(server.work, SyncStatus::Synced, at_noon),
        ]
    );
}

#[test]
fn a_sync_ending_after_its_account_was_removed_says_nothing_about_it() {
    let radicale = Radicale::shared();
    let user = radicale.user();
    let proxy = RewritingProxy::passthrough(radicale);
    let (harness, core, account, urls) = three_calendars(radicale, &user, &proxy);
    let remover = Arc::downgrade(&core);
    let signals = harness.signals.clone();
    once_first_synced(&harness, &core, move || {
        remover.upgrade().unwrap().remove_account(account).unwrap();
        signals.clear();
    });

    let _ = core.add_calendars(account, &urls);

    let after_removal = harness.signals.received();
    assert!(
        after_removal.iter().all(|signal| !matches!(
            signal,
            Signal::AccountSyncStatusChanged(_) | Signal::CalendarSyncStatusChanged(_)
        )),
        "{after_removal:?}"
    );
    let statuses = core.sync_statuses().unwrap();
    assert!(statuses.accounts.is_empty());
    assert!(statuses.calendars.is_empty());
    assert_eq!(statuses.overall.state, OverallState::Synced);
}
