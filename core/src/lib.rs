//! FreeCal's core: all domain logic behind one application interface, [`Core`].
//!
//! The frontend talks only to [`Core`]. Besides answering requests, the core
//! pushes [`Signal`]s to a [`SignalSink`].

mod account;
mod calendar;
mod clock;
mod signal;
mod store;

use std::path::Path;

use chrono::NaiveDate;
use std::sync::{Arc, Mutex};

pub use account::{Account, AccountId, Provider};
pub use calendar::{Calendar, CalendarId, Colour};
pub use clock::{Clock, SystemClock};
pub use signal::{Signal, SignalSink};

use calendar::calendar_name;
use store::Store;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("the Local Store failed: {0}")]
    Store(#[from] rusqlite::Error),
    #[error("could not create the data directory: {0}")]
    DataDirectory(#[from] std::io::Error),
    #[error("there is no Account {0:?}")]
    AccountNotFound(AccountId),
    #[error("there is no Calendar {0:?}")]
    CalendarNotFound(CalendarId),
    #[error("a Calendar needs a name")]
    EmptyCalendarName,
    #[error("{0:?} is not a colour of the form #rrggbb")]
    InvalidColour(String),
    #[error("the Local Account cannot be removed")]
    LocalAccountCannotBeRemoved,
    #[error(
        "the Local Store was written by a newer FreeCal \
         (schema {found}, this build knows {known})"
    )]
    NewerLocalStore { found: i64, known: i64 },
    #[error("the Local Store has an Account with an unknown Provider {0:?}")]
    UnknownProvider(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;

/// The core application interface.
pub struct Core {
    store: Mutex<Store>,
    clock: Arc<dyn Clock>,
    signals: Arc<dyn SignalSink>,
}

impl Core {
    /// Opens the Local Store in `data_dir`, creating it on first launch.
    pub fn open(
        data_dir: &Path,
        clock: Arc<dyn Clock>,
        signals: Arc<dyn SignalSink>,
    ) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let store = Store::open(&data_dir.join("freecal.sqlite3"))?;
        Ok(Self {
            store: Mutex::new(store),
            clock,
            signals,
        })
    }

    /// The current local date, on which the views open.
    pub fn today(&self) -> NaiveDate {
        self.clock.now().date_naive()
    }

    pub fn list_accounts(&self) -> Result<Vec<Account>> {
        self.store().list_accounts()
    }

    pub fn remove_account(&self, id: AccountId) -> Result<()> {
        let account = self.store().find_account(id)?;
        match account.ok_or(CoreError::AccountNotFound(id))?.provider {
            Provider::Local => Err(CoreError::LocalAccountCannotBeRemoved),
        }
    }

    /// Every Calendar of every Account, in the order they were added.
    pub fn list_calendars(&self) -> Result<Vec<Calendar>> {
        self.store().list_calendars()
    }

    /// Creates a Calendar in the Local Account. Server Accounts' Calendars
    /// are added from their Available Calendars instead (ticket 14).
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
                Provider::Local => store.insert_calendar(account_id, name, &colour)?,
            }
        };
        self.signals.send(Signal::CalendarsChanged);
        Ok(created)
    }

    pub fn rename_calendar(&self, id: CalendarId, name: &str) -> Result<()> {
        let name = calendar_name(name)?;
        self.store().set_calendar_name(id, name)?;
        self.signals.send(Signal::CalendarsChanged);
        Ok(())
    }

    pub fn recolour_calendar(&self, id: CalendarId, colour: Colour) -> Result<()> {
        self.store().set_calendar_colour(id, &colour)?;
        self.signals.send(Signal::CalendarsChanged);
        Ok(())
    }

    /// Deletes a Local Calendar with all its Events.
    pub fn delete_calendar(&self, id: CalendarId) -> Result<()> {
        self.store().delete_calendar(id)?;
        self.signals.send(Signal::CalendarsChanged);
        Ok(())
    }

    /// Shows or hides a Calendar's Events.
    pub fn set_calendar_shown(&self, id: CalendarId, shown: bool) -> Result<()> {
        self.store().set_calendar_shown(id, shown)?;
        self.signals.send(Signal::CalendarsChanged);
        Ok(())
    }

    /// Shows one Calendar and hides every other Calendar in all Accounts.
    pub fn show_only_calendar(&self, id: CalendarId) -> Result<()> {
        self.store().show_only_calendar(id)?;
        self.signals.send(Signal::CalendarsChanged);
        Ok(())
    }

    /// The Local Store, locked. Signals are sent only after the lock is
    /// released, so a subscriber can read straight away.
    fn store(&self) -> std::sync::MutexGuard<'_, Store> {
        self.store.lock().unwrap()
    }
}
