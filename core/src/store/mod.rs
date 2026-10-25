//! The Local Store: one SQLite database (ADR 0004).
//!
//! Journaling is one of its invariants (ADR 0011): code outside this module
//! changes an Event only through a [`Unit`] of the Change Journal. Its SQL
//! lives in two places: [`schema`] holds the schema and its migrations,
//! [`sql`] every query and write. [`journal`] holds the Change Journal and no
//! SQL; it makes the [`Journaled`](journal::Journaled) that writing the
//! tables it guards takes, so that nothing else can.

mod journal;
mod schema;
mod sql;

pub use journal::{Changed, UnitId};
pub(crate) use journal::{Origin, Recorded, Unit, delete_account, delete_calendar, record, undo};

use std::path::Path;

use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::event::EventFields;
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
        let tx = Transaction::new_unchecked(&store.conn, TransactionBehavior::Immediate)?;
        schema::migrate(&tx)?;
        journal::recompute_instants(&store)?;
        tx.commit()?;
        Ok(store)
    }

    /// The connection, for tests that set rows up or look at them directly.
    #[cfg(test)]
    fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Runs `change` in one transaction: if it fails, nothing it did is kept.
    pub fn transaction<T>(&self, change: impl FnOnce() -> Result<T>) -> Result<T> {
        let tx = self.conn.unchecked_transaction()?;
        let done = change()?;
        tx.commit()?;
        Ok(done)
    }
}
