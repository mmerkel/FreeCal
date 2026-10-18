use serde::{Deserialize, Serialize};

use crate::{AccountId, CoreError, Result};

/// Branded in TypeScript, so that it can't be mixed up with other IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    test,
    derive(ts_rs::TS),
    ts(type = "number & { readonly __brand: \"CalendarId\" }")
)]
#[serde(transparent)]
pub struct CalendarId(pub i64);

/// A Calendar's colour, always `#rrggbb` in lower case, so that it can only
/// ever be a colour wherever it is used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(type = "string"))]
#[serde(transparent)]
pub struct Colour(String);

impl Colour {
    pub fn parse(text: &str) -> Result<Self> {
        let digits = text.strip_prefix('#').unwrap_or_default();
        if digits.len() == 6 && digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            Ok(Self(text.to_ascii_lowercase()))
        } else {
            Err(CoreError::InvalidColour(text.to_owned()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for Colour {
    /// The colour of a Calendar whose stored colour is unusable.
    fn default() -> Self {
        Self("#808080".to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Calendar {
    pub id: CalendarId,
    pub account_id: AccountId,
    pub name: String,
    pub colour: Colour,
    /// Whether the user shows this Calendar's Events or has hidden them.
    pub shown: bool,
    /// A Read-only Calendar: FreeCal never changes its Events. It is one
    /// when its server doesn't allow changing it or the user marked it
    /// read-only; until 1.0, every newly added server Calendar starts
    /// marked. Marking comes with ticket 17.
    pub read_only: bool,
    /// Whether its server doesn't allow the user to change it, so that it
    /// stays read-only whatever the user marks.
    pub server_read_only: bool,
    /// Whether its first sync is running, so that its Events are still
    /// loading; the sidebar says so. Sync Status may take this over (ticket
    /// 15).
    pub first_sync: bool,
}

/// A Calendar a server Account offers that the user hasn't added. FreeCal
/// stores nothing for it; it is fetched from the server each time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AvailableCalendar {
    /// Where it lives on the server, which also names it to the core.
    pub url: String,
    /// Its name on the server, untrusted like every name; the last part of
    /// its address when the server gives none.
    pub name: String,
    /// Its colour on the server, if it has a usable one.
    pub colour: Option<Colour>,
    /// Whether the server doesn't allow the user to change it, so that it
    /// becomes a Read-only Calendar.
    pub read_only: bool,
}

/// A Calendar name as the user typed it, without surrounding whitespace.
pub(crate) fn calendar_name(text: &str) -> Result<&str> {
    match text.trim() {
        "" => Err(CoreError::EmptyCalendarName),
        name => Ok(name),
    }
}
