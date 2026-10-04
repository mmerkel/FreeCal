//! The Local Store: one SQLite database (ADR 0004).

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::event::{StoredRange, StoredWhen};
use crate::{
    Account, AccountId, Calendar, CalendarId, Colour, CoreError, EventId, Provider, Result,
};

/// Schema migrations, applied in order. `PRAGMA user_version` records how many
/// have run.
const MIGRATIONS: &[&str] = &[
    "
    CREATE TABLE account (
        id INTEGER PRIMARY KEY,
        provider TEXT NOT NULL
    );
    INSERT INTO account (provider) VALUES ('local');
    ",
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
];

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
        let mut stmt = self
            .conn
            .prepare(
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
        self.conn.execute(
            "INSERT INTO event (calendar_id, uid, title, all_day, start_at, end_at, time_zone, \
             location, description, description_html) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                calendar_id.0,
                uuid::Uuid::new_v4().to_string(),
                fields.title,
                fields.when.all_day,
                fields.when.start,
                fields.when.end,
                fields.when.time_zone,
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
                "SELECT calendar_id, title, all_day, start_at, end_at, time_zone, location, \
                 description, description_html FROM event WHERE id = ?1",
                [id.0],
                |row| {
                    Ok(StoredEvent {
                        calendar_id: CalendarId(row.get(0)?),
                        fields: EventFields {
                            title: row.get(1)?,
                            when: StoredWhen {
                                all_day: row.get(2)?,
                                start: row.get(3)?,
                                end: row.get(4)?,
                                time_zone: row.get(5)?,
                            },
                            location: row.get(6)?,
                            description: row.get(7)?,
                            description_html: row.get(8)?,
                        },
                    })
                },
            )
            .optional()?
            .ok_or(CoreError::EventNotFound(id))
    }

    pub fn update_event(&self, id: EventId, event: &StoredEvent) -> Result<()> {
        let fields = &event.fields;
        match self.conn.execute(
            "UPDATE event SET calendar_id = ?2, title = ?3, all_day = ?4, start_at = ?5, \
             end_at = ?6, time_zone = ?7, location = ?8, description = ?9, \
             description_html = ?10 WHERE id = ?1",
            params![
                id.0,
                event.calendar_id.0,
                fields.title,
                fields.when.all_day,
                fields.when.start,
                fields.when.end,
                fields.when.time_zone,
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
        match self.conn.execute("DELETE FROM event WHERE id = ?1", [id.0])? {
            0 => Err(CoreError::EventNotFound(id)),
            _ => Ok(()),
        }
    }

    /// The Events of shown Calendars that overlap `range`. An Event of no
    /// length counts when it starts inside the range.
    pub fn shown_events_in(&self, range: &StoredRange) -> Result<Vec<RangeEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT event.id, event.calendar_id, event.title, event.all_day, event.start_at, \
             event.end_at, event.time_zone \
             FROM event JOIN calendar ON calendar.id = event.calendar_id \
             WHERE calendar.shown AND CASE event.all_day \
               WHEN 1 THEN event.start_at < ?2 AND event.end_at > ?1 \
               ELSE event.start_at < ?4 AND (event.end_at > ?3 \
                 OR (event.end_at = event.start_at AND event.start_at >= ?3)) \
             END",
        )?;
        let events = stmt
            .query_map(
                params![
                    range.from_date,
                    range.to_date,
                    range.from_instant,
                    range.to_instant
                ],
                |row| {
                    Ok(RangeEvent {
                        id: EventId(row.get(0)?),
                        calendar_id: CalendarId(row.get(1)?),
                        title: row.get(2)?,
                        when: StoredWhen {
                            all_day: row.get(3)?,
                            start: row.get(4)?,
                            end: row.get(5)?,
                            time_zone: row.get(6)?,
                        },
                    })
                },
            )?
            .collect::<rusqlite::Result<_>>()?;
        Ok(events)
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
        tx.execute_batch(migration)?;
        tx.pragma_update(None, "user_version", index as i64 + 1)?;
    }
    tx.commit()?;
    Ok(())
}
