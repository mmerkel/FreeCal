use serde::Serialize;

use crate::{AccountId, CalendarId, EventId, ImportId};

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
    #[error("{file:?} couldn't be read")]
    ImportUnreadable { file: String },
    #[error("{file:?} is not an iCalendar file FreeCal can read")]
    ImportNotCalendar { file: String },
    #[error("{file:?} has no .ics files")]
    ImportZipWithoutCalendars { file: String },
    #[error("{file:?} is larger than an import may be")]
    ImportTooLarge { file: String },
    #[error("there is no import {0:?} to undo")]
    ImportNotFound(ImportId),
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
            CoreError::ImportUnreadable { file } => {
                ErrorCode::ImportUnreadable { file: file.clone() }
            }
            CoreError::ImportNotCalendar { file } => {
                ErrorCode::ImportNotCalendar { file: file.clone() }
            }
            CoreError::ImportZipWithoutCalendars { file } => {
                ErrorCode::ImportZipWithoutCalendars { file: file.clone() }
            }
            CoreError::ImportTooLarge { file } => ErrorCode::ImportTooLarge { file: file.clone() },
            CoreError::ImportNotFound(_) => ErrorCode::ImportNotFound,
        }
    }
}

/// A failure as it crosses the application interface: a code the frontend
/// translates through its i18n layer, with the names of files it is about.
/// Most codes stand for a [`CoreError`]; the last ones are failures of the
/// shell's own commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
    /// A file to import couldn't be read, or a `.zip` is damaged. `file` is
    /// the file's name, untrusted like every name.
    ImportUnreadable {
        file: String,
    },
    /// A file to import, or an `.ics` file in a `.zip` to import, isn't an
    /// iCalendar file FreeCal can read. `file` names it, an `.ics` in a
    /// `.zip` as `takeout.zip/Calendar/Work.ics`.
    ImportNotCalendar {
        file: String,
    },
    /// A `.zip` to import has no `.ics` files.
    ImportZipWithoutCalendars {
        file: String,
    },
    /// A file to import is larger than `MAX_IMPORT_SIZE`, or a `.zip` to
    /// import has more entries than `MAX_IMPORT_ENTRIES` or unpacks larger.
    ImportTooLarge {
        file: String,
    },
    /// An import to undo was undone already, or its Calendar deleted.
    ImportNotFound,
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
    pub(crate) fn all() -> Vec<ErrorCode> {
        let file = || "takeout.zip/Calendar/Work.ics".to_owned();
        vec![
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
            ErrorCode::ImportUnreadable { file: file() },
            ErrorCode::ImportNotCalendar { file: file() },
            ErrorCode::ImportZipWithoutCalendars { file: file() },
            ErrorCode::ImportTooLarge { file: file() },
            ErrorCode::ImportNotFound,
            ErrorCode::LinkNotAllowed,
            ErrorCode::LinkNotOpened,
            ErrorCode::WindowNotShown,
        ]
    }
}

impl From<CoreError> for ErrorCode {
    fn from(error: CoreError) -> Self {
        error.code()
    }
}
