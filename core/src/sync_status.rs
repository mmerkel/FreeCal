//! Sync Status (spec: "Sync engine" and "Overall status"): the current
//! sync state of each server Account and Calendar, which "Sync status
//! changed" carries, and the overall status that the window's status
//! indicator and the tray show. The Local Account and its Calendars have
//! none.
//!
//! The running syncs live in memory, as [`SyncStatusTracker`]. How each
//! one's last sync ended is its [`SyncRecord`], which the Local Store keeps
//! too, so that a relaunch shows it until the next sync. Unsent Changes are
//! counted from the Local Store whenever a [`SyncStatusReport`] is made.

use std::collections::HashMap;

use chrono::{DateTime, FixedOffset, NaiveDateTime};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::{AccountId, CalendarId, CoreError, ErrorCode};

/// Whether the user started a sync (sync now, adding Calendars), so that
/// its Syncing shows at once, or FreeCal did (polls and the other
/// triggers), so that it shows only once it has run for a while (spec:
/// "Overall status"). The user ranks above FreeCal, so that the
/// stronger of two is their `max`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StartedBy {
    FreeCal,
    User,
}

/// The Sync Status of one server Account or Calendar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SyncStatus {
    /// A sync runs. `by_user` says that the user started it, so that the
    /// window shows it at once. `first_sync` says that it is a Calendar's
    /// First Sync, whose Events are still loading; never for an Account.
    #[serde(rename_all = "camelCase")]
    Syncing {
        by_user: bool,
        first_sync: bool,
    },
    Synced,
    /// Its server can't be reached at the network level. Only an Account
    /// is Offline: its Calendars keep their own Sync Status meanwhile.
    Offline,
    /// A problem stopped it from syncing; `error` says which, for the
    /// frontend to put into words.
    SyncError {
        error: ErrorCode,
    },
}

/// How a sync ended. The Local Store keeps it as its JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SyncOutcome {
    Synced,
    Offline,
    SyncError { error: ErrorCode },
}

impl From<SyncOutcome> for SyncStatus {
    fn from(outcome: SyncOutcome) -> Self {
        match outcome {
            SyncOutcome::Synced => SyncStatus::Synced,
            SyncOutcome::Offline => SyncStatus::Offline,
            SyncOutcome::SyncError { error } => SyncStatus::SyncError { error },
        }
    }
}

impl SyncOutcome {
    /// Reads a stored Sync Outcome, untrusted: none when it can't be read,
    /// and a Sync Error's text made plain again (see [`ErrorCode::checked`]).
    fn from_stored(json: &str) -> Option<Self> {
        match serde_json::from_str(json).ok()? {
            SyncOutcome::SyncError { error } => Some(SyncOutcome::SyncError {
                error: error.checked()?,
            }),
            outcome => Some(outcome),
        }
    }

    /// How a sync that failed with `error` ended: Offline when its server
    /// couldn't be reached at the network level, a Sync Error otherwise.
    pub(crate) fn of_failure(error: &CoreError) -> Self {
        match error {
            CoreError::ServerUnreachable(_) => SyncOutcome::Offline,
            error => SyncOutcome::SyncError {
                error: error.code(),
            },
        }
    }
}

/// How the last sync of a server Account or Calendar ended, and when its
/// last successful one finished (new term: Sync Record). The Local Store
/// keeps it, so that a relaunch shows it until the next sync.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct SyncRecord {
    pub last: Option<(DateTime<FixedOffset>, SyncOutcome)>,
    pub last_synced: Option<DateTime<FixedOffset>>,
}

impl SyncRecord {
    /// A Sync Record as the Local Store keeps it, untrusted: a time in Unix
    /// seconds, or an outcome, that can't be read leaves its part out.
    pub fn from_stored(
        last_at: Option<i64>,
        last_outcome: Option<&str>,
        last_synced_at: Option<i64>,
    ) -> Self {
        let time = |seconds: Option<i64>| {
            DateTime::from_timestamp(seconds?, 0).map(|at| at.fixed_offset())
        };
        SyncRecord {
            last: time(last_at).zip(last_outcome.and_then(SyncOutcome::from_stored)),
            last_synced: time(last_synced_at),
        }
    }

    /// The Sync Record as the Local Store keeps it: when the last sync
    /// finished, its Sync Outcome as JSON, and when the last successful one
    /// finished.
    pub fn stored(&self) -> (Option<i64>, Option<String>, Option<i64>) {
        let (at, outcome) = self
            .last
            .as_ref()
            .map(|(at, outcome)| {
                let json =
                    serde_json::to_string(outcome).expect("a Sync Outcome always serializes");
                (at.timestamp(), json)
            })
            .unzip();
        (at, outcome, self.last_synced.map(|at| at.timestamp()))
    }

    fn finish(&mut self, at: DateTime<FixedOffset>, outcome: SyncOutcome) {
        if outcome == SyncOutcome::Synced {
            self.last_synced = Some(at);
        }
        self.last = Some((at, outcome));
    }
}

/// The last sync of an Account or Calendar that finished.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct FinishedSync {
    /// When it finished, in the Display Time Zone.
    #[cfg_attr(test, ts(type = "string"))]
    pub at: NaiveDateTime,
    pub outcome: SyncOutcome,
}

/// What "Sync status changed" says about one server Account or Calendar
/// (new term: Sync Status Report).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SyncStatusReport {
    pub status: SyncStatus,
    /// None until a sync of it has finished.
    pub last_sync: Option<FinishedSync>,
    /// Its Unsent Changes; an Account's are those of all its Calendars.
    pub unsent_changes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AccountSyncStatus {
    pub account_id: AccountId,
    pub report: SyncStatusReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CalendarSyncStatus {
    pub calendar_id: CalendarId,
    pub report: SyncStatusReport,
}

/// The most important state of all server Accounts and Calendars, as the
/// status indicator and the tray show it (spec: "Overall status").
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum OverallState {
    /// An Event is in Conflict. Conflicts come with ticket 20.
    Conflict,
    /// An Account or Calendar has a Sync Error.
    SyncError,
    /// There is a server Account, and every server Account is Offline.
    Offline,
    /// A sync runs; `by_user` when the user started one of those running.
    #[serde(rename_all = "camelCase")]
    Syncing { by_user: bool },
    /// Online and synced, also with no server Account at all.
    Synced,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct OverallStatus {
    pub state: OverallState,
    /// The oldest of the server Accounts' and Calendars' last successful
    /// syncs, in the Display Time Zone: all of them are synced at least up
    /// to then. None while one of them hasn't synced since FreeCal started.
    #[cfg_attr(test, ts(type = "string | null"))]
    pub last_sync: Option<NaiveDateTime>,
}

/// The Sync Status of every server Account and Calendar, with the overall
/// status, as the window reads them on starting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SyncStatuses {
    pub overall: OverallStatus,
    pub accounts: Vec<AccountSyncStatus>,
    pub calendars: Vec<CalendarSyncStatus>,
}

/// Whether a failure is about the whole Account, rather than the Calendar
/// whose sync it stopped: being offline, a TLS failure, or a login the
/// server or the keyring doesn't give (spec: Account-level states). Every
/// other failure is the Calendar's.
pub(crate) fn about_account(error: &CoreError) -> bool {
    matches!(
        error,
        CoreError::ServerUnreachable(_)
            | CoreError::SecureConnectionFailed(_)
            | CoreError::WrongCredentials
            | CoreError::KeyringUnavailable(_)
            | CoreError::PasswordNotInKeyring
            | CoreError::InvalidServerUrl
    )
}

/// One server Account's or Calendar's sync state, without its Unsent
/// Changes.
#[derive(Debug, Clone, Default)]
struct SyncState {
    /// How many syncs of it run, and whether the user started one of them.
    running: u32,
    by_user: bool,
    /// Whether the running sync is a Calendar's First Sync.
    first_sync: bool,
    record: SyncRecord,
}

impl SyncState {
    fn start(&mut self, started_by: StartedBy, first_sync: bool) {
        self.running += 1;
        self.by_user |= started_by == StartedBy::User;
        self.first_sync = first_sync;
    }

    /// Ends a running sync, with `last` when it finished, none when it
    /// didn't get to.
    fn end(&mut self, last: Option<(DateTime<FixedOffset>, SyncOutcome)>) {
        self.running = self.running.saturating_sub(1);
        if self.running == 0 {
            self.by_user = false;
            self.first_sync = false;
        }
        if let Some((at, outcome)) = last {
            self.record.finish(at, outcome);
        }
    }

    /// Whether it is Offline, or still taken as Offline while FreeCal
    /// tries it again by itself, so that the overall status doesn't flicker
    /// to Syncing on each try.
    fn offline(&self) -> bool {
        let last_offline = matches!(self.record.last, Some((_, SyncOutcome::Offline)));
        last_offline && (self.running == 0 || !self.by_user)
    }

    fn status(&self) -> SyncStatus {
        if self.running > 0 {
            return SyncStatus::Syncing {
                by_user: self.by_user,
                first_sync: self.first_sync,
            };
        }
        self.record
            .last
            .as_ref()
            .map_or(SyncStatus::Synced, |(_, outcome)| outcome.clone().into())
    }

    fn report(&self, unsent_changes: u32, zone: Tz) -> SyncStatusReport {
        SyncStatusReport {
            status: self.status(),
            last_sync: self.record.last.as_ref().map(|(at, outcome)| FinishedSync {
                at: in_zone(*at, zone),
                outcome: outcome.clone(),
            }),
            unsent_changes,
        }
    }
}

/// The state of one that has never synced.
const NEVER_SYNCED: SyncState = SyncState {
    running: 0,
    by_user: false,
    first_sync: false,
    record: SyncRecord {
        last: None,
        last_synced: None,
    },
};

fn in_zone(at: DateTime<FixedOffset>, zone: Tz) -> NaiveDateTime {
    at.with_timezone(&zone).naive_local()
}

/// The sync state of every server Account and Calendar: the running syncs,
/// and the Sync Records, loaded from the Local Store on launch. One with no
/// entry has never synced: it is Synced.
#[derive(Debug, Default)]
pub(crate) struct SyncStatusTracker {
    accounts: HashMap<AccountId, SyncState>,
    calendars: HashMap<CalendarId, SyncState>,
}

impl SyncStatusTracker {
    /// The Sync Records the Local Store keeps, on launch.
    pub fn load(
        accounts: Vec<(AccountId, SyncRecord)>,
        calendars: Vec<(CalendarId, SyncRecord)>,
    ) -> Self {
        let state = |record| SyncState {
            record,
            ..SyncState::default()
        };
        SyncStatusTracker {
            accounts: accounts.into_iter().map(|(id, r)| (id, state(r))).collect(),
            calendars: calendars
                .into_iter()
                .map(|(id, r)| (id, state(r)))
                .collect(),
        }
    }

    pub fn start_account(&mut self, id: AccountId, started_by: StartedBy) {
        self.accounts
            .entry(id)
            .or_default()
            .start(started_by, false);
    }

    /// Ends the Account's running sync. Returns its new Sync Record, for
    /// the Local Store; none for an Account removed meanwhile.
    pub fn end_account(
        &mut self,
        id: AccountId,
        at: DateTime<FixedOffset>,
        outcome: SyncOutcome,
    ) -> Option<SyncRecord> {
        let state = self.accounts.get_mut(&id)?;
        state.end(Some((at, outcome)));
        Some(state.record.clone())
    }

    /// Makes the Account Offline at `at` without a sync, as when the
    /// network went away. Returns its new Sync Record, for the Local Store;
    /// none while it syncs, which finds out by itself.
    pub fn account_offline(
        &mut self,
        id: AccountId,
        at: DateTime<FixedOffset>,
    ) -> Option<SyncRecord> {
        let state = self.accounts.entry(id).or_default();
        if state.running > 0 {
            return None;
        }
        state.record.finish(at, SyncOutcome::Offline);
        Some(state.record.clone())
    }

    /// Whether a sync of the Calendar runs, which another one leaves out.
    pub fn calendar_syncing(&self, id: CalendarId) -> bool {
        self.calendars
            .get(&id)
            .is_some_and(|state| state.running > 0)
    }

    pub fn start_calendar(&mut self, id: CalendarId, started_by: StartedBy, first_sync: bool) {
        self.calendars
            .entry(id)
            .or_default()
            .start(started_by, first_sync);
    }

    /// Ends a Calendar's running sync: with how it finished, or none when it
    /// didn't get to, as when its Account went Offline. Returns its new
    /// Sync Record when it finished, for the Local Store. A Calendar removed
    /// meanwhile stays forgotten.
    pub fn end_calendar(
        &mut self,
        id: CalendarId,
        finished: Option<(DateTime<FixedOffset>, SyncOutcome)>,
    ) -> Option<SyncRecord> {
        let state = self.calendars.get_mut(&id)?;
        let ended = finished.is_some();
        state.end(finished);
        ended.then(|| state.record.clone())
    }

    /// Forgets a removed Calendar.
    pub fn forget_calendar(&mut self, id: CalendarId) {
        self.calendars.remove(&id);
    }

    /// Forgets a removed Account; its Calendars are forgotten one by one.
    pub fn forget_account(&mut self, id: AccountId) {
        self.accounts.remove(&id);
    }

    pub fn account(&self, id: AccountId, unsent_changes: u32, zone: Tz) -> AccountSyncStatus {
        AccountSyncStatus {
            account_id: id,
            report: state(&self.accounts, &id).report(unsent_changes, zone),
        }
    }

    pub fn calendar(&self, id: CalendarId, unsent_changes: u32, zone: Tz) -> CalendarSyncStatus {
        CalendarSyncStatus {
            calendar_id: id,
            report: state(&self.calendars, &id).report(unsent_changes, zone),
        }
    }

    /// The overall status of the server Accounts `accounts` and their
    /// Calendars `calendars`.
    pub fn overall(
        &self,
        accounts: &[AccountId],
        calendars: &[CalendarId],
        zone: Tz,
    ) -> OverallStatus {
        let account_states: Vec<&SyncState> = accounts
            .iter()
            .map(|id| state(&self.accounts, id))
            .collect();
        let account_statuses: Vec<SyncStatus> =
            account_states.iter().map(|state| state.status()).collect();
        let calendar_statuses: Vec<SyncStatus> = calendars
            .iter()
            .map(|id| state(&self.calendars, id).status())
            .collect();
        let all = || account_statuses.iter().chain(&calendar_statuses);
        let syncing_by = |by_user: bool| {
            all().any(|status| matches!(status, SyncStatus::Syncing { by_user: user, .. } if *user == by_user))
        };
        let state = if all().any(|status| matches!(status, SyncStatus::SyncError { .. })) {
            OverallState::SyncError
        } else if !account_states.is_empty() && account_states.iter().all(|state| state.offline()) {
            OverallState::Offline
        } else if syncing_by(true) {
            OverallState::Syncing { by_user: true }
        } else if syncing_by(false) {
            OverallState::Syncing { by_user: false }
        } else {
            OverallState::Synced
        };
        let last_synced = |state: Option<&SyncState>| state?.record.last_synced;
        let last_sync = accounts
            .iter()
            .map(|id| last_synced(self.accounts.get(id)))
            .chain(
                calendars
                    .iter()
                    .map(|id| last_synced(self.calendars.get(id))),
            )
            .collect::<Option<Vec<_>>>()
            .and_then(|times| times.into_iter().min())
            .map(|at| in_zone(at, zone));
        OverallStatus { state, last_sync }
    }
}

/// The sync state of `id`, which has never synced without an entry.
fn state<'a, K: std::hash::Hash + Eq>(states: &'a HashMap<K, SyncState>, id: &K) -> &'a SyncState {
    states.get(id).unwrap_or(&NEVER_SYNCED)
}
