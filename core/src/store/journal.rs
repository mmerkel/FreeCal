//! The Change Journal (ADR 0009) and the only writer of Events: every change
//! to an Event goes through a [`JournalTransaction`] in [`record`], which
//! keeps the Event's previous Event Version and a hash of its new one, so
//! that no change can skip the Journal. Changes made together, such as an
//! import, are one Journal Transaction. A Journal Transaction writes the
//! Server Objects and their Combined Event Keys too (ADR 0010, 0011), which
//! the Journal doesn't record: an Event Version holds FreeCal's fields only,
//! and Event Rows find their Server Object through the Combined Event Keys.
//! Every Journal Transaction but a sync's adds the Combined Event Keys of the
//! Events it changes to the Unsent Changes, what FreeCal has to send.
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
/// Journal doesn't record them, so this writes outside any Journal
/// Transaction. An Event Row whose time can't be read keeps what it had.
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

/// A Journal Transaction: what undoes a change, or changes made together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    test,
    derive(ts_rs::TS),
    ts(type = "number & { readonly __brand: \"JournalTransactionId\" }")
)]
#[serde(transparent)]
pub struct JournalTransactionId(pub i64);

/// What a change to Events returns: its value, and the Journal Transaction
/// that undoes it, none when it changed nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Changed<T> {
    pub journal_transaction: Option<JournalTransactionId>,
    pub value: T,
}

/// Where a Journal Transaction's changes came from, which decides where they
/// may write.
/// Snapshot restore and Conflict resolution come with their tickets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransactionOrigin {
    UserAction,
    Import,
    Undo,
    /// A Calendar's First Sync: it writes like any Transaction Origin but
    /// records nothing (ADR 0009), so its Journal Transaction is never
    /// stored.
    FirstSync,
    /// An incremental sync of one Calendar, what its server changed.
    Sync,
}

impl TransactionOrigin {
    /// How a stored Journal Transaction records it; none for a Transaction
    /// Origin that isn't recorded.
    fn recorded_as(self) -> Option<&'static str> {
        match self {
            TransactionOrigin::UserAction => Some("user"),
            TransactionOrigin::Import => Some("import"),
            TransactionOrigin::Undo => Some("undo"),
            TransactionOrigin::Sync => Some("sync"),
            TransactionOrigin::FirstSync => None,
        }
    }

    /// Whether it is a sync, whose changes come from the server: only a sync
    /// writes the Mirror.
    fn is_sync(self) -> bool {
        matches!(self, TransactionOrigin::FirstSync | TransactionOrigin::Sync)
    }
}

/// What [`record`] did: the Journal Transaction, none when nothing changed,
/// and the Calendars whose Events changed, in the order they were first
/// changed.
pub(crate) struct Recorded<T> {
    pub journal_transaction: Option<JournalTransactionId>,
    pub calendars: Vec<CalendarId>,
    pub value: T,
}

/// Runs `change` in one database transaction as one Journal Transaction of
/// `origin`, made at `made_at` (Unix seconds). If it fails, nothing it did is
/// kept. A Journal Transaction that changed nothing isn't stored.
pub(crate) fn record<T>(
    store: &Store,
    origin: TransactionOrigin,
    made_at: i64,
    change: impl FnOnce(&mut JournalTransaction) -> Result<T>,
) -> Result<Recorded<T>> {
    store.transaction(|| {
        let mut journal_transaction = JournalTransaction {
            store,
            origin,
            made_at,
            id: None,
            calendars: Vec::new(),
        };
        let value = change(&mut journal_transaction)?;
        Ok(Recorded {
            journal_transaction: journal_transaction.id,
            calendars: journal_transaction.calendars,
            value,
        })
    })
}

/// Undoes the Journal Transaction `id` whole, as a Journal Transaction with
/// the Transaction Origin undo: every Event gets its previous Event Version
/// back under the same identity. Refused while an Event changed since, and
/// for a Journal Transaction undone already.
pub(crate) fn undo(store: &Store, id: JournalTransactionId, made_at: i64) -> Result<Recorded<()>> {
    record(
        store,
        TransactionOrigin::Undo,
        made_at,
        |journal_transaction| {
            let modifications = store.modifications(id)?;
            if modifications.iter().any(|modification| modification.undone) {
                return Err(CoreError::AlreadyUndone(id));
            }
            // Latest first, so that each Modification finds the Event Version
            // it made.
            for modification in modifications.iter().rev() {
                let current = store.current_event(modification.event_id, &modification.uid)?;
                if current.map(|event| hash(&modification.uid, &event)) != modification.hash {
                    return Err(CoreError::ChangedSinceJournalTransaction(id));
                }
                match &modification.previous {
                    None => journal_transaction.delete(modification.event_id)?,
                    Some(previous) => journal_transaction.put_back(
                        modification.event_id,
                        &modification.uid,
                        previous,
                    )?,
                }
            }
            store.mark_undone(JOURNALED, id)
        },
    )
}

/// Deletes a Calendar with its Events. This isn't recorded (ADR 0009): the
/// Modifications of its Events go too, and Journal Transactions left without
/// Modifications. So do the Modifications that would put an Event back into
/// it, such as one moved out or deleted, with that Event's earlier
/// Modifications, which are undone only through them; its later
/// Modifications stay.
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

/// [`delete_calendar`] inside a database transaction already begun.
fn delete_calendar_within(store: &Store, id: CalendarId) -> Result<()> {
    store.delete_calendar_events(JOURNALED, id)?;
    store.delete_calendar_row(JOURNALED, id)
}

/// One Journal Transaction being recorded, inside [`record`]'s database
/// transaction. Each write reads the Event's previous Event Version itself.
pub(crate) struct JournalTransaction<'a> {
    store: &'a Store,
    origin: TransactionOrigin,
    made_at: i64,
    id: Option<JournalTransactionId>,
    calendars: Vec<CalendarId>,
}

impl JournalTransaction<'_> {
    pub fn find_event(&self, id: EventId) -> Result<StoredEvent> {
        self.store.find_event(id)
    }

    /// The Server Objects of a Calendar at `href`, in the order they were
    /// added.
    pub fn objects_at(&self, calendar_id: CalendarId, href: &str) -> Result<Vec<ObjectId>> {
        self.store.objects_at(calendar_id, href)
    }

    /// The Corresponding Server Object of the UID `uid` in a Calendar
    /// ([`Store::corresponding_server_object`]).
    pub fn corresponding_server_object(
        &self,
        calendar_id: CalendarId,
        uid: &str,
    ) -> Result<Option<ObjectId>> {
        self.store.corresponding_server_object(calendar_id, uid)
    }

    /// The UIDs whose Corresponding Server Object is `id`.
    pub fn corresponding_uids(&self, id: ObjectId) -> Result<Vec<String>> {
        self.store.corresponding_uids(id)
    }

    /// The UIDs the Server Object `id` holds, whether it is their
    /// Corresponding Server Object or not.
    pub fn held_uids(&self, id: ObjectId) -> Result<Vec<String>> {
        self.store.held_uids(id)
    }

    /// The Server Object `id` as the Mirror holds it, none if it is gone.
    pub fn server_object(&self, id: ObjectId) -> Result<Option<ServerObject>> {
        self.store.server_object(id)
    }

    /// Makes `object`, as the server holds it now, the Mirror's version of
    /// the Server Object `over` in a Calendar, address included, or of a
    /// new one, and returns its ID. Its Combined Event Keys become
    /// `combined_event_keys`, one per component: its UID and recurrence ID,
    /// normalised as the Event Rows hold it. Only a sync writes the Mirror,
    /// as only it knows what the server holds.
    pub fn mirror(
        &self,
        calendar_id: CalendarId,
        object: &ServerObject,
        over: Option<ObjectId>,
        combined_event_keys: &[(&str, Option<&str>)],
    ) -> Result<ObjectId> {
        debug_assert!(self.origin.is_sync());
        self.writable(calendar_id)?;
        self.store
            .write_object(JOURNALED, calendar_id, object, over, combined_event_keys)
    }

    /// Makes `combined_event_keys`, all with the UID `uid`, the Combined
    /// Event Keys with that UID of the Server Object `id`, whose data stays
    /// as it is: for when its Event Rows come to hold their recurrence IDs
    /// otherwise, as when it becomes the UID's Corresponding Server Object
    /// in place of one that held its Recurring Event.
    pub fn rewrite_combined_event_keys(
        &self,
        id: ObjectId,
        uid: &str,
        combined_event_keys: &[(&str, Option<&str>)],
    ) -> Result<()> {
        debug_assert!(self.origin.is_sync());
        self.store
            .write_uid_combined_event_keys(JOURNALED, id, uid, combined_event_keys)
    }

    /// Renormalises the recurrence IDs of the Combined Event Keys with the
    /// UID `uid` of a Calendar's Server Objects and Unsent Changes as
    /// `rejoined` gives them, for the step that renormalises the Event
    /// Rows' ([`Loaded::join`]); a Combined Event Key it gives none for
    /// keeps its own.
    ///
    /// [`Loaded::join`]: crate::event::Loaded::join
    pub fn rejoin_combined_event_keys(
        &self,
        calendar_id: CalendarId,
        uid: &str,
        rejoined: impl Fn(&str) -> Option<String>,
    ) -> Result<()> {
        let joined = |rows: Vec<(i64, Option<String>)>| {
            rows.into_iter()
                .filter_map(|(row, rid)| Some((row, rejoined(rid.as_deref()?)?)))
        };
        for (row, joined) in joined(self.store.combined_event_keys_with_uid(calendar_id, uid)?) {
            self.store
                .set_combined_event_key_recurrence_id(JOURNALED, row, &joined)?;
        }
        for (row, joined) in joined(self.store.unsent_with_uid(calendar_id, uid)?) {
            self.store
                .set_unsent_recurrence_id(JOURNALED, row, &joined)?;
        }
        Ok(())
    }

    /// Records that FreeCal couldn't read what the server holds now at
    /// `href` in a Calendar: the Server Objects there keep the last version
    /// it could with their Combined Event Keys, and their Event Rows stay as
    /// they are.
    pub fn unreadable(&self, calendar_id: CalendarId, href: &str) -> Result<()> {
        debug_assert!(self.origin.is_sync());
        self.store.mark_unreadable(JOURNALED, calendar_id, href)
    }

    /// Removes the Server Object `id` from the Mirror with its Combined Event
    /// Keys, as a sync finds it gone from the server. The Event Rows of the
    /// UIDs it was the Corresponding Server Object of stay, for the sync to
    /// give to their next Corresponding Server Object or delete.
    pub fn remove_object(&self, id: ObjectId) -> Result<()> {
        debug_assert!(self.origin.is_sync());
        self.store.delete_object(JOURNALED, id)
    }

    /// Records, with the Journal Transaction's changes, that a sync of a
    /// Calendar finished with the sync token `token`.
    pub fn synced(&self, calendar_id: CalendarId, token: Option<&str>) -> Result<()> {
        self.store.set_synced(calendar_id, token)
    }

    /// Adds an Event with the UID `uid`.
    pub fn create(&mut self, uid: &str, event: &StoredEvent) -> Result<EventId> {
        self.writable(event.calendar_id)?;
        let id = self
            .store
            .write_event(JOURNALED, None, uid, event, rule::span(&event.fields)?)?;
        self.record_modification(id, uid, None, Some(event))?;
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
        self.store
            .write_event(JOURNALED, Some(id), &uid, event, rule::span(&event.fields)?)?;
        self.record_modification(id, &uid, Some(&previous), Some(event))
    }

    pub fn delete(&mut self, id: EventId) -> Result<()> {
        let (uid, previous) = self.previous(id)?;
        self.writable(previous.calendar_id)?;
        self.store.delete_event(JOURNALED, id)?;
        self.record_modification(id, &uid, Some(&previous), None)
    }

    /// Writes an earlier Event Version back under its own ID: over the Event
    /// `id` while it is the one with the UID `uid`, otherwise as a new Event.
    /// No other Event has that ID, as IDs are never given out again (ticket
    /// 85).
    fn put_back(&mut self, id: EventId, uid: &str, event: &StoredEvent) -> Result<()> {
        if self.store.current_event(id, uid)?.is_some() {
            return self.replace(id, event);
        }
        self.writable(event.calendar_id)?;
        // An Event Version FreeCal can't read the time of was stored like
        // that; it reads as unreadable again rather than stopping the undo.
        let instants = rule::span(&event.fields).unwrap_or_default();
        self.store
            .write_event(JOURNALED, Some(id), uid, event, instants)?;
        self.record_modification(id, uid, None, Some(event))
    }

    fn previous(&self, id: EventId) -> Result<(String, StoredEvent)> {
        self.store
            .event_and_uid(id)?
            .ok_or(CoreError::EventNotFound(id))
    }

    /// Refuses writes into a missing Calendar, and into a Read-only one for
    /// every Transaction Origin but a sync, which loads what the server has.
    /// Every write checks this itself; a change that targets a Calendar, such
    /// as an import, checks it first, so that it is refused even when it
    /// would write nothing.
    pub fn writable(&self, id: CalendarId) -> Result<()> {
        let read_only = self.store.find_calendar(id)?.read_only;
        match self.origin {
            TransactionOrigin::UserAction | TransactionOrigin::Import | TransactionOrigin::Undo
                if read_only =>
            {
                Err(CoreError::CalendarReadOnly(id))
            }
            _ => Ok(()),
        }
    }

    /// Records that the Event `id` went from `previous` to `new`, none for
    /// an Event created or deleted, as a Modification, starting the Journal
    /// Transaction with its first one. A Transaction Origin that isn't
    /// recorded only notes the Calendars.
    ///
    /// Every Transaction Origin but a sync also adds both Event Versions'
    /// Combined Event Keys to the Unsent Changes, in a server Calendar (ADR
    /// 0011): a write into a Read-only one was refused already. A sync's
    /// changes are what the server holds.
    fn record_modification(
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
            if !self.origin.is_sync() {
                self.store.add_unsent_change(
                    JOURNALED,
                    event.calendar_id,
                    uid,
                    event.fields.recurrence_id.as_deref(),
                    self.made_at,
                )?;
            }
        }
        let Some(origin) = self.origin.recorded_as() else {
            return Ok(());
        };
        let journal_transaction = match self.id {
            Some(journal_transaction) => journal_transaction,
            None => *self.id.insert(self.store.insert_journal_transaction(
                JOURNALED,
                origin,
                self.made_at,
            )?),
        };
        self.store.insert_modification(
            JOURNALED,
            journal_transaction,
            id,
            uid,
            previous,
            new.map(|event| hash(uid, event)),
        )
    }
}

impl Source for JournalTransaction<'_> {
    fn events_with_uid(&self, uid: Uid) -> Result<Vec<UidEvent>> {
        self.store.events_with_uid(uid)
    }
}

/// One Event's change in a Journal Transaction, as the Change Journal holds it.
pub(super) struct Modification {
    pub event_id: EventId,
    /// The Event's UID, which its hashes cover.
    pub uid: String,
    /// Its Event Version before the change; none when the change created it.
    pub previous: Option<StoredEvent>,
    /// The hash of its Event Version after the change; none when the change
    /// deleted it.
    pub hash: Option<Vec<u8>>,
    pub undone: bool,
}

/// The version of [`hash`]'s format, its first byte.
const HASH_FORMAT: u8 = 2;

/// The SHA-256 hash of an Event Version: its Calendar, UID and fields, as
/// a list of items each prefixed with its length, after [`HASH_FORMAT`].
/// FreeCal's fields only: which Server Object holds the Event isn't part of
/// it (ADR 0011). Format 1 covered the Server Object too.
///
/// A field added to Events joins the list. Before the first release it goes
/// into the baseline schema; after it, it bumps [`HASH_FORMAT`], and its
/// migration adds it to the Change Journal's Modifications too, whose hashes
/// then have to be made again in code.
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
    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventFields, StoredWhen};
    use crate::store::sql::UnsentChange;

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
            },
        }
    }

    /// The Server Object holding [`dentist`], as a server gives it.
    fn dentist_object() -> ServerObject {
        ServerObject {
            href: "/user/work/dentist.ics".to_owned(),
            etag: Some("\"1\"".to_owned()),
            data: "BEGIN:VCALENDAR\r\nX-UNKNOWN:kept\r\nEND:VCALENDAR\r\n".to_owned(),
        }
    }

    /// The Combined Event Keys of [`dentist_object`].
    const DENTIST_COMBINED_EVENT_KEYS: &[(&str, Option<&str>)] = &[("dentist", None)];

    const EXCEPTION: &str = "2026-10-19T09:00:00";

    /// `event` with another title.
    fn renamed(event: &StoredEvent) -> StoredEvent {
        let mut renamed = event.clone();
        renamed.fields.title = "Dentist, renamed".to_owned();
        renamed
    }

    /// `event` as the Exception with the recurrence ID `rid`.
    fn exception(event: &StoredEvent, rid: &str) -> StoredEvent {
        let mut exception = event.clone();
        exception.fields.rule = None;
        exception.fields.recurrence_id = Some(rid.to_owned());
        exception
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
            "f7e0c2e491d467b8c3f69ffe92aa06fc17219b6f1613034bf2b49ddb1628dc37"
        );
    }

    /// A first sync writes, into a Read-only Calendar too, and records
    /// nothing.
    #[test]
    fn a_first_sync_writes_without_recording() {
        let directory = tempfile::TempDir::new().unwrap();
        let store = Store::open(&directory.path().join("freecal.sqlite3")).unwrap();
        let mut event = dentist();
        event.calendar_id = store.list_calendars().unwrap()[0].id;
        let conn = store.connection();
        conn.execute(
            "UPDATE calendar SET read_only = 1 WHERE id = ?1",
            [event.calendar_id.0],
        )
        .unwrap();

        let recorded = record(
            &store,
            TransactionOrigin::FirstSync,
            0,
            |journal_transaction| journal_transaction.create("dentist", &event),
        )
        .unwrap();

        assert_eq!(recorded.journal_transaction, None);
        assert_eq!(recorded.calendars, [event.calendar_id]);
        assert_eq!(
            store.find_event(recorded.value).unwrap().fields.title,
            "Dentist"
        );
        let count = |sql: &str| -> i64 { conn.query_row(sql, [], |row| row.get(0)).unwrap() };
        assert_eq!(count("SELECT COUNT(*) FROM journal_transaction"), 0);
        assert_eq!(count("SELECT COUNT(*) FROM modification"), 0);
    }

    /// A Local Store with [`dentist`] pulled by a sync into its first
    /// Calendar, in its Server Object.
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
            let (object, id, event) =
                record(&store, TransactionOrigin::Sync, 0, |journal_transaction| {
                    let object = journal_transaction.mirror(
                        calendar_id,
                        &dentist_object(),
                        None,
                        DENTIST_COMBINED_EVENT_KEYS,
                    )?;
                    let mut event = dentist();
                    event.calendar_id = calendar_id;
                    Ok((
                        object,
                        journal_transaction.create("dentist", &event)?,
                        event,
                    ))
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

        /// Mirrors another Server Object holding the UID of [`dentist`] with
        /// `combined_event_keys`, at `href`.
        fn mirror(&self, href: &str, combined_event_keys: &[(&str, Option<&str>)]) -> ObjectId {
            let object = ServerObject {
                href: href.to_owned(),
                ..dentist_object()
            };
            record(
                &self.store,
                TransactionOrigin::Sync,
                0,
                |journal_transaction| {
                    journal_transaction.mirror(
                        self.event.calendar_id,
                        &object,
                        None,
                        combined_event_keys,
                    )
                },
            )
            .unwrap()
            .value
        }

        fn corresponding_server_object(&self) -> Option<ObjectId> {
            self.store
                .corresponding_server_object(self.event.calendar_id, "dentist")
                .unwrap()
        }
    }

    #[test]
    fn a_sync_mirrors_the_server_object_with_its_combined_event_keys_and_records_neither() {
        let pulled = Pulled::new();
        let again = ServerObject {
            etag: Some("\"2\"".to_owned()),
            ..dentist_object()
        };
        let combined_event_keys = [("dentist", None), ("dentist", Some(EXCEPTION))];

        let mirrored = record(
            &pulled.store,
            TransactionOrigin::Sync,
            0,
            |journal_transaction| {
                journal_transaction.mirror(
                    pulled.event.calendar_id,
                    &again,
                    Some(pulled.object),
                    &combined_event_keys,
                )
            },
        )
        .unwrap();

        assert_eq!(mirrored.value, pulled.object);
        assert_eq!(
            mirrored.journal_transaction, None,
            "the Journal doesn't record Server Objects"
        );
        assert_eq!(
            pulled.store.server_object(pulled.object).unwrap(),
            Some(again)
        );
        assert_eq!(
            pulled.store.combined_event_keys(pulled.object).unwrap(),
            combined_event_keys.map(|(uid, rid)| (uid.to_owned(), rid.map(str::to_owned)))
        );
    }

    /// ADR 0011: an Event Row isn't linked to its Server Object, so no
    /// Transaction Origin's change moves it to another, and none is in the
    /// way of an undo.
    #[test]
    fn event_rows_find_their_server_object_through_the_combined_event_keys_whatever_changed_them() {
        for origin in [
            TransactionOrigin::UserAction,
            TransactionOrigin::Import,
            TransactionOrigin::Sync,
        ] {
            let pulled = Pulled::new();
            let renamed = renamed(&pulled.event);
            let journal_transaction = record(&pulled.store, origin, 0, |journal_transaction| {
                journal_transaction.replace(pulled.id, &renamed)
            })
            .unwrap()
            .journal_transaction
            .unwrap();
            undo(&pulled.store, journal_transaction, 0).unwrap();

            assert_eq!(
                pulled.store.object_of_event(pulled.id).unwrap(),
                Some(pulled.object),
                "{origin:?}"
            );
        }
    }

    #[test]
    fn an_event_row_moved_to_another_calendar_has_no_server_object_there() {
        let pulled = Pulled::new();
        let mut moved = pulled.event.clone();
        moved.calendar_id = pulled.store.list_calendars().unwrap()[1].id;

        record(
            &pulled.store,
            TransactionOrigin::UserAction,
            0,
            |journal_transaction| journal_transaction.replace(pulled.id, &moved),
        )
        .unwrap();

        assert_eq!(pulled.store.object_of_event(pulled.id).unwrap(), None);
    }

    /// As a CalDAV Server Object holds a Recurring Event with its Exceptions.
    #[test]
    fn a_new_event_row_belongs_to_the_server_object_holding_its_uid() {
        let pulled = Pulled::new();
        let exception = exception(&pulled.event, EXCEPTION);

        let (joined, other) = record(
            &pulled.store,
            TransactionOrigin::UserAction,
            0,
            |journal_transaction| {
                Ok((
                    journal_transaction.create("dentist", &exception)?,
                    journal_transaction.create("other", &exception)?,
                ))
            },
        )
        .unwrap()
        .value;

        let object = |id| pulled.store.object_of_event(id).unwrap();
        assert_eq!(object(joined), Some(pulled.object));
        assert_eq!(object(other), None);
    }

    #[test]
    fn of_two_server_objects_holding_a_uid_the_one_holding_the_recurring_event_corresponds() {
        let pulled = Pulled::new();
        let later = pulled.mirror("/user/work/copy.ics", &[("dentist", Some(EXCEPTION))]);

        assert_eq!(pulled.corresponding_server_object(), Some(pulled.object));
        assert_eq!(
            pulled.store.corresponding_uids(later).unwrap(),
            Vec::<String>::new()
        );
        record(
            &pulled.store,
            TransactionOrigin::Sync,
            0,
            |journal_transaction| {
                journal_transaction.mirror(
                    pulled.event.calendar_id,
                    &dentist_object(),
                    Some(pulled.object),
                    &[("dentist", Some(EXCEPTION))],
                )?;
                journal_transaction.mirror(
                    pulled.event.calendar_id,
                    &dentist_object(),
                    Some(later),
                    DENTIST_COMBINED_EVENT_KEYS,
                )
            },
        )
        .unwrap();
        assert_eq!(
            pulled.corresponding_server_object(),
            Some(later),
            "whichever came first"
        );
    }

    /// A Local Store with a writable server Calendar holding [`dentist`],
    /// pulled by a sync into its Server Object, and a Local Calendar.
    struct OnServer {
        _directory: tempfile::TempDir,
        store: Store,
        server: CalendarId,
        local: CalendarId,
        id: EventId,
        event: StoredEvent,
    }

    impl OnServer {
        fn new() -> Self {
            use crate::{AvailableCalendar, Provider, ServerLogin};
            let directory = tempfile::TempDir::new().unwrap();
            let store = Store::open(&directory.path().join("freecal.sqlite3")).unwrap();
            let local = store.list_calendars().unwrap()[0].id;
            let login = ServerLogin {
                url: "https://dav.example/".to_owned(),
                username: "anna".to_owned(),
            };
            let account = store
                .insert_server_account(Provider::CalDav, &login, "key")
                .unwrap();
            let available = AvailableCalendar {
                url: "/user/work/".to_owned(),
                name: "Work".to_owned(),
                colour: None,
                read_only: false,
            };
            let server = store
                .insert_server_calendar(account.id, &available)
                .unwrap()
                .id;
            // A new server Calendar starts marked read-only.
            store
                .connection()
                .execute(
                    "UPDATE calendar SET read_only = 0 WHERE id = ?1",
                    [server.0],
                )
                .unwrap();
            let mut event = dentist();
            event.calendar_id = server;
            let id = record(&store, TransactionOrigin::Sync, 0, |journal_transaction| {
                journal_transaction.mirror(
                    server,
                    &dentist_object(),
                    None,
                    DENTIST_COMBINED_EVENT_KEYS,
                )?;
                journal_transaction.create("dentist", &event)
            })
            .unwrap()
            .value;
            Self {
                _directory: directory,
                store,
                server,
                local,
                id,
                event,
            }
        }

        /// Records `change` as a user's change made at `made_at`.
        fn change<T>(
            &self,
            made_at: i64,
            change: impl FnOnce(&mut JournalTransaction) -> Result<T>,
        ) -> T {
            record(&self.store, TransactionOrigin::UserAction, made_at, change)
                .unwrap()
                .value
        }

        fn unsent(&self) -> Vec<UnsentChange> {
            self.store.unsent_changes().unwrap()
        }

        /// An Unsent Change in the server Calendar.
        fn unsent_change(
            &self,
            uid: &str,
            recurrence_id: Option<&str>,
            since: i64,
        ) -> UnsentChange {
            UnsentChange {
                calendar_id: self.server,
                uid: uid.to_owned(),
                recurrence_id: recurrence_id.map(str::to_owned),
                since,
            }
        }
    }

    #[test]
    fn a_sync_leaves_nothing_unsent() {
        let on_server = OnServer::new();

        assert_eq!(on_server.unsent(), []);
    }

    #[test]
    fn a_create_leaves_its_combined_event_key_unsent() {
        let on_server = OnServer::new();
        let exception = exception(&on_server.event, EXCEPTION);

        on_server.change(7, |journal_transaction| {
            journal_transaction.create("dentist", &exception)
        });

        assert_eq!(
            on_server.unsent(),
            [on_server.unsent_change("dentist", Some(EXCEPTION), 7)]
        );
    }

    /// An existing Unsent Change keeps its `since`.
    #[test]
    fn an_edit_leaves_its_previous_and_new_combined_event_key_unsent_once() {
        let on_server = OnServer::new();
        let renamed = renamed(&on_server.event);
        on_server.change(7, |journal_transaction| {
            journal_transaction.replace(on_server.id, &renamed)
        });
        let exception = exception(&renamed, EXCEPTION);

        on_server.change(9, |journal_transaction| {
            journal_transaction.replace(on_server.id, &exception)
        });

        assert_eq!(
            on_server.unsent(),
            [
                on_server.unsent_change("dentist", None, 7),
                on_server.unsent_change("dentist", Some(EXCEPTION), 9)
            ]
        );
    }

    #[test]
    fn a_move_to_a_local_calendar_leaves_only_the_previous_combined_event_key_unsent() {
        let on_server = OnServer::new();
        let mut moved = on_server.event.clone();
        moved.calendar_id = on_server.local;

        on_server.change(7, |journal_transaction| {
            journal_transaction.replace(on_server.id, &moved)
        });

        assert_eq!(
            on_server.unsent(),
            [on_server.unsent_change("dentist", None, 7)]
        );
    }

    #[test]
    fn a_delete_and_its_undo_leave_the_combined_event_key_unsent() {
        let on_server = OnServer::new();

        let journal_transaction = record(
            &on_server.store,
            TransactionOrigin::UserAction,
            7,
            |journal_transaction| journal_transaction.delete(on_server.id),
        )
        .unwrap()
        .journal_transaction
        .unwrap();
        assert_eq!(
            on_server.unsent(),
            [on_server.unsent_change("dentist", None, 7)]
        );
        undo(&on_server.store, journal_transaction, 9).unwrap();

        assert_eq!(
            on_server.unsent(),
            [on_server.unsent_change("dentist", None, 7)]
        );
    }

    #[test]
    fn a_change_in_a_local_calendar_leaves_nothing_unsent() {
        let on_server = OnServer::new();
        let mut local = on_server.event.clone();
        local.calendar_id = on_server.local;

        on_server.change(7, |journal_transaction| {
            let id = journal_transaction.create("local", &local)?;
            local.fields.title = "Renamed".to_owned();
            journal_transaction.replace(id, &local)?;
            journal_transaction.delete(id)
        });

        assert_eq!(on_server.unsent(), []);
    }

    /// The Server Object holding the Event Row changes; the Event doesn't,
    /// and the server already holds it.
    #[test]
    fn a_sync_that_moves_an_event_row_to_another_server_object_leaves_nothing_unsent() {
        let on_server = OnServer::new();
        let renamed = renamed(&on_server.event);

        record(
            &on_server.store,
            TransactionOrigin::Sync,
            7,
            |journal_transaction| {
                let old = journal_transaction
                    .corresponding_server_object(on_server.server, "dentist")?
                    .unwrap();
                journal_transaction.remove_object(old)?;
                let moved = ServerObject {
                    href: "/user/work/moved.ics".to_owned(),
                    ..dentist_object()
                };
                journal_transaction.mirror(
                    on_server.server,
                    &moved,
                    None,
                    DENTIST_COMBINED_EVENT_KEYS,
                )?;
                journal_transaction.replace(on_server.id, &renamed)
            },
        )
        .unwrap();

        assert_eq!(on_server.unsent(), []);
    }

    /// As [`Loaded::join`] renormalises an Exception's recurrence ID when
    /// its Recurring Event arrives: the Unsent Change goes with the Event
    /// Row, and the earliest `since` stays.
    ///
    /// [`Loaded::join`]: crate::event::Loaded::join
    #[test]
    fn rejoining_combined_event_keys_renormalises_the_unsent_changes() {
        let on_server = OnServer::new();
        let zoned = "2026-10-19T09:00:00 Europe/Berlin";
        let mut exception = exception(&on_server.event, zoned);
        on_server.change(7, |journal_transaction| {
            journal_transaction.create("dentist", &exception)
        });
        exception.fields.recurrence_id = Some(EXCEPTION.to_owned());
        on_server.change(9, |journal_transaction| {
            journal_transaction.create("dentist", &exception)
        });
        exception.fields.recurrence_id = Some("other".to_owned());
        on_server.change(11, |journal_transaction| {
            journal_transaction.create("dentist", &exception)
        });

        record(
            &on_server.store,
            TransactionOrigin::Sync,
            13,
            |journal_transaction| {
                journal_transaction.rejoin_combined_event_keys(
                    on_server.server,
                    "dentist",
                    |text| (text == zoned).then(|| EXCEPTION.to_owned()),
                )
            },
        )
        .unwrap();

        assert_eq!(
            on_server.unsent(),
            [
                on_server.unsent_change("dentist", Some(EXCEPTION), 7),
                on_server.unsent_change("dentist", Some("other"), 11)
            ]
        );
    }

    #[test]
    fn of_two_server_objects_holding_a_uid_alike_the_first_mirrored_corresponds_until_it_goes() {
        let pulled = Pulled::new();
        let later = pulled.mirror("/user/work/copy.ics", DENTIST_COMBINED_EVENT_KEYS);
        assert_eq!(pulled.corresponding_server_object(), Some(pulled.object));
        assert_eq!(
            pulled.store.corresponding_uids(later).unwrap(),
            Vec::<String>::new()
        );

        record(
            &pulled.store,
            TransactionOrigin::Sync,
            0,
            |journal_transaction| journal_transaction.remove_object(pulled.object),
        )
        .unwrap();

        assert_eq!(pulled.corresponding_server_object(), Some(later));
        assert_eq!(pulled.store.combined_event_keys(pulled.object).unwrap(), []);
        assert_eq!(pulled.store.corresponding_uids(later).unwrap(), ["dentist"]);
        assert_eq!(
            pulled.store.object_of_event(pulled.id).unwrap(),
            Some(later)
        );
    }
}
