//! The Change Journal (ADR 0009) and the only writer of Events: every change
//! to an Event goes through a [`Unit`] in [`record`], which keeps the Event's
//! previous version and a hash of its new one, so that no change can skip
//! the Journal. Changes made together, such as an import, are one unit.
//! A unit writes the server objects too (ADR 0010), which the Journal doesn't
//! record: each Event's version holds only the object it refers to.
//!
//! This file holds what the Journal decides and no SQL: it reads and writes
//! through [`sql`](super::sql), whose writes to the tables it guards take a
//! [`Journaled`], which only this file can make.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{Store, StoredEvent, Uid, UidEvent};
use crate::event::{ObjectId, ServerObject, Source, rule};
use crate::{AccountId, CalendarId, CoreError, EventId, Result};

/// Leave to write the tables the Change Journal guards. Its field is private
/// to this file, so that nothing else can make one.
pub(super) struct Journaled(());

/// The [`Journaled`] this file's writes pass.
const JOURNALED: &Journaled = &Journaled(());

/// The time zone rules FreeCal was built with, which the derived UTC columns
/// are computed with.
const TIME_ZONE_RULES: &str = chrono_tz::IANA_TZDB_VERSION;

/// Recomputes every Event's derived UTC instants when the Local Store's were
/// computed with other time zone rules than this build's, while it opens. The
/// Journal doesn't record them, so this writes outside any unit. A row whose
/// time can't be read keeps what it had.
pub(super) fn recompute_instants(store: &Store) -> Result<()> {
    if store.time_zone_rules()? == TIME_ZONE_RULES {
        return Ok(());
    }
    for (id, fields) in store.all_event_fields()? {
        if let Ok(instants) = rule::span(&fields) {
            store.write_instants(JOURNALED, id, instants)?;
        }
    }
    store.set_time_zone_rules(TIME_ZONE_RULES)
}

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

    /// Whether it is a sync, whose changes come from the server: only a sync
    /// writes the mirror, and only other origins' changes to an object's
    /// rows mark the object pending, for FreeCal to send.
    fn is_sync(self) -> bool {
        matches!(self, Origin::FirstSync | Origin::Sync)
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
            pending: Vec::new(),
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
        let entries = store.entries(id)?;
        if entries.iter().any(|entry| entry.undone) {
            return Err(CoreError::AlreadyUndone(id));
        }
        // Latest first, so that each entry finds the version it made.
        for entry in entries.iter().rev() {
            let current = store.current_event(entry.event_id, &entry.uid)?;
            if current.map(|event| hash(&entry.uid, &event)) != entry.hash {
                return Err(CoreError::ChangedSinceUnit(id));
            }
            match &entry.previous {
                None => unit.delete(entry.event_id)?,
                Some(previous) => unit.put_back(entry.event_id, &entry.uid, previous)?,
            }
        }
        store.mark_undone(JOURNALED, id)
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
        for calendar in store.account_calendars(id)? {
            delete_calendar_within(store, calendar)?;
        }
        store.delete_account_row(JOURNALED, id)
    })
}

/// [`delete_calendar`] inside a transaction already begun.
fn delete_calendar_within(store: &Store, id: CalendarId) -> Result<()> {
    store.delete_calendar_events(JOURNALED, id)?;
    store.delete_calendar_row(JOURNALED, id)
}

/// One unit being recorded, inside [`record`]'s transaction. Each write reads
/// the Event's previous version itself.
pub(crate) struct Unit<'a> {
    store: &'a Store,
    origin: Origin,
    made_at: i64,
    id: Option<UnitId>,
    calendars: Vec<CalendarId>,
    /// The server objects it marked pending.
    pending: Vec<ObjectId>,
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
        debug_assert!(self.origin.is_sync());
        self.writable(calendar_id)?;
        self.store
            .write_object(JOURNALED, calendar_id, object, over)
    }

    /// Records that FreeCal couldn't read what the server holds now at
    /// `href` in a Calendar: the objects there keep the last version it
    /// could, and their rows stay as they are.
    pub fn unreadable(&self, calendar_id: CalendarId, href: &str) -> Result<()> {
        debug_assert!(self.origin.is_sync());
        self.store.mark_unreadable(JOURNALED, calendar_id, href)
    }

    /// Removes the server object `id` from the mirror, and its rows with it,
    /// as a sync finds it gone from the server.
    pub fn remove_object(&mut self, id: ObjectId) -> Result<()> {
        debug_assert!(self.origin.is_sync());
        for event in self.events_in_object(id)? {
            self.delete(event.id)?;
        }
        self.store.delete_object(JOURNALED, id)
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
        let id = self
            .store
            .write_event(JOURNALED, None, uid, event, rule::span(&event.fields)?)?;
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
        self.store
            .write_event(JOURNALED, Some(id), &uid, event, rule::span(&event.fields)?)?;
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
        let object = match self.origin {
            Origin::FirstSync | Origin::Sync => return Ok(event.clone()),
            Origin::Undo => match event.fields.object_id {
                Some(object) => self.store.object_in(object, event.calendar_id)?,
                None => None,
            },
            Origin::UserAction | Origin::Import => match previous {
                Some(previous) => previous
                    .fields
                    .object_id
                    .filter(|_| previous.calendar_id == event.calendar_id),
                None => self.store.object_of_uid(event.calendar_id, uid)?,
            },
        };
        let mut event = event.clone();
        event.fields.object_id = object;
        Ok(event)
    }

    pub fn delete(&mut self, id: EventId) -> Result<()> {
        let (uid, previous) = self.previous(id)?;
        self.writable(previous.calendar_id)?;
        self.store.delete_event(JOURNALED, id)?;
        self.entry(id, &uid, Some(&previous), None)
    }

    /// Writes an earlier version back under its own ID: over the Event `id`
    /// while it is the one with the UID `uid`, otherwise as a new Event. No
    /// other Event has that ID, as IDs are never given out again (ticket 85).
    fn put_back(&mut self, id: EventId, uid: &str, event: &StoredEvent) -> Result<()> {
        if self.store.current_event(id, uid)?.is_some() {
            return self.replace(id, event);
        }
        self.writable(event.calendar_id)?;
        let event = &self.placed(None, uid, event)?;
        // A version FreeCal can't read the time of was stored like that; it
        // reads as unreadable again rather than stopping the undo.
        let instants = rule::span(&event.fields).unwrap_or_default();
        self.store
            .write_event(JOURNALED, Some(id), uid, event, instants)?;
        self.entry(id, uid, None, Some(event))
    }

    fn previous(&self, id: EventId) -> Result<(String, StoredEvent)> {
        self.store
            .event_and_uid(id)?
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
        for event in previous.into_iter().chain(new) {
            if !self.calendars.contains(&event.calendar_id) {
                self.calendars.push(event.calendar_id);
            }
            if let Some(object) = event.fields.object_id
                && !self.origin.is_sync()
                && !self.pending.contains(&object)
            {
                self.store.mark_pending(JOURNALED, object)?;
                self.pending.push(object);
            }
        }
        let Some(origin) = self.origin.recorded_as() else {
            return Ok(());
        };
        let unit = match self.id {
            Some(unit) => unit,
            None => *self
                .id
                .insert(self.store.insert_unit(JOURNALED, origin, self.made_at)?),
        };
        self.store.insert_entry(
            JOURNALED,
            unit,
            id,
            uid,
            previous,
            new.map(|event| hash(uid, event)),
        )
    }
}

impl Source for Unit<'_> {
    fn events_with_uid(&self, uid: Uid) -> Result<Vec<UidEvent>> {
        self.store.events_with_uid(uid)
    }
}

/// One Event's change in a unit, as the Change Journal holds it.
pub(super) struct Entry {
    pub event_id: EventId,
    /// The Event's UID, which its hashes cover.
    pub uid: String,
    /// Its version before the change; none when the change created it.
    pub previous: Option<StoredEvent>,
    /// The hash of its version after the change; none when the change
    /// deleted it.
    pub hash: Option<Vec<u8>>,
    pub undone: bool,
}

/// The version of [`hash`]'s format, its first byte.
const HASH_FORMAT: u8 = 1;

/// The SHA-256 hash of an Event's version: its Calendar, UID and fields, as
/// a list of items each prefixed with its length, after [`HASH_FORMAT`].
///
/// A field added to Events joins the list. Before the first release it goes
/// into the baseline schema; after it, it bumps [`HASH_FORMAT`], and its
/// migration adds it to the Change Journal's entries too, whose hashes then
/// have to be made again in code.
fn hash(uid: &str, event: &StoredEvent) -> Vec<u8> {
    let fields = &event.fields;
    let when = &fields.when;
    let mut hasher = Sha256::new();
    hasher.update([HASH_FORMAT]);
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
    item(&optional(&fields.rule));
    item(fields.exdates.as_bytes());
    item(&optional(&fields.recurrence_id));
    match fields.object_id {
        None => item(&[0]),
        Some(object) => item(&[&[1], &object.0.to_be_bytes()[..]].concat()),
    }
    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventFields, StoredWhen};

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
            "843961e16d587cdc1b783e9dc7e58bc02db94ed18830ac77bf376c37d0086a27"
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
}
