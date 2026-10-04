use serde::Serialize;

use crate::{AccountId, CoreError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct CalendarId(pub i64);

/// A Calendar's colour, always `#rrggbb` in lower case, so that it can only
/// ever be a colour wherever it is used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
#[serde(rename_all = "camelCase")]
pub struct Calendar {
    pub id: CalendarId,
    pub account_id: AccountId,
    pub name: String,
    pub colour: Colour,
    /// Whether the user shows this Calendar's Events or has hidden them.
    pub shown: bool,
}

/// A Calendar name as the user typed it, without surrounding whitespace.
pub(crate) fn calendar_name(text: &str) -> Result<&str> {
    match text.trim() {
        "" => Err(CoreError::EmptyCalendarName),
        name => Ok(name),
    }
}
