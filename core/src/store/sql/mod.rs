//! Every query and write of the Local Store, but its schema: the SQL lives
//! here and nowhere else. This file holds the readers, methods of
//! [`Store`]. Every write is a method of a
//! [`DatabaseTransaction`](super::DatabaseTransaction), and sits in the file
//! of the Guard whose Write Permission it takes (ADR 0015): [`journal`] for
//! the Change Journal's, [`mirror`] for the Sync Engine's, and [`unguarded`]
//! for the Unguarded Writes.
//!
//! Writing the tables a Guard holds, `event`, `caldav_server_object`,
//! `caldav_ignored_resource`, `unsent_change`, `journal_transaction` and
//! `journal_event_modification`, takes that Guard's Write Permission
//! ([`JournalWritePermission`], [`SyncEngineWritePermission`]), which only
//! its module can make: so the compiler keeps everything else from writing
//! them. A new write to one of those tables takes one too.
//!
//! [`JournalWritePermission`]: crate::journal::JournalWritePermission
//! [`SyncEngineWritePermission`]: crate::sync::SyncEngineWritePermission

mod journal;
/// The Sync Engine's writes of the Mirror, one file per Provider (ADR 0015).
mod mirror {
    mod caldav;
}
mod unguarded;

use std::collections::{HashMap, HashSet};

use rusqlite::types::Value;
use rusqlite::{OptionalExtension, Row, params, params_from_iter};

use super::{RangeEvent, Store, StoredEvent, Uid, UidEvent};
use crate::event::{EventFields, RecurrenceId, ServerObject, StoredRange, StoredWhen};
use crate::journal::{JournalEventModification, JournalTransactionId};
use crate::remote::SyncPoint;
use crate::sync::caldav::KnownServerResource;
use crate::sync_status::SyncRecord;
use crate::{
    Account, AccountId, Appearance, Calendar, CalendarId, Colour, CoreError, EventId, Provider,
    Result, ServerLogin, Settings, SidebarState, View,
};

/// The Sync Record in columns 1 to 3 of `row`; a part with a value of
/// another type is left out, as one that can't be read.
fn sync_record(row: &Row) -> rusqlite::Result<SyncRecord> {
    let integer = |index| match row.get(index)? {
        Value::Integer(value) => Ok::<_, rusqlite::Error>(Some(value)),
        _ => Ok(None),
    };
    let text = |index| match row.get(index)? {
        Value::Text(value) => Ok::<_, rusqlite::Error>(Some(value)),
        _ => Ok(None),
    };
    Ok(SyncRecord::from_stored(
        integer(1)?,
        text(2)?.as_deref(),
        integer(3)?,
    ))
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
/// Journal's Journal Event Modifications share with the `event` table.
const EVENT_FIELDS: &str = "title, all_day, start_at, end_at, start_zone, end_zone, location, \
     description, description_html, zone_pinned, rrule, exdates, recurrence_id";

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
        recurrence_id: row
            .get::<_, Option<String>>(first + 12)?
            .map(RecurrenceId::obtained),
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
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(calendars)
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

    /// How many Unsent Changes each server Account's Calendar has, in the
    /// order they were added.
    pub fn unsent_change_counts(&self) -> Result<Vec<UnsentChangeCount>> {
        let mut stmt = self.conn.prepare(
            "SELECT calendar.id, calendar.account_id, \
                 (SELECT COUNT(*) FROM unsent_change WHERE calendar_id = calendar.id) \
             FROM calendar JOIN account ON account.id = calendar.account_id \
             WHERE account.provider != 'local' ORDER BY calendar.id",
        )?;
        readable(stmt.query_map([], |row| {
            Ok(UnsentChangeCount {
                calendar_id: CalendarId(row.get(0)?),
                account_id: AccountId(row.get(1)?),
                count: row.get(2)?,
            })
        })?)
    }

    /// The Sync Record of each server Account that has one.
    pub fn account_sync_records(&self) -> Result<Vec<(AccountId, SyncRecord)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, last_sync_at, last_sync_outcome, last_synced_at FROM account \
             WHERE provider != 'local'",
        )?;
        readable(stmt.query_map([], |row| Ok((AccountId(row.get(0)?), sync_record(row)?)))?)
    }

    /// The Sync Record of each server Account's Calendar.
    pub fn calendar_sync_records(&self) -> Result<Vec<(CalendarId, SyncRecord)>> {
        let mut stmt = self.conn.prepare(
            "SELECT calendar.id, calendar.last_sync_at, calendar.last_sync_outcome, \
                 calendar.last_synced_at \
             FROM calendar JOIN account ON account.id = calendar.account_id \
             WHERE account.provider != 'local'",
        )?;
        readable(stmt.query_map([], |row| Ok((CalendarId(row.get(0)?), sync_record(row)?)))?)
    }

    /// The Known Server Resources of a CalDAV Calendar, by their address as
    /// the server gave it: its Server Objects, each with the UID it keeps,
    /// and its Ignored Resources, each with the UID it holds that a Server
    /// Object keeps, none for one holding several or not read.
    pub fn known_server_resources(
        &self,
        calendar_id: CalendarId,
    ) -> Result<HashMap<String, KnownServerResource>> {
        let mut stmt = self.conn.prepare(
            "SELECT href, etag, uid, FALSE FROM caldav_server_object \
             WHERE calendar_id = ?1 \
             UNION ALL SELECT href, etag, uid, TRUE FROM caldav_ignored_resource \
             WHERE calendar_id = ?1",
        )?;
        let known = readable(stmt.query_map([calendar_id.0], |row| {
            Ok((
                row.get(0)?,
                KnownServerResource {
                    etag: row.get(1)?,
                    uid: row.get(2)?,
                    ignored: row.get(3)?,
                },
            ))
        })?)?;
        Ok(known.into_iter().collect())
    }

    /// The UID that the CalDAV Server Object at `href` in a Calendar keeps,
    /// if one is there.
    pub fn uid_at(&self, calendar_id: CalendarId, href: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT uid FROM caldav_server_object WHERE calendar_id = ?1 AND href = ?2",
                params![calendar_id.0, href],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// The Corresponding Server Object of the UID `uid` in a Calendar, the
    /// one the Calendar's Event Rows with the UID belong to, as the server
    /// last gave it: the one CalDAV Server Object holding it, as the Local
    /// Store keeps a UID to one (ADR 0015); none when none holds it.
    pub fn server_object(
        &self,
        calendar_id: CalendarId,
        uid: &str,
    ) -> Result<Option<ServerObject>> {
        Ok(self
            .conn
            .query_row(
                "SELECT href, etag, data FROM caldav_server_object \
                 WHERE calendar_id = ?1 AND uid = ?2",
                params![calendar_id.0, uid],
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

    /// The Server Object the Event `id` belongs to, as the server last gave
    /// it: the Corresponding Server Object of its UID in its Calendar. None
    /// for an Event no Server Object holds, such as one not on a server yet.
    pub fn server_object_of_event(&self, id: EventId) -> Result<Option<ServerObject>> {
        match self.event_and_uid(id)? {
            Some((uid, event)) => self.server_object(event.calendar_id, &uid),
            None => Err(CoreError::EventNotFound(id)),
        }
    }

    pub fn find_calendar(&self, id: CalendarId) -> Result<Calendar> {
        self.list_calendars()?
            .into_iter()
            .find(|calendar| calendar.id == id)
            .ok_or(CoreError::CalendarNotFound(id))
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

    pub fn sidebar(&self) -> Result<SidebarState> {
        let (shown, width) = self.conn.query_row(
            "SELECT sidebar_shown, sidebar_width FROM app_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Ok(sidebar_state(shown, width))
    }

    pub fn settings(&self) -> Result<Settings> {
        let appearance: String =
            self.conn
                .query_row("SELECT appearance FROM app_state", [], |row| row.get(0))?;
        Ok(Settings {
            appearance: Appearance::parse(&appearance),
        })
    }

    /// The Event `id` with its UID.
    pub(crate) fn event_and_uid(&self, id: EventId) -> Result<Option<(String, StoredEvent)>> {
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
    pub(crate) fn current_event(&self, id: EventId, uid: &str) -> Result<Option<StoredEvent>> {
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

    /// Every Unsent Change, in the order they were added.
    #[cfg(test)]
    pub fn unsent_changes(&self) -> Result<Vec<UnsentChange>> {
        let mut stmt = self.conn.prepare(
            "SELECT calendar_id, uid, recurrence_id, since FROM unsent_change ORDER BY rowid",
        )?;
        let unsent = stmt
            .query_map([], |row| {
                Ok(UnsentChange {
                    calendar_id: CalendarId(row.get(0)?),
                    uid: row.get(1)?,
                    recurrence_id: row.get(2)?,
                    since: row.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(unsent)
    }

    /// The Journal Event Modifications of the Journal Transaction `id`, in the
    /// order they were made.
    pub(crate) fn journal_event_modifications(
        &self,
        id: JournalTransactionId,
    ) -> Result<Vec<JournalEventModification>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT event_id, uid, hash, undone, calendar_id, {EVENT_FIELDS} \
             FROM journal_event_modification WHERE journal_transaction_id = ?1 ORDER BY id"
        ))?;
        let modifications: Vec<JournalEventModification> = stmt
            .query_map([id.0], |row| {
                let created = row.get::<_, Option<i64>>(4)?.is_none();
                Ok(JournalEventModification {
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
        if modifications.is_empty() {
            return Err(CoreError::JournalTransactionNotFound(id));
        }
        Ok(modifications)
    }

    /// The Calendars of an Account.
    pub(crate) fn account_calendars(&self, id: AccountId) -> Result<Vec<CalendarId>> {
        let calendars = self
            .conn
            .prepare("SELECT id FROM calendar WHERE account_id = ?1")?
            .query_map([id.0], |row| row.get(0).map(CalendarId))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(calendars)
    }

    /// The time zone rules the derived UTC instants were computed with.
    pub(crate) fn time_zone_rules(&self) -> Result<String> {
        Ok(self
            .conn
            .query_row("SELECT time_zone_rules FROM app_state", [], |row| {
                row.get(0)
            })?)
    }

    /// Every Event's fields, but those of Event Rows that can't be read.
    pub(crate) fn all_event_fields(&self) -> Result<Vec<(EventId, EventFields)>> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT id, {EVENT_FIELDS} FROM event"))?;
        readable(stmt.query_map([], |row| Ok((EventId(row.get(0)?), event_fields(row, 1)?)))?)
    }
}

/// How many Unsent Changes a server Account's Calendar has.
#[derive(Debug, Clone, Copy)]
pub struct UnsentChangeCount {
    pub calendar_id: CalendarId,
    pub account_id: AccountId,
    pub count: u32,
}

/// A row of `unsent_change` as stored, its recurrence ID as text, for tests
/// to compare.
#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
pub struct UnsentChange {
    pub calendar_id: CalendarId,
    pub uid: String,
    pub recurrence_id: Option<String>,
    pub since: i64,
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
