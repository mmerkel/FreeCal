//! The Sync Scheduler (new term): decides when FreeCal syncs (spec: "Sync
//! timing"). It polls every server Account every 5 minutes while the
//! window is open and every 15 minutes in the background, and syncs on
//! window focus, on waking from sleep, after local edits, when the network
//! returns, and on sync now. With no network at all it syncs nothing.
//!
//! It only decides; [`crate::Core::run_scheduled_syncs`] runs the syncs it
//! finds due, on the one thread the shell keeps for them.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, FixedOffset, TimeDelta};

use crate::{AccountId, CalendarId, StartedBy};

/// How often every server Account is polled while the window is open
/// (US-64).
const POLL_WITH_WINDOW: TimeDelta = TimeDelta::minutes(5);

/// How often every server Account is polled while FreeCal runs in the
/// background (US-65).
const POLL_IN_BACKGROUND: TimeDelta = TimeDelta::minutes(15);

/// How long after a sync of every server Account focusing the window
/// syncs nothing, so that the focus right after launch, or switching back
/// and forth between windows, doesn't sync again and again.
const FOCUS_QUIET: TimeDelta = TimeDelta::minutes(1);

/// Something besides a poll, a local edit or sync now that makes every
/// server Account sync at once, as the shell notices it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncTrigger {
    WindowFocused,
    /// The computer woke from sleep.
    Woke,
}

/// The syncs asked for and not started yet: of every server Account, of
/// some, of some of their Calendars, or all of these, each with who asked.
/// The user wins over FreeCal, and a sync of an Account takes in those of
/// its Calendars.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct SyncRequests {
    every_account: Option<StartedBy>,
    accounts: BTreeMap<AccountId, StartedBy>,
    /// Calendars to sync without the rest of their Account, by Account, as
    /// after a local change; always asked for by FreeCal.
    calendars: BTreeMap<AccountId, BTreeSet<CalendarId>>,
    /// Whether they hold the sync of every Account at launch, the first
    /// that runs.
    at_launch: bool,
}

impl SyncRequests {
    pub fn is_empty(&self) -> bool {
        self.every_account.is_none() && self.accounts.is_empty() && self.calendars.is_empty()
    }

    /// Whether every server Account is to sync, not only those named.
    pub fn every_account(&self) -> bool {
        self.every_account.is_some()
    }

    /// The Accounts named on their own or by some of their Calendars, each
    /// once, which [`SyncRequests::every_account`] may add to.
    pub fn accounts(&self) -> impl Iterator<Item = AccountId> + '_ {
        let named: BTreeSet<AccountId> = self
            .accounts
            .keys()
            .chain(self.calendars.keys())
            .copied()
            .collect();
        named.into_iter()
    }

    /// The Calendars of the Account `id` to sync: none for all of them,
    /// as when the Account is to sync as a whole.
    pub fn calendars_of(&self, id: AccountId) -> Option<&BTreeSet<CalendarId>> {
        if self.every_account.is_some() || self.accounts.contains_key(&id) {
            return None;
        }
        self.calendars.get(&id)
    }

    /// Who the sync of the Account `id` counts as started by.
    pub fn started_by(&self, id: AccountId) -> StartedBy {
        self.every_account
            .max(self.accounts.get(&id).copied())
            .unwrap_or(StartedBy::FreeCal)
    }

    /// Whether the sync of the Account `id` reads again the privileges of
    /// its Server read-only Calendars, to notice a permission the server
    /// granted again: a sync the user starts and the one at launch do
    /// (ticket 130).
    pub fn looks_for_grants(&self, id: AccountId) -> bool {
        self.at_launch || self.started_by(id) == StartedBy::User
    }

    fn ask_every_account(&mut self, by: StartedBy) {
        self.every_account = self.every_account.max(Some(by));
    }

    fn ask_account(&mut self, id: AccountId, by: StartedBy) {
        let asked = self.accounts.entry(id).or_insert(by);
        *asked = (*asked).max(by);
    }

    fn ask_calendar(&mut self, account: AccountId, calendar: CalendarId) {
        self.calendars.entry(account).or_default().insert(calendar);
    }
}

#[derive(Debug)]
pub(crate) struct SyncScheduler {
    window_open: bool,
    network_available: bool,
    /// When the last sync of every server Account was due; none before the
    /// first, which is due at once on launch.
    last_poll: Option<DateTime<FixedOffset>>,
    requests: SyncRequests,
    /// Whether the sync of every Account at launch hasn't run yet: with no
    /// network at launch, it is the one the network's return asks for.
    launch_sync_due: bool,
}

impl Default for SyncScheduler {
    /// On launch the window opens, and the network is taken as available
    /// until the shell says otherwise.
    fn default() -> Self {
        SyncScheduler {
            window_open: true,
            network_available: true,
            last_poll: None,
            requests: SyncRequests::default(),
            launch_sync_due: true,
        }
    }
}

impl SyncScheduler {
    pub fn set_window_open(&mut self, open: bool) {
        self.window_open = open;
    }

    /// Takes in the system's network status. Returns whether the network
    /// went away, so that every server Account is to be marked Offline. Its
    /// return asks for a sync of every server Account, which sends what
    /// waited.
    pub fn set_network_available(&mut self, available: bool) -> bool {
        let was = std::mem::replace(&mut self.network_available, available);
        if available && !was {
            self.requests.ask_every_account(StartedBy::FreeCal);
        }
        was && !available
    }

    pub fn network_available(&self) -> bool {
        self.network_available
    }

    pub fn ask_every_account(&mut self, by: StartedBy) {
        self.requests.ask_every_account(by);
    }

    /// Asks for a sync of every server Account because of `trigger` at
    /// `now`; a window focus less than [`FOCUS_QUIET`] after the last sync
    /// of every Account asks for none.
    pub fn trigger(&mut self, trigger: SyncTrigger, now: DateTime<FixedOffset>) {
        let quiet = trigger == SyncTrigger::WindowFocused
            && self.last_poll.is_some_and(|last| now - last < FOCUS_QUIET);
        if !quiet {
            self.requests.ask_every_account(StartedBy::FreeCal);
        }
    }

    pub fn ask_account(&mut self, id: AccountId, by: StartedBy) {
        self.requests.ask_account(id, by);
    }

    /// Asks FreeCal for a sync of the Calendar `calendar` of the Account
    /// `account`, without the rest of the Account.
    pub fn ask_calendar(&mut self, account: AccountId, calendar: CalendarId) {
        self.requests.ask_calendar(account, calendar);
    }

    pub fn has_requests(&self) -> bool {
        !self.requests.is_empty()
    }

    /// Takes the syncs due at `now`: those asked for, and a poll once its
    /// period has passed since the last sync of every Account. With no
    /// network, none is due and those asked for are dropped: the network's
    /// return syncs every Account anyway.
    pub fn take_due(&mut self, now: DateTime<FixedOffset>) -> SyncRequests {
        let period = if self.window_open {
            POLL_WITH_WINDOW
        } else {
            POLL_IN_BACKGROUND
        };
        if self.last_poll.is_none_or(|last| now - last >= period) {
            self.requests.ask_every_account(StartedBy::FreeCal);
        }
        if self.requests.every_account() {
            self.last_poll = Some(now);
        }
        let mut due = std::mem::take(&mut self.requests);
        if !self.network_available {
            return SyncRequests::default();
        }
        if due.every_account() {
            due.at_launch = std::mem::take(&mut self.launch_sync_due);
        }
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(time: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(&format!("2026-10-03T{time}+02:00")).unwrap()
    }

    /// Ticket 130: the sync of every Account at launch looks for
    /// permissions granted again, also when it waits for the network.
    #[test]
    fn the_launch_sync_looks_for_grants_once_it_runs() {
        let account = AccountId(2);
        let mut scheduler = SyncScheduler::default();
        scheduler.set_network_available(false);
        assert!(scheduler.take_due(at("12:00:00")).is_empty());

        scheduler.set_network_available(true);
        assert!(scheduler.take_due(at("12:00:10")).looks_for_grants(account));

        let poll = scheduler.take_due(at("12:05:10"));
        assert!(poll.every_account() && !poll.looks_for_grants(account));
        scheduler.ask_account(account, StartedBy::User);
        assert!(scheduler.take_due(at("12:05:20")).looks_for_grants(account));
    }
}
