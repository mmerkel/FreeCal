//! Sync timing and triggers (ticket 16): polls every 2 minutes with the
//! window open and every 15 minutes in the background, syncs on window
//! focus, on waking from sleep and after local edits, sync now, and the
//! system's network status.

mod support;

use freecal_core::{
    AccountId, CalDavLogin, CalendarId, Core, OverallState, Signal, SyncStatus, SyncTrigger,
};
use support::radicale::{NewCalendar, Radicale};
use support::{Harness, calendar, draft, make_writable, timed};

/// A new Core with two server Accounts on Radicale, each with one
/// Calendar, and the Local Account with its starting Calendars. The
/// launch's first poll has run, at 12:00, and no Signals are recorded yet.
struct Synced {
    harness: Harness,
    core: Core,
    first: AccountId,
    second: AccountId,
    first_calendar: CalendarId,
}

impl Synced {
    fn new() -> Self {
        let radicale = Radicale::shared();
        let harness = Harness::new();
        let core = harness.open();
        let (first, first_calendar) = add_account(&core, radicale);
        let (second, _) = add_account(&core, radicale);
        core.run_scheduled_syncs();
        harness.signals.clear();
        Self {
            harness,
            core,
            first,
            second,
            first_calendar,
        }
    }

    /// Moves the harness clock to `hh:mm:ss` on the launch day, in Berlin.
    fn at(&self, time: &str) {
        self.harness.clock.set(&format!("2026-10-03T{time}+02:00"));
    }

    /// The Accounts that started a sync since the Signals were cleared, in
    /// order, each with whether the user started it.
    fn syncs(&self) -> Vec<(AccountId, bool)> {
        self.harness
            .signals
            .received()
            .into_iter()
            .filter_map(|signal| match signal {
                Signal::AccountSyncStatusChanged(status) => match status.report.status {
                    SyncStatus::Syncing { by_user, .. } => Some((status.account_id, by_user)),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    }

    fn run_and_take_syncs(&self) -> Vec<(AccountId, bool)> {
        self.core.run_scheduled_syncs();
        let syncs = self.syncs();
        self.harness.signals.clear();
        syncs
    }

    fn by_freecal(&self) -> Vec<(AccountId, bool)> {
        vec![(self.first, false), (self.second, false)]
    }

    fn account_status(&self, id: AccountId) -> SyncStatus {
        self.core
            .sync_statuses()
            .unwrap()
            .accounts
            .into_iter()
            .find(|status| status.account_id == id)
            .unwrap()
            .report
            .status
    }
}

/// Adds a CalDAV Account for a new Radicale user, with one Calendar.
fn add_account(core: &Core, radicale: &Radicale) -> (AccountId, CalendarId) {
    let user = radicale.user();
    let url = radicale.create_calendar(&user, NewCalendar::default());
    let account = core
        .add_caldav_account(&CalDavLogin {
            url: radicale.url(),
            username: user.name.clone(),
            password: user.password.clone(),
        })
        .unwrap()
        .account
        .id;
    let calendar = core.add_calendars(account, &[url]).unwrap()[0].id;
    (account, calendar)
}

#[test]
fn launching_syncs_every_server_account_at_once() {
    let radicale = Radicale::shared();
    let harness = Harness::new();
    let core = harness.open();
    let (first, _) = add_account(&core, radicale);
    let (second, _) = add_account(&core, radicale);
    drop(core);
    let core = harness.open();
    harness.signals.clear();

    core.run_scheduled_syncs();

    let synced: Vec<AccountId> = harness
        .signals
        .received()
        .into_iter()
        .filter_map(|signal| match signal {
            Signal::AccountSyncStatusChanged(status)
                if status.report.status == SyncStatus::Synced =>
            {
                Some(status.account_id)
            }
            _ => None,
        })
        .collect();
    assert_eq!(synced, [first, second]);
}

#[test]
fn with_the_window_open_it_polls_every_two_minutes() {
    let synced = Synced::new();

    synced.at("12:01:59");
    assert_eq!(synced.run_and_take_syncs(), []);
    synced.at("12:02:00");
    assert_eq!(synced.run_and_take_syncs(), synced.by_freecal());
    synced.at("12:03:00");
    assert_eq!(synced.run_and_take_syncs(), []);
    synced.at("12:04:00");
    assert_eq!(synced.run_and_take_syncs(), synced.by_freecal());
}

#[test]
fn in_the_background_it_polls_every_fifteen_minutes() {
    let synced = Synced::new();
    synced.core.set_window_open(false);

    synced.at("12:14:59");
    assert_eq!(synced.run_and_take_syncs(), []);
    synced.at("12:15:00");
    assert_eq!(synced.run_and_take_syncs(), synced.by_freecal());
}

#[test]
fn opening_the_window_again_polls_once_two_minutes_have_passed() {
    let synced = Synced::new();
    synced.core.set_window_open(false);
    synced.at("12:05:00");
    assert_eq!(synced.run_and_take_syncs(), []);

    synced.core.set_window_open(true);

    assert_eq!(synced.run_and_take_syncs(), synced.by_freecal());
}

#[test]
fn focusing_the_window_and_waking_from_sleep_sync_at_once() {
    for trigger in [SyncTrigger::WindowFocused, SyncTrigger::Woke] {
        let synced = Synced::new();
        synced.at("12:01:00");

        synced.core.trigger_sync(trigger);

        assert_eq!(
            synced.run_and_take_syncs(),
            synced.by_freecal(),
            "{trigger:?}"
        );
        // The trigger's sync counts as the poll.
        synced.at("12:02:59");
        assert_eq!(synced.run_and_take_syncs(), [], "{trigger:?}");
        synced.at("12:03:00");
        assert_eq!(
            synced.run_and_take_syncs(),
            synced.by_freecal(),
            "{trigger:?}"
        );
    }
}

#[test]
fn focusing_the_window_within_a_minute_of_a_sync_syncs_nothing() {
    let synced = Synced::new();
    synced.at("12:00:59");

    synced.core.trigger_sync(SyncTrigger::WindowFocused);
    assert_eq!(synced.run_and_take_syncs(), []);

    synced.core.trigger_sync(SyncTrigger::Woke);
    assert_eq!(synced.run_and_take_syncs(), synced.by_freecal());
}

#[test]
fn a_local_edit_in_a_server_calendar_syncs_its_account() {
    let synced = Synced::new();
    make_writable(&synced.harness, synced.first_calendar);

    synced
        .core
        .create_event(draft(
            synced.first_calendar,
            "Dentist",
            timed("2026-10-05T09:00:00", "2026-10-05T10:00:00"),
        ))
        .unwrap();

    assert_eq!(synced.run_and_take_syncs(), [(synced.first, false)]);
}

#[test]
fn a_local_edit_in_a_local_calendar_syncs_nothing() {
    let synced = Synced::new();
    let mine = calendar(&synced.core, "Mine");

    synced
        .core
        .create_event(draft(
            mine.id,
            "Dentist",
            timed("2026-10-05T09:00:00", "2026-10-05T10:00:00"),
        ))
        .unwrap();

    assert_eq!(synced.run_and_take_syncs(), []);
}

#[test]
fn sync_now_syncs_one_account_or_all_as_started_by_the_user() {
    let synced = Synced::new();

    synced.core.sync_now(Some(synced.second));
    assert_eq!(synced.run_and_take_syncs(), [(synced.second, true)]);

    synced.core.sync_now(None);
    assert_eq!(
        synced.run_and_take_syncs(),
        [(synced.first, true), (synced.second, true)]
    );
}

#[test]
fn syncs_asked_for_meanwhile_run_once() {
    let synced = Synced::new();

    synced.core.trigger_sync(SyncTrigger::WindowFocused);
    synced.core.sync_now(Some(synced.first));
    synced.core.trigger_sync(SyncTrigger::Woke);

    assert_eq!(
        synced.run_and_take_syncs(),
        [(synced.first, true), (synced.second, false)]
    );
    assert_eq!(synced.run_and_take_syncs(), []);
}

#[test]
fn sync_now_of_the_local_account_syncs_nothing() {
    let synced = Synced::new();
    let local = synced.core.list_accounts().unwrap()[0].id;

    synced.core.sync_now(Some(local));

    assert_eq!(synced.run_and_take_syncs(), []);
}

#[test]
fn with_no_network_every_server_account_is_offline_without_trying() {
    let synced = Synced::new();
    synced.at("12:00:30");

    synced.core.set_network_available(false);

    // Radicale is still there: only not trying makes them Offline.
    assert_eq!(synced.account_status(synced.first), SyncStatus::Offline);
    assert_eq!(synced.account_status(synced.second), SyncStatus::Offline);
    assert_eq!(
        synced.core.sync_statuses().unwrap().overall.state,
        OverallState::Offline
    );
    assert!(
        synced
            .harness
            .signals
            .received()
            .contains(&Signal::AccountSyncStatusChanged(
                synced
                    .core
                    .sync_statuses()
                    .unwrap()
                    .accounts
                    .into_iter()
                    .find(|status| status.account_id == synced.first)
                    .unwrap()
            ))
    );
}

#[test]
fn with_no_network_polls_and_triggers_are_skipped() {
    let synced = Synced::new();
    synced.core.set_network_available(false);
    synced.harness.signals.clear();

    synced.at("12:10:00");
    synced.core.trigger_sync(SyncTrigger::WindowFocused);
    synced.core.sync_now(None);

    assert_eq!(synced.run_and_take_syncs(), []);
}

#[test]
fn when_the_network_returns_every_server_account_syncs_at_once() {
    let synced = Synced::new();
    synced.core.set_network_available(false);
    synced.at("12:00:30");
    synced.harness.signals.clear();

    synced.core.set_network_available(true);

    assert_eq!(synced.run_and_take_syncs(), synced.by_freecal());
    assert_eq!(synced.account_status(synced.first), SyncStatus::Synced);
}

#[test]
fn a_network_status_that_stays_available_syncs_nothing() {
    let synced = Synced::new();

    synced.core.set_network_available(true);

    assert_eq!(synced.run_and_take_syncs(), []);
}
