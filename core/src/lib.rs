//! FreeCal's core: all domain logic behind one application interface, [`Core`].
//!
//! The frontend talks only to [`Core`]. Besides answering requests, the core
//! pushes [`Signal`]s to a [`SignalSink`].

mod account;
mod clock;
mod signal;
mod store;

use std::path::Path;

use chrono::NaiveDate;
use std::sync::{Arc, Mutex};

pub use account::{Account, AccountId, Provider};
pub use clock::{Clock, SystemClock};
pub use signal::{Signal, SignalSink};

use store::Store;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("the Local Store failed: {0}")]
    Store(#[from] rusqlite::Error),
    #[error("could not create the data directory: {0}")]
    DataDirectory(#[from] std::io::Error),
    #[error("there is no Account {0:?}")]
    AccountNotFound(AccountId),
    #[error("the Local Account cannot be removed")]
    LocalAccountCannotBeRemoved,
}

pub type Result<T> = std::result::Result<T, CoreError>;

/// The core application interface.
pub struct Core {
    store: Mutex<Store>,
    clock: Arc<dyn Clock>,
    #[allow(dead_code)]
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
        self.store.lock().unwrap().list_accounts()
    }

    pub fn remove_account(&self, id: AccountId) -> Result<()> {
        let account = self
            .list_accounts()?
            .into_iter()
            .find(|account| account.id == id)
            .ok_or(CoreError::AccountNotFound(id))?;
        match account.provider {
            Provider::Local => Err(CoreError::LocalAccountCannotBeRemoved),
        }
    }
}
