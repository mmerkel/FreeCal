//! The Sync Scheduler (new term): decides when FreeCal syncs (spec: "Sync
//! timing"). It polls every server Account every 2 minutes while the
//! window is open and every 15 minutes in the background, and syncs on
//! window focus, on waking from sleep, after local edits, when the network
//! returns, and on sync now. With no network at all it syncs nothing.
//!
//! It only decides; [`crate::Core::run_scheduled_syncs`] runs the syncs it
//! finds due, on the one thread the shell keeps for them.

use std::collections::BTreeMap;

use chrono::{DateTime, FixedOffset, TimeDelta};

use crate::{AccountId, StartedBy};

/// How often every server Account is polled while the window is open
/// (US-64).
const POLL_WITH_WINDOW: TimeDelta = TimeDelta::minutes(2);

/// How often every server Account is polled while FreeCal runs in the
/// background (US-65).
const POLL_IN_BACKGROUND: TimeDelta = TimeDelta::minutes(15);

/// Something besides a poll, a local edit or sync now that makes every
/// server Account sync at once, as the shell notices it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncTrigger {
    WindowFocused,
    /// The computer woke from sleep.
    Woke,
}

/// The syncs asked for and not started yet: of every server Account, of
/// some, or both, each with who asked. The user wins over FreeCal.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct SyncRequests {
    every_account: Option<StartedBy>,
    accounts: BTreeMap<AccountId, StartedBy>,
}

impl SyncRequests {
    pub fn is_empty(&self) -> bool {
        self.every_account.is_none() && self.accounts.is_empty()
    }

    /// Whether every server Account is to sync, not only those named.
    pub fn every_account(&self) -> bool {
        self.every_account.is_some()
    }

    /// The Accounts named on their own, which [`SyncRequests::every_account`]
    /// may add to.
    pub fn accounts(&self) -> impl Iterator<Item = AccountId> + '_ {
        self.accounts.keys().copied()
    }

    /// Who the sync of the Account `id` counts as started by.
    pub fn started_by(&self, id: AccountId) -> StartedBy {
        let user = |asked: Option<&StartedBy>| asked == Some(&StartedBy::User);
        if user(self.every_account.as_ref()) || user(self.accounts.get(&id)) {
            StartedBy::User
        } else {
            StartedBy::FreeCal
        }
    }

    fn ask_every_account(&mut self, by: StartedBy) {
        self.every_account = Some(stronger(self.every_account, by));
    }

    fn ask_account(&mut self, id: AccountId, by: StartedBy) {
        let asked = self.accounts.get(&id).copied();
        self.accounts.insert(id, stronger(asked, by));
    }
}

fn stronger(asked: Option<StartedBy>, by: StartedBy) -> StartedBy {
    if asked == Some(StartedBy::User) {
        StartedBy::User
    } else {
        by
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

    pub fn ask_every_account(&mut self, by: StartedBy) {
        self.requests.ask_every_account(by);
    }

    pub fn ask_account(&mut self, id: AccountId, by: StartedBy) {
        self.requests.ask_account(id, by);
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
        let due = std::mem::take(&mut self.requests);
        if self.network_available {
            due
        } else {
            SyncRequests::default()
        }
    }
}
