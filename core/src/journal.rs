//! The Change Journal (ADR 0009) and the only writer of Events: every change
//! to an Event goes through a [`Unit`] in [`record`], which keeps the Event's
//! previous version and a hash of its new one, so that no change can skip
//! the Journal. Changes made together, such as an import, are one unit.

use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::store::{EVENT_FIELDS, EventFields, Store, StoredEvent, event_fields};
use crate::{CalendarId, CoreError, EventId, Result};

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
/// Sync, Snapshot restore and Conflict resolution come with their tickets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Origin {
    UserAction,
    Import,
    Undo,
}

impl Origin {
    fn as_str(self) -> &'static str {
        match self {
            Origin::UserAction => "user",
            Origin::Import => "import",
            Origin::Undo => "undo",
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
    store.transaction(|| {
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
    })
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

    pub fn find_event_by_uid(
        &self,
        calendar_id: CalendarId,
        uid: &str,
    ) -> Result<Option<(EventId, EventFields)>> {
        self.store.find_event_by_uid(calendar_id, uid)
    }

    /// Adds an Event with the UID `uid`.
    pub fn create(&mut self, uid: &str, event: &StoredEvent) -> Result<EventId> {
        self.writable(event.calendar_id)?;
        let id = write(self.store, None, uid, event, event.fields.when.instants()?)?;
        self.entry(id, uid, None, Some(event))?;
        Ok(id)
    }

    /// Changes the Event `id`, possibly moving it to another Calendar. A
    /// change that changes nothing isn't recorded.
    pub fn replace(&mut self, id: EventId, event: &StoredEvent) -> Result<()> {
        let (uid, previous) = self.previous(id)?;
        self.writable(previous.calendar_id)?;
        self.writable(event.calendar_id)?;
        if hash(&uid, &previous) == hash(&uid, event) {
            return Ok(());
        }
        write(
            self.store,
            Some(id),
            &uid,
            event,
            event.fields.when.instants()?,
        )?;
        self.entry(id, &uid, Some(&previous), Some(event))
    }

    pub fn delete(&mut self, id: EventId) -> Result<()> {
        let (uid, previous) = self.previous(id)?;
        self.writable(previous.calendar_id)?;
        self.store
            .connection()
            .execute("DELETE FROM event WHERE id = ?1", [id.0])?;
        self.entry(id, &uid, Some(&previous), None)
    }

    /// Writes an earlier version back: over the Event `id` while it is the
    /// one with the UID `uid`, otherwise as a new Event, under `id` while no
    /// other Event has it.
    fn put_back(&mut self, id: EventId, uid: &str, event: &StoredEvent) -> Result<()> {
        if current(self.store, id, uid)?.is_some() {
            return self.replace(id, event);
        }
        self.writable(event.calendar_id)?;
        let taken: bool = self.store.connection().query_row(
            "SELECT EXISTS (SELECT 1 FROM event WHERE id = ?1)",
            [id.0],
            |row| row.get(0),
        )?;
        // A version FreeCal can't read the time of was stored like that; it
        // reads as unreadable again rather than stopping the undo.
        let instants = event.fields.when.instants().unwrap_or_default();
        let id = write(self.store, (!taken).then_some(id), uid, event, instants)?;
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
    /// every origin so far. Incremental syncs will write there (ticket 54).
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
    /// an Event created or deleted, starting the unit with its first entry.
    fn entry(
        &mut self,
        id: EventId,
        uid: &str,
        previous: Option<&StoredEvent>,
        new: Option<&StoredEvent>,
    ) -> Result<()> {
        for event in previous.into_iter().chain(new) {
            if !self.calendars.contains(&event.calendar_id) {
                self.calendars.push(event.calendar_id);
            }
        }
        let conn = self.store.connection();
        let unit = match self.id {
            Some(unit) => unit,
            None => {
                conn.execute(
                    "INSERT INTO journal_unit (origin, made_at) VALUES (?1, ?2)",
                    params![self.origin.as_str(), self.made_at],
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
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)"
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
            ],
        )?;
        Ok(())
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
         description_html, zone_pinned) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15) \
         ON CONFLICT (id) DO UPDATE SET uid = excluded.uid, \
           calendar_id = excluded.calendar_id, title = excluded.title, \
           all_day = excluded.all_day, start_at = excluded.start_at, \
           end_at = excluded.end_at, start_zone = excluded.start_zone, \
           end_zone = excluded.end_zone, start_utc = excluded.start_utc, \
           end_utc = excluded.end_utc, location = excluded.location, \
           description = excluded.description, \
           description_html = excluded.description_html, \
           zone_pinned = excluded.zone_pinned",
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
    /// The Event's UID, which tells it apart from a later Event that got its
    /// ID after it was deleted.
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

/// The version of [`hash`]'s format, its first byte.
const HASH_FORMAT: u8 = 1;

/// The SHA-256 hash of an Event's version: its Calendar, UID and fields, as
/// a list of items each prefixed with its length, after [`HASH_FORMAT`].
///
/// A field added to Events, such as the original server data (ADR 0004),
/// joins the list and bumps [`HASH_FORMAT`], and a migration re-hashes every
/// entry: the new version an entry hashes is the previous version the next
/// entry for that Event holds, or the Event as it is now for its last entry.
fn hash(uid: &str, event: &StoredEvent) -> Vec<u8> {
    let fields = &event.fields;
    let when = &fields.when;
    let mut hasher = Sha256::new();
    hasher.update([HASH_FORMAT]);
    let mut item = |bytes: &[u8]| {
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    };
    // An absent zone is told apart from an empty one by its first byte.
    let zone = |zone: &Option<String>| match zone {
        None => vec![0],
        Some(zone) => [&[1], zone.as_bytes()].concat(),
    };
    item(&event.calendar_id.0.to_be_bytes());
    item(uid.as_bytes());
    item(fields.title.as_bytes());
    item(&[when.all_day.into()]);
    item(when.start.as_bytes());
    item(when.end.as_bytes());
    item(&zone(&when.start_zone));
    item(&zone(&when.end_zone));
    item(fields.location.as_bytes());
    item(fields.description.as_bytes());
    item(&[fields.description_html.into()]);
    item(&[fields.zone_pinned.into()]);
    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::StoredWhen;

    /// Stored hashes must stay comparable: a change here needs a new
    /// [`HASH_FORMAT`] and a migration.
    #[test]
    fn the_hash_format_is_pinned() {
        let event = StoredEvent {
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
            },
        };

        let hex: String = hash("dentist", &event)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();

        assert_eq!(
            hex,
            "b58a7dada5f3790c3b13fafabbe5a5b084fed79180f02f4a973156a43a3cd67e"
        );
    }
}
