//! The Local Store: one SQLite database (ADR 0004).

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Row, Transaction, TransactionBehavior, params};

use crate::event::{StoredRange, StoredWhen};
use crate::{
    Account, AccountId, Appearance, Calendar, CalendarId, Colour, CoreError, EventId, Provider,
    Result, Settings, View,
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
];

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
        tx.prepare("SELECT id, all_day, start_at, end_at, start_zone, end_zone FROM event")?
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, stored_when(row, 1)?)))?,
    )?;
    for (id, when) in events {
        if let Ok((start, end)) = when.instants() {
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
}

#[derive(Debug, Clone)]
pub struct StoredEvent {
    pub calendar_id: CalendarId,
    pub fields: EventFields,
}

/// What the grid needs of an Event in a date range.
pub struct RangeEvent {
    pub id: EventId,
    pub calendar_id: CalendarId,
    pub title: String,
    pub when: StoredWhen,
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
            .prepare("SELECT id, provider FROM account ORDER BY id")?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get::<_, String>(1)?)))?
            .collect::<rusqlite::Result<Vec<(i64, String)>>>()?;
        rows.into_iter()
            .map(|(id, provider)| {
                Ok(Account {
                    id: AccountId(id),
                    provider: Provider::parse(&provider)
                        .ok_or(CoreError::UnknownProvider(provider))?,
                })
            })
            .collect()
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

    pub fn delete_calendar(&self, id: CalendarId) -> Result<()> {
        match self
            .conn
            .execute("DELETE FROM calendar WHERE id = ?1", [id.0])?
        {
            0 => Err(CoreError::CalendarNotFound(id)),
            _ => Ok(()),
        }
    }

    pub fn insert_event(&self, calendar_id: CalendarId, fields: &EventFields) -> Result<EventId> {
        let (start_utc, end_utc) = fields.when.instants()?;
        self.conn.execute(
            "INSERT INTO event (calendar_id, uid, title, all_day, start_at, end_at, start_zone, \
             end_zone, start_utc, end_utc, location, description, description_html) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                calendar_id.0,
                uuid::Uuid::new_v4().to_string(),
                fields.title,
                fields.when.all_day,
                fields.when.start,
                fields.when.end,
                fields.when.start_zone,
                fields.when.end_zone,
                start_utc,
                end_utc,
                fields.location,
                fields.description,
                fields.description_html,
            ],
        )?;
        Ok(EventId(self.conn.last_insert_rowid()))
    }

    pub fn find_event(&self, id: EventId) -> Result<StoredEvent> {
        self.conn
            .query_row(
                "SELECT calendar_id, title, all_day, start_at, end_at, start_zone, end_zone, \
                 location, description, description_html FROM event WHERE id = ?1",
                [id.0],
                |row| {
                    Ok(StoredEvent {
                        calendar_id: CalendarId(row.get(0)?),
                        fields: EventFields {
                            title: row.get(1)?,
                            when: stored_when(row, 2)?,
                            location: row.get(7)?,
                            description: row.get(8)?,
                            description_html: row.get(9)?,
                        },
                    })
                },
            )
            .optional()?
            .ok_or(CoreError::EventNotFound(id))
    }

    pub fn update_event(&self, id: EventId, event: &StoredEvent) -> Result<()> {
        let fields = &event.fields;
        let (start_utc, end_utc) = fields.when.instants()?;
        match self.conn.execute(
            "UPDATE event SET calendar_id = ?2, title = ?3, all_day = ?4, start_at = ?5, \
             end_at = ?6, start_zone = ?7, end_zone = ?8, start_utc = ?9, end_utc = ?10, \
             location = ?11, description = ?12, description_html = ?13 WHERE id = ?1",
            params![
                id.0,
                event.calendar_id.0,
                fields.title,
                fields.when.all_day,
                fields.when.start,
                fields.when.end,
                fields.when.start_zone,
                fields.when.end_zone,
                start_utc,
                end_utc,
                fields.location,
                fields.description,
                fields.description_html,
            ],
        )? {
            0 => Err(CoreError::EventNotFound(id)),
            _ => Ok(()),
        }
    }

    pub fn delete_event(&self, id: EventId) -> Result<()> {
        match self
            .conn
            .execute("DELETE FROM event WHERE id = ?1", [id.0])?
        {
            0 => Err(CoreError::EventNotFound(id)),
            _ => Ok(()),
        }
    }

    /// The Events of shown Calendars that overlap `range`, found by their
    /// derived UTC instants alone. An Event of no length counts when it
    /// starts inside the range.
    pub fn shown_events_in(&self, range: &StoredRange) -> Result<Vec<RangeEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT event.id, event.calendar_id, event.title, event.all_day, event.start_at, \
             event.end_at, event.start_zone, event.end_zone, event.start_utc \
             FROM event JOIN calendar ON calendar.id = event.calendar_id \
             WHERE calendar.shown AND CASE event.all_day \
               WHEN 1 THEN event.start_utc < ?2 AND event.end_utc > ?1 \
               ELSE event.start_utc < ?4 AND (event.end_utc > ?3 \
                 OR (event.end_utc = event.start_utc AND event.start_utc >= ?3)) \
             END",
        )?;
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
                        title: row.get(2)?,
                        when: stored_when(row, 3)?,
                        start_utc: row.get(8)?,
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
    recompute_instants(&tx)?;
    tx.commit()?;
    Ok(())
}
