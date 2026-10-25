//! The Local Store: one SQLite database (ADR 0004).

use std::collections::{HashMap, HashSet};
use std::path::Path;

use rusqlite::types::Value;
use rusqlite::{
    Connection, OptionalExtension, Row, Transaction, TransactionBehavior, params, params_from_iter,
};

use crate::event::{EventFields, ObjectId, ServerObject, StoredRange, StoredWhen, rule};
use crate::remote::{Held, SyncPoint};
use crate::{
    Account, AccountId, Appearance, AvailableCalendar, Calendar, CalendarId, Colour, CoreError,
    EventId, Provider, Result, ServerLogin, Settings, SidebarState, View,
};

/// Schema migrations, applied in order. `PRAGMA user_version` records how many
/// have run.
const MIGRATIONS: &[&str] = &[
    // The baseline: the schema as it was when the migrations of development
    // were squashed (ticket 63, ADR 0008). The Local Account is the one row
    // a new Local Store starts with; `app_state` has exactly one row.
    //
    // A Calendar's `read_only` is the user's mark, `server_read_only` whether
    // its server doesn't allow changing it. A server Account's Calendar has
    // the `url` it lives at, which names its Available Calendar; whether its
    // first sync finished; and the sync token (RFC 6578) its server gave for
    // the last sync, if it gives them. A CalDAV Account's password lives in
    // the keyring under `keyring_key`.
    //
    // An Event keeps wall times with an Event Time Zone each for start and
    // end, with the UTC instants derived (ADR 0007). AUTOINCREMENT keeps an
    // Event's ID from being given out again, so that the Change Journal
    // never takes two Events for one. An Exception is an Event with the UID
    // of its Recurring Event and the recurrence ID of the Occurrence it
    // changed.
    //
    // Server objects mirror the server, with their data as received in its
    // `format`, `pending` once FreeCal has changes to send, and `unreadable`
    // once FreeCal couldn't read the server's version, so that the mirror
    // holds the last version it could (ADR 0010).
    //
    // A Change Journal entry holds the Event's previous version with its
    // Calendar, none when the change created it, and the hash of its new
    // version, none when the change deleted it (ADR 0009). None of its
    // `event_id`, `calendar_id` and `object_id` has a foreign key, because
    // what they name may be gone since.
    "
    CREATE TABLE account (
        id INTEGER PRIMARY KEY,
        provider TEXT NOT NULL,
        url TEXT,
        username TEXT,
        keyring_key TEXT
    );
    CREATE UNIQUE INDEX one_local_account ON account (provider) WHERE provider = 'local';
    INSERT INTO account (provider) VALUES ('local');
    CREATE TABLE calendar (
        id INTEGER PRIMARY KEY,
        account_id INTEGER NOT NULL REFERENCES account (id) ON DELETE CASCADE,
        name TEXT NOT NULL,
        colour TEXT NOT NULL,
        shown INTEGER NOT NULL DEFAULT 1,
        read_only INTEGER NOT NULL DEFAULT 0,
        url TEXT,
        synced INTEGER NOT NULL DEFAULT 0,
        sync_token TEXT,
        server_read_only INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX calendar_account ON calendar (account_id);
    CREATE UNIQUE INDEX calendar_account_url ON calendar (account_id, url) WHERE url IS NOT NULL;
    CREATE TABLE server_object (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        calendar_id INTEGER NOT NULL REFERENCES calendar (id) ON DELETE CASCADE,
        href TEXT NOT NULL,
        etag TEXT,
        data TEXT NOT NULL,
        format TEXT NOT NULL,
        pending INTEGER NOT NULL DEFAULT 0,
        unreadable INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX server_object_calendar_href ON server_object (calendar_id, href);
    CREATE TABLE event (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
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
        zone_pinned INTEGER NOT NULL DEFAULT 0,
        rrule TEXT,
        exdates TEXT NOT NULL DEFAULT '',
        recurrence_id TEXT,
        object_id INTEGER REFERENCES server_object (id) ON DELETE SET NULL,
        CHECK (CASE WHEN all_day
            THEN start_zone IS NULL AND end_zone IS NULL
            ELSE start_zone IS NOT NULL AND end_zone IS NOT NULL END)
    );
    CREATE INDEX event_calendar_start ON event (calendar_id, start_utc);
    CREATE INDEX event_calendar_uid ON event (calendar_id, uid);
    CREATE INDEX event_object ON event (object_id);
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
        zone_pinned INTEGER,
        rrule TEXT,
        exdates TEXT,
        recurrence_id TEXT,
        object_id INTEGER
    );
    CREATE INDEX journal_entry_unit ON journal_entry (unit_id);
    CREATE INDEX journal_entry_event ON journal_entry (event_id, uid);
    CREATE TABLE app_state (
        id INTEGER PRIMARY KEY CHECK (id = 1),
        view TEXT NOT NULL DEFAULT 'month',
        appearance TEXT NOT NULL DEFAULT 'system',
        time_zone_rules TEXT NOT NULL DEFAULT '',
        sidebar_shown INTEGER NOT NULL DEFAULT 1,
        sidebar_width REAL
    );
    INSERT INTO app_state (id) VALUES (1);
    ",
];

/// Whether a newly added server Calendar starts marked read-only, so that
/// the user can watch sync work before trusting FreeCal to write (US-18).
/// Until 1.0.
const NEW_SERVER_CALENDARS_READ_ONLY: bool = true;

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

/// How a server object's data is written: iCalendar, as CalDAV holds it.
/// Google's will be its JSON as received (ADR 0010).
pub(crate) const ICALENDAR: &str = "icalendar";

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
        if let Ok((start, end)) = rule::span(&fields) {
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
     description, description_html, zone_pinned, rrule, exdates, recurrence_id, object_id";

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
        object_id: row.get::<_, Option<i64>>(first + 13)?.map(ObjectId),
    })
}

/// An Event read as `id, calendar_id, uid,` then [`EVENT_FIELDS`].
fn uid_event(row: &Row) -> rusqlite::Result<UidEvent> {
    Ok(UidEvent {
        id: EventId(row.get(0)?),
        calendar_id: CalendarId(row.get(1)?),
        uid: row.get(2)?,
        fields: event_fields(row, 3)?,
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

/// Which Events [`Store::events_with_uid`] gives.
#[derive(Clone, Copy)]
pub enum Uid<'a> {
    /// Those with the Calendar and UID of the Event.
    Of(EventId),
    /// Those of the Calendar with the UID.
    In(CalendarId, &'a str),
}

/// An Event with its Calendar and UID.
pub struct UidEvent {
    pub id: EventId,
    pub calendar_id: CalendarId,
    pub uid: String,
    pub fields: EventFields,
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

    pub fn find_account(&self, id: AccountId) -> Result<Option<Account>> {
        Ok(self
            .list_accounts()?
            .into_iter()
            .find(|account| account.id == id))
    }

    pub fn list_calendars(&self) -> Result<Vec<Calendar>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, account_id, name, colour, shown, read_only, server_read_only \
             FROM calendar ORDER BY id",
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
                    read_only: row.get::<_, bool>(5)? || row.get::<_, bool>(6)?,
                    server_read_only: row.get(6)?,
                    first_sync: false,
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
            server_read_only: false,
            first_sync: false,
        })
    }

    /// Adds an Available Calendar as a server Account's Calendar, in the
    /// default colour if it has none, read-only if its server says so.
    pub fn insert_server_calendar(
        &self,
        account_id: AccountId,
        available: &AvailableCalendar,
    ) -> Result<Calendar> {
        let colour = available.colour.clone().unwrap_or_default();
        self.conn.execute(
            "INSERT INTO calendar (account_id, url, name, colour, read_only, server_read_only) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                account_id.0,
                available.url,
                available.name,
                colour.as_str(),
                NEW_SERVER_CALENDARS_READ_ONLY,
                available.read_only
            ],
        )?;
        self.find_calendar(CalendarId(self.conn.last_insert_rowid()))
    }

    /// Where an Account's Calendars live on its server; none for the Local
    /// Account.
    pub fn calendar_urls(&self, account_id: AccountId) -> Result<HashSet<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT url FROM calendar WHERE account_id = ?1 AND url IS NOT NULL")?;
        let urls = readable(stmt.query_map([account_id.0], |row| row.get(0))?)?;
        Ok(urls.into_iter().collect())
    }

    /// Each Calendar of a server Account with its address on the server and
    /// how far it is synced: none until its first sync finished.
    pub fn server_calendars(
        &self,
        account_id: AccountId,
    ) -> Result<Vec<(CalendarId, String, Option<SyncPoint>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, url, synced, sync_token FROM calendar \
             WHERE account_id = ?1 AND url IS NOT NULL ORDER BY id",
        )?;
        readable(stmt.query_map([account_id.0], |row| {
            let synced: bool = row.get(2)?;
            Ok((
                CalendarId(row.get(0)?),
                row.get(1)?,
                synced
                    .then(|| row.get(3).map(|token| SyncPoint { token }))
                    .transpose()?,
            ))
        })?)
    }

    /// Records that a Calendar's sync finished, with the sync token its
    /// server gave, if any.
    pub fn set_synced(&self, id: CalendarId, token: Option<&str>) -> Result<()> {
        self.update_calendar(
            "UPDATE calendar SET synced = 1, sync_token = ?2 WHERE id = ?1",
            id,
            token,
        )
    }

    /// The server objects of a Calendar, by their address as the server
    /// gave it: the first at each, should it have several.
    pub fn server_objects(&self, calendar_id: CalendarId) -> Result<HashMap<String, Held>> {
        let mut stmt = self.conn.prepare(
            "SELECT href, etag, unreadable FROM server_object WHERE calendar_id = ?1 \
             ORDER BY id DESC",
        )?;
        let objects = readable(stmt.query_map([calendar_id.0], |row| {
            Ok((
                row.get(0)?,
                Held {
                    etag: row.get(1)?,
                    unreadable: row.get(2)?,
                },
            ))
        })?)?;
        Ok(objects.into_iter().collect())
    }

    /// The server objects of a Calendar at `href`, in the order they were
    /// added.
    pub fn objects_at(&self, calendar_id: CalendarId, href: &str) -> Result<Vec<ObjectId>> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM server_object WHERE calendar_id = ?1 AND href = ?2 ORDER BY id",
        )?;
        let objects = stmt
            .query_map(params![calendar_id.0, href], |row| row.get(0).map(ObjectId))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(objects)
    }

    /// The rows of the server object `id`, in the order they were added.
    pub fn events_in_object(&self, id: ObjectId) -> Result<Vec<UidEvent>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT id, calendar_id, uid, {EVENT_FIELDS} FROM event WHERE object_id = ?1 \
             ORDER BY id"
        ))?;
        let events = stmt
            .query_map([id.0], uid_event)?
            .collect::<rusqlite::Result<_>>()?;
        Ok(events)
    }

    /// The server object `id` as the server last gave it, none if it is
    /// gone.
    pub fn server_object(&self, id: ObjectId) -> Result<Option<ServerObject>> {
        Ok(self
            .conn
            .query_row(
                "SELECT href, etag, data FROM server_object WHERE id = ?1",
                [id.0],
                |row| {
                    Ok(ServerObject {
                        href: row.get(0)?,
                        etag: row.get(1)?,
                        data: row.get(2)?,
                    })
                },
            )
            .optional()?)
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

    /// Every Event of a Calendar with a UID, Exceptions too, in the order
    /// they were added: those of `uid`, or those sharing an Event's.
    pub fn events_with_uid(&self, uid: Uid) -> Result<Vec<UidEvent>> {
        let (condition, key) = match uid {
            Uid::Of(id) => (
                "(calendar_id, uid) = (SELECT calendar_id, uid FROM event WHERE id = ?1)",
                vec![Value::Integer(id.0)],
            ),
            Uid::In(calendar_id, uid) => (
                "calendar_id = ?1 AND uid = ?2",
                vec![Value::Integer(calendar_id.0), Value::Text(uid.to_owned())],
            ),
        };
        let mut stmt = self.conn.prepare(&format!(
            "SELECT id, calendar_id, uid, {EVENT_FIELDS} FROM event WHERE {condition} ORDER BY id"
        ))?;
        let events = stmt
            .query_map(params_from_iter(key), uid_event)?
            .collect::<rusqlite::Result<_>>()?;
        Ok(events)
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
        tx.execute_batch(migration)?;
        tx.pragma_update(None, "user_version", index as i64 + 1)?;
    }
    if applied == 0 {
        add_starting_calendars(&tx)?;
    }
    recompute_instants(&tx)?;
    tx.commit()?;
    Ok(())
}
