//! Sync Status (spec: "Sync engine" and "Overall status"): the current
//! sync state of each server Account and Calendar, which "Sync status
//! changed" carries, and the overall status that the window's status
//! indicator and the tray show. The Local Account and its Calendars have
//! none.
//!
//! Only the Running Syncs live in memory, as [`RunningSyncs`]. How each
//! one's last sync ended is its [`SyncRecord`], which only the Local Store
//! keeps, so that a relaunch shows it until the next sync, and removing an
//! Account or Calendar removes it. The Sync Records are read, the Unsent
//! Changes counted and whether a Calendar's First Sync is unfinished read
//! in one Local Store read whenever a [`SyncStatusReport`] is made.
//!
//! [`SyncStatusKeeper`] is the only code that touches the Running Syncs,
//! writes a Sync Record or sends "Sync status changed".

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use chrono::{DateTime, FixedOffset, Utc};
use serde::{Deserialize, Serialize};

use crate::store::{ServerCalendarState, Store};
use crate::{
    AccountId, Calendar, CalendarId, Clock, CoreError, ErrorCode, Result, Signal, SignalSink,
};

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
    /// window shows it at once.
    #[serde(rename_all = "camelCase")]
    Syncing {
        by_user: bool,
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
    /// The Sync Outcome as the Local Store keeps it, as JSON.
    pub(crate) fn stored(&self) -> String {
        serde_json::to_string(self).expect("a Sync Outcome always serializes")
    }

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
}

/// The last sync of an Account or Calendar that finished.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct FinishedSync {
    /// When it finished: an instant, in RFC 3339 UTC, as the overall
    /// status's last sync.
    #[cfg_attr(test, ts(type = "string"))]
    pub at: DateTime<Utc>,
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
    /// Whether its First Sync is unfinished, whatever its Sync Status: its
    /// Events may be missing. The Local Store says so, not the running
    /// syncs, so that it holds after a relaunch too.
    pub first_sync_unfinished: bool,
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
    /// syncs: all of them are synced at least up to then. None while one
    /// of them hasn't synced since FreeCal started. An instant, in RFC 3339
    /// UTC, not a wall time, so that the time since it doesn't jump across
    /// a DST change.
    #[cfg_attr(test, ts(type = "string | null"))]
    pub last_sync: Option<DateTime<Utc>>,
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

/// The running syncs of one server Account or Calendar: how many run, and
/// whether the user started one of them.
#[derive(Debug, Clone, Copy, Default)]
struct Running {
    syncs: u32,
    by_user: bool,
}

/// Every Running Sync: all the sync state that lives in memory. Only
/// [`SyncStatusKeeper`] locks it.
#[derive(Debug, Default)]
pub(crate) struct RunningSyncs(Mutex<RunningSyncCounts>);

/// The running syncs by the server Account or Calendar they sync. One with
/// no entry runs no sync.
#[derive(Debug, Default)]
struct RunningSyncCounts {
    accounts: HashMap<AccountId, Running>,
    calendars: HashMap<CalendarId, Running>,
}

/// What a Running Sync syncs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Syncing {
    Account(AccountId),
    Calendar(CalendarId),
}

impl RunningSyncCounts {
    fn of(&mut self, syncing: Syncing) -> &mut Running {
        match syncing {
            Syncing::Account(id) => self.accounts.entry(id).or_default(),
            Syncing::Calendar(id) => self.calendars.entry(id).or_default(),
        }
    }

    fn start(&mut self, syncing: Syncing, started_by: StartedBy) {
        let running = self.of(syncing);
        running.syncs += 1;
        running.by_user |= started_by == StartedBy::User;
    }

    /// Ends one of the running syncs of `syncing`, and forgets it once none
    /// runs.
    fn end(&mut self, syncing: Syncing) {
        let running = self.of(syncing);
        running.syncs = running.syncs.saturating_sub(1);
        if running.syncs == 0 {
            match syncing {
                Syncing::Account(id) => self.accounts.remove(&id),
                Syncing::Calendar(id) => self.calendars.remove(&id),
            };
        }
    }

    fn account(&self, id: AccountId) -> Running {
        self.accounts.get(&id).copied().unwrap_or_default()
    }

    fn calendar(&self, id: CalendarId) -> Running {
        self.calendars.get(&id).copied().unwrap_or_default()
    }
}

/// One server Account's or Calendar's sync state, without its Unsent
/// Changes: its running syncs and its Sync Record.
struct SyncState<'a> {
    running: Running,
    record: &'a SyncRecord,
}

impl SyncState<'_> {
    /// Whether it is Offline, or still taken as Offline while FreeCal
    /// tries it again by itself, so that the overall status doesn't flicker
    /// to Syncing on each try.
    fn offline(&self) -> bool {
        let last_offline = matches!(self.record.last, Some((_, SyncOutcome::Offline)));
        last_offline && (self.running.syncs == 0 || !self.running.by_user)
    }

    fn status(&self) -> SyncStatus {
        if self.running.syncs > 0 {
            return SyncStatus::Syncing {
                by_user: self.running.by_user,
            };
        }
        self.record
            .last
            .as_ref()
            .map_or(SyncStatus::Synced, |(_, outcome)| outcome.clone().into())
    }

    fn report(&self, unsent_changes: u32) -> SyncStatusReport {
        SyncStatusReport {
            status: self.status(),
            last_sync: self.record.last.as_ref().map(|(at, outcome)| FinishedSync {
                at: at.with_timezone(&Utc),
                outcome: outcome.clone(),
            }),
            unsent_changes,
        }
    }
}

/// The Sync Status module (new term: Sync Status Keeper): the only code
/// that touches the Running Syncs, writes a Sync Record or sends "Sync
/// status changed". The core makes one on demand. It takes the Store lock
/// and then its own, always in that order, and sends Signals only once
/// both are released, so that a subscriber can read straight away.
#[derive(Clone, Copy)]
pub(crate) struct SyncStatusKeeper<'a> {
    pub store: &'a Mutex<Store>,
    pub running: &'a RunningSyncs,
    pub clock: &'a dyn Clock,
    pub signals: &'a dyn SignalSink,
}

impl<'a> SyncStatusKeeper<'a> {
    /// Locks the Local Store, then the Running Syncs.
    fn lock(&self) -> (MutexGuard<'a, Store>, MutexGuard<'a, RunningSyncCounts>) {
        let store = self.store.lock().unwrap();
        (store, self.running.0.lock().unwrap())
    }

    /// Starts a sync of the server Account `id`, started by `started_by`,
    /// and sends its Sync Status.
    pub fn start_account(&self, id: AccountId, started_by: StartedBy) -> RunningSync<'a> {
        self.lock().1.start(Syncing::Account(id), started_by);
        self.send(&[id], &[]);
        self.running_sync(Syncing::Account(id))
    }

    /// Starts a sync of the server Calendar `id`, started by `started_by`,
    /// and sends its Sync Status; none while one of it runs already.
    pub fn start_calendar(&self, id: CalendarId, started_by: StartedBy) -> Option<RunningSync<'a>> {
        {
            let (_store, mut running) = self.lock();
            if running.calendar(id).syncs > 0 {
                return None;
            }
            running.start(Syncing::Calendar(id), started_by);
        }
        self.send(&[], &[id]);
        Some(self.running_sync(Syncing::Calendar(id)))
    }

    /// Adds Calendars to the server Account `account` through `add`, and
    /// starts the syncs of the Account and of each Calendar added as the
    /// user's, under the same Store lock, so that they say Syncing from when
    /// they are added. Sends "Calendars changed", then their Sync Statuses.
    /// Returns the Calendars added, with the Account's Running Sync and
    /// theirs.
    pub fn add_calendars(
        &self,
        account: AccountId,
        add: impl FnOnce(&Store) -> Result<Vec<Calendar>>,
    ) -> Result<AddedCalendars<'a>> {
        let calendars = {
            let (store, mut running) = self.lock();
            let calendars = add(&store)?;
            running.start(Syncing::Account(account), StartedBy::User);
            for calendar in &calendars {
                running.start(Syncing::Calendar(calendar.id), StartedBy::User);
            }
            calendars
        };
        self.signals.send(Signal::CalendarsChanged);
        let ids: Vec<CalendarId> = calendars.iter().map(|calendar| calendar.id).collect();
        self.send(&[account], &ids);
        Ok(AddedCalendars {
            account_sync: self.running_sync(Syncing::Account(account)),
            calendar_syncs: ids
                .into_iter()
                .map(|id| self.running_sync(Syncing::Calendar(id)))
                .collect(),
            calendars,
        })
    }

    fn running_sync(&self, syncing: Syncing) -> RunningSync<'a> {
        RunningSync {
            keeper: *self,
            syncing,
            ended: false,
        }
    }

    /// Makes every server Account that runs no sync Offline, as if its sync
    /// had just found the network gone, keeps its Sync Record and sends its
    /// Sync Status.
    pub fn mark_offline_without_trying(&self) {
        let marked = (|| {
            let (store, running) = self.lock();
            let now = self.clock.now();
            let marked: Vec<AccountId> = store
                .account_sync_records()?
                .into_iter()
                .map(|(id, _)| id)
                .filter(|id| running.account(*id).syncs == 0)
                .collect();
            store.write(|database_transaction| {
                marked.iter().try_for_each(|id| {
                    database_transaction.finish_account_sync(*id, now, &SyncOutcome::Offline)
                })
            })?;
            Ok::<_, CoreError>(marked)
        })();
        match marked {
            Ok(marked) => self.send(&marked, &[]),
            Err(error) => log::warn!("Couldn't mark the Accounts Offline: {error}"),
        }
    }

    /// Sends the Sync Status of those of `calendars` that are a server
    /// Account's, and of their Accounts, after a local change, as their
    /// Unsent Changes may have changed. Returns those Calendars whose
    /// Account isn't Offline, with their Accounts, to send the change at
    /// once; none should the Local Store fail.
    pub fn after_local_change(&self, calendars: &[CalendarId]) -> Vec<(AccountId, CalendarId)> {
        let Some(current) = self.read_to_send() else {
            return Vec::new();
        };
        let changed: Vec<&ServerCalendarState> = current
            .calendars
            .iter()
            .map(|(calendar, _)| calendar)
            .filter(|calendar| calendars.contains(&calendar.calendar_id))
            .collect();
        let accounts: Vec<AccountId> = changed.iter().map(|calendar| calendar.account_id).collect();
        self.send_signals(current.signals(&accounts, calendars));
        let offline: Vec<AccountId> = current
            .account_states()
            .filter(|(_, state)| state.offline())
            .map(|(id, _)| id)
            .collect();
        changed
            .into_iter()
            .filter(|calendar| !offline.contains(&calendar.account_id))
            .map(|calendar| (calendar.account_id, calendar.calendar_id))
            .collect()
    }

    /// The Sync Status of every server Account and Calendar, with the
    /// overall status.
    pub fn sync_statuses(&self) -> Result<SyncStatuses> {
        self.read().map(|current| current.sync_statuses())
    }

    /// The Sync Records and Server Calendar States, read together with the
    /// Running Syncs.
    fn read(&self) -> Result<Current> {
        let (store, running) = self.lock();
        Ok(Current {
            accounts: store
                .account_sync_records()?
                .into_iter()
                .map(|(id, record)| (id, running.account(id), record))
                .collect(),
            calendars: store
                .server_calendar_states()?
                .into_iter()
                .map(|calendar| {
                    let calendar_running = running.calendar(calendar.calendar_id);
                    (calendar, calendar_running)
                })
                .collect(),
        })
    }

    /// [`Self::read`] for sending Signals: should the Local Store fail, logs
    /// why and gives none.
    fn read_to_send(&self) -> Option<Current> {
        self.read()
            .inspect_err(|error| log::warn!("Couldn't read the Sync Status to send: {error}"))
            .ok()
    }

    /// Sends "Sync status changed" for the server Accounts `accounts` and
    /// Calendars `calendars` as they are now. Those not in the Local Store,
    /// as the Local Account's and those removed meanwhile, are left out.
    /// Should the Local Store fail, none is sent; the next one says it all
    /// again.
    fn send(&self, accounts: &[AccountId], calendars: &[CalendarId]) {
        if let Some(current) = self.read_to_send() {
            self.send_signals(current.signals(accounts, calendars));
        }
    }

    /// Sends "Sync status changed" for what a Running Sync syncs.
    fn send_about(&self, syncing: Syncing) {
        match syncing {
            Syncing::Account(id) => self.send(&[id], &[]),
            Syncing::Calendar(id) => self.send(&[], &[id]),
        }
    }

    fn send_signals(&self, signals: Vec<Signal>) {
        signals
            .into_iter()
            .for_each(|signal| self.signals.send(signal));
    }
}

/// Calendars just added with [`SyncStatusKeeper::add_calendars`], and the
/// Running Syncs of their First Syncs and of their Account's sync.
pub(crate) struct AddedCalendars<'a> {
    pub calendars: Vec<Calendar>,
    pub account_sync: RunningSync<'a>,
    pub calendar_syncs: Vec<RunningSync<'a>>,
}

/// The server Accounts and Calendars with their Sync Records and running
/// syncs, as read together.
struct Current {
    accounts: Vec<(AccountId, Running, SyncRecord)>,
    calendars: Vec<(ServerCalendarState, Running)>,
}

impl Current {
    fn account_states(&self) -> impl Iterator<Item = (AccountId, SyncState<'_>)> {
        self.accounts.iter().map(|(id, running, record)| {
            (
                *id,
                SyncState {
                    running: *running,
                    record,
                },
            )
        })
    }

    fn calendar_states(&self) -> impl Iterator<Item = (&ServerCalendarState, SyncState<'_>)> {
        self.calendars.iter().map(|(calendar, running)| {
            (
                calendar,
                SyncState {
                    running: *running,
                    record: &calendar.record,
                },
            )
        })
    }

    fn account_status(&self, id: AccountId, state: &SyncState) -> AccountSyncStatus {
        let unsent_changes = self
            .calendars
            .iter()
            .filter(|(calendar, _)| calendar.account_id == id)
            .map(|(calendar, _)| calendar.unsent_changes)
            .sum();
        AccountSyncStatus {
            account_id: id,
            report: state.report(unsent_changes),
        }
    }

    fn calendar_status(calendar: &ServerCalendarState, state: &SyncState) -> CalendarSyncStatus {
        CalendarSyncStatus {
            calendar_id: calendar.calendar_id,
            report: state.report(calendar.unsent_changes),
            first_sync_unfinished: calendar.first_sync_unfinished,
        }
    }

    /// "Sync status changed" for those of `accounts` and `calendars` read:
    /// the Calendars' first, each in the order they were added.
    fn signals(&self, accounts: &[AccountId], calendars: &[CalendarId]) -> Vec<Signal> {
        let calendar_signals = self
            .calendar_states()
            .filter(|(calendar, _)| calendars.contains(&calendar.calendar_id))
            .map(|(calendar, state)| {
                Signal::CalendarSyncStatusChanged(Self::calendar_status(calendar, &state))
            });
        let account_signals = self
            .account_states()
            .filter(|(id, _)| accounts.contains(id))
            .map(|(id, state)| Signal::AccountSyncStatusChanged(self.account_status(id, &state)));
        calendar_signals.chain(account_signals).collect()
    }

    fn sync_statuses(&self) -> SyncStatuses {
        SyncStatuses {
            overall: self.overall(),
            accounts: self
                .account_states()
                .map(|(id, state)| self.account_status(id, &state))
                .collect(),
            calendars: self
                .calendar_states()
                .map(|(calendar, state)| Self::calendar_status(calendar, &state))
                .collect(),
        }
    }

    /// The overall status of every server Account and Calendar.
    fn overall(&self) -> OverallStatus {
        let account_states: Vec<SyncState> =
            self.account_states().map(|(_, state)| state).collect();
        let all_states = || {
            self.account_states()
                .map(|(_, state)| state)
                .chain(self.calendar_states().map(|(_, state)| state))
        };
        let statuses: Vec<SyncStatus> = all_states().map(|state| state.status()).collect();
        let syncing_by = |by_user: bool| {
            statuses.iter().any(|status| matches!(status, SyncStatus::Syncing { by_user: user } if *user == by_user))
        };
        let state = if statuses
            .iter()
            .any(|status| matches!(status, SyncStatus::SyncError { .. }))
        {
            OverallState::SyncError
        } else if !account_states.is_empty() && account_states.iter().all(SyncState::offline) {
            OverallState::Offline
        } else if syncing_by(true) {
            OverallState::Syncing { by_user: true }
        } else if syncing_by(false) {
            OverallState::Syncing { by_user: false }
        } else {
            OverallState::Synced
        };
        let last_sync = all_states()
            .map(|state| state.record.last_synced)
            .collect::<Option<Vec<_>>>()
            .and_then(|times| times.into_iter().min())
            .map(|at| at.with_timezone(&Utc));
        OverallStatus { state, last_sync }
    }
}

/// A Running Sync: one sync of a server Account or Calendar while it runs,
/// which says Syncing from its start. [`RunningSync::end`] gives its Sync
/// Status how it ended; dropped without that, it ends as before the sync.
#[must_use = "a Running Sync dropped at once ends as before the sync"]
pub(crate) struct RunningSync<'a> {
    keeper: SyncStatusKeeper<'a>,
    syncing: Syncing,
    ended: bool,
}

impl RunningSync<'_> {
    /// Ends the sync with its `result`, keeps the Sync Record and sends the
    /// new Sync Status.
    ///
    /// A failure ends it with a Sync Error, or Offline when its server
    /// couldn't be reached at the network level, and is returned. For a
    /// Calendar, a failure about the whole Account (see [`about_account`])
    /// leaves its Sync Status as before the sync, and is returned for the
    /// Account's; its removal meanwhile is no failure and gives none.
    /// Failing to keep the Sync Record of a sync that succeeded is
    /// returned. One removed meanwhile keeps nothing and sends nothing.
    pub fn end<T>(mut self, result: Result<T>) -> Result<Option<T>> {
        self.ended = true;
        let syncing = self.syncing;
        let (outcome, result) = match (syncing, result) {
            (Syncing::Calendar(id), Err(CoreError::CalendarNotFound(removed))) if removed == id => {
                (None, Ok(None))
            }
            (Syncing::Calendar(_), Err(error)) if about_account(&error) => (None, Err(error)),
            (_, Err(error)) => (Some(SyncOutcome::of_failure(&error)), Err(error)),
            (_, Ok(value)) => (Some(SyncOutcome::Synced), Ok(Some(value))),
        };
        let kept = self.finish(outcome);
        self.keeper.send_about(syncing);
        result.and_then(|value| kept.map(|()| value))
    }

    /// Ends one of the running syncs, with `outcome`, or as before the sync
    /// with none, and keeps the Sync Record, in one hold of the locks.
    fn finish(&self, outcome: Option<SyncOutcome>) -> Result<()> {
        let (store, mut running) = self.keeper.lock();
        running.end(self.syncing);
        let Some(outcome) = outcome else {
            return Ok(());
        };
        let now = self.keeper.clock.now();
        store.write(|database_transaction| match self.syncing {
            Syncing::Account(id) => database_transaction.finish_account_sync(id, now, &outcome),
            Syncing::Calendar(id) => database_transaction.finish_calendar_sync(id, now, &outcome),
        })
    }
}

impl Drop for RunningSync<'_> {
    /// Ends as before the sync, as for a Calendar a sync didn't reach.
    fn drop(&mut self) {
        // The locks may be poisoned while a panic unwinds.
        if self.ended || std::thread::panicking() {
            return;
        }
        if let Err(error) = self.finish(None) {
            log::warn!("Couldn't end a sync: {error}");
        }
        self.keeper.send_about(self.syncing);
    }
}
