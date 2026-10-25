//! Every query and write of the Local Store, but its schema: the SQL lives
//! here and nowhere else.
//!
//! Writing the tables the Change Journal guards (ADR 0011), `event`,
//! `server_object`, `journal_unit` and `journal_entry`, takes a
//! [`Journaled`], which only [`journal`](super::journal) can make: so the
//! compiler keeps everything else from writing them. A new write to one of
//! those tables takes one too.

use std::collections::{HashMap, HashSet};

use rusqlite::types::Value;
use rusqlite::{OptionalExtension, Row, params, params_from_iter};

use super::journal::{Entry, Journaled};
use super::{RangeEvent, Store, StoredEvent, Uid, UidEvent, UnitId};
use crate::event::{EventFields, ObjectId, ServerObject, StoredRange, StoredWhen};
use crate::remote::{Held, SyncPoint};
use crate::{
    Account, AccountId, Appearance, AvailableCalendar, Calendar, CalendarId, Colour, CoreError,
    EventId, Provider, Result, ServerLogin, Settings, SidebarState, View,
};

/// Whether a newly added server Calendar starts marked read-only, so that
/// the user can watch sync work before trusting FreeCal to write (US-18).
/// Until 1.0.
const NEW_SERVER_CALENDARS_READ_ONLY: bool = true;

/// How a server object's data is written: iCalendar, as CalDAV holds it.
/// Google's will be its JSON as received (ADR 0010).
const ICALENDAR: &str = "icalendar";

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
const EVENT_FIELDS: &str = "title, all_day, start_at, end_at, start_zone, end_zone, location, \
     description, description_html, zone_pinned, rrule, exdates, recurrence_id, object_id";

/// Reads [`EVENT_FIELDS`] starting at column `first`.
fn event_fields(row: &Row, first: usize) -> rusqlite::Result<EventFields> {
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

/// Reads `calendar_id` and [`EVENT_FIELDS`] starting at column `first`.
fn stored_event(row: &Row, first: usize) -> rusqlite::Result<StoredEvent> {
    Ok(StoredEvent {
        calendar_id: CalendarId(row.get(first)?),
        fields: event_fields(row, first + 1)?,
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

impl Store {
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

    /// The Event `id` with its UID.
    pub(super) fn event_and_uid(&self, id: EventId) -> Result<Option<(String, StoredEvent)>> {
        Ok(self
            .conn
            .query_row(
                &format!("SELECT uid, calendar_id, {EVENT_FIELDS} FROM event WHERE id = ?1"),
                [id.0],
                |row| Ok((row.get(0)?, stored_event(row, 1)?)),
            )
            .optional()?)
    }

    /// The Event `id` as it is now, if it is still the one with the UID
    /// `uid`.
    pub(super) fn current_event(&self, id: EventId, uid: &str) -> Result<Option<StoredEvent>> {
        Ok(self
            .conn
            .query_row(
                &format!(
                    "SELECT calendar_id, {EVENT_FIELDS} FROM event WHERE id = ?1 AND uid = ?2"
                ),
                params![id.0, uid],
                |row| stored_event(row, 0),
            )
            .optional()?)
    }

    /// The server object `id`, if it exists in the Calendar `calendar_id`.
    pub(super) fn object_in(
        &self,
        id: ObjectId,
        calendar_id: CalendarId,
    ) -> Result<Option<ObjectId>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id FROM server_object WHERE id = ?1 AND calendar_id = ?2",
                params![id.0, calendar_id.0],
                |row| row.get(0).map(ObjectId),
            )
            .optional()?)
    }

    /// The server object of the first of a Calendar's rows with the UID
    /// `uid` that has one.
    pub(super) fn object_of_uid(
        &self,
        calendar_id: CalendarId,
        uid: &str,
    ) -> Result<Option<ObjectId>> {
        Ok(self
            .conn
            .query_row(
                "SELECT object_id FROM event WHERE calendar_id = ?1 AND uid = ?2 \
                 AND object_id IS NOT NULL ORDER BY id LIMIT 1",
                params![calendar_id.0, uid],
                |row| row.get(0).map(ObjectId),
            )
            .optional()?)
    }

    /// The Change Journal's entries of the unit `id`, in the order they were
    /// made.
    pub(super) fn entries(&self, id: UnitId) -> Result<Vec<Entry>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT event_id, uid, hash, undone, calendar_id, {EVENT_FIELDS} FROM journal_entry \
             WHERE unit_id = ?1 ORDER BY id"
        ))?;
        let entries: Vec<Entry> = stmt
            .query_map([id.0], |row| {
                let created = row.get::<_, Option<i64>>(4)?.is_none();
                Ok(Entry {
                    event_id: EventId(row.get(0)?),
                    uid: row.get(1)?,
                    hash: row.get(2)?,
                    undone: row.get(3)?,
                    previous: if created {
                        None
                    } else {
                        Some(stored_event(row, 4)?)
                    },
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        if entries.is_empty() {
            return Err(CoreError::UnitNotFound(id));
        }
        Ok(entries)
    }

    /// The Calendars of an Account.
    pub(super) fn account_calendars(&self, id: AccountId) -> Result<Vec<CalendarId>> {
        let calendars = self
            .conn
            .prepare("SELECT id FROM calendar WHERE account_id = ?1")?
            .query_map([id.0], |row| row.get(0).map(CalendarId))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(calendars)
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

    /// The time zone rules the derived UTC instants were computed with.
    pub(super) fn time_zone_rules(&self) -> Result<String> {
        Ok(self
            .conn
            .query_row("SELECT time_zone_rules FROM app_state", [], |row| {
                row.get(0)
            })?)
    }

    /// Records the time zone rules the derived UTC instants are computed
    /// with now.
    pub(super) fn set_time_zone_rules(&self, rules: &str) -> Result<()> {
        self.conn
            .execute("UPDATE app_state SET time_zone_rules = ?1", [rules])?;
        Ok(())
    }

    /// Every Event's fields, but those of rows that can't be read.
    pub(super) fn all_event_fields(&self) -> Result<Vec<(EventId, EventFields)>> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT id, {EVENT_FIELDS} FROM event"))?;
        readable(stmt.query_map([], |row| Ok((EventId(row.get(0)?), event_fields(row, 1)?)))?)
    }
}

/// The writes of the tables the Change Journal guards, each with a
/// [`Journaled`].
impl Store {
    /// The one way an Event row is written: as a new Event when `id` is none,
    /// otherwise over the Event `id` or as a new one with that ID.
    pub(super) fn write_event(
        &self,
        _: &Journaled,
        id: Option<EventId>,
        uid: &str,
        event: &StoredEvent,
        (start_utc, end_utc): (i64, i64),
    ) -> Result<EventId> {
        let fields = &event.fields;
        self.conn.execute(
            "INSERT INTO event (id, uid, calendar_id, title, all_day, start_at, end_at, \
             start_zone, end_zone, start_utc, end_utc, location, description, \
             description_html, zone_pinned, rrule, exdates, recurrence_id, object_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, \
               ?18, ?19) \
             ON CONFLICT (id) DO UPDATE SET uid = excluded.uid, \
               calendar_id = excluded.calendar_id, title = excluded.title, \
               all_day = excluded.all_day, start_at = excluded.start_at, \
               end_at = excluded.end_at, start_zone = excluded.start_zone, \
               end_zone = excluded.end_zone, start_utc = excluded.start_utc, \
               end_utc = excluded.end_utc, location = excluded.location, \
               description = excluded.description, \
               description_html = excluded.description_html, \
               zone_pinned = excluded.zone_pinned, rrule = excluded.rrule, \
               exdates = excluded.exdates, recurrence_id = excluded.recurrence_id, \
               object_id = excluded.object_id",
            params![
                id.map(|id| id.0),
                uid,
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
                fields.zone_pinned,
                fields.rule,
                fields.exdates,
                fields.recurrence_id,
                fields.object_id.map(|object| object.0),
            ],
        )?;
        Ok(id.unwrap_or(EventId(self.conn.last_insert_rowid())))
    }

    /// Deletes the Event row `id`.
    pub(super) fn delete_event(&self, _: &Journaled, id: EventId) -> Result<()> {
        self.conn
            .execute("DELETE FROM event WHERE id = ?1", [id.0])?;
        Ok(())
    }

    /// Writes `object` over the server object `over`, address included, or
    /// as a new object of a Calendar, and returns its ID.
    pub(super) fn write_object(
        &self,
        _: &Journaled,
        calendar_id: CalendarId,
        object: &ServerObject,
        over: Option<ObjectId>,
    ) -> Result<ObjectId> {
        match over {
            Some(id) => {
                self.conn.execute(
                    "UPDATE server_object SET href = ?2, etag = ?3, data = ?4, format = ?5, \
                     unreadable = 0 WHERE id = ?1",
                    params![id.0, object.href, object.etag, object.data, ICALENDAR],
                )?;
                Ok(id)
            }
            None => {
                self.conn.execute(
                    "INSERT INTO server_object (calendar_id, href, etag, data, format) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        calendar_id.0,
                        object.href,
                        object.etag,
                        object.data,
                        ICALENDAR
                    ],
                )?;
                Ok(ObjectId(self.conn.last_insert_rowid()))
            }
        }
    }

    /// Marks the server objects at `href` in a Calendar unreadable.
    pub(super) fn mark_unreadable(
        &self,
        _: &Journaled,
        calendar_id: CalendarId,
        href: &str,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE server_object SET unreadable = 1 WHERE calendar_id = ?1 AND href = ?2",
            params![calendar_id.0, href],
        )?;
        Ok(())
    }

    /// Deletes the server object `id`; its rows keep no object.
    pub(super) fn delete_object(&self, _: &Journaled, id: ObjectId) -> Result<()> {
        self.conn
            .execute("DELETE FROM server_object WHERE id = ?1", [id.0])?;
        Ok(())
    }

    /// Marks the server object `id` as having changes for FreeCal to send.
    pub(super) fn mark_pending(&self, _: &Journaled, id: ObjectId) -> Result<()> {
        self.conn
            .execute("UPDATE server_object SET pending = 1 WHERE id = ?1", [id.0])?;
        Ok(())
    }

    /// Stores a new unit, as `origin` records it, made at `made_at`.
    pub(super) fn insert_unit(&self, _: &Journaled, origin: &str, made_at: i64) -> Result<UnitId> {
        self.conn.execute(
            "INSERT INTO journal_unit (origin, made_at) VALUES (?1, ?2)",
            params![origin, made_at],
        )?;
        Ok(UnitId(self.conn.last_insert_rowid()))
    }

    /// Stores an entry of `unit`: the Event `id` with the UID `uid` went from
    /// `previous`, none when it was created, to a version with the hash
    /// `hash`, none when it was deleted.
    pub(super) fn insert_entry(
        &self,
        _: &Journaled,
        unit: UnitId,
        id: EventId,
        uid: &str,
        previous: Option<&StoredEvent>,
        hash: Option<Vec<u8>>,
    ) -> Result<()> {
        let fields = previous.map(|event| &event.fields);
        let when = fields.map(|fields| &fields.when);
        self.conn.execute(
            &format!(
                "INSERT INTO journal_entry (unit_id, event_id, uid, hash, calendar_id, \
                 {EVENT_FIELDS}) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, \
                   ?16, ?17, ?18, ?19)"
            ),
            params![
                unit.0,
                id.0,
                uid,
                hash,
                previous.map(|event| event.calendar_id.0),
                fields.map(|fields| &fields.title),
                when.map(|when| when.all_day),
                when.map(|when| &when.start),
                when.map(|when| &when.end),
                when.and_then(|when| when.start_zone.as_ref()),
                when.and_then(|when| when.end_zone.as_ref()),
                fields.map(|fields| &fields.location),
                fields.map(|fields| &fields.description),
                fields.map(|fields| fields.description_html),
                fields.map(|fields| fields.zone_pinned),
                fields.and_then(|fields| fields.rule.as_ref()),
                fields.map(|fields| &fields.exdates),
                fields.and_then(|fields| fields.recurrence_id.as_ref()),
                fields.and_then(|fields| fields.object_id.map(|object| object.0)),
            ],
        )?;
        Ok(())
    }

    /// Marks every entry of `unit` undone.
    pub(super) fn mark_undone(&self, _: &Journaled, unit: UnitId) -> Result<()> {
        self.conn.execute(
            "UPDATE journal_entry SET undone = 1 WHERE unit_id = ?1",
            [unit.0],
        )?;
        Ok(())
    }

    /// Deletes a Calendar's Events with their entries, the entries that
    /// would put an Event back into it with that Event's earlier entries,
    /// and the units left without entries.
    pub(super) fn delete_calendar_events(&self, _: &Journaled, id: CalendarId) -> Result<()> {
        self.conn.execute(
            "DELETE FROM journal_entry WHERE EXISTS (SELECT 1 FROM event \
             WHERE event.calendar_id = ?1 AND event.id = journal_entry.event_id \
               AND event.uid = journal_entry.uid)",
            [id.0],
        )?;
        self.conn.execute(
            "DELETE FROM journal_entry WHERE id <= (SELECT MAX(back.id) \
             FROM journal_entry AS back WHERE back.calendar_id = ?1 \
               AND back.event_id = journal_entry.event_id AND back.uid = journal_entry.uid)",
            [id.0],
        )?;
        self.conn
            .execute("DELETE FROM event WHERE calendar_id = ?1", [id.0])?;
        self.conn.execute(
            "DELETE FROM journal_unit WHERE NOT EXISTS \
             (SELECT 1 FROM journal_entry WHERE unit_id = journal_unit.id)",
            [],
        )?;
        Ok(())
    }

    /// Writes the Event `id`'s derived UTC instants, which the Change Journal
    /// doesn't record.
    pub(super) fn write_instants(
        &self,
        _: &Journaled,
        id: EventId,
        (start_utc, end_utc): (i64, i64),
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE event SET start_utc = ?2, end_utc = ?3 WHERE id = ?1",
            params![id.0, start_utc, end_utc],
        )?;
        Ok(())
    }

    /// Deletes the Calendar row `id`, once the Journal has deleted its
    /// Events. Its server objects go with it, by the schema's cascade.
    pub(super) fn delete_calendar_row(&self, _: &Journaled, id: CalendarId) -> Result<()> {
        match self
            .conn
            .execute("DELETE FROM calendar WHERE id = ?1", [id.0])?
        {
            0 => Err(CoreError::CalendarNotFound(id)),
            _ => Ok(()),
        }
    }

    /// Deletes the Account row `id`, once the Journal has deleted its
    /// Calendars.
    pub(super) fn delete_account_row(&self, _: &Journaled, id: AccountId) -> Result<()> {
        self.conn
            .execute("DELETE FROM account WHERE id = ?1", [id.0])?;
        Ok(())
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
