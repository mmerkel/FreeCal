//! The Sync Engine's writes for CalDAV (ADR 0011, ADR 0013): a Server
//! Object's data in the Mirror (`server_object`) and its Combined Event Keys
//! (`combined_event_key`), which hold its recurrence IDs as obtained (ADR
//! 0012). Each takes a [`SyncEngineWritePermission`], which only
//! [`crate::sync`] can make, and is a method of a [`DatabaseTransaction`].

use rusqlite::params;

use crate::event::{ObjectId, RecurrenceId, ServerObject};
use crate::store::{DatabaseTransaction, EventKeyInCalendar};
use crate::sync::SyncEngineWritePermission;
use crate::{CalendarId, Result};

/// How a Server Object's data is written: iCalendar, as CalDAV holds it.
/// Google's will be its JSON as received (ADR 0010).
const ICALENDAR: &str = "icalendar";

impl DatabaseTransaction<'_> {
    /// Writes `object` over the Server Object `over`, address included, or
    /// as a new one of a Calendar, with `event_keys` as its Combined Event
    /// Keys, and returns its ID.
    pub(crate) fn write_object(
        &self,
        permission: &SyncEngineWritePermission,
        calendar_id: CalendarId,
        object: &ServerObject,
        over: Option<ObjectId>,
        event_keys: &[EventKeyInCalendar],
    ) -> Result<ObjectId> {
        let id = self.write_object_row(permission, calendar_id, object, over)?;
        self.tx.execute(
            "DELETE FROM combined_event_key WHERE object_id = ?1",
            [id.0],
        )?;
        self.insert_combined_event_keys(permission, id, event_keys)?;
        Ok(id)
    }

    /// Adds `event_keys` to the Server Object `id`'s Combined Event Keys.
    fn insert_combined_event_keys(
        &self,
        _: &SyncEngineWritePermission,
        id: ObjectId,
        event_keys: &[EventKeyInCalendar],
    ) -> Result<()> {
        let mut stmt = self.tx.prepare(
            "INSERT INTO combined_event_key (object_id, uid, recurrence_id) VALUES (?1, ?2, ?3)",
        )?;
        for event_key in event_keys {
            stmt.execute(params![
                id.0,
                event_key.uid,
                event_key
                    .recurrence_id
                    .as_ref()
                    .map(RecurrenceId::as_obtained)
            ])?;
        }
        Ok(())
    }

    /// [`DatabaseTransaction::write_object`]'s write of the object itself.
    fn write_object_row(
        &self,
        _: &SyncEngineWritePermission,
        calendar_id: CalendarId,
        object: &ServerObject,
        over: Option<ObjectId>,
    ) -> Result<ObjectId> {
        match over {
            Some(id) => {
                self.tx.execute(
                    "UPDATE server_object SET href = ?2, etag = ?3, data = ?4, format = ?5, \
                     unreadable = 0 WHERE id = ?1",
                    params![id.0, object.href, object.etag, object.data, ICALENDAR],
                )?;
                Ok(id)
            }
            None => {
                self.tx.execute(
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
                Ok(ObjectId(self.tx.last_insert_rowid()))
            }
        }
    }

    /// Marks the Server Objects at `href` in a Calendar unreadable.
    pub(crate) fn mark_unreadable(
        &self,
        _: &SyncEngineWritePermission,
        calendar_id: CalendarId,
        href: &str,
    ) -> Result<()> {
        self.tx.execute(
            "UPDATE server_object SET unreadable = 1 WHERE calendar_id = ?1 AND href = ?2",
            params![calendar_id.0, href],
        )?;
        Ok(())
    }

    /// Deletes the Server Object `id` with its Combined Event Keys, by the
    /// schema's cascade; Event Rows aren't written.
    pub(crate) fn delete_object(&self, _: &SyncEngineWritePermission, id: ObjectId) -> Result<()> {
        self.tx
            .execute("DELETE FROM server_object WHERE id = ?1", [id.0])?;
        Ok(())
    }
}
