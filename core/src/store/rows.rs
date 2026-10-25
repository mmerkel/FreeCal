//! The SQL that writes the tables the Change Journal guards: Event rows,
//! server objects, and the Journal's own units and entries. Every function
//! here is visible inside `store` only, so that code outside it changes an
//! Event through a [`Unit`](super::Unit) alone (ADR 0011). Inside `store`,
//! only [`journal`](super::journal) calls them, and opening the Local Store
//! calls [`write_instants`]; the compiler enforces the module's boundary,
//! not that. Reads stay in `store` itself.

use rusqlite::{Connection, params};

use super::{EVENT_FIELDS, ICALENDAR, Store, StoredEvent, UnitId};
use crate::event::{ObjectId, ServerObject};
use crate::{CalendarId, EventId, Result};

/// Writes the Event `id`'s derived UTC instants, which the Change Journal
/// doesn't record. It runs while the Local Store opens, before there is a
/// [`Store`], so it takes the connection.
pub(super) fn write_instants(
    conn: &Connection,
    id: EventId,
    (start_utc, end_utc): (i64, i64),
) -> Result<()> {
    conn.execute(
        "UPDATE event SET start_utc = ?2, end_utc = ?3 WHERE id = ?1",
        params![id.0, start_utc, end_utc],
    )?;
    Ok(())
}

impl Store {
    /// The one way an Event row is written: as a new Event when `id` is none,
    /// otherwise over the Event `id` or as a new one with that ID.
    pub(super) fn write_event(
        &self,
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
    pub(super) fn delete_event(&self, id: EventId) -> Result<()> {
        self.conn
            .execute("DELETE FROM event WHERE id = ?1", [id.0])?;
        Ok(())
    }

    /// Writes `object` over the server object `over`, address included, or
    /// as a new object of a Calendar, and returns its ID.
    pub(super) fn write_object(
        &self,
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
    pub(super) fn mark_unreadable(&self, calendar_id: CalendarId, href: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE server_object SET unreadable = 1 WHERE calendar_id = ?1 AND href = ?2",
            params![calendar_id.0, href],
        )?;
        Ok(())
    }

    /// Deletes the server object `id`; its rows keep no object.
    pub(super) fn delete_object(&self, id: ObjectId) -> Result<()> {
        self.conn
            .execute("DELETE FROM server_object WHERE id = ?1", [id.0])?;
        Ok(())
    }

    /// Marks the server object `id` as having changes for FreeCal to send.
    pub(super) fn mark_pending(&self, id: ObjectId) -> Result<()> {
        self.conn
            .execute("UPDATE server_object SET pending = 1 WHERE id = ?1", [id.0])?;
        Ok(())
    }

    /// Stores a new unit, as `origin` records it, made at `made_at`.
    pub(super) fn insert_unit(&self, origin: &str, made_at: i64) -> Result<UnitId> {
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
    pub(super) fn mark_undone(&self, unit: UnitId) -> Result<()> {
        self.conn.execute(
            "UPDATE journal_entry SET undone = 1 WHERE unit_id = ?1",
            [unit.0],
        )?;
        Ok(())
    }

    /// Deletes a Calendar's Events with their entries, the entries that
    /// would put an Event back into it with that Event's earlier entries,
    /// and the units left without entries.
    pub(super) fn delete_calendar_events(&self, id: CalendarId) -> Result<()> {
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
}
