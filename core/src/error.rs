use serde::Serialize;

use crate::{AccountId, CalendarId, EventId};

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("the Local Store failed: {0}")]
    Store(#[from] rusqlite::Error),
    #[error("the data directory failed: {0}")]
    DataDirectory(#[from] std::io::Error),
    #[error("another running FreeCal has the Local Store open")]
    LocalStoreInUse,
    #[error("there is no Account {0:?}")]
    AccountNotFound(AccountId),
    #[error("there is no Calendar {0:?}")]
    CalendarNotFound(CalendarId),
    #[error("Calendar {0:?} is read-only")]
    CalendarReadOnly(CalendarId),
    #[error("there is no Event {0:?}")]
    EventNotFound(EventId),
    #[error("an Event can't end before it starts")]
    EndBeforeStart,
    #[error("the Local Store has an Event time FreeCal can't read: {0:?}")]
    UnreadableEventTime(String),
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

impl CoreError {
    /// What the frontend is told: a code it translates, never this English
    /// text, which is for logs.
    pub fn code(&self) -> ErrorCode {
        match self {
            CoreError::Store(_) => ErrorCode::LocalStoreFailed,
            CoreError::DataDirectory(_) => ErrorCode::DataDirectoryFailed,
            CoreError::LocalStoreInUse => ErrorCode::LocalStoreInUse,
            CoreError::AccountNotFound(_) => ErrorCode::AccountNotFound,
            CoreError::CalendarNotFound(_) => ErrorCode::CalendarNotFound,
            CoreError::CalendarReadOnly(_) => ErrorCode::CalendarReadOnly,
            CoreError::EventNotFound(_) => ErrorCode::EventNotFound,
            CoreError::EndBeforeStart => ErrorCode::EndBeforeStart,
            CoreError::UnreadableEventTime(_) => ErrorCode::UnreadableEventTime,
            CoreError::EmptyCalendarName => ErrorCode::EmptyCalendarName,
            CoreError::InvalidColour(_) => ErrorCode::InvalidColour,
            CoreError::LocalAccountCannotBeRemoved => ErrorCode::LocalAccountCannotBeRemoved,
            CoreError::NewerLocalStore { .. } => ErrorCode::NewerLocalStore,
            CoreError::UnknownProvider(_) => ErrorCode::UnknownProvider,
        }
    }
}

/// A failure as it crosses the application interface: a code the frontend
/// translates through its i18n layer. Most codes stand for a [`CoreError`];
/// the last ones are failures of the shell's own commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(tag = "code", rename_all = "camelCase")]
pub enum ErrorCode {
    /// The Local Store couldn't be read or written, as when it is damaged.
    LocalStoreFailed,
    DataDirectoryFailed,
    LocalStoreInUse,
    AccountNotFound,
    CalendarNotFound,
    CalendarReadOnly,
    EventNotFound,
    EndBeforeStart,
    UnreadableEventTime,
    EmptyCalendarName,
    InvalidColour,
    LocalAccountCannotBeRemoved,
    NewerLocalStore,
    UnknownProvider,
    /// The window asked to open a link that isn't `http(s)://` or `mailto:`.
    LinkNotAllowed,
    /// The system couldn't open a link in the browser.
    LinkNotOpened,
    /// The window couldn't be shown.
    WindowNotShown,
}

impl ErrorCode {
    /// Every code, for the generated TypeScript samples.
    #[cfg(test)]
    pub(crate) const ALL: [ErrorCode; 17] = [
        ErrorCode::LocalStoreFailed,
        ErrorCode::DataDirectoryFailed,
        ErrorCode::LocalStoreInUse,
        ErrorCode::AccountNotFound,
        ErrorCode::CalendarNotFound,
        ErrorCode::CalendarReadOnly,
        ErrorCode::EventNotFound,
        ErrorCode::EndBeforeStart,
        ErrorCode::UnreadableEventTime,
        ErrorCode::EmptyCalendarName,
        ErrorCode::InvalidColour,
        ErrorCode::LocalAccountCannotBeRemoved,
        ErrorCode::NewerLocalStore,
        ErrorCode::UnknownProvider,
        ErrorCode::LinkNotAllowed,
        ErrorCode::LinkNotOpened,
        ErrorCode::WindowNotShown,
    ];
}

impl From<CoreError> for ErrorCode {
    fn from(error: CoreError) -> Self {
        error.code()
    }
}
