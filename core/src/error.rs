use serde::Serialize;

use crate::{AccountId, CalendarId, EventId, KeyringError, UnitId};

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
    #[error(
        "a repeat rule needs an interval and a count of at least 1, and can't end before it starts"
    )]
    InvalidRepeat,
    #[error("Event {0:?} has no such Occurrence")]
    OccurrenceNotFound(EventId),
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
    #[error("there is no Change Journal unit {0:?}")]
    UnitNotFound(UnitId),
    #[error("an Event of Change Journal unit {0:?} changed since")]
    ChangedSinceUnit(UnitId),
    #[error("Change Journal unit {0:?} is undone already")]
    AlreadyUndone(UnitId),
    #[error("Account {0:?} gets its Calendars from its server")]
    NotLocalAccount(AccountId),
    #[error("Calendar {0:?} comes from a server and is removed, not deleted")]
    NotLocalCalendar(CalendarId),
    #[error("Calendar {0:?} is a Local Calendar and is deleted, not removed")]
    NotServerCalendar(CalendarId),
    /// The URL is the frontend's, untrusted.
    #[error("the Account's server doesn't offer the Calendar {0:?}, or it was added already")]
    CalendarNotAvailable(String),
    #[error("a server's address must start with http:// or https://")]
    InvalidServerUrl,
    #[error("the server didn't accept the username and password")]
    WrongCredentials,
    #[error("the server couldn't be reached: {0}")]
    ServerUnreachable(String),
    #[error("no secure connection to the server could be made: {0}")]
    SecureConnectionFailed(String),
    #[error("the server answered, but not as a CalDAV server")]
    NotCalDavServer,
    /// `message` is the server's own text, untrusted, made plain by
    /// [`crate::caldav`].
    #[error("the server refused with status {status}: {message:?}")]
    ServerRefused { status: u16, message: String },
    #[error("the system keyring couldn't be used: {0}")]
    KeyringUnavailable(KeyringError),
    #[error("the system keyring has no password for the Account")]
    PasswordNotInKeyring,
    /// Only redirects within the server the user named are followed.
    #[error("the server redirected to another server: {0}")]
    RedirectedElsewhere(url::Url),
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
            CoreError::InvalidRepeat => ErrorCode::InvalidRepeat,
            CoreError::OccurrenceNotFound(_) => ErrorCode::OccurrenceNotFound,
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
            CoreError::UnitNotFound(_) => ErrorCode::UnitNotFound,
            CoreError::ChangedSinceUnit(_) => ErrorCode::ChangedSinceUnit,
            CoreError::AlreadyUndone(_) => ErrorCode::AlreadyUndone,
            CoreError::NotLocalAccount(_) => ErrorCode::NotLocalAccount,
            CoreError::NotLocalCalendar(_) => ErrorCode::NotLocalCalendar,
            CoreError::NotServerCalendar(_) => ErrorCode::NotServerCalendar,
            CoreError::CalendarNotAvailable(_) => ErrorCode::CalendarNotAvailable,
            CoreError::InvalidServerUrl => ErrorCode::InvalidServerUrl,
            CoreError::WrongCredentials => ErrorCode::WrongCredentials,
            CoreError::ServerUnreachable(_) => ErrorCode::ServerUnreachable,
            CoreError::SecureConnectionFailed(_) => ErrorCode::SecureConnectionFailed,
            CoreError::NotCalDavServer => ErrorCode::NotCalDavServer,
            CoreError::ServerRefused { status, message } => ErrorCode::ServerRefused {
                status: *status,
                message: message.clone(),
            },
            CoreError::KeyringUnavailable(_) => ErrorCode::KeyringUnavailable,
            CoreError::PasswordNotInKeyring => ErrorCode::PasswordNotInKeyring,
            CoreError::RedirectedElsewhere(url) => ErrorCode::RedirectedElsewhere {
                url: url.to_string(),
            },
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
    InvalidRepeat,
    OccurrenceNotFound,
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
    /// A change to undo is no longer in the Change Journal, as when its
    /// Calendar was deleted.
    UnitNotFound,
    /// A change to undo can't be: an Event it changed was changed since.
    ChangedSinceUnit,
    /// A change to undo was undone already.
    AlreadyUndone,
    /// Calendars can be created only in the Local Account; a server
    /// Account's come from its server.
    NotLocalAccount,
    /// A server Account's Calendar is removed from FreeCal, which leaves it
    /// on the server, rather than deleted.
    NotLocalCalendar,
    /// A Local Calendar is deleted rather than removed: it has no server to
    /// stay on.
    NotServerCalendar,
    /// A Calendar to add isn't among the Account's Available Calendars: its
    /// server no longer offers it, or it was added already.
    CalendarNotAvailable,
    /// A server's address isn't an `http://` or `https://` URL.
    InvalidServerUrl,
    /// The server didn't accept the username and password.
    WrongCredentials,
    /// The server couldn't be reached at the network level: no such host,
    /// connection refused, or no answer in time.
    ServerUnreachable,
    /// No secure connection could be made, as when the server's certificate
    /// isn't trusted.
    SecureConnectionFailed,
    /// The server answered, but not as a CalDAV server.
    NotCalDavServer,
    /// The server answered with an error `status`. `message` is the
    /// server's own text, untrusted and only ever shown as text: plain, on
    /// one line, at most 200 characters, empty unless the server sent plain
    /// text.
    ServerRefused {
        status: u16,
        message: String,
    },
    /// There is no working system keyring to keep a password in, or it
    /// stayed locked.
    KeyringUnavailable,
    /// The system keyring has no password for a server Account, as when
    /// the user deleted it there.
    PasswordNotInKeyring,
    /// The server sent FreeCal on to `url` on another server, where it
    /// doesn't take the password. `url` comes from the server, untrusted,
    /// and is ASCII, as a parsed URL is written.
    RedirectedElsewhere {
        url: String,
    },
    /// The window asked to preview or run an import before a file was
    /// picked, or after it was run or discarded.
    NoFileToImport,
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
            ErrorCode::InvalidRepeat,
            ErrorCode::OccurrenceNotFound,
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
            ErrorCode::UnitNotFound,
            ErrorCode::ChangedSinceUnit,
            ErrorCode::AlreadyUndone,
            ErrorCode::NotLocalAccount,
            ErrorCode::NotLocalCalendar,
            ErrorCode::NotServerCalendar,
            ErrorCode::CalendarNotAvailable,
            ErrorCode::InvalidServerUrl,
            ErrorCode::WrongCredentials,
            ErrorCode::ServerUnreachable,
            ErrorCode::SecureConnectionFailed,
            ErrorCode::NotCalDavServer,
            ErrorCode::ServerRefused {
                status: 403,
                message: "Access to the requested resource forbidden.".to_owned(),
            },
            ErrorCode::KeyringUnavailable,
            ErrorCode::PasswordNotInKeyring,
            ErrorCode::RedirectedElsewhere {
                url: "https://caldav.example.org/dav/".to_owned(),
            },
            ErrorCode::NoFileToImport,
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
