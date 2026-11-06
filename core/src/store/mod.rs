//! The Local Store: one SQLite database (ADR 0004).
//!
//! Journaling is one of its invariants (ADR 0015): code changes an Event only
//! through a [`JournalTransaction`](crate::journal::JournalTransaction). All
//! of the Local Store's SQL lives in this module: [`schema`] holds the schema
//! and its migrations, [`sql`] every query and write. Every write is a method
//! of a [`DatabaseTransaction`], so none runs outside one. The Guards that
//! alone write a set of tables, the Change Journal ([`crate::journal`]) and
//! the Sync Engine ([`crate::sync`]), hold no SQL; their writes take their
//! Write Permission too.

mod schema;
mod sql;

#[cfg(test)]
pub use sql::UnsentChange;

use std::ops::Deref;
use std::path::Path;

use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::event::{EventFields, RecurrenceId};
use crate::{CalendarId, EventId, Result};

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

/// A Combined Event Key: a Calendar, a UID and a recurrence ID as the Event
/// Row holds it (ADR 0015), what an Unsent Change is about.
#[derive(Debug, Clone)]
pub struct CombinedEventKey {
    pub calendar_id: CalendarId,
    pub uid: String,
    /// As obtained (ADR 0012): compared by Occurrence, never as text.
    pub recurrence_id: Option<RecurrenceId>,
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
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "foreign_keys", true)?;
        let store = Self { conn };
        // IMMEDIATE takes the write lock before `user_version` is read, so two
        // FreeCal processes opening a fresh Local Store at once can't both see
        // version 0 and both try to create the schema.
        let database_transaction = store.begin_with(TransactionBehavior::Immediate)?;
        schema::migrate(&database_transaction.tx)?;
        crate::journal::recompute_instants(&database_transaction)?;
        database_transaction.commit()?;
        Ok(store)
    }

    /// The connection, for tests that set rows up or look at them directly.
    #[cfg(test)]
    pub(crate) fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Adds a CalDAV Account with one server Calendar that isn't Read-only,
    /// and returns the Calendar.
    #[cfg(test)]
    pub(crate) fn writable_server_calendar(&self) -> CalendarId {
        use crate::{AvailableCalendar, Provider, ServerLogin};
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
        let calendar_id = self
            .write(|database_transaction| {
                let account =
                    database_transaction.insert_server_account(Provider::CalDav, &login, "key")?;
                database_transaction.insert_server_calendar(account.id, &available)
            })
            .unwrap()
            .id;
        // A new server Calendar starts marked read-only.
        self.conn
            .execute(
                "UPDATE calendar SET read_only = 0 WHERE id = ?1",
                [calendar_id.0],
            )
            .unwrap();
        calendar_id
    }

    /// Runs `write` in a Database Transaction of its own, and commits it
    /// once `write` succeeds: if it fails, nothing it wrote is kept. How
    /// code writes, but where something must hold the Database Transaction,
    /// as a Journal Transaction holds its own ([`Store::begin`]).
    pub fn write<T>(&self, write: impl FnOnce(&DatabaseTransaction) -> Result<T>) -> Result<T> {
        let database_transaction = self.begin()?;
        let written = write(&database_transaction)?;
        database_transaction.commit()?;
        Ok(written)
    }

    /// Begins a Database Transaction, which writes nothing unless it is
    /// committed, for what must hold it; everything else writes through
    /// [`Store::write`].
    pub fn begin(&self) -> Result<DatabaseTransaction<'_>> {
        self.begin_with(TransactionBehavior::Deferred)
    }

    fn begin_with(&self, behavior: TransactionBehavior) -> Result<DatabaseTransaction<'_>> {
        Ok(DatabaseTransaction {
            store: self,
            tx: Transaction::new_unchecked(&self.conn, behavior)?,
        })
    }
}

/// A Database Transaction (ADR 0015): every write to the Local Store is one
/// of its methods, in [`sql`], so a write outside one doesn't compile. It
/// writes nothing until [`DatabaseTransaction::commit`], and dropped without
/// it, nothing it wrote is kept. It reads as the Local Store it is on.
#[must_use = "a Database Transaction dropped without `commit` keeps nothing it wrote"]
pub struct DatabaseTransaction<'a> {
    store: &'a Store,
    tx: Transaction<'a>,
}

impl DatabaseTransaction<'_> {
    /// Keeps everything written in it, all or nothing.
    pub fn commit(self) -> Result<()> {
        self.tx.commit()?;
        Ok(())
    }
}

impl Deref for DatabaseTransaction<'_> {
    type Target = Store;

    fn deref(&self) -> &Store {
        self.store
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    /// Every Rust file under `directory`.
    fn rust_files(directory: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                files.extend(rust_files(&path));
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
        files
    }

    /// What a compile-fail doctest can't show here, as the crate's modules
    /// are private to it: a Write Permission's field stays private, so the
    /// compiler lets only its Guard's module make one (ADR 0015). The Guard
    /// mustn't hand one out either: no derive, and its one value, the
    /// `WRITE_PERMISSION` constant, stays private. This reads the source, so
    /// it catches the usual ways, not every way, such as a function the
    /// Guard adds that returns one.
    #[test]
    fn only_its_guard_makes_a_write_permission() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        for (permission, guard) in [
            ("JournalWritePermission", "journal"),
            ("SyncEngineWritePermission", "sync"),
        ] {
            let made = [format!("{permission}(())"), format!("{permission} {{")];
            let makers: Vec<PathBuf> = rust_files(&source)
                .into_iter()
                .filter(|file| {
                    let text = fs::read_to_string(file).unwrap();
                    made.iter().any(|made| text.contains(made.as_str()))
                })
                .collect();
            assert_eq!(makers, [source.join(guard).join("mod.rs")], "{permission}");
            let guard_source = fs::read_to_string(&makers[0]).unwrap();
            let definition = format!("\npub(crate) struct {permission}(());");
            let at = guard_source
                .find(&definition)
                .unwrap_or_else(|| panic!("{permission} keeps its field private"));
            let before = guard_source[..at].lines().next_back().unwrap_or_default();
            assert!(!before.starts_with("#["), "{permission} derives nothing");
            assert!(
                guard_source.contains(&format!(
                    "\nconst WRITE_PERMISSION: &{permission} = &{permission}(());"
                )),
                "{permission}'s constant stays private"
            );
        }
    }
}
