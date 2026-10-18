use std::fmt;

use serde::{Deserialize, Serialize};

use crate::AvailableCalendar;

/// Branded in TypeScript, so that it can't be mixed up with other IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(
    test,
    derive(ts_rs::TS),
    ts(type = "number & { readonly __brand: \"AccountId\" }")
)]
#[serde(transparent)]
pub struct AccountId(pub i64);

/// A kind of calendar service FreeCal can talk to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Local,
    CalDav,
}

impl Provider {
    /// How the Local Store records the Provider.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Provider::Local => "local",
            Provider::CalDav => "caldav",
        }
    }

    pub(crate) fn parse(stored: &str) -> Option<Self> {
        [Provider::Local, Provider::CalDav]
            .into_iter()
            .find(|provider| provider.as_str() == stored)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Account {
    pub id: AccountId,
    pub provider: Provider,
    /// Where a server Account syncs and who it logs in as; none for the
    /// Local Account.
    pub server: Option<ServerLogin>,
}

/// A server Account just added, with the Calendars its server offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct AddedAccount {
    pub account: Account,
    pub available: Vec<AvailableCalendar>,
}

/// A server Account's server and the username it logs in with. Its
/// password lives in the keyring.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ServerLogin {
    pub url: String,
    pub username: String,
}

/// What the user enters to add a CalDAV Account.
#[derive(Clone, Deserialize)]
pub struct CalDavLogin {
    /// The server's address, `http://` or `https://`.
    pub url: String,
    pub username: String,
    pub password: String,
}

/// Leaves the password out, so that it never reaches a log.
impl fmt::Debug for CalDavLogin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CalDavLogin")
            .field("url", &self.url)
            .field("username", &self.username)
            .finish_non_exhaustive()
    }
}
