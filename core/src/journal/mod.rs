//! The Change Journal (ADR 0009) and the only writer of Events: every change
//! to an Event goes through a [`JournalTransaction`] in [`record`], which
//! keeps the Event's previous Event Version and a hash of its new one, so
//! that no change can skip the Journal. Changes made together, such as an
//! import, are one Journal Transaction, which runs inside one Database
//! Transaction. The Journal doesn't record Server Objects (ADR 0010, 0011):
//! an Event Version holds FreeCal's fields only, and Event Rows find their
//! Server Object through the Combined Event Keys. An Incoming Sync writes the
//! Mirror in the Journal Transaction's Database Transaction, outside the
//! Journal Transaction. Every Journal Transaction but an Incoming Sync's adds
//! the Combined Event Keys of the Events it changes to the Unsent Changes,
//! what FreeCal has to send.
//!
//! This module is a Guard and holds no SQL: it reads and writes through the
//! Local Store's SQL, whose writes to the tables it guards take a
//! [`JournalWritePermission`], which only this module can make.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::event::{ObjectId, ServerObject, Source, rule};
use crate::store::{
    CombinedEventKey, DatabaseTransaction, EventKeyInCalendar, Store, StoredEvent, Uid, UidEvent,
};
use crate::{AccountId, CalendarId, CoreError, EventId, Result};

/// The Change Journal's Write Permission (ADR 0011), which the writes of
/// the tables it guards take. Its field is private to this module, so that
/// nothing else can make one.
pub(crate) struct JournalWritePermission(());

/// The [`JournalWritePermission`] this module's writes pass.
const WRITE_PERMISSION: &JournalWritePermission = &JournalWritePermission(());

/// The time zone rules FreeCal was built with, which the derived UTC columns
/// are computed with.
const TIME_ZONE_RULES: &str = chrono_tz::IANA_TZDB_VERSION;

/// Recomputes every Event's derived UTC instants when the Local Store's were
/// computed with other time zone rules than this build's, while it opens, in
/// its Database Transaction `database_transaction`. The Journal doesn't
/// record them, so this writes outside any Journal Transaction. An Event Row
/// whose time can't be read keeps what it had.
pub(crate) fn recompute_instants(database_transaction: &DatabaseTransaction) -> Result<()> {
    if database_transaction.time_zone_rules()? == TIME_ZONE_RULES {
        return Ok(());
    }
    for (id, fields) in database_transaction.all_event_fields()? {
        if let Ok(instants) = rule::span(&fields) {
            database_transaction.write_instants(WRITE_PERMISSION, id, instants)?;
        }
    }
    database_transaction.set_time_zone_rules(TIME_ZONE_RULES)
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
    /// An Incoming Sync of one Calendar after its First Sync, what its
    /// server changed.
    IncomingSync,
}

impl TransactionOrigin {
    /// How a stored Journal Transaction records it; none for a Transaction
    /// Origin that isn't recorded.
    fn recorded_as(self) -> Option<&'static str> {
        match self {
            TransactionOrigin::UserAction => Some("user"),
            TransactionOrigin::Import => Some("import"),
            TransactionOrigin::Undo => Some("undo"),
            TransactionOrigin::IncomingSync => Some("incoming_sync"),
            TransactionOrigin::FirstSync => None,
        }
    }

    /// Whether it is an Incoming Sync, whose changes come from the server
    /// and so aren't Unsent Changes.
    fn is_incoming_sync(self) -> bool {
        matches!(
            self,
            TransactionOrigin::FirstSync | TransactionOrigin::IncomingSync
        )
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

/// Runs `change` in one Database Transaction as one Journal Transaction of
/// `origin`, made at `made_at` (Unix seconds). If it fails, nothing it did is
/// kept. A Journal Transaction that changed nothing isn't stored.
pub(crate) fn record<T>(
    store: &Store,
    origin: TransactionOrigin,
    made_at: i64,
    change: impl FnOnce(&mut JournalTransaction) -> Result<T>,
) -> Result<Recorded<T>> {
    let mut journal_transaction = JournalTransaction {
        database_transaction: store.begin()?,
        origin,
        made_at,
        id: None,
        calendars: Vec::new(),
    };
    let value = change(&mut journal_transaction)?;
    let JournalTransaction {
        database_transaction,
        id,
        calendars,
        ..
    } = journal_transaction;
    database_transaction.commit()?;
    Ok(Recorded {
        journal_transaction: id,
        calendars,
        value,
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
            let modifications = store.journal_event_modifications(id)?;
            if modifications.iter().any(|modification| modification.undone) {
                return Err(CoreError::AlreadyUndone(id));
            }
            // Latest first, so that each Journal Event Modification finds the
            // Event Version it made.
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
            journal_transaction
                .database_transaction
                .mark_undone(WRITE_PERMISSION, id)
        },
    )
}

/// Deletes a Calendar with its Events. This isn't recorded (ADR 0009): the
/// Journal Event Modifications of its Events go too, and Journal Transactions
/// left without Journal Event Modifications. So do the Journal Event
/// Modifications that would put an Event back into it, such as one moved out or
/// deleted, with that Event's earlier Journal Event Modifications, which are
/// undone only through them; its later Journal Event Modifications stay.
pub(crate) fn delete_calendar(store: &Store, id: CalendarId) -> Result<()> {
    store.write(|database_transaction| delete_calendar_within(database_transaction, id))
}

/// Deletes a server Account with its Calendars, each as
/// [`delete_calendar`] does, all or none.
pub(crate) fn delete_account(store: &Store, id: AccountId) -> Result<()> {
    store.write(|database_transaction| {
        for calendar in database_transaction.account_calendars(id)? {
            delete_calendar_within(database_transaction, calendar)?;
        }
        database_transaction.delete_account_row(WRITE_PERMISSION, id)
    })
}

/// [`delete_calendar`] in the Database Transaction `database_transaction`.
fn delete_calendar_within(
    database_transaction: &DatabaseTransaction,
    id: CalendarId,
) -> Result<()> {
    database_transaction.delete_calendar_events(WRITE_PERMISSION, id)?;
    database_transaction.delete_calendar_row(WRITE_PERMISSION, id)
}

/// One Journal Transaction being recorded, in its Database Transaction,
/// which [`record`] commits. Each write reads the Event's previous Event
/// Version itself.
pub(crate) struct JournalTransaction<'a> {
    database_transaction: DatabaseTransaction<'a>,
    origin: TransactionOrigin,
    made_at: i64,
    id: Option<JournalTransactionId>,
    calendars: Vec<CalendarId>,
}

impl JournalTransaction<'_> {
    pub fn find_event(&self, id: EventId) -> Result<StoredEvent> {
        self.database_transaction.find_event(id)
    }

    /// The Server Objects of a Calendar at `href`, in the order they were
    /// added.
    pub fn objects_at(&self, calendar_id: CalendarId, href: &str) -> Result<Vec<ObjectId>> {
        self.database_transaction.objects_at(calendar_id, href)
    }

    /// The Corresponding Server Object of the UID `uid` in a Calendar
    /// ([`Store::corresponding_server_object`]).
    pub fn corresponding_server_object(
        &self,
        calendar_id: CalendarId,
        uid: &str,
    ) -> Result<Option<ObjectId>> {
        self.database_transaction
            .corresponding_server_object(calendar_id, uid)
    }

    /// The UIDs whose Corresponding Server Object is `id`.
    pub fn corresponding_uids(&self, id: ObjectId) -> Result<Vec<String>> {
        self.database_transaction.corresponding_uids(id)
    }

    /// The UIDs the Server Object `id` holds, whether it is their
    /// Corresponding Server Object or not.
    pub fn held_uids(&self, id: ObjectId) -> Result<Vec<String>> {
        self.database_transaction.held_uids(id)
    }

    /// The Server Object `id` as the Mirror holds it, none if it is gone.
    pub fn server_object(&self, id: ObjectId) -> Result<Option<ServerObject>> {
        self.database_transaction.server_object(id)
    }

    /// The Database Transaction it runs in, for writes outside the Journal
    /// Transaction that commit with it, such as an Incoming Sync's to the
    /// Mirror.
    pub fn database_transaction(&self) -> &DatabaseTransaction<'_> {
        &self.database_transaction
    }

    /// Records, with the Journal Transaction's changes, that an Incoming Sync
    /// of a Calendar finished with the sync token `token`.
    pub fn synced(&self, calendar_id: CalendarId, token: Option<&str>) -> Result<()> {
        self.database_transaction.set_synced(calendar_id, token)
    }

    /// Adds an Event with the UID `uid`.
    pub fn create(&mut self, uid: &str, event: &StoredEvent) -> Result<EventId> {
        self.writable(event.calendar_id)?;
        let id = self.database_transaction.write_event(
            WRITE_PERMISSION,
            None,
            uid,
            event,
            rule::span(&event.fields)?,
        )?;
        self.record_journal_event_modification(id, uid, None, Some(event))?;
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
        self.database_transaction.write_event(
            WRITE_PERMISSION,
            Some(id),
            &uid,
            event,
            rule::span(&event.fields)?,
        )?;
        self.record_journal_event_modification(id, &uid, Some(&previous), Some(event))
    }

    pub fn delete(&mut self, id: EventId) -> Result<()> {
        let (uid, previous) = self.previous(id)?;
        self.writable(previous.calendar_id)?;
        self.database_transaction
            .delete_event(WRITE_PERMISSION, id)?;
        self.record_journal_event_modification(id, &uid, Some(&previous), None)
    }

    /// Writes an earlier Event Version back under its own ID: over the Event
    /// `id` while it is the one with the UID `uid`, otherwise as a new Event.
    /// No other Event has that ID, as IDs are never given out again (ticket
    /// 85).
    fn put_back(&mut self, id: EventId, uid: &str, event: &StoredEvent) -> Result<()> {
        if self.database_transaction.current_event(id, uid)?.is_some() {
            return self.replace(id, event);
        }
        self.writable(event.calendar_id)?;
        // An Event Version FreeCal can't read the time of was stored like
        // that; it reads as unreadable again rather than stopping the undo.
        let instants = rule::span(&event.fields).unwrap_or_default();
        self.database_transaction
            .write_event(WRITE_PERMISSION, Some(id), uid, event, instants)?;
        self.record_journal_event_modification(id, uid, None, Some(event))
    }

    fn previous(&self, id: EventId) -> Result<(String, StoredEvent)> {
        self.database_transaction
            .event_and_uid(id)?
            .ok_or(CoreError::EventNotFound(id))
    }

    /// Refuses writes into a missing Calendar, and into a Read-only one for
    /// every Transaction Origin but an Incoming Sync, which loads what the
    /// server has. Every write checks this itself; a change that targets a
    /// Calendar, such as an import, checks it first, so that it is refused
    /// even when it would write nothing.
    pub fn writable(&self, id: CalendarId) -> Result<()> {
        let read_only = self.database_transaction.find_calendar(id)?.read_only;
        match self.origin {
            TransactionOrigin::UserAction | TransactionOrigin::Import | TransactionOrigin::Undo
                if read_only =>
            {
                Err(CoreError::CalendarReadOnly(id))
            }
            _ => Ok(()),
        }
    }

    /// Records that the Event `id` went from `previous` to `new`, none for an
    /// Event created or deleted, as a Journal Event Modification, starting the
    /// Journal Transaction with its first one. A Transaction Origin that isn't
    /// recorded only notes the Calendars.
    ///
    /// Every Transaction Origin but an Incoming Sync also adds both Event
    /// Versions' Combined Event Keys to the Unsent Changes, in a server
    /// Calendar (ADR 0011): a write into a Read-only one was refused already.
    /// An Incoming Sync's changes are what the server holds.
    fn record_journal_event_modification(
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
            if !self.origin.is_incoming_sync() {
                let combined_event_key = CombinedEventKey {
                    calendar_id: event.calendar_id,
                    event_key: EventKeyInCalendar::new(uid, event.fields.recurrence_id.as_ref()),
                };
                self.database_transaction.add_unsent_change(
                    WRITE_PERMISSION,
                    &combined_event_key,
                    self.made_at,
                )?;
            }
        }
        let Some(origin) = self.origin.recorded_as() else {
            return Ok(());
        };
        let journal_transaction = match self.id {
            Some(journal_transaction) => journal_transaction,
            None => *self
                .id
                .insert(self.database_transaction.insert_journal_transaction(
                    WRITE_PERMISSION,
                    origin,
                    self.made_at,
                )?),
        };
        self.database_transaction.insert_journal_event_modification(
            WRITE_PERMISSION,
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
        self.database_transaction.events_with_uid(uid)
    }
}

/// One Event's change in a Journal Transaction, as the Change Journal holds it.
pub(crate) struct JournalEventModification {
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
/// migration adds it to the Change Journal's Journal Event Modifications too,
/// whose hashes then have to be made again in code.
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
    let recurrence_id = fields.recurrence_id.as_ref();
    item(&optional(
        &recurrence_id.map(|obtained| obtained.as_obtained().to_owned()),
    ));
    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventFields, RecurrenceId, StoredWhen};
    use crate::store::UnsentChange;
    use crate::sync;

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
    fn dentist_event_keys() -> [EventKeyInCalendar; 1] {
        [EventKeyInCalendar::new("dentist", None)]
    }

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
        exception.fields.recurrence_id = Some(RecurrenceId::obtained(rid));
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

    /// A First Sync writes, into a Read-only Calendar too, and records
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
        assert_eq!(count("SELECT COUNT(*) FROM journal_event_modification"), 0);
    }

    /// A Local Store with [`dentist`] pulled by an Incoming Sync into its first
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
            let (object, id, event) = record(
                &store,
                TransactionOrigin::IncomingSync,
                0,
                |journal_transaction| {
                    let object = sync::testing::mirror(
                        journal_transaction,
                        calendar_id,
                        &dentist_object(),
                        None,
                        &dentist_event_keys(),
                    )?;
                    let mut event = dentist();
                    event.calendar_id = calendar_id;
                    Ok((
                        object,
                        journal_transaction.create("dentist", &event)?,
                        event,
                    ))
                },
            )
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
        /// the Combined Event Keys `event_keys`, at `href`.
        fn mirror(&self, href: &str, event_keys: &[EventKeyInCalendar]) -> ObjectId {
            let object = ServerObject {
                href: href.to_owned(),
                ..dentist_object()
            };
            record(
                &self.store,
                TransactionOrigin::IncomingSync,
                0,
                |journal_transaction| {
                    sync::testing::mirror(
                        journal_transaction,
                        self.event.calendar_id,
                        &object,
                        None,
                        event_keys,
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
    fn an_incoming_sync_mirrors_the_server_object_with_its_combined_event_keys_and_records_neither()
    {
        let pulled = Pulled::new();
        let again = ServerObject {
            etag: Some("\"2\"".to_owned()),
            ..dentist_object()
        };
        let event_keys = [
            EventKeyInCalendar::new("dentist", None),
            EventKeyInCalendar::new("dentist", Some(&RecurrenceId::obtained(EXCEPTION))),
        ];

        let mirrored = record(
            &pulled.store,
            TransactionOrigin::IncomingSync,
            0,
            |journal_transaction| {
                sync::testing::mirror(
                    journal_transaction,
                    pulled.event.calendar_id,
                    &again,
                    Some(pulled.object),
                    &event_keys,
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
            [
                ("dentist".to_owned(), None),
                ("dentist".to_owned(), Some(EXCEPTION.to_owned()))
            ]
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
            TransactionOrigin::IncomingSync,
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
        let later = pulled.mirror(
            "/user/work/copy.ics",
            &[EventKeyInCalendar::new(
                "dentist",
                Some(&RecurrenceId::obtained(EXCEPTION)),
            )],
        );

        assert_eq!(pulled.corresponding_server_object(), Some(pulled.object));
        assert_eq!(
            pulled.store.corresponding_uids(later).unwrap(),
            Vec::<String>::new()
        );
        record(
            &pulled.store,
            TransactionOrigin::IncomingSync,
            0,
            |journal_transaction| {
                sync::testing::mirror(
                    journal_transaction,
                    pulled.event.calendar_id,
                    &dentist_object(),
                    Some(pulled.object),
                    &[EventKeyInCalendar::new(
                        "dentist",
                        Some(&RecurrenceId::obtained(EXCEPTION)),
                    )],
                )?;
                sync::testing::mirror(
                    journal_transaction,
                    pulled.event.calendar_id,
                    &dentist_object(),
                    Some(later),
                    &dentist_event_keys(),
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
    /// pulled by an Incoming Sync into its Server Object, and a Local Calendar.
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
            let available = AvailableCalendar {
                url: "/user/work/".to_owned(),
                name: "Work".to_owned(),
                colour: None,
                read_only: false,
            };
            let server = store
                .write(|database_transaction| {
                    let account = database_transaction.insert_server_account(
                        Provider::CalDav,
                        &login,
                        "key",
                    )?;
                    database_transaction.insert_server_calendar(account.id, &available)
                })
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
            let id = record(
                &store,
                TransactionOrigin::IncomingSync,
                0,
                |journal_transaction| {
                    sync::testing::mirror(
                        journal_transaction,
                        server,
                        &dentist_object(),
                        None,
                        &dentist_event_keys(),
                    )?;
                    journal_transaction.create("dentist", &event)
                },
            )
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
    fn an_incoming_sync_leaves_nothing_unsent() {
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
    fn an_incoming_sync_that_moves_an_event_row_to_another_server_object_leaves_nothing_unsent() {
        let on_server = OnServer::new();
        let renamed = renamed(&on_server.event);

        record(
            &on_server.store,
            TransactionOrigin::IncomingSync,
            7,
            |journal_transaction| {
                let old = journal_transaction
                    .corresponding_server_object(on_server.server, "dentist")?
                    .unwrap();
                sync::testing::remove_object(journal_transaction, old)?;
                let moved = ServerObject {
                    href: "/user/work/moved.ics".to_owned(),
                    ..dentist_object()
                };
                sync::testing::mirror(
                    journal_transaction,
                    on_server.server,
                    &moved,
                    None,
                    &dentist_event_keys(),
                )?;
                journal_transaction.replace(on_server.id, &renamed)
            },
        )
        .unwrap();

        assert_eq!(on_server.unsent(), []);
    }

    #[test]
    fn of_two_server_objects_holding_a_uid_alike_the_first_mirrored_corresponds_until_it_goes() {
        let pulled = Pulled::new();
        let later = pulled.mirror("/user/work/copy.ics", &dentist_event_keys());
        assert_eq!(pulled.corresponding_server_object(), Some(pulled.object));
        assert_eq!(
            pulled.store.corresponding_uids(later).unwrap(),
            Vec::<String>::new()
        );

        record(
            &pulled.store,
            TransactionOrigin::IncomingSync,
            0,
            |journal_transaction| sync::testing::remove_object(journal_transaction, pulled.object),
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
