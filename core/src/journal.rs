//! The Change Journal (ADR 0009) and the only writer of Events: every change
//! to an Event goes through a [`Unit`] in [`record`], which keeps the Event's
//! previous version and a hash of its new one, so that no change can skip
//! the Journal. Changes made together, such as an import, are one unit.
//! A unit writes the server objects too (ADR 0010), which the Journal doesn't
//! record: each Event's version holds only the object it refers to.

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::event::{ObjectId, ServerObject, Source, rule};
use crate::store::{EVENT_FIELDS, ICALENDAR, Store, StoredEvent, Uid, UidEvent, event_fields};
use crate::{AccountId, CalendarId, CoreError, EventId, Result};

/// A Change Journal unit: what undoes a change, or changes made together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    test,
    derive(ts_rs::TS),
    ts(type = "number & { readonly __brand: \"UnitId\" }")
)]
#[serde(transparent)]
pub struct UnitId(pub i64);

/// What a change to Events returns: its value, and the unit that undoes it,
/// none when it changed nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Changed<T> {
    pub unit: Option<UnitId>,
    pub value: T,
}

/// Where a unit's changes came from, which decides where they may write.
/// Snapshot restore and Conflict resolution come with their tickets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Origin {
    UserAction,
    Import,
    Undo,
    /// A Calendar's first sync, the baseline: it writes like any origin but
    /// records nothing (ADR 0009), so its unit is never stored.
    FirstSync,
    /// An incremental sync of one Calendar, what its server changed.
    Sync,
}

impl Origin {
    /// How a stored unit records it; none for an origin that isn't recorded.
    fn recorded_as(self) -> Option<&'static str> {
        match self {
            Origin::UserAction => Some("user"),
            Origin::Import => Some("import"),
            Origin::Undo => Some("undo"),
            Origin::Sync => Some("sync"),
            Origin::FirstSync => None,
        }
    }

    /// Whether its changes to an object's rows mark the object pending, for
    /// FreeCal to send: a sync's come from the server.
    fn marks_pending(self) -> bool {
        match self {
            Origin::UserAction | Origin::Import | Origin::Undo => true,
            Origin::FirstSync | Origin::Sync => false,
        }
    }
}

/// What [`record`] did: the unit, none when nothing changed, and the
/// Calendars whose Events changed, in the order they were first changed.
pub(crate) struct Recorded<T> {
    pub unit: Option<UnitId>,
    pub calendars: Vec<CalendarId>,
    pub value: T,
}

/// Runs `change` in one transaction as one unit of `origin`, made at
/// `made_at` (Unix seconds). If it fails, nothing it did is kept. A unit that
/// changed nothing isn't stored.
pub(crate) fn record<T>(
    store: &Store,
    origin: Origin,
    made_at: i64,
    change: impl FnOnce(&mut Unit) -> Result<T>,
) -> Result<Recorded<T>> {
    store.transaction(|| {
        let mut unit = Unit {
            store,
            origin,
            made_at,
            id: None,
            calendars: Vec::new(),
        };
        let value = change(&mut unit)?;
        Ok(Recorded {
            unit: unit.id,
            calendars: unit.calendars,
            value,
        })
    })
}

/// Undoes the unit `id` whole, as a unit of origin undo: every Event gets its
/// previous version back under the same identity. Refused while an Event
/// changed since, and for a unit undone already.
pub(crate) fn undo(store: &Store, id: UnitId, made_at: i64) -> Result<Recorded<()>> {
    record(store, Origin::Undo, made_at, |unit| {
        let entries = entries(store, id)?;
        if entries.iter().any(|entry| entry.undone) {
            return Err(CoreError::AlreadyUndone(id));
        }
        // Latest first, so that each entry finds the version it made.
        for entry in entries.iter().rev() {
            let current = current(store, entry.event_id, &entry.uid)?;
            if current.map(|event| hash(&entry.uid, &event)) != entry.hash {
                return Err(CoreError::ChangedSinceUnit(id));
            }
            match &entry.previous {
                None => unit.delete(entry.event_id)?,
                Some(previous) => unit.put_back(entry.event_id, &entry.uid, previous)?,
            }
        }
        store.connection().execute(
            "UPDATE journal_entry SET undone = 1 WHERE unit_id = ?1",
            [id.0],
        )?;
        Ok(())
    })
}

/// Deletes a Calendar with its Events. This isn't recorded (ADR 0009): the
/// entries of its Events go too, and units left without entries. So do the
/// entries that would put an Event back into it, such as one moved out or
/// deleted, with that Event's earlier entries, which are undone only through
/// them; its later entries stay.
pub(crate) fn delete_calendar(store: &Store, id: CalendarId) -> Result<()> {
    store.transaction(|| delete_calendar_within(store, id))
}

/// Deletes a server Account with its Calendars, each as
/// [`delete_calendar`] does, all or none.
pub(crate) fn delete_account(store: &Store, id: AccountId) -> Result<()> {
    store.transaction(|| {
        let conn = store.connection();
        let calendars = conn
            .prepare("SELECT id FROM calendar WHERE account_id = ?1")?
            .query_map([id.0], |row| row.get(0).map(CalendarId))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for calendar in calendars {
            delete_calendar_within(store, calendar)?;
        }
        conn.execute("DELETE FROM account WHERE id = ?1", [id.0])?;
        Ok(())
    })
}

/// [`delete_calendar`] inside a transaction already begun.
fn delete_calendar_within(store: &Store, id: CalendarId) -> Result<()> {
    let conn = store.connection();
    conn.execute(
        "DELETE FROM journal_entry WHERE EXISTS (SELECT 1 FROM event \
         WHERE event.calendar_id = ?1 AND event.id = journal_entry.event_id \
           AND event.uid = journal_entry.uid)",
        [id.0],
    )?;
    conn.execute(
        "DELETE FROM journal_entry WHERE id <= (SELECT MAX(back.id) \
         FROM journal_entry AS back WHERE back.calendar_id = ?1 \
           AND back.event_id = journal_entry.event_id AND back.uid = journal_entry.uid)",
        [id.0],
    )?;
    conn.execute("DELETE FROM event WHERE calendar_id = ?1", [id.0])?;
    conn.execute(
        "DELETE FROM journal_unit WHERE NOT EXISTS \
         (SELECT 1 FROM journal_entry WHERE unit_id = journal_unit.id)",
        [],
    )?;
    match conn.execute("DELETE FROM calendar WHERE id = ?1", [id.0])? {
        0 => Err(CoreError::CalendarNotFound(id)),
        _ => Ok(()),
    }
}

/// One unit being recorded, inside [`record`]'s transaction. Each write reads
/// the Event's previous version itself.
pub(crate) struct Unit<'a> {
    store: &'a Store,
    origin: Origin,
    made_at: i64,
    id: Option<UnitId>,
    calendars: Vec<CalendarId>,
}

impl Unit<'_> {
    pub fn find_event(&self, id: EventId) -> Result<StoredEvent> {
        self.store.find_event(id)
    }

    /// The server objects of a Calendar at `href`, in the order they were
    /// added.
    pub fn objects_at(&self, calendar_id: CalendarId, href: &str) -> Result<Vec<ObjectId>> {
        self.store.objects_at(calendar_id, href)
    }

    /// The rows of the server object `id`.
    pub fn events_in_object(&self, id: ObjectId) -> Result<Vec<UidEvent>> {
        self.store.events_in_object(id)
    }

    /// Makes `object`, as the server holds it now, the mirror's version of
    /// the object `over` in a Calendar, address included, or of a new
    /// object, and returns its ID. Only a sync writes the mirror, as only it
    /// knows what the server holds.
    pub fn mirror(
        &self,
        calendar_id: CalendarId,
        object: &ServerObject,
        over: Option<ObjectId>,
    ) -> Result<ObjectId> {
        debug_assert!(matches!(self.origin, Origin::FirstSync | Origin::Sync));
        self.writable(calendar_id)?;
        let conn = self.store.connection();
        match over {
            Some(id) => {
                conn.execute(
                    "UPDATE server_object SET href = ?2, etag = ?3, data = ?4, format = ?5, \
                     unreadable = 0 WHERE id = ?1",
                    params![id.0, object.href, object.etag, object.data, ICALENDAR],
                )?;
                Ok(id)
            }
            None => {
                conn.execute(
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
                Ok(ObjectId(conn.last_insert_rowid()))
            }
        }
    }

    /// Records that FreeCal couldn't read what the server holds now at
    /// `href` in a Calendar: the objects there keep the last version it
    /// could, and their rows stay as they are.
    pub fn unreadable(&self, calendar_id: CalendarId, href: &str) -> Result<()> {
        debug_assert!(matches!(self.origin, Origin::FirstSync | Origin::Sync));
        self.store.connection().execute(
            "UPDATE server_object SET unreadable = 1 WHERE calendar_id = ?1 AND href = ?2",
            params![calendar_id.0, href],
        )?;
        Ok(())
    }

    /// Removes the server object `id` from the mirror, and its rows with it,
    /// as a sync finds it gone from the server.
    pub fn remove_object(&mut self, id: ObjectId) -> Result<()> {
        debug_assert!(matches!(self.origin, Origin::FirstSync | Origin::Sync));
        for event in self.events_in_object(id)? {
            self.delete(event.id)?;
        }
        self.store
            .connection()
            .execute("DELETE FROM server_object WHERE id = ?1", [id.0])?;
        Ok(())
    }

    /// Records, with the unit's changes, that a sync of a Calendar
    /// finished with the sync token `token`.
    pub fn synced(&self, calendar_id: CalendarId, token: Option<&str>) -> Result<()> {
        self.store.set_synced(calendar_id, token)
    }

    /// Adds an Event with the UID `uid`.
    pub fn create(&mut self, uid: &str, event: &StoredEvent) -> Result<EventId> {
        self.writable(event.calendar_id)?;
        let event = &self.placed(None, uid, event)?;
        let id = write(self.store, None, uid, event, rule::span(&event.fields)?)?;
        self.entry(id, uid, None, Some(event))?;
        Ok(id)
    }

    /// Changes the Event `id`, possibly moving it to another Calendar. A
    /// change that changes nothing isn't recorded.
    pub fn replace(&mut self, id: EventId, event: &StoredEvent) -> Result<()> {
        let (uid, previous) = self.previous(id)?;
        self.writable(previous.calendar_id)?;
        self.writable(event.calendar_id)?;
        let event = &self.placed(Some(&previous), &uid, event)?;
        if hash(&uid, &previous) == hash(&uid, event) {
            return Ok(());
        }
        write(
            self.store,
            Some(id),
            &uid,
            event,
            rule::span(&event.fields)?,
        )?;
        self.entry(id, &uid, Some(&previous), Some(event))
    }

    /// `event`, a version of a row with the UID `uid`, in the server object
    /// (ADR 0010) it gets: a sync's as given, an undo's while that object
    /// exists, otherwise none, so that the row counts as not yet on the
    /// server. A user action or an import keeps the object of the
    /// `previous` version, none with a move to another Calendar, which
    /// leaves the object without the row; a new row of theirs joins the
    /// object of its Calendar's rows with its UID, as a CalDAV object holds
    /// a Recurring Event with its Exceptions.
    fn placed(
        &self,
        previous: Option<&StoredEvent>,
        uid: &str,
        event: &StoredEvent,
    ) -> Result<StoredEvent> {
        let conn = self.store.connection();
        let object = match self.origin {
            Origin::FirstSync | Origin::Sync => return Ok(event.clone()),
            Origin::Undo => match event.fields.object_id {
                Some(object) => conn
                    .query_row(
                        "SELECT id FROM server_object WHERE id = ?1 AND calendar_id = ?2",
                        params![object.0, event.calendar_id.0],
                        |row| row.get(0).map(ObjectId),
                    )
                    .optional()?,
                None => None,
            },
            Origin::UserAction | Origin::Import => match previous {
                Some(previous) => previous
                    .fields
                    .object_id
                    .filter(|_| previous.calendar_id == event.calendar_id),
                None => conn
                    .query_row(
                        "SELECT object_id FROM event WHERE calendar_id = ?1 AND uid = ?2 \
                         AND object_id IS NOT NULL ORDER BY id LIMIT 1",
                        params![event.calendar_id.0, uid],
                        |row| row.get(0).map(ObjectId),
                    )
                    .optional()?,
            },
        };
        let mut event = event.clone();
        event.fields.object_id = object;
        Ok(event)
    }

    pub fn delete(&mut self, id: EventId) -> Result<()> {
        let (uid, previous) = self.previous(id)?;
        self.writable(previous.calendar_id)?;
        self.store
            .connection()
            .execute("DELETE FROM event WHERE id = ?1", [id.0])?;
        self.entry(id, &uid, Some(&previous), None)
    }

    /// Writes an earlier version back under its own ID: over the Event `id`
    /// while it is the one with the UID `uid`, otherwise as a new Event. No
    /// other Event has that ID, as IDs are never given out again (ticket 85).
    fn put_back(&mut self, id: EventId, uid: &str, event: &StoredEvent) -> Result<()> {
        if current(self.store, id, uid)?.is_some() {
            return self.replace(id, event);
        }
        self.writable(event.calendar_id)?;
        let event = &self.placed(None, uid, event)?;
        // A version FreeCal can't read the time of was stored like that; it
        // reads as unreadable again rather than stopping the undo.
        let instants = rule::span(&event.fields).unwrap_or_default();
        write(self.store, Some(id), uid, event, instants)?;
        self.entry(id, uid, None, Some(event))
    }

    fn previous(&self, id: EventId) -> Result<(String, StoredEvent)> {
        self.store
            .connection()
            .query_row(
                &format!("SELECT uid, calendar_id, {EVENT_FIELDS} FROM event WHERE id = ?1"),
                [id.0],
                |row| Ok((row.get(0)?, stored_event(row, 1)?)),
            )
            .optional()?
            .ok_or(CoreError::EventNotFound(id))
    }

    /// Refuses writes into a missing Calendar, and into a Read-only one for
    /// every origin but a sync, which loads what the server has.
    /// Every write checks this itself; a change that targets a Calendar, such
    /// as an import, checks it first, so that it is refused even when it
    /// would write nothing.
    pub fn writable(&self, id: CalendarId) -> Result<()> {
        let read_only = self.store.find_calendar(id)?.read_only;
        match self.origin {
            Origin::UserAction | Origin::Import | Origin::Undo if read_only => {
                Err(CoreError::CalendarReadOnly(id))
            }
            _ => Ok(()),
        }
    }

    /// Records that the Event `id` went from `previous` to `new`, none for
    /// an Event created or deleted, starting the unit with its first entry,
    /// and marks the objects of both pending if the origin does. An origin
    /// that isn't recorded only notes the Calendars.
    fn entry(
        &mut self,
        id: EventId,
        uid: &str,
        previous: Option<&StoredEvent>,
        new: Option<&StoredEvent>,
    ) -> Result<()> {
        let conn = self.store.connection();
        for event in previous.into_iter().chain(new) {
            if !self.calendars.contains(&event.calendar_id) {
                self.calendars.push(event.calendar_id);
            }
            if let Some(object) = event.fields.object_id
                && self.origin.marks_pending()
            {
                conn.execute(
                    "UPDATE server_object SET pending = 1 WHERE id = ?1",
                    [object.0],
                )?;
            }
        }
        let Some(origin) = self.origin.recorded_as() else {
            return Ok(());
        };
        let unit = match self.id {
            Some(unit) => unit,
            None => {
                conn.execute(
                    "INSERT INTO journal_unit (origin, made_at) VALUES (?1, ?2)",
                    params![origin, self.made_at],
                )?;
                *self.id.insert(UnitId(conn.last_insert_rowid()))
            }
        };
        let fields = previous.map(|event| &event.fields);
        let when = fields.map(|fields| &fields.when);
        conn.execute(
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
                new.map(|event| hash(uid, event)),
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
}

impl Source for Unit<'_> {
    fn events_with_uid(&self, uid: Uid) -> Result<Vec<UidEvent>> {
        self.store.events_with_uid(uid)
    }
}

/// The one way an Event row is written: as a new Event when `id` is none,
/// otherwise over the Event `id` or as a new one with that ID.
fn write(
    store: &Store,
    id: Option<EventId>,
    uid: &str,
    event: &StoredEvent,
    (start_utc, end_utc): (i64, i64),
) -> Result<EventId> {
    let fields = &event.fields;
    let conn = store.connection();
    conn.execute(
        "INSERT INTO event (id, uid, calendar_id, title, all_day, start_at, end_at, \
         start_zone, end_zone, start_utc, end_utc, location, description, \
         description_html, zone_pinned, rrule, exdates, recurrence_id, object_id) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, \
           ?19) \
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
    Ok(id.unwrap_or(EventId(conn.last_insert_rowid())))
}

/// The Event `id` as it is now, if it is still the one with the UID `uid`.
fn current(store: &Store, id: EventId, uid: &str) -> Result<Option<StoredEvent>> {
    Ok(store
        .connection()
        .query_row(
            &format!("SELECT calendar_id, {EVENT_FIELDS} FROM event WHERE id = ?1 AND uid = ?2"),
            params![id.0, uid],
            |row| stored_event(row, 0),
        )
        .optional()?)
}

/// One Event's change in a unit, as the Change Journal holds it.
struct Entry {
    event_id: EventId,
    /// The Event's UID, which its hashes cover.
    uid: String,
    /// Its version before the change; none when the change created it.
    previous: Option<StoredEvent>,
    /// The hash of its version after the change; none when the change
    /// deleted it.
    hash: Option<Vec<u8>>,
    undone: bool,
}

/// The entries of the unit `id`, in the order they were made.
fn entries(store: &Store, id: UnitId) -> Result<Vec<Entry>> {
    let mut stmt = store.connection().prepare(&format!(
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

/// Reads `calendar_id` and [`EVENT_FIELDS`] starting at column `first`.
fn stored_event(row: &Row, first: usize) -> rusqlite::Result<StoredEvent> {
    Ok(StoredEvent {
        calendar_id: CalendarId(row.get(first)?),
        fields: event_fields(row, first + 1)?,
    })
}

/// The version of [`hash`]'s format, its first byte. Format 2 added the
/// repeat rule, the cancelled Occurrences and the recurrence ID (ticket 07),
/// format 3 the original server data (ticket 81, ADR 0004), and format 4
/// covers the server object the row refers to instead (ticket 86, ADR 0010).
const HASH_FORMAT: u8 = 4;

/// The SHA-256 hash of an Event's version: its Calendar, UID and fields, as
/// a list of items each prefixed with its length, after [`HASH_FORMAT`].
///
/// A field added to Events joins the list and bumps [`HASH_FORMAT`], and the
/// migration that adds it adds it to the Change Journal's entries too; the
/// Local Store then re-hashes every entry ([`rehash`]).
fn hash(uid: &str, event: &StoredEvent) -> Vec<u8> {
    hash_as(HASH_FORMAT, uid, event)
}

/// [`hash`] in the format `format`, for [`rehash`].
fn hash_as(format: u8, uid: &str, event: &StoredEvent) -> Vec<u8> {
    hash_with(format, uid, event, None)
}

/// [`hash_as`], with `server`, the original server data a row held in
/// format 3, which only the migration to server objects still reads
/// ([`rehash_server_data`]).
fn hash_with(format: u8, uid: &str, event: &StoredEvent, server: Option<&ServerObject>) -> Vec<u8> {
    let fields = &event.fields;
    let when = &fields.when;
    let mut hasher = Sha256::new();
    hasher.update([format]);
    let mut item = |bytes: &[u8]| {
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    };
    // An absent value is told apart from an empty one by its first byte.
    let optional = |value: &Option<String>| match value {
        None => vec![0],
        Some(zone) => [&[1], zone.as_bytes()].concat(),
    };
    item(&event.calendar_id.0.to_be_bytes());
    item(uid.as_bytes());
    item(fields.title.as_bytes());
    item(&[when.all_day.into()]);
    item(when.start.as_bytes());
    item(when.end.as_bytes());
    item(&optional(&when.start_zone));
    item(&optional(&when.end_zone));
    item(fields.location.as_bytes());
    item(fields.description.as_bytes());
    item(&[fields.description_html.into()]);
    item(&[fields.zone_pinned.into()]);
    if format >= 2 {
        item(&optional(&fields.rule));
        item(fields.exdates.as_bytes());
        item(&optional(&fields.recurrence_id));
    }
    if format == 3 {
        match server {
            None => item(&[0]),
            Some(server) => {
                item(&[1]);
                item(server.href.as_bytes());
                item(&optional(&server.etag));
                item(server.data.as_bytes());
            }
        }
    }
    if format >= 4 {
        match fields.object_id {
            None => item(&[0]),
            Some(object) => item(&[&[1], &object.0.to_be_bytes()[..]].concat()),
        }
    }
    hasher.finalize().to_vec()
}

/// Makes every entry's hash in an earlier format again in the current
/// [`HASH_FORMAT`], after migrations that added fields to Events: the new
/// version an entry hashes is the previous version the next entry for that
/// Event holds, or the Event as it is now for its last entry. A hash that
/// doesn't match that version in any format stays as it is, so that it
/// still doesn't match. Format 3 covered columns that are gone; the
/// migration that dropped them re-hashed its entries.
pub(crate) fn rehash(conn: &Connection) -> Result<()> {
    for hashed in hashed_versions(conn, "", |_, _| Ok(()))? {
        if let Some((version, ())) = &hashed.version
            && [1, 2]
                .into_iter()
                .any(|format| hash_as(format, &hashed.uid, version) == hashed.stored)
        {
            hashed.set(conn, hash(&hashed.uid, version))?;
        }
    }
    Ok(())
}

/// [`rehash`] for entries in format 3, while the Event rows and entries
/// still hold the original server data it covered, next to the server
/// object they refer to now (ticket 86).
pub(crate) fn rehash_server_data(conn: &Connection) -> Result<()> {
    let server = |row: &Row, first: usize| -> rusqlite::Result<Option<ServerObject>> {
        let href: Option<String> = row.get(first)?;
        let etag = row.get(first + 1)?;
        let data: Option<String> = row.get(first + 2)?;
        Ok(href
            .zip(data)
            .map(|(href, data)| ServerObject { href, etag, data }))
    };
    let columns = ", server_href, server_etag, server_data";
    for hashed in hashed_versions(conn, columns, server)? {
        if let Some((version, server)) = &hashed.version
            && hash_with(3, &hashed.uid, version, server.as_ref()) == hashed.stored
        {
            hashed.set(conn, hash(&hashed.uid, version))?;
        }
    }
    Ok(())
}

/// An entry's hash with the version it hashes, as [`rehash`] finds it.
struct Hashed<T> {
    id: i64,
    uid: String,
    stored: Vec<u8>,
    /// The version, with what `extra` columns hold; none if it can't be read.
    version: Option<(StoredEvent, T)>,
}

impl<T> Hashed<T> {
    fn set(&self, conn: &Connection, hash: Vec<u8>) -> Result<()> {
        conn.execute(
            "UPDATE journal_entry SET hash = ?2 WHERE id = ?1",
            params![self.id, hash],
        )?;
        Ok(())
    }
}

/// Every entry with a hash, with the version it hashes, read from
/// [`EVENT_FIELDS`] followed by the columns `extra` with `read`.
fn hashed_versions<T>(
    conn: &Connection,
    extra: &str,
    read: impl Fn(&Row, usize) -> rusqlite::Result<T>,
) -> Result<Vec<Hashed<T>>> {
    let entries: Vec<(i64, EventId, String, Vec<u8>)> = conn
        .prepare("SELECT id, event_id, uid, hash FROM journal_entry WHERE hash IS NOT NULL")?
        .query_map([], |row| {
            Ok((row.get(0)?, EventId(row.get(1)?), row.get(2)?, row.get(3)?))
        })?
        .filter_map(|entry| entry.ok())
        .collect();
    let columns = format!("calendar_id, {EVENT_FIELDS}{extra}");
    let extra_at = 1 + EVENT_FIELDS.split(',').count();
    let version =
        |row: &Row| -> rusqlite::Result<_> { Ok((stored_event(row, 0)?, read(row, extra_at)?)) };
    let mut hashed = Vec::new();
    for (id, event_id, uid, stored) in entries {
        let next = conn
            .query_row(
                &format!(
                    "SELECT {columns} FROM journal_entry \
                     WHERE event_id = ?1 AND uid = ?2 AND id > ?3 ORDER BY id LIMIT 1"
                ),
                params![event_id.0, uid, id],
                |row| {
                    let created = row.get::<_, Option<i64>>(0)?.is_none();
                    Ok((!created).then(|| version(row)).transpose())
                },
            )
            .optional()?;
        let version = match next {
            Some(previous) => previous.ok().flatten(),
            None => conn
                .query_row(
                    &format!("SELECT {columns} FROM event WHERE id = ?1 AND uid = ?2"),
                    params![event_id.0, uid],
                    |row| Ok(version(row).ok()),
                )
                .optional()?
                .flatten(),
        };
        hashed.push(Hashed {
            id,
            uid,
            stored,
            version,
        });
    }
    Ok(hashed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventFields, StoredWhen};
    use crate::store::{MIGRATION_COUNT, migrated_to};

    fn dentist() -> StoredEvent {
        StoredEvent {
            calendar_id: CalendarId(2),
            fields: EventFields {
                title: "Dentist".to_owned(),
                when: StoredWhen {
                    all_day: false,
                    start: "2026-10-05T09:00:00".to_owned(),
                    end: "2026-10-05T10:30:00".to_owned(),
                    start_zone: Some("Europe/Berlin".to_owned()),
                    end_zone: Some("Europe/Berlin".to_owned()),
                },
                location: "Main Street 1".to_owned(),
                description: "<b>Bring the card</b>".to_owned(),
                description_html: true,
                zone_pinned: false,
                rule: Some("FREQ=WEEKLY;INTERVAL=2".to_owned()),
                exdates: "2026-10-19T09:00:00".to_owned(),
                recurrence_id: None,
                object_id: Some(ObjectId(7)),
            },
        }
    }

    /// The object holding [`dentist`], as a server gives it.
    fn dentist_object() -> ServerObject {
        ServerObject {
            href: "/user/work/dentist.ics".to_owned(),
            etag: Some("\"1\"".to_owned()),
            data: "BEGIN:VCALENDAR\r\nX-UNKNOWN:kept\r\nEND:VCALENDAR\r\n".to_owned(),
        }
    }

    fn hex(hash: Vec<u8>) -> String {
        hash.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    /// Stored hashes must stay comparable: a change here needs a new
    /// [`HASH_FORMAT`] and a migration.
    #[test]
    fn the_hash_format_is_pinned() {
        assert_eq!(
            hex(hash("dentist", &dentist())),
            "504502fffd075c19137680ff6a036cea6c6cea463231a133b67601c826fecd4e"
        );
    }

    /// [`rehash`] and [`rehash_server_data`] recognise entries by their
    /// hash in the formats before.
    #[test]
    fn the_formats_before_are_pinned_too() {
        assert_eq!(
            hex(hash_as(1, "dentist", &dentist())),
            "b58a7dada5f3790c3b13fafabbe5a5b084fed79180f02f4a973156a43a3cd67e"
        );
        assert_eq!(
            hex(hash_as(2, "dentist", &dentist())),
            "f7e0c2e491d467b8c3f69ffe92aa06fc17219b6f1613034bf2b49ddb1628dc37"
        );
        assert_eq!(
            hex(hash_with(3, "dentist", &dentist(), Some(&dentist_object()))),
            "fe4126d5eb6b65344ab1724f86cd2625dcb74db6beb96dac243d25ae6901ee1c"
        );
    }

    /// The object a row refers to counts, and having none differs from
    /// having one.
    #[test]
    fn the_hash_covers_the_object() {
        let with = |object| {
            let mut event = dentist();
            event.fields.object_id = object;
            hash("dentist", &event)
        };
        let hashes = [with(Some(ObjectId(7))), with(Some(ObjectId(8))), with(None)];
        let distinct: std::collections::HashSet<_> = hashes.iter().collect();
        assert_eq!(distinct.len(), hashes.len());
    }

    /// A first sync writes, into a Read-only Calendar too, and records
    /// nothing.
    #[test]
    fn a_first_sync_writes_without_recording() {
        let directory = tempfile::TempDir::new().unwrap();
        let store = Store::open(&directory.path().join("freecal.sqlite3")).unwrap();
        let mut event = dentist();
        event.calendar_id = store.list_calendars().unwrap()[0].id;
        event.fields.object_id = None;
        let conn = store.connection();
        conn.execute(
            "UPDATE calendar SET read_only = 1 WHERE id = ?1",
            [event.calendar_id.0],
        )
        .unwrap();

        let recorded = record(&store, Origin::FirstSync, 0, |unit| {
            unit.create("dentist", &event)
        })
        .unwrap();

        assert_eq!(recorded.unit, None);
        assert_eq!(recorded.calendars, [event.calendar_id]);
        assert_eq!(
            store.find_event(recorded.value).unwrap().fields.title,
            "Dentist"
        );
        let count = |sql: &str| -> i64 { conn.query_row(sql, [], |row| row.get(0)).unwrap() };
        assert_eq!(count("SELECT COUNT(*) FROM journal_unit"), 0);
        assert_eq!(count("SELECT COUNT(*) FROM journal_entry"), 0);
    }

    /// A Local Store with [`dentist`] pulled by a sync into its first
    /// Calendar, in its object.
    struct Pulled {
        _directory: tempfile::TempDir,
        store: Store,
        object: ObjectId,
        id: EventId,
        event: StoredEvent,
    }

    impl Pulled {
        fn new() -> Self {
            let directory = tempfile::TempDir::new().unwrap();
            let store = Store::open(&directory.path().join("freecal.sqlite3")).unwrap();
            let calendar_id = store.list_calendars().unwrap()[0].id;
            let (object, id, event) = record(&store, Origin::Sync, 0, |unit| {
                let object = unit.mirror(calendar_id, &dentist_object(), None)?;
                let mut event = dentist();
                event.calendar_id = calendar_id;
                event.fields.object_id = Some(object);
                Ok((object, unit.create("dentist", &event)?, event))
            })
            .unwrap()
            .value;
            Self {
                _directory: directory,
                store,
                object,
                id,
                event,
            }
        }

        fn pending(&self) -> bool {
            self.store
                .connection()
                .query_row(
                    "SELECT pending FROM server_object WHERE id = ?1",
                    [self.object.0],
                    |row| row.get(0),
                )
                .unwrap()
        }

        fn renamed(&self, origin: Origin) -> Option<UnitId> {
            let mut event = self.event.clone();
            event.fields.title = "Dentist, renamed".to_owned();
            record(&self.store, origin, 0, |unit| unit.replace(self.id, &event))
                .unwrap()
                .unit
        }
    }

    #[test]
    fn a_sync_mirrors_the_object_and_never_marks_it_pending() {
        let pulled = Pulled::new();
        pulled.renamed(Origin::Sync);
        let again = ServerObject {
            etag: Some("\"2\"".to_owned()),
            ..dentist_object()
        };

        let mirrored = record(&pulled.store, Origin::Sync, 0, |unit| {
            unit.mirror(pulled.event.calendar_id, &again, Some(pulled.object))
        })
        .unwrap();

        assert_eq!(mirrored.value, pulled.object);
        assert_eq!(mirrored.unit, None, "the Journal doesn't record objects");
        assert_eq!(
            pulled.store.server_object(pulled.object).unwrap(),
            Some(again)
        );
        assert!(!pulled.pending());
    }

    #[test]
    fn a_user_action_an_import_and_an_undo_mark_the_object_pending() {
        for origin in [Origin::UserAction, Origin::Import] {
            let pulled = Pulled::new();
            pulled.renamed(origin);
            assert!(pulled.pending(), "{origin:?}");
            assert_eq!(
                pulled.store.find_event(pulled.id).unwrap().fields.object_id,
                Some(pulled.object)
            );
        }
        let pulled = Pulled::new();
        let unit = pulled.renamed(Origin::Sync).unwrap();
        undo(&pulled.store, unit, 0).unwrap();
        assert!(pulled.pending());
    }

    #[test]
    fn a_move_to_another_calendar_leaves_the_object_without_the_row() {
        let pulled = Pulled::new();
        let mut moved = pulled.event.clone();
        moved.calendar_id = pulled.store.list_calendars().unwrap()[1].id;

        record(&pulled.store, Origin::UserAction, 0, |unit| {
            unit.replace(pulled.id, &moved)
        })
        .unwrap();

        let row = pulled.store.find_event(pulled.id).unwrap();
        assert_eq!(row.calendar_id, moved.calendar_id);
        assert_eq!(row.fields.object_id, None);
        assert!(pulled.pending());
    }

    /// As a CalDAV object holds a Recurring Event with its Exceptions.
    #[test]
    fn a_new_row_joins_the_object_of_its_uid() {
        let pulled = Pulled::new();
        let mut exception = pulled.event.clone();
        exception.fields.rule = None;
        exception.fields.recurrence_id = Some("2026-10-19T09:00:00".to_owned());
        exception.fields.object_id = None;

        let (joined, other) = record(&pulled.store, Origin::UserAction, 0, |unit| {
            Ok((
                unit.create("dentist", &exception)?,
                unit.create("other", &exception)?,
            ))
        })
        .unwrap()
        .value;

        let object = |id| pulled.store.find_event(id).unwrap().fields.object_id;
        assert_eq!(object(joined), Some(pulled.object));
        assert_eq!(object(other), None);
        assert!(pulled.pending());
    }

    /// After a migration that added fields, an entry's hash in a format
    /// before is made again in the current one; any other hash stays.
    #[test]
    fn rehash_makes_hashes_of_the_formats_before_again() {
        let directory = tempfile::TempDir::new().unwrap();
        let store = Store::open(&directory.path().join("freecal.sqlite3")).unwrap();
        let mut event = dentist();
        event.calendar_id = store.list_calendars().unwrap()[0].id;
        // A user's Event, which has none.
        event.fields.object_id = None;
        let mut moved = event.clone();
        moved.fields.title = "Dentist, moved".to_owned();
        let created = record(&store, Origin::UserAction, 0, |unit| {
            unit.create("dentist", &event)
        })
        .unwrap()
        .value;
        record(&store, Origin::UserAction, 0, |unit| {
            unit.replace(created, &moved)
        })
        .unwrap();
        let conn = store.connection();
        let hashes = || -> Vec<Vec<u8>> {
            conn.prepare("SELECT hash FROM journal_entry ORDER BY id")
                .unwrap()
                .query_map([], |row| row.get(0))
                .unwrap()
                .collect::<rusqlite::Result<_>>()
                .unwrap()
        };
        let set = |id: i64, hash: Vec<u8>| {
            conn.execute(
                "UPDATE journal_entry SET hash = ?2 WHERE id = (SELECT MIN(id) FROM journal_entry) + ?1",
                params![id, hash],
            )
            .unwrap();
        };
        set(0, hash_as(1, "dentist", &event));
        set(1, b"unknown".to_vec());

        rehash(conn).unwrap();

        assert_eq!(hashes(), [hash("dentist", &event), b"unknown".to_vec()]);
        set(1, hash_as(2, "dentist", &moved));
        rehash(conn).unwrap();
        assert_eq!(hashes()[1], hash("dentist", &moved));
    }

    /// Adds a unit of origin sync to a Local Store of an earlier version.
    fn old_unit(conn: &Connection) -> i64 {
        conn.execute(
            "INSERT INTO journal_unit (origin, made_at) VALUES ('sync', 0)",
            [],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    /// Opening a Local Store from before the original server data re-hashes
    /// its entries once the migrations ran.
    #[test]
    fn opening_a_local_store_from_before_rehashes_its_entries() {
        let directory = tempfile::TempDir::new().unwrap();
        let path = directory.path().join("freecal.sqlite3");
        // Before ticket 81's migration and those of tickets 82, 56, 85, 86 and 87.
        let conn = migrated_to(&path, MIGRATION_COUNT - 6);
        let mut event = dentist();
        event.fields.object_id = None;
        conn.execute_batch(
            "INSERT INTO calendar (account_id, name, colour) VALUES (1, 'Work', '#3f51b5');
             INSERT INTO event (id, calendar_id, uid, title, all_day, start_at, end_at,
                 start_zone, end_zone, start_utc, end_utc, location, description,
                 description_html, rrule, exdates)
             VALUES (1, 1, 'dentist', 'Dentist', 0, '2026-10-05T09:00:00',
                 '2026-10-05T10:30:00', 'Europe/Berlin', 'Europe/Berlin', 0, 0,
                 'Main Street 1', '<b>Bring the card</b>', 1, 'FREQ=WEEKLY;INTERVAL=2',
                 '2026-10-19T09:00:00');",
        )
        .unwrap();
        event.calendar_id = CalendarId(1);
        let unit = old_unit(&conn);
        conn.execute(
            "INSERT INTO journal_entry (unit_id, event_id, uid, hash) VALUES (?1, 1, 'dentist', ?2)",
            params![unit, hash_as(2, "dentist", &event)],
        )
        .unwrap();
        drop(conn);

        let store = Store::open(&path).unwrap();

        let stored: Vec<u8> = store
            .connection()
            .query_row("SELECT hash FROM journal_entry", [], |row| row.get(0))
            .unwrap();
        assert_eq!(stored, hash("dentist", &event));
    }

    /// Ticket 86: a row's original server data becomes an object, which its
    /// Exceptions and the entries of its versions refer to, and entries
    /// whose hashes covered the data still undo.
    #[test]
    fn opening_a_local_store_from_before_server_objects_moves_the_data_into_them() {
        let directory = tempfile::TempDir::new().unwrap();
        let path = directory.path().join("freecal.sqlite3");
        let conn = migrated_to(&path, MIGRATION_COUNT - 2);
        let server = dentist_object();
        conn.execute_batch(
            "INSERT INTO calendar (account_id, name, colour) VALUES (1, 'Work', '#3f51b5');",
        )
        .unwrap();
        let insert =
            |id: i64, title: &str, recurrence_id: Option<&str>, server: Option<&ServerObject>| {
                conn.execute(
                    "INSERT INTO event (id, calendar_id, uid, title, all_day, start_at, end_at,
                     start_zone, end_zone, start_utc, end_utc, location, description,
                     description_html, rrule, exdates, recurrence_id, server_href, server_etag,
                     server_data)
                 VALUES (?1, 1, 'dentist', ?2, 0, '2026-10-05T09:00:00',
                     '2026-10-05T10:30:00', 'Europe/Berlin', 'Europe/Berlin', 0, 0,
                     'Main Street 1', '<b>Bring the card</b>', 1, ?3,
                     '2026-10-19T09:00:00', ?4, ?5, ?6, ?7)",
                    params![
                        id,
                        title,
                        recurrence_id.is_none().then_some("FREQ=WEEKLY;INTERVAL=2"),
                        recurrence_id,
                        server.map(|server| &server.href),
                        server.and_then(|server| server.etag.as_ref()),
                        server.map(|server| &server.data),
                    ],
                )
                .unwrap();
            };
        insert(1, "Dentist", None, Some(&server));
        insert(2, "Dentist, later", Some("2026-10-19T09:00:00"), None);
        // A sync renamed the Recurring Event: its previous version has the
        // object's address with older data.
        let mut event = dentist();
        event.calendar_id = CalendarId(1);
        event.fields.object_id = None;
        let unit = old_unit(&conn);
        conn.execute(
            "INSERT INTO journal_entry (unit_id, event_id, uid, hash, calendar_id, title,
                 all_day, start_at, end_at, start_zone, end_zone, location, description,
                 description_html, zone_pinned, rrule, exdates, server_href, server_etag,
                 server_data)
             SELECT ?1, id, uid, ?2, calendar_id, 'Dentist, before', all_day, start_at,
                 end_at, start_zone, end_zone, location, description, description_html,
                 zone_pinned, rrule, exdates, server_href, '\"0\"', 'BEGIN:VCALENDAR'
             FROM event WHERE id = 1",
            params![unit, hash_with(3, "dentist", &event, Some(&server))],
        )
        .unwrap();
        drop(conn);

        let store = Store::open(&path).unwrap();

        let object = store.find_event(EventId(1)).unwrap().fields.object_id;
        assert!(object.is_some());
        assert_eq!(store.server_object(object.unwrap()).unwrap(), Some(server));
        assert_eq!(
            store.find_event(EventId(2)).unwrap().fields.object_id,
            object
        );
        undo(&store, UnitId(unit), 0).unwrap();
        let undone = store.find_event(EventId(1)).unwrap();
        assert_eq!(undone.fields.title, "Dentist, before");
        assert_eq!(undone.fields.object_id, object);
    }
}
