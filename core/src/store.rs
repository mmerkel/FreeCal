//! The Local Store: one SQLite database (ADR 0004).

use std::collections::HashSet;
use std::path::Path;

use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, Row, Transaction, TransactionBehavior, params};

use crate::event::{StoredRange, StoredWhen};
use crate::{
    Account, AccountId, Appearance, Calendar, CalendarId, Colour, CoreError, EventId, Provider,
    Result, ServerLogin, Settings, SidebarState, View,
};

/// Schema migrations, applied in order. `PRAGMA user_version` records how many
/// have run.
const MIGRATIONS: &[Migration] = &[
    Migration::Sql(
        "
    CREATE TABLE account (
        id INTEGER PRIMARY KEY,
        provider TEXT NOT NULL
    );
    INSERT INTO account (provider) VALUES ('local');
    ",
    ),
    Migration::Sql(
        "
    CREATE TABLE calendar (
        id INTEGER PRIMARY KEY,
        account_id INTEGER NOT NULL REFERENCES account (id) ON DELETE CASCADE,
        name TEXT NOT NULL,
        colour TEXT NOT NULL,
        shown INTEGER NOT NULL DEFAULT 1
    );
    CREATE INDEX calendar_account ON calendar (account_id);
    ",
    ),
    Migration::Sql(
        "
    ALTER TABLE calendar ADD COLUMN read_only INTEGER NOT NULL DEFAULT 0;
    CREATE TABLE event (
        id INTEGER PRIMARY KEY,
        calendar_id INTEGER NOT NULL REFERENCES calendar (id) ON DELETE CASCADE,
        uid TEXT NOT NULL,
        title TEXT NOT NULL,
        all_day INTEGER NOT NULL,
        start_at TEXT NOT NULL,
        end_at TEXT NOT NULL,
        time_zone TEXT,
        location TEXT NOT NULL,
        description TEXT NOT NULL,
        description_html INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX event_calendar_start ON event (calendar_id, start_at);
    ",
    ),
    Migration::Sql(
        "
    CREATE TABLE app_state (
        id INTEGER PRIMARY KEY CHECK (id = 1),
        view TEXT NOT NULL DEFAULT 'month'
    );
    INSERT INTO app_state (id) VALUES (1);
    ",
    ),
    Migration::Sql(
        "
    ALTER TABLE app_state ADD COLUMN appearance TEXT NOT NULL DEFAULT 'system';
    ",
    ),
    Migration::Code(event_times_as_wall_time),
    Migration::Sql(
        "
    ALTER TABLE app_state ADD COLUMN sidebar_shown INTEGER NOT NULL DEFAULT 1;
    ALTER TABLE app_state ADD COLUMN sidebar_width REAL;
    ",
    ),
    Migration::Sql(
        "
    CREATE UNIQUE INDEX one_local_account ON account (provider) WHERE provider = 'local';
    ",
    ),
    // Ticket 58: the pinned mark (ADR 0007) and the Change Journal's imports.
    // An entry holds the Event's previous version, or none when the import
    // created it; `event_id` has no foreign key because the Event may be
    // deleted since.
    Migration::Sql(
        "
    ALTER TABLE event ADD COLUMN zone_pinned INTEGER NOT NULL DEFAULT 0;
    CREATE INDEX event_calendar_uid ON event (calendar_id, uid);
    CREATE TABLE journal_import (
        id INTEGER PRIMARY KEY,
        calendar_id INTEGER NOT NULL REFERENCES calendar (id) ON DELETE CASCADE,
        made_at INTEGER NOT NULL
    );
    CREATE TABLE journal_entry (
        id INTEGER PRIMARY KEY,
        import_id INTEGER NOT NULL REFERENCES journal_import (id) ON DELETE CASCADE,
        event_id INTEGER NOT NULL,
        uid TEXT NOT NULL,
        created INTEGER NOT NULL,
        title TEXT,
        all_day INTEGER,
        start_at TEXT,
        end_at TEXT,
        start_zone TEXT,
        end_zone TEXT,
        location TEXT,
        description TEXT,
        description_html INTEGER,
        zone_pinned INTEGER
    );
    CREATE INDEX journal_entry_import ON journal_entry (import_id);
    ",
    ),
    // Ticket 69: the Change Journal records every change (ADR 0009). An entry
    // holds the Event's previous version with its Calendar, none when the
    // change created it, and the hash of its new version, none when the
    // change deleted it. Neither `event_id` nor `calendar_id` has a foreign
    // key, because the Event or the Calendar may be deleted since.
    Migration::Sql(
        "
    DROP TABLE journal_entry;
    DROP TABLE journal_import;
    CREATE TABLE journal_unit (
        id INTEGER PRIMARY KEY,
        origin TEXT NOT NULL,
        made_at INTEGER NOT NULL
    );
    CREATE TABLE journal_entry (
        id INTEGER PRIMARY KEY,
        unit_id INTEGER NOT NULL REFERENCES journal_unit (id) ON DELETE CASCADE,
        event_id INTEGER NOT NULL,
        uid TEXT NOT NULL,
        hash BLOB,
        undone INTEGER NOT NULL DEFAULT 0,
        calendar_id INTEGER,
        title TEXT,
        all_day INTEGER,
        start_at TEXT,
        end_at TEXT,
        start_zone TEXT,
        end_zone TEXT,
        location TEXT,
        description TEXT,
        description_html INTEGER,
        zone_pinned INTEGER
    );
    CREATE INDEX journal_entry_unit ON journal_entry (unit_id);
    CREATE INDEX journal_entry_event ON journal_entry (event_id, uid);
    ",
    ),
    Migration::Code(recurring_events),
    // Ticket 76: server Accounts. A CalDAV Account has its server's URL and
    // a username; its password lives in the keyring under `keyring_key`.
    Migration::Sql(
        "
    ALTER TABLE account ADD COLUMN url TEXT;
    ALTER TABLE account ADD COLUMN username TEXT;
    ALTER TABLE account ADD COLUMN keyring_key TEXT;
    ",
    ),
];

/// The Local Calendars a new Local Store starts with: name, colour and whether
/// it is shown. The colours are the first two of the frontend's
/// `NEW_CALENDAR_ORDER`. The names are English because v1 ships English only.
const STARTING_CALENDARS: &[(&str, &str, bool)] =
    &[("Work", "#3f51b5", true), ("Home", "#ad1457", false)];

/// Puts the starting Calendars into the Local Account of a Local Store that
/// was just created. An existing Local Store never gets them, so deleted
/// ones never come back.
fn add_starting_calendars(tx: &Transaction) -> Result<()> {
    for (name, colour, shown) in STARTING_CALENDARS {
        tx.execute(
            "INSERT INTO calendar (account_id, name, colour, shown) \
             SELECT id, ?1, ?2, ?3 FROM account WHERE provider = 'local'",
            params![name, colour, shown],
        )?;
    }
    Ok(())
}

enum Migration {
    Sql(&'static str),
    Code(fn(&Transaction) -> Result<()>),
}

/// Ticket 35 (ADR 0007): timed Events become wall times with an Event Time
/// Zone each for start and end, and the UTC instants are derived.
fn event_times_as_wall_time(tx: &Transaction) -> Result<()> {
    tx.execute_batch(
        "
        CREATE TABLE event_new (
            id INTEGER PRIMARY KEY,
            calendar_id INTEGER NOT NULL REFERENCES calendar (id) ON DELETE CASCADE,
            uid TEXT NOT NULL,
            title TEXT NOT NULL,
            all_day INTEGER NOT NULL,
            start_at TEXT NOT NULL,
            end_at TEXT NOT NULL,
            start_zone TEXT,
            end_zone TEXT,
            start_utc INTEGER NOT NULL,
            end_utc INTEGER NOT NULL,
            location TEXT NOT NULL,
            description TEXT NOT NULL,
            description_html INTEGER NOT NULL DEFAULT 0,
            CHECK (CASE WHEN all_day
                THEN start_zone IS NULL AND end_zone IS NULL
                ELSE start_zone IS NOT NULL AND end_zone IS NOT NULL END)
        );
        INSERT INTO event_new (id, calendar_id, uid, title, all_day, start_at, end_at,
            start_zone, end_zone, start_utc, end_utc, location, description, description_html)
        SELECT id, calendar_id, uid, title, all_day, start_at, end_at,
            CASE WHEN all_day THEN NULL ELSE 'UTC' END,
            CASE WHEN all_day THEN NULL ELSE 'UTC' END,
            0, 0, location, description, description_html
        FROM event;
        ",
    )?;
    let timed = readable(
        tx.prepare("SELECT id, start_at, end_at, time_zone FROM event WHERE NOT all_day")?
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })?,
    )?;
    for (id, start, end, zone) in timed {
        let when = StoredWhen::from_utc_instants(&start, &end, zone.as_deref());
        tx.execute(
            "UPDATE event_new SET start_at = ?2, end_at = ?3, start_zone = ?4, end_zone = ?5 \
             WHERE id = ?1",
            params![id, when.start, when.end, when.start_zone, when.end_zone],
        )?;
    }
    tx.execute_batch(
        "
        DROP TABLE event;
        ALTER TABLE event_new RENAME TO event;
        CREATE INDEX event_calendar_start ON event (calendar_id, start_utc);
        ALTER TABLE app_state ADD COLUMN time_zone_rules TEXT NOT NULL DEFAULT '';
        ",
    )?;
    Ok(())
}

/// Ticket 07: Recurring Events. A Recurring Event keeps its rule and its
/// cancelled Occurrences; a changed Occurrence, an Exception, is an Event of
/// its own with the UID of its Recurring Event and the recurrence ID of the
/// Occurrence it changed. The Change Journal's entries hold them too, and its
/// hashes, which now cover them, are made again.
fn recurring_events(tx: &Transaction) -> Result<()> {
    tx.execute_batch(
        "
        ALTER TABLE event ADD COLUMN rrule TEXT;
        ALTER TABLE event ADD COLUMN exdates TEXT NOT NULL DEFAULT '';
        ALTER TABLE event ADD COLUMN recurrence_id TEXT;
        ALTER TABLE journal_entry ADD COLUMN rrule TEXT;
        ALTER TABLE journal_entry ADD COLUMN exdates TEXT;
        ALTER TABLE journal_entry ADD COLUMN recurrence_id TEXT;
        UPDATE journal_entry SET exdates = '' WHERE calendar_id IS NOT NULL;
        ",
    )?;
    crate::journal::rehash(tx, 1)
}

/// The time zone rules FreeCal was built with, which the derived UTC columns
/// are computed with.
const TIME_ZONE_RULES: &str = chrono_tz::IANA_TZDB_VERSION;

/// Recomputes every Event's derived UTC instants when the Local Store's were
/// computed with other time zone rules than this build's. A row whose time
/// can't be read keeps what it had.
fn recompute_instants(tx: &Transaction) -> Result<()> {
    let rules: String = tx.query_row("SELECT time_zone_rules FROM app_state", [], |row| {
        row.get(0)
    })?;
    if rules == TIME_ZONE_RULES {
        return Ok(());
    }
    let events = readable(
        tx.prepare(&format!("SELECT id, {EVENT_FIELDS} FROM event"))?
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, event_fields(row, 1)?)))?,
    )?;
    for (id, fields) in events {
        if let Ok((start, end)) = fields.instants() {
            tx.execute(
                "UPDATE event SET start_utc = ?2, end_utc = ?3 WHERE id = ?1",
                params![id, start, end],
            )?;
        }
    }
    tx.execute(
        "UPDATE app_state SET time_zone_rules = ?1",
        [TIME_ZONE_RULES],
    )?;
    Ok(())
}

/// The rows whose values have the types asked for. A row with a value of
/// another type is skipped rather than keeping the Local Store from opening;
/// it reads as unreadable later, like any value FreeCal didn't write.
fn readable<T>(rows: impl Iterator<Item = rusqlite::Result<T>>) -> Result<Vec<T>> {
    let mut readable = Vec::new();
    for row in rows {
        match row {
            Ok(row) => readable.push(row),
            Err(
                rusqlite::Error::InvalidColumnType(..)
                | rusqlite::Error::FromSqlConversionFailure(..)
                | rusqlite::Error::IntegralValueOutOfRange(..),
            ) => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(readable)
}

/// The columns [`event_fields`] reads, in its order, which the Change
/// Journal's entries share with the `event` table.
pub(crate) const EVENT_FIELDS: &str = "title, all_day, start_at, end_at, start_zone, end_zone, location, \
     description, description_html, zone_pinned, rrule, exdates, recurrence_id";

/// Reads [`EVENT_FIELDS`] starting at column `first`.
pub(crate) fn event_fields(row: &Row, first: usize) -> rusqlite::Result<EventFields> {
    Ok(EventFields {
        title: row.get(first)?,
        when: stored_when(row, first + 1)?,
        location: row.get(first + 6)?,
        description: row.get(first + 7)?,
        description_html: row.get(first + 8)?,
        zone_pinned: row.get(first + 9)?,
        rule: row.get(first + 10)?,
        exdates: row.get(first + 11)?,
        recurrence_id: row.get(first + 12)?,
    })
}

/// Reads an Event's `all_day, start_at, end_at, start_zone, end_zone`
/// starting at column `first`.
fn stored_when(row: &Row, first: usize) -> rusqlite::Result<StoredWhen> {
    Ok(StoredWhen {
        all_day: row.get(first)?,
        start: row.get(first + 1)?,
        end: row.get(first + 2)?,
        start_zone: row.get(first + 3)?,
        end_zone: row.get(first + 4)?,
    })
}

pub struct Store {
    conn: Connection,
}

/// An Event's fields as the Local Store holds them.
#[derive(Debug, Clone)]
pub struct EventFields {
    pub title: String,
    pub when: StoredWhen,
    pub location: String,
    /// The description exactly as it was received or written.
    pub description: String,
    /// Whether `description` is HTML (from Google or an import) rather than
    /// plain text.
    pub description_html: bool,
    /// Whether the Event arrived with a floating time that was pinned to its
    /// Event Time Zone (ADR 0007).
    pub zone_pinned: bool,
    /// A Recurring Event's repeat rule, an RFC 5545 `RRULE` value.
    pub rule: Option<String>,
    /// A Recurring Event's cancelled Occurrences: their recurrence IDs,
    /// separated by commas.
    pub exdates: String,
    /// For an Exception, the recurrence ID of the Occurrence it changed.
    pub recurrence_id: Option<String>,
}

impl EventFields {
    /// The derived start and end as Unix seconds, for a Recurring Event
    /// those of all its Occurrences together.
    pub fn instants(&self) -> Result<(i64, i64)> {
        crate::recurrence::instants(self)
    }
}

#[derive(Debug, Clone)]
pub struct StoredEvent {
    pub calendar_id: CalendarId,
    pub fields: EventFields,
}

/// An Event in a date range.
pub struct RangeEvent {
    pub id: EventId,
    pub calendar_id: CalendarId,
    pub uid: String,
    pub fields: EventFields,
    pub start_utc: i64,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let mut conn = Connection::open(path)?;
        conn.pragma_update(None, "foreign_keys", true)?;
        migrate(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn list_accounts(&self) -> Result<Vec<Account>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, provider, url, username FROM account ORDER BY id")?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows.into_iter()
            .map(|(id, provider, url, username)| {
                Ok(Account {
                    id: AccountId(id),
                    provider: Provider::parse(&provider)
                        .ok_or(CoreError::UnknownProvider(provider))?,
                    server: url
                        .zip(username)
                        .map(|(url, username)| ServerLogin { url, username }),
                })
            })
            .collect()
    }

    /// Adds a server Account whose secret is in the keyring under
    /// `keyring_key`.
    pub fn insert_server_account(
        &self,
        provider: Provider,
        server: &ServerLogin,
        keyring_key: &str,
    ) -> Result<Account> {
        self.conn.execute(
            "INSERT INTO account (provider, url, username, keyring_key) VALUES (?1, ?2, ?3, ?4)",
            params![provider.as_str(), server.url, server.username, keyring_key],
        )?;
        Ok(Account {
            id: AccountId(self.conn.last_insert_rowid()),
            provider,
            server: Some(server.clone()),
        })
    }

    /// Where the keyring keeps the Account's secret; none for the Local
    /// Account.
    pub fn account_keyring_key(&self, id: AccountId) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT keyring_key FROM account WHERE id = ?1",
                [id.0],
                |row| row.get(0),
            )
            .optional()?
            .flatten())
    }

    /// Deletes an Account with its Calendars and their Events.
    pub fn delete_account(&self, id: AccountId) -> Result<()> {
        self.conn
            .execute("DELETE FROM account WHERE id = ?1", [id.0])?;
        Ok(())
    }

    pub fn find_account(&self, id: AccountId) -> Result<Option<Account>> {
        Ok(self
            .list_accounts()?
            .into_iter()
            .find(|account| account.id == id))
    }

    pub fn list_calendars(&self) -> Result<Vec<Calendar>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, account_id, name, colour, shown, read_only FROM calendar ORDER BY id",
        )?;
        let calendars = stmt
            .query_map([], |row| {
                Ok(Calendar {
                    id: CalendarId(row.get(0)?),
                    account_id: AccountId(row.get(1)?),
                    name: row.get(2)?,
                    // A stored colour is untrusted like every stored field.
                    colour: Colour::parse(&row.get::<_, String>(3)?).unwrap_or_default(),
                    shown: row.get(4)?,
                    read_only: row.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(calendars)
    }

    pub fn insert_calendar(
        &self,
        account_id: AccountId,
        name: &str,
        colour: &Colour,
    ) -> Result<Calendar> {
        self.conn.execute(
            "INSERT INTO calendar (account_id, name, colour) VALUES (?1, ?2, ?3)",
            params![account_id.0, name, colour.as_str()],
        )?;
        Ok(Calendar {
            id: CalendarId(self.conn.last_insert_rowid()),
            account_id,
            name: name.to_owned(),
            colour: colour.clone(),
            shown: true,
            read_only: false,
        })
    }

    pub fn find_calendar(&self, id: CalendarId) -> Result<Calendar> {
        self.list_calendars()?
            .into_iter()
            .find(|calendar| calendar.id == id)
            .ok_or(CoreError::CalendarNotFound(id))
    }

    pub fn set_calendar_name(&self, id: CalendarId, name: &str) -> Result<()> {
        self.update_calendar("UPDATE calendar SET name = ?2 WHERE id = ?1", id, name)
    }

    pub fn set_calendar_colour(&self, id: CalendarId, colour: &Colour) -> Result<()> {
        self.update_calendar(
            "UPDATE calendar SET colour = ?2 WHERE id = ?1",
            id,
            colour.as_str(),
        )
    }

    pub fn set_calendar_shown(&self, id: CalendarId, shown: bool) -> Result<()> {
        self.update_calendar("UPDATE calendar SET shown = ?2 WHERE id = ?1", id, shown)
    }

    /// Shows `id` and hides every other Calendar, in one statement so that
    /// nothing changes when `id` doesn't exist.
    pub fn show_only_calendar(&self, id: CalendarId) -> Result<()> {
        match self.conn.execute(
            "UPDATE calendar SET shown = (id = ?1) \
             WHERE EXISTS (SELECT 1 FROM calendar WHERE id = ?1)",
            [id.0],
        )? {
            0 => Err(CoreError::CalendarNotFound(id)),
            _ => Ok(()),
        }
    }

    /// The connection, for the Change Journal module, which alone writes
    /// Events and the Journal.
    pub(crate) fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Runs `change` in one transaction: if it fails, nothing it did is kept.
    pub fn transaction<T>(&self, change: impl FnOnce() -> Result<T>) -> Result<T> {
        let tx = self.conn.unchecked_transaction()?;
        let done = change()?;
        tx.commit()?;
        Ok(done)
    }

    pub fn find_event(&self, id: EventId) -> Result<StoredEvent> {
        self.conn
            .query_row(
                &format!("SELECT calendar_id, {EVENT_FIELDS} FROM event WHERE id = ?1"),
                [id.0],
                |row| {
                    Ok(StoredEvent {
                        calendar_id: CalendarId(row.get(0)?),
                        fields: event_fields(row, 1)?,
                    })
                },
            )
            .optional()?
            .ok_or(CoreError::EventNotFound(id))
    }

    /// The Event of a Calendar with the UID `uid`, the first if the Local
    /// Store has several, and not an Exception.
    pub fn find_event_by_uid(
        &self,
        calendar_id: CalendarId,
        uid: &str,
    ) -> Result<Option<(EventId, EventFields)>> {
        Ok(self
            .conn
            .query_row(
                &format!(
                    "SELECT id, {EVENT_FIELDS} FROM event WHERE calendar_id = ?1 AND uid = ?2 \
                     AND recurrence_id IS NULL ORDER BY id LIMIT 1"
                ),
                params![calendar_id.0, uid],
                |row| Ok((EventId(row.get(0)?), event_fields(row, 1)?)),
            )
            .optional()?)
    }

    /// The Exceptions of a Calendar with the UID `uid`, in the order they
    /// were added.
    pub fn exceptions(
        &self,
        calendar_id: CalendarId,
        uid: &str,
    ) -> Result<Vec<(EventId, EventFields)>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT id, {EVENT_FIELDS} FROM event WHERE calendar_id = ?1 AND uid = ?2 \
             AND recurrence_id IS NOT NULL ORDER BY id"
        ))?;
        let exceptions = stmt
            .query_map(params![calendar_id.0, uid], |row| {
                Ok((EventId(row.get(0)?), event_fields(row, 1)?))
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(exceptions)
    }

    /// The UID of the Event `id`.
    pub fn event_uid(&self, id: EventId) -> Result<String> {
        self.conn
            .query_row("SELECT uid FROM event WHERE id = ?1", [id.0], |row| {
                row.get(0)
            })
            .optional()?
            .ok_or(CoreError::EventNotFound(id))
    }

    /// The UIDs of a Calendar's Events.
    pub fn event_uids(&self, calendar_id: CalendarId) -> Result<HashSet<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT uid FROM event WHERE calendar_id = ?1")?;
        let uids = readable(stmt.query_map([calendar_id.0], |row| row.get(0))?)?;
        Ok(uids.into_iter().collect())
    }

    /// The Events of shown Calendars that overlap `range`, found by their
    /// derived UTC instants alone, which for a Recurring Event span all its
    /// Occurrences. An Event of no length counts when it starts inside the
    /// range (see [`StoredRange::overlaps`]).
    pub fn shown_events_in(&self, range: &StoredRange) -> Result<Vec<RangeEvent>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT event.id, event.calendar_id, event.uid, event.start_utc, \
             {} \
             FROM event JOIN calendar ON calendar.id = event.calendar_id \
             WHERE calendar.shown AND CASE event.all_day \
               WHEN 1 THEN event.start_utc < ?2 AND event.end_utc > ?1 \
               ELSE event.start_utc < ?4 AND (event.end_utc > ?3 \
                 OR (event.end_utc = event.start_utc AND event.start_utc >= ?3)) \
             END",
            EVENT_FIELDS
                .split(", ")
                .map(|column| format!("event.{}", column.trim()))
                .collect::<Vec<_>>()
                .join(", ")
        ))?;
        let events = stmt
            .query_map(
                params![
                    range.from_day_seconds,
                    range.to_day_seconds,
                    range.from_instant,
                    range.to_instant
                ],
                |row| {
                    Ok(RangeEvent {
                        id: EventId(row.get(0)?),
                        calendar_id: CalendarId(row.get(1)?),
                        uid: row.get(2)?,
                        start_utc: row.get(3)?,
                        fields: event_fields(row, 4)?,
                    })
                },
            )?
            .collect::<rusqlite::Result<_>>()?;
        Ok(events)
    }

    pub fn last_used_view(&self) -> Result<View> {
        let stored: String = self
            .conn
            .query_row("SELECT view FROM app_state", [], |row| row.get(0))?;
        Ok(View::parse(&stored))
    }

    pub fn set_last_used_view(&self, view: View) -> Result<()> {
        self.conn
            .execute("UPDATE app_state SET view = ?1", [view.as_str()])?;
        Ok(())
    }

    pub fn sidebar(&self) -> Result<SidebarState> {
        let (shown, width) = self.conn.query_row(
            "SELECT sidebar_shown, sidebar_width FROM app_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Ok(sidebar_state(shown, width))
    }

    pub fn set_sidebar(&self, sidebar: SidebarState) -> Result<()> {
        self.conn.execute(
            "UPDATE app_state SET sidebar_shown = ?1, sidebar_width = ?2",
            params![sidebar.shown, sidebar.width],
        )?;
        Ok(())
    }

    pub fn settings(&self) -> Result<Settings> {
        let appearance: String =
            self.conn
                .query_row("SELECT appearance FROM app_state", [], |row| row.get(0))?;
        Ok(Settings {
            appearance: Appearance::parse(&appearance),
        })
    }

    pub fn set_appearance(&self, appearance: Appearance) -> Result<()> {
        self.conn.execute(
            "UPDATE app_state SET appearance = ?1",
            [appearance.as_str()],
        )?;
        Ok(())
    }

    fn update_calendar(
        &self,
        sql: &str,
        id: CalendarId,
        value: impl rusqlite::ToSql,
    ) -> Result<()> {
        match self.conn.execute(sql, params![id.0, value])? {
            0 => Err(CoreError::CalendarNotFound(id)),
            _ => Ok(()),
        }
    }
}

/// Reads the sidebar's stored columns, which are untrusted like every stored
/// field: anything but 0 shows the sidebar, and anything but a positive number
/// is the default width.
fn sidebar_state(shown: Value, width: Value) -> SidebarState {
    let width = match width {
        Value::Real(width) if width.is_finite() && width > 0.0 => Some(width),
        Value::Integer(width) if width > 0 => Some(width as f64),
        _ => None,
    };
    SidebarState {
        shown: !matches!(shown, Value::Integer(0)),
        width,
    }
}

fn migrate(conn: &mut Connection) -> Result<()> {
    // IMMEDIATE takes the write lock before `user_version` is read, so two
    // FreeCal processes opening a fresh Local Store at once can't both see
    // version 0 and both try to create the schema.
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let applied: i64 = tx.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let known = MIGRATIONS.len() as i64;
    if applied > known {
        return Err(CoreError::NewerLocalStore {
            found: applied,
            known,
        });
    }
    for (index, migration) in MIGRATIONS.iter().enumerate().skip(applied as usize) {
        match migration {
            Migration::Sql(sql) => tx.execute_batch(sql)?,
            Migration::Code(migrate) => migrate(&tx)?,
        }
        tx.pragma_update(None, "user_version", index as i64 + 1)?;
    }
    if applied == 0 {
        add_starting_calendars(&tx)?;
    }
    recompute_instants(&tx)?;
    tx.commit()?;
    Ok(())
}
