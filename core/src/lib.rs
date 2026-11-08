//! FreeCal's core: all domain logic behind one application interface, [`Core`].
//!
//! The frontend talks only to [`Core`]. Besides answering requests, the core
//! pushes [`Signal`]s to a [`SignalSink`].

mod account;
mod caldav;
mod calendar;
mod clock;
mod error;
mod event;
mod guests;
mod import;
mod journal;
mod keyring;
mod remote;
mod settings;
mod sidebar;
mod signal;
mod store;
mod structured_text;
mod sync;
mod sync_scheduler;
mod sync_status;
#[cfg(test)]
mod typescript;
mod view;

use std::fs::{File, TryLockError};
use std::path::Path;

use chrono::{NaiveDate, Utc};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

pub use account::{Account, AccountId, AddedAccount, CalDavLogin, Provider, ServerLogin};
pub use calendar::{AvailableCalendar, Calendar, CalendarId, Colour};
pub use clock::{Clock, SystemClock};
pub use error::{CoreError, ErrorCode};
pub use event::{
    Event, EventDraft, EventId, ExceptionCounts, Frequency, Occurrence, Repeat, RepeatEnd, Scope,
    ServerObject, When,
};
pub use import::{
    ExistingEvents, ImportFile, ImportOutcome, ImportPreview, ImportSummary, ImportedEvent,
    ImportedWhen, MAX_IMPORT_ENTRIES, MAX_IMPORT_SIZE, PinnedZone,
};
pub use journal::{Changed, JournalTransactionId};
pub use keyring::{Keyring, KeyringError};
pub use remote::FIRST_SYNC_PAGE;
pub use settings::{Appearance, Settings, SettingsUpdate};
pub use sidebar::SidebarState;
pub use signal::{Signal, SignalSink};
pub use structured_text::{Piece, StructuredText, link_allowed};
pub use sync_scheduler::SyncTrigger;
pub use sync_status::{
    AccountSyncStatus, CalendarSyncStatus, FinishedSync, OverallState, OverallStatus, StartedBy,
    SyncOutcome, SyncStatus, SyncStatusReport, SyncStatuses,
};
pub use view::View;

use calendar::calendar_name;
use event::{Edit, Opened};
use journal::{JournalTransaction, TransactionOrigin};
use remote::Remote;
use store::{CalendarToSync, Store};
use sync_scheduler::SyncScheduler;
use sync_status::{RunningSync, RunningSyncs, SyncStatusKeeper};

pub type Result<T> = std::result::Result<T, CoreError>;

/// The core application interface.
pub struct Core {
    /// Held for the lifetime of `Core`, so that only one running FreeCal
    /// opens the Local Store (ADR 0005). The OS releases it when the process
    /// ends, even after a crash.
    _lock: File,
    store: Mutex<Store>,
    clock: Arc<dyn Clock>,
    signals: Arc<dyn SignalSink>,
    keyring: Arc<dyn Keyring>,
    /// The local date as [`Core::check_date`] last saw it.
    date: Mutex<NaiveDate>,
    /// The Running Syncs, which only the Sync Status Keeper locks (see
    /// [`Core::sync_status`]).
    running_syncs: RunningSyncs,
    /// When to sync. Never locked together with the Store lock.
    scheduler: Mutex<SyncScheduler>,
    /// Wakes [`Core::wait_for_sync_request`] when a sync is asked for.
    sync_requested: Condvar,
}

impl Core {
    /// Opens the Local Store in `data_dir`, creating it on first launch.
    /// Refuses with [`CoreError::LocalStoreInUse`] while another running
    /// FreeCal has it open.
    pub fn open(
        data_dir: &Path,
        clock: Arc<dyn Clock>,
        signals: Arc<dyn SignalSink>,
        keyring: Arc<dyn Keyring>,
    ) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let lock = lock_local_store(data_dir)?;
        let store = Store::open(&data_dir.join("freecal.sqlite3"))?;
        let date = Mutex::new(clock.now().date_naive());
        Ok(Self {
            _lock: lock,
            store: Mutex::new(store),
            clock,
            signals,
            keyring,
            date,
            running_syncs: RunningSyncs::default(),
            scheduler: Mutex::new(SyncScheduler::default()),
            sync_requested: Condvar::new(),
        })
    }

    /// The current local date, on which the views open.
    pub fn today(&self) -> NaiveDate {
        self.clock.now().date_naive()
    }

    /// Sends "Date changed" when the local date differs from the one last
    /// seen. The shell calls this every few seconds, which notices midnight
    /// and also waking up on a new day, when timers set for midnight would
    /// have slept through it.
    pub fn check_date(&self) {
        let today = self.today();
        let changed = {
            let mut seen = self.date.lock().unwrap();
            std::mem::replace(&mut *seen, today) != today
        };
        if changed {
            self.signals.send(Signal::DateChanged { date: today });
        }
    }

    /// The view FreeCal opens in: month on first launch, then the last-used one.
    pub fn last_used_view(&self) -> Result<View> {
        self.store().last_used_view()
    }

    pub fn set_last_used_view(&self, view: View) -> Result<()> {
        self.store()
            .write(|database_transaction| database_transaction.set_last_used_view(view))
    }

    /// The sidebar as it was left: on first launch shown at the default width.
    pub fn sidebar(&self) -> Result<SidebarState> {
        self.store().sidebar()
    }

    pub fn set_sidebar(&self, sidebar: SidebarState) -> Result<()> {
        self.store()
            .write(|database_transaction| database_transaction.set_sidebar(sidebar))
    }

    pub fn settings(&self) -> Result<Settings> {
        self.store().settings()
    }

    /// Changes the settings `update` gives and keeps the others. Returns the
    /// settings as they are now.
    pub fn update_settings(&self, update: SettingsUpdate) -> Result<Settings> {
        let store = self.store();
        store.write(|database_transaction| {
            if let Some(appearance) = update.appearance {
                database_transaction.set_appearance(appearance)?;
            }
            Ok(())
        })?;
        store.settings()
    }

    pub fn list_accounts(&self) -> Result<Vec<Account>> {
        self.store().list_accounts()
    }

    /// Adds a CalDAV Account once its server accepts the username and
    /// password, which go into the keyring, and returns it with the
    /// Calendars its server offers. Stores nothing when the server or the
    /// keyring fails.
    pub fn add_caldav_account(&self, login: &CalDavLogin) -> Result<AddedAccount> {
        let client = caldav::Client::new(login)?;
        let discovered = client.discover()?;
        let own_addresses =
            guests::caldav_own_addresses(&discovered.user_addresses, &login.username);
        let server = ServerLogin {
            url: client.url().to_string(),
            username: login.username.clone(),
        };
        let key = uuid::Uuid::new_v4().to_string();
        let label = format!(
            "FreeCal: {} on {}",
            login.username,
            client.url().host_str().unwrap_or_default()
        );
        self.keyring
            .store(&key, &label, &login.password)
            .map_err(CoreError::KeyringUnavailable)?;
        let added = self.store().write(|database_transaction| {
            database_transaction.insert_server_account(
                Provider::CalDav,
                &server,
                &key,
                &own_addresses,
            )
        });
        if added.is_err() {
            // Best effort: the Account failed already.
            let _ = self.keyring.delete(&key);
        }
        let account = added?;
        self.signals.send(Signal::CalendarsChanged);
        Ok(AddedAccount {
            account,
            available: discovered.calendars,
        })
    }

    /// The Available Calendars of a server Account, fetched from its server
    /// each time: those it offers that aren't added yet. None for the Local
    /// Account. Stores nothing.
    pub fn available_calendars(&self, id: AccountId) -> Result<Vec<AvailableCalendar>> {
        let mut offered = self.offered_calendars(id)?;
        let added = self.store().calendar_urls(id)?;
        offered.retain(|calendar| !added.contains(&calendar.url));
        Ok(offered)
    }

    /// Adds Available Calendars of a server Account, named by their `url`,
    /// as its Calendars, all or none. Their names and colours are the
    /// server's as it offers them now.
    ///
    /// Then their First Syncs pull their Events, as a sync of the Account
    /// that the user started and that syncs only them (see
    /// [`Core::sync_account`]): their Sync Status and the Account's say
    /// Syncing from when they are added, and each First Sync commits page
    /// by page. A failure ends their syncs and the Account's as in any sync
    /// of the Account, and is returned; the Calendars stay added, with the
    /// Events pulled so far, and the next sync runs the rest of their First
    /// Syncs.
    pub fn add_calendars(&self, account_id: AccountId, urls: &[String]) -> Result<Vec<Calendar>> {
        let remote = self.remote(account_id)?;
        let offered = offered_by(remote.as_deref())?;
        let add = |store: &Store| {
            // Added already, or earlier in `urls`.
            let mut taken = store.calendar_urls(account_id)?;
            store.write(|database_transaction| {
                urls.iter()
                    .map(|url| {
                        let available = offered.iter().find(|calendar| &calendar.url == url);
                        match available {
                            Some(available) if taken.insert(url.clone()) => {
                                database_transaction.insert_server_calendar(account_id, available)
                            }
                            _ => Err(CoreError::CalendarNotAvailable(url.clone())),
                        }
                    })
                    .collect::<Result<Vec<_>>>()
            })
        };
        let Some(remote) = remote else {
            let added = add(&self.store())?;
            self.signals.send(Signal::CalendarsChanged);
            return Ok(added);
        };
        let added = self.sync_status().add_calendars(account_id, add)?;
        let calendars = added
            .calendars
            .iter()
            .zip(urls)
            .zip(added.calendar_syncs)
            .map(|((calendar, url), sync)| ((calendar.id, url.clone(), None), Some(sync)))
            .collect();
        self.sync_calendars(
            added.account_sync,
            StartedBy::User,
            remote.as_ref(),
            calendars,
        )?;
        Ok(added.calendars)
    }

    /// Syncs a server Account on demand: each writable Calendar's Unsent
    /// Changes are sent first (see [`Core::sync_calendar`]), then each
    /// Calendar gets what changed on its server since its last sync,
    /// fetching only that where the server can tell (see
    /// [`Remote::changes`]). Each Calendar's
    /// Incoming Sync is one Journal Transaction with the Transaction Origin
    /// Incoming Sync, in Read-only Calendars too, and sends one "Occurrences
    /// changed" once committed; an Incoming Sync that changes nothing records
    /// no Journal Transaction. A Calendar whose First Sync is unfinished gets
    /// the rest of it, unrecorded like the rest: page by page while it holds
    /// nothing from its server yet, otherwise as one Incoming Sync. A
    /// Calendar syncing now, or removed meanwhile, is left out.
    ///
    /// The Account's Sync Status and each Calendar's say Syncing while they
    /// sync, started by `started_by`, and then how it ended, with "Sync status
    /// changed" each time. A failure about the whole Account (see
    /// [`sync_status::about_account`]) makes the Account Offline, when its
    /// server couldn't be reached at the network level, or gives it a Sync
    /// Error, and stops the sync: the Calendars not synced yet keep their
    /// Sync Status. Any other failure is a Sync Error of the Calendar whose
    /// sync it stopped, and the other Calendars still sync. The first error
    /// is returned. Returns each Calendar synced, with its Journal
    /// Transaction. None for the Local Account, which has no Sync Status.
    pub fn sync_account(
        &self,
        id: AccountId,
        started_by: StartedBy,
    ) -> Result<Vec<Changed<CalendarId>>> {
        let account = self.store().find_account(id)?;
        if account.ok_or(CoreError::AccountNotFound(id))?.provider == Provider::Local {
            return Ok(Vec::new());
        }
        let account_sync = self.sync_status().start_account(id, started_by);
        let to_sync = self.remote(id).and_then(|remote| {
            let calendars = self.store().server_calendars(id)?;
            Ok(remote.map(|remote| (remote, calendars)))
        });
        match to_sync {
            Ok(Some((remote, calendars))) => self.sync_calendars(
                account_sync,
                started_by,
                remote.as_ref(),
                calendars
                    .into_iter()
                    .map(|calendar| (calendar, None))
                    .collect(),
            ),
            // No server to sync with, or a failure about the Account before
            // any Calendar.
            other => account_sync.end(other.map(drop)).map(|_| Vec::new()),
        }
    }

    /// Syncs the Calendars `calendars` of a server Account, in order,
    /// through its server `remote`, in the Account's Running Sync
    /// `account_sync`, started by `started_by`, and ends that. The Calendars
    /// paired with a Running Sync are syncing already, as
    /// [`Core::add_calendars`] starts them; any other syncing now is left
    /// out. A failure about the whole Account stops it: the Calendars it
    /// didn't reach keep their Sync Status as it was before, and the next
    /// sync reaches them. Each Calendar's sync and the Account's end as
    /// [`Core::sync_account`] says; the first error is returned.
    fn sync_calendars(
        &self,
        account_sync: RunningSync,
        started_by: StartedBy,
        remote: &dyn Remote,
        calendars: Vec<(CalendarToSync, Option<RunningSync>)>,
    ) -> Result<Vec<Changed<CalendarId>>> {
        let mut synced = Vec::new();
        let mut failed = None;
        let mut about_account = None;
        // The Calendars it doesn't reach end as before the sync, as the
        // loop drops their Running Syncs.
        for (calendar @ (calendar_id, ..), started) in calendars {
            let calendar_sync = match started {
                Some(calendar_sync) => calendar_sync,
                None => match self.sync_status().start_calendar(calendar_id, started_by) {
                    Some(calendar_sync) => calendar_sync,
                    None => continue,
                },
            };
            match calendar_sync.end(self.sync_calendar(remote, &calendar)) {
                Ok(changed) => synced.extend(changed),
                Err(error) if sync_status::about_account(&error) => {
                    about_account = Some(error);
                    break;
                }
                Err(error) => _ = failed.get_or_insert(error),
            }
        }
        let stopped = about_account.is_some();
        let ended = account_sync.end(about_account.map_or(Ok(()), Err));
        match failed {
            Some(error) if !stopped => Err(error),
            _ => ended.map(|_| synced),
        }
    }

    /// Runs the syncs the Sync Scheduler finds due now, one Account after
    /// another, and returns once they end: polls, and the syncs asked for
    /// since the last call. The shell calls it on a thread of its own, the
    /// only one that does, every few seconds and whenever
    /// [`Core::wait_for_sync_request`] returns. A sync's failure is its
    /// Sync Status; one that isn't Offline is also logged.
    pub fn run_scheduled_syncs(&self) {
        let due = self.scheduler().take_due(self.clock.now());
        if due.is_empty() {
            return;
        }
        let accounts = if due.every_account() {
            server_accounts_of(&self.store())
        } else {
            Ok(due.accounts().collect())
        };
        let accounts = match accounts {
            Ok(accounts) => accounts,
            Err(error) => return log::warn!("Couldn't read the Accounts to sync: {error}"),
        };
        for id in accounts {
            // The network went away meanwhile; its return syncs them all.
            if !self.scheduler().network_available() {
                break;
            }
            match self.sync_account(id, due.started_by(id)) {
                Ok(_) | Err(CoreError::ServerUnreachable(_) | CoreError::AccountNotFound(_)) => {}
                Err(error) => log::warn!("Syncing Account {} failed: {error}", id.0),
            }
        }
    }

    /// Waits until a sync is asked for, or at most `timeout`, for the
    /// shell's thread that runs [`Core::run_scheduled_syncs`].
    pub fn wait_for_sync_request(&self, timeout: Duration) {
        let scheduler = self.scheduler();
        let _ = self
            .sync_requested
            .wait_timeout_while(scheduler, timeout, |scheduler| !scheduler.has_requests());
    }

    /// Tells the Sync Scheduler whether the window is open, which polls
    /// every 5 minutes, or FreeCal runs in the background, which polls every
    /// 15 minutes. On launch the window is open.
    pub fn set_window_open(&self, open: bool) {
        self.scheduler().set_window_open(open);
    }

    /// Asks for a sync of every server Account because of `trigger`, but
    /// not for a window focus within a minute of the last sync of every
    /// Account.
    pub fn trigger_sync(&self, trigger: SyncTrigger) {
        let now = self.clock.now();
        self.ask_for_sync(|scheduler| scheduler.trigger(trigger, now));
    }

    /// Sync now: asks for a sync of the Account `id`, or of every server
    /// Account, as started by the user. Returns at once; the Sync Status
    /// tells how it goes.
    pub fn sync_now(&self, id: Option<AccountId>) {
        self.ask_for_sync(|scheduler| match id {
            Some(id) => scheduler.ask_account(id, StartedBy::User),
            None => scheduler.ask_every_account(StartedBy::User),
        });
    }

    /// Takes in the system's network status, as the shell reads it. When
    /// the network goes away, every server Account not syncing right now is
    /// Offline without trying, and nothing syncs until it returns. Its
    /// return syncs every server Account at once, which sends what waited.
    /// Otherwise whether an Account is Offline is decided by its requests.
    pub fn set_network_available(&self, available: bool) {
        let went_away = self.ask_for_sync(|scheduler| scheduler.set_network_available(available));
        if went_away {
            self.sync_status().mark_offline_without_trying();
        }
    }

    /// Changes the Sync Scheduler with `ask` and wakes the thread that runs
    /// the syncs when one is asked for now.
    fn ask_for_sync<T>(&self, ask: impl FnOnce(&mut SyncScheduler) -> T) -> T {
        let mut scheduler = self.scheduler();
        let asked = ask(&mut scheduler);
        if scheduler.has_requests() {
            self.sync_requested.notify_all();
        }
        asked
    }

    fn scheduler(&self) -> std::sync::MutexGuard<'_, SyncScheduler> {
        self.scheduler.lock().unwrap()
    }

    /// The Sync Status of every server Account and Calendar, with the
    /// overall status. The Local Account and its Calendars have none.
    pub fn sync_statuses(&self) -> Result<SyncStatuses> {
        self.sync_status().sync_statuses()
    }

    /// The Sync Status Keeper, the only code that touches the Running
    /// Syncs, writes a Sync Record or sends "Sync status changed".
    fn sync_status(&self) -> SyncStatusKeeper<'_> {
        SyncStatusKeeper {
            store: &self.store,
            running: &self.running_syncs,
            clock: &*self.clock,
            signals: &*self.signals,
        }
    }

    /// Syncs `calendar` both ways through its Account's server `remote`:
    /// first its Outgoing Sync sends its Unsent Changes
    /// ([`sync::outgoing`]), then its Incoming Sync brings in what changed
    /// on the server, as the Sync Engine decides ([`sync::incoming`]).
    /// Sending first gets the user's changes to the server soonest, and the
    /// Incoming Sync right after fetches what the server stored of a write
    /// answered without an ETag, and finds what failed a precondition. The
    /// price: a server change since the last sync makes a write fail its
    /// precondition, a request in vain, where an Incoming Sync first would
    /// have read the new version beforehand. A failure of the Outgoing Sync
    /// about the whole Account stops the sync; any other still lets the
    /// Incoming Sync run, and is returned once it ran, so that the Calendar
    /// shows a Sync Error; its Incoming Sync's Journal Transaction is then
    /// committed but not returned. Otherwise returns the Calendar with the
    /// Journal Transaction of its Incoming Sync, as the Outgoing Sync
    /// records none.
    fn sync_calendar(
        &self,
        remote: &dyn Remote,
        calendar: &CalendarToSync,
    ) -> Result<Changed<CalendarId>> {
        let record: sync::Recorder = &|origin, change| {
            self.record(origin, change)
                .map(|changed| changed.journal_transaction)
        };
        let (calendar_id, ref url, _) = *calendar;
        // A permission the server revokes or grants later is noticed here.
        let server_read_only = remote.server_read_only(url)?;
        let changed = self.store().write(|database_transaction| {
            database_transaction.set_server_read_only(calendar_id, server_read_only)
        })?;
        if changed {
            self.signals.send(Signal::CalendarsChanged);
        }
        let now = self.clock.now().with_timezone(&Utc);
        let sent = match sync::outgoing(remote, &self.store, calendar, now) {
            Err(error) if sync_status::about_account(&error) => return Err(error),
            sent => sent,
        };
        let journal_transaction = sync::incoming(
            remote,
            &self.store,
            calendar,
            self.clock.time_zone(),
            record,
        )?;
        sent?;
        Ok(Changed {
            journal_transaction,
            value: calendar_id,
        })
    }

    /// Every Calendar a server Account's server offers now, added or not;
    /// none for the Local Account.
    fn offered_calendars(&self, id: AccountId) -> Result<Vec<AvailableCalendar>> {
        offered_by(self.remote(id)?.as_deref())
    }

    /// The Provider interface to a server Account's server, logged in with
    /// the password from the keyring; none for the Local Account.
    fn remote(&self, id: AccountId) -> Result<Option<Box<dyn Remote>>> {
        let (account, key) = {
            let store = self.store();
            (store.find_account(id)?, store.account_keyring_key(id)?)
        };
        let account = account.ok_or(CoreError::AccountNotFound(id))?;
        let (Some(server), Some(key)) = (account.server, key) else {
            return Ok(None);
        };
        let password = self
            .keyring
            .secret(&key)
            .map_err(CoreError::KeyringUnavailable)?
            .ok_or(CoreError::PasswordNotInKeyring)?;
        remote::connect(account.provider, server, password)
    }

    /// Removes a server Account with its Calendars, as
    /// [`Core::remove_calendar`] removes each, and deletes its secret from
    /// the keyring. The Local Account stays.
    pub fn remove_account(&self, id: AccountId) -> Result<()> {
        let (account, key) = {
            let store = self.store();
            (store.find_account(id)?, store.account_keyring_key(id)?)
        };
        match account.ok_or(CoreError::AccountNotFound(id))?.provider {
            Provider::Local => return Err(CoreError::LocalAccountCannotBeRemoved),
            Provider::CalDav => {}
        }
        // The keyring first: should it fail, the Account stays, and removing
        // it again can delete the secret.
        if let Some(key) = key {
            self.keyring
                .delete(&key)
                .map_err(CoreError::KeyringUnavailable)?;
        }
        journal::delete_account(&self.store(), id)?;
        self.signals.send(Signal::CalendarsChanged);
        Ok(())
    }

    /// Every Calendar of every Account, in the order they were added.
    pub fn list_calendars(&self) -> Result<Vec<Calendar>> {
        self.store().list_calendars()
    }

    /// Creates a Calendar in the Local Account. Server Accounts' Calendars
    /// are added from their Available Calendars instead.
    pub fn create_calendar(
        &self,
        account_id: AccountId,
        name: &str,
        colour: Colour,
    ) -> Result<Calendar> {
        let name = calendar_name(name)?;
        let created = {
            let store = self.store();
            let account = store.find_account(account_id)?;
            match account
                .ok_or(CoreError::AccountNotFound(account_id))?
                .provider
            {
                Provider::Local => store.write(|database_transaction| {
                    database_transaction.insert_calendar(account_id, name, &colour)
                })?,
                Provider::CalDav => return Err(CoreError::NotLocalAccount(account_id)),
            }
        };
        self.signals.send(Signal::CalendarsChanged);
        Ok(created)
    }

    pub fn rename_calendar(&self, id: CalendarId, name: &str) -> Result<()> {
        let name = calendar_name(name)?;
        self.store()
            .write(|database_transaction| database_transaction.set_calendar_name(id, name))?;
        self.signals.send(Signal::CalendarsChanged);
        Ok(())
    }

    pub fn recolour_calendar(&self, id: CalendarId, colour: Colour) -> Result<()> {
        self.store()
            .write(|database_transaction| database_transaction.set_calendar_colour(id, &colour))?;
        self.signals.send(Signal::CalendarsChanged);
        Ok(())
    }

    /// Deletes a Local Calendar with all its Events.
    /// Its changes leave the Change Journal (ADR 0009).
    pub fn delete_calendar(&self, id: CalendarId) -> Result<()> {
        self.delete_local_copy(id, false)
    }

    /// Removes a server Account's Calendar from FreeCal: deletes its local
    /// copy with all its Events, as [`Core::delete_calendar`] does, and
    /// leaves it on the server, where it is an Available Calendar again.
    pub fn remove_calendar(&self, id: CalendarId) -> Result<()> {
        self.delete_local_copy(id, true)
    }

    /// Deletes a Calendar's local copy if it is a server Account's Calendar,
    /// as `from_server` asks, or a Local Calendar if not.
    fn delete_local_copy(&self, id: CalendarId, from_server: bool) -> Result<()> {
        {
            let store = self.store();
            let account_id = store.find_calendar(id)?.account_id;
            let account = store.find_account(account_id)?;
            match (
                account
                    .ok_or(CoreError::AccountNotFound(account_id))?
                    .provider,
                from_server,
            ) {
                (Provider::Local, false) | (Provider::CalDav, true) => {}
                (Provider::Local, true) => return Err(CoreError::NotServerCalendar(id)),
                (Provider::CalDav, false) => return Err(CoreError::NotLocalCalendar(id)),
            }
            journal::delete_calendar(&store, id)?;
        }
        self.signals.send(Signal::CalendarsChanged);
        Ok(())
    }

    /// Shows or hides a Calendar's Events.
    pub fn set_calendar_shown(&self, id: CalendarId, shown: bool) -> Result<()> {
        self.store()
            .write(|database_transaction| database_transaction.set_calendar_shown(id, shown))?;
        self.signals.send(Signal::CalendarsChanged);
        Ok(())
    }

    /// Marks a Calendar read-only in FreeCal, or unmarks it (US-17, US-19).
    /// A Calendar whose server doesn't let the user change it stays
    /// read-only whatever the mark.
    pub fn set_calendar_read_only(&self, id: CalendarId, read_only: bool) -> Result<()> {
        self.store().write(|database_transaction| {
            database_transaction.set_calendar_read_only(id, read_only)
        })?;
        self.signals.send(Signal::CalendarsChanged);
        Ok(())
    }

    /// Shows one Calendar and hides every other Calendar in all Accounts.
    pub fn show_only_calendar(&self, id: CalendarId) -> Result<()> {
        self.store()
            .write(|database_transaction| database_transaction.show_only_calendar(id))?;
        self.signals.send(Signal::CalendarsChanged);
        Ok(())
    }

    /// The Occurrences of shown Calendars on the dates `from` up to `to`
    /// (exclusive), in the Display Time Zone and ordered by start. Recurring
    /// Events are expanded here, in wall time in their Event Time Zone, and
    /// their Exceptions stand in for the Occurrences they changed.
    pub fn occurrences(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<Occurrence>> {
        event::occurrences(&self.store(), from, to, self.clock.time_zone())
    }

    /// An Event with everything its details and editor show.
    pub fn event(&self, id: EventId) -> Result<Event> {
        let store = self.store();
        Opened::load(&*store, id)?.to_event(&store, self.clock.time_zone())
    }

    /// The Server Object an Event belongs to (ADR 0015), as the server last
    /// gave it: the object holding its UID in its Calendar, for an
    /// Exception too. None for an Event no object holds, such as one not on
    /// a server yet.
    pub fn original_server_data(&self, id: EventId) -> Result<Option<ServerObject>> {
        self.store().server_object_of_event(id)
    }

    /// Creates an Event in a writable Calendar, a Recurring Event when the
    /// draft has a repeat rule.
    pub fn create_event(&self, draft: EventDraft) -> Result<Changed<Event>> {
        let zone = self.clock.time_zone();
        let created = self.record(TransactionOrigin::UserAction, |journal_transaction| {
            event::create_event(journal_transaction, zone, draft)
        })?;
        self.changed_event(created)
    }

    /// Changes an Event, possibly moving it to another writable Calendar.
    /// A timed Event keeps its Event Time Zones, and what the user left as
    /// it was stays exactly as stored: an unchanged time its wall time, an
    /// unchanged description its original bytes, even if it was HTML, and an
    /// unchanged repeat rule its stored rule. A Recurring Event changes with
    /// all its Occurrences, from its first date, and an Exception alone (see
    /// [`Core::edit_occurrence`]).
    pub fn edit_event(&self, id: EventId, draft: EventDraft) -> Result<Changed<Event>> {
        self.edit_whole(id, Edit::Draft(draft))
    }

    /// Moves an Event to another time, as a drag or a resize in the grid
    /// does, and leaves everything else as stored. `when` is in the Display
    /// Time Zone; a timed Event keeps its Event Time Zones. A Recurring Event
    /// moves with all its Occurrences, and an Exception alone.
    pub fn reschedule_event(&self, id: EventId, when: When) -> Result<Changed<Event>> {
        self.edit_whole(id, Edit::Reschedule(when))
    }

    fn edit_whole(&self, id: EventId, edit: Edit) -> Result<Changed<Event>> {
        let zone = self.clock.time_zone();
        let edited = self.record(TransactionOrigin::UserAction, |journal_transaction| {
            event::edit_event(journal_transaction, zone, id, edit)
        })?;
        self.changed_event(Changed {
            journal_transaction: edited.journal_transaction,
            value: edited.value.id,
        })
    }

    /// Changes one Occurrence of a Recurring Event, given by the Event and
    /// the recurrence ID its [`Occurrence`] has, in `scope`: alone, as an
    /// Exception; with every later one, as a Recurring Event of their own;
    /// or with all of them. What the user changed is what differs from the
    /// Occurrence as it was. Exceptions in the scope keep what they changed
    /// on their own, unless `exceptions_too` asks for them to take the
    /// change as well (see [`Core::exception_counts`]). Returns the
    /// Occurrence as it is now.
    pub fn edit_occurrence(
        &self,
        id: EventId,
        recurrence_id: &str,
        scope: Scope,
        exceptions_too: bool,
        draft: EventDraft,
    ) -> Result<Changed<Occurrence>> {
        self.edit_in_scope(id, recurrence_id, scope, exceptions_too, Edit::Draft(draft))
    }

    /// Moves one Occurrence of a Recurring Event to another time in `scope`,
    /// as a drag or a resize in the grid does (see [`Core::edit_occurrence`]).
    pub fn reschedule_occurrence(
        &self,
        id: EventId,
        recurrence_id: &str,
        scope: Scope,
        exceptions_too: bool,
        when: When,
    ) -> Result<Changed<Occurrence>> {
        self.edit_in_scope(
            id,
            recurrence_id,
            scope,
            exceptions_too,
            Edit::Reschedule(when),
        )
    }

    /// How many Exceptions with values of their own a change to one
    /// Occurrence of a Recurring Event, given as for
    /// [`Core::edit_occurrence`], reaches in "this and following events" and
    /// in "all events", besides the Occurrence itself: those whose title,
    /// location, description or time differs from what the Recurring Event
    /// gives them.
    pub fn exception_counts(&self, id: EventId, recurrence_id: &str) -> Result<ExceptionCounts> {
        event::exception_counts(&*self.store(), id, recurrence_id)
    }

    fn edit_in_scope(
        &self,
        id: EventId,
        recurrence_id: &str,
        scope: Scope,
        exceptions_too: bool,
        edit: Edit,
    ) -> Result<Changed<Occurrence>> {
        let zone = self.clock.time_zone();
        let edited = self.record(TransactionOrigin::UserAction, |journal_transaction| {
            event::edit_occurrence(
                journal_transaction,
                zone,
                id,
                recurrence_id,
                scope,
                exceptions_too,
                edit,
            )
        })?;
        let occurrence = edited.value;
        Ok(Changed {
            journal_transaction: edited.journal_transaction,
            value: Occurrence {
                event_id: occurrence.id,
                calendar_id: occurrence.calendar_id,
                title: occurrence.title,
                when: occurrence.when.to_when(zone)?,
                recurrence_id: occurrence.recurrence_id,
                // The Change Journal changes no Event with guests.
                has_guests: false,
            },
        })
    }

    /// Deletes an Event from a writable Calendar. For a Recurring Event or
    /// one of its Exceptions, that is the whole Recurring Event with all its
    /// Exceptions.
    pub fn delete_event(&self, id: EventId) -> Result<Changed<()>> {
        self.record(TransactionOrigin::UserAction, |journal_transaction| {
            Opened::load(journal_transaction, id)?.delete(journal_transaction)
        })
    }

    /// Deletes one Occurrence of a Recurring Event, given by the Event and
    /// the recurrence ID its [`Occurrence`] has, and leaves the rest of the
    /// Recurring Event in place: the Occurrence becomes a cancelled
    /// Exception, and an Exception that changed it is deleted. The only
    /// Occurrence of a single Event is the Event.
    pub fn delete_occurrence(&self, id: EventId, recurrence_id: &str) -> Result<Changed<()>> {
        self.record(TransactionOrigin::UserAction, |journal_transaction| {
            event::cancel_occurrence(journal_transaction, id, recurrence_id)
        })
    }

    /// Previews importing `file`, an `.ics` file or a `.zip` of them, into a
    /// Calendar: how many Events are new, how many it already has (same
    /// UID), and which zones floating times were pinned to. Stores nothing.
    pub fn preview_import(
        &self,
        file: &ImportFile,
        calendar_id: CalendarId,
    ) -> Result<ImportPreview> {
        let existing = {
            let store = self.store();
            store.find_calendar(calendar_id)?;
            store.event_uids(calendar_id)?
        };
        import::preview(file, calendar_id, &existing, self.clock.time_zone())
    }

    /// Runs a previewed import into a writable Calendar, doing with the
    /// Events it already has what `existing` says, as one Journal Transaction.
    pub fn run_import(
        &self,
        preview: &ImportPreview,
        existing: ExistingEvents,
    ) -> Result<ImportOutcome> {
        let run = self.record(TransactionOrigin::Import, |journal_transaction| {
            import::run(journal_transaction, preview, existing)
        })?;
        Ok(ImportOutcome {
            journal_transaction: run.journal_transaction,
            ..run.value
        })
    }

    /// Undoes a Journal Transaction whole: every Event it changed gets its
    /// previous Event Version back under the same identity. Refused while an
    /// Event changed since, and for a Journal Transaction undone already. The
    /// undo is a Journal Transaction itself, and undoing it redoes the
    /// change.
    pub fn undo_journal_transaction(&self, id: JournalTransactionId) -> Result<Changed<()>> {
        let undone = journal::undo(&self.store(), id, self.clock.now().timestamp())?;
        Ok(self.signal(undone, TransactionOrigin::Undo))
    }

    /// Runs `change` as one Journal Transaction of `origin` and sends
    /// "Occurrences changed" for the Calendars it changed Events in.
    fn record<T>(
        &self,
        origin: TransactionOrigin,
        change: impl FnOnce(&mut JournalTransaction) -> Result<T>,
    ) -> Result<Changed<T>> {
        let recorded =
            journal::record(&self.store(), origin, self.clock.now().timestamp(), change)?;
        Ok(self.signal(recorded, origin))
    }

    /// Sends "Occurrences changed" for a recorded Journal Transaction's
    /// Calendars, once the Store lock is released. A change of `origin`
    /// other than a sync can add Unsent Changes, so it also sends "Sync
    /// status changed" for those of its Calendars that are a server
    /// Account's, and their Accounts; a sync sends its own as it ends.
    fn signal<T>(&self, recorded: journal::Recorded<T>, origin: TransactionOrigin) -> Changed<T> {
        if !recorded.calendars.is_empty() {
            if !matches!(
                origin,
                TransactionOrigin::FirstSync | TransactionOrigin::IncomingSync
            ) {
                self.after_local_change(&recorded.calendars);
            }
            self.signals.send(Signal::OccurrencesChanged {
                calendar_ids: recorded.calendars,
            });
        }
        Changed {
            journal_transaction: recorded.journal_transaction,
            value: recorded.value,
        }
    }

    /// Sends "Sync status changed" for those of `calendars` that are a
    /// server Account's, and for their Accounts, as their Unsent Changes may
    /// have changed, and asks for a sync of those Accounts, which sends the
    /// change right after it.
    fn after_local_change(&self, calendars: &[CalendarId]) {
        let accounts = self.sync_status().after_local_change(calendars);
        if !accounts.is_empty() {
            self.ask_for_sync(|scheduler| {
                for id in &accounts {
                    scheduler.ask_account(*id, StartedBy::FreeCal);
                }
            });
        }
    }

    fn changed_event(&self, changed: Changed<EventId>) -> Result<Changed<Event>> {
        self.event(changed.value).map(|value| Changed {
            journal_transaction: changed.journal_transaction,
            value,
        })
    }

    /// The Local Store, locked. Signals are sent only after the lock is
    /// released, so a subscriber can read straight away.
    fn store(&self) -> std::sync::MutexGuard<'_, Store> {
        self.store.lock().unwrap()
    }
}

/// Takes the exclusive lock on the Local Store in `data_dir`, which
/// `freecal.lock` next to it stands for.
fn lock_local_store(data_dir: &Path) -> Result<File> {
    let lock = File::options()
        .write(true)
        .create(true)
        .truncate(false)
        .open(data_dir.join("freecal.lock"))?;
    match lock.try_lock() {
        Ok(()) => Ok(lock),
        Err(TryLockError::WouldBlock) => Err(CoreError::LocalStoreInUse),
        Err(TryLockError::Error(error)) => Err(error.into()),
    }
}

/// Every server Account in the Local Store, in the order they were added.
fn server_accounts_of(store: &Store) -> Result<Vec<AccountId>> {
    Ok(store
        .list_accounts()?
        .into_iter()
        .filter(|account| account.provider != Provider::Local)
        .map(|account| account.id)
        .collect())
}

/// Every Calendar a server offers, added or not; none without a server.
fn offered_by(remote: Option<&dyn Remote>) -> Result<Vec<AvailableCalendar>> {
    remote.map_or_else(|| Ok(Vec::new()), Remote::discover)
}
