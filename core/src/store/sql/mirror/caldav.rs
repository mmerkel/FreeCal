//! The Sync Engine's writes of CalDAV's Mirror table, `caldav_server_object`
//! (ADR 0011, ADR 0013): a CalDAV Server Object's iCalendar text as
//! received, with the one UID it holds and whether it holds the Event of
//! that UID that isn't an Exception. Each takes a
//! [`SyncEngineWritePermission`], which only [`crate::sync`] can make, and is
//! a method of a [`DatabaseTransaction`].

use rusqlite::params;

use crate::event::{ObjectId, ServerObject};
use crate::store::DatabaseTransaction;
use crate::sync::SyncEngineWritePermission;
use crate::{CalendarId, Result};

impl DatabaseTransaction<'_> {
    /// Writes `object` over the CalDAV Server Object `over`, address
    /// included, or as a new one of a Calendar, holding the UID `uid`, and
    /// the Event of it that isn't an Exception when `holds_non_exception`,
    /// and returns its ID.
    pub(crate) fn write_object(
        &self,
        _: &SyncEngineWritePermission,
        calendar_id: CalendarId,
        object: &ServerObject,
        over: Option<ObjectId>,
        uid: &str,
        holds_non_exception: bool,
    ) -> Result<ObjectId> {
        match over {
            Some(id) => {
                self.tx.execute(
                    "UPDATE caldav_server_object SET href = ?2, etag = ?3, data = ?4, \
                     unreadable = 0, uid = ?5, holds_non_exception = ?6 WHERE id = ?1",
                    params![
                        id.0,
                        object.href,
                        object.etag,
                        object.data,
                        uid,
                        holds_non_exception
                    ],
                )?;
                Ok(id)
            }
            None => {
                self.tx.execute(
                    "INSERT INTO caldav_server_object \
                     (calendar_id, href, etag, data, uid, holds_non_exception) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        calendar_id.0,
                        object.href,
                        object.etag,
                        object.data,
                        uid,
                        holds_non_exception
                    ],
                )?;
                Ok(ObjectId(self.tx.last_insert_rowid()))
            }
        }
    }

    /// Marks the CalDAV Server Object at `href` in a Calendar unreadable.
    pub(crate) fn mark_unreadable(
        &self,
        _: &SyncEngineWritePermission,
        calendar_id: CalendarId,
        href: &str,
    ) -> Result<()> {
        self.tx.execute(
            "UPDATE caldav_server_object SET unreadable = 1 WHERE calendar_id = ?1 AND href = ?2",
            params![calendar_id.0, href],
        )?;
        Ok(())
    }

    /// Deletes the CalDAV Server Object `id`; Event Rows aren't written.
    pub(crate) fn delete_object(&self, _: &SyncEngineWritePermission, id: ObjectId) -> Result<()> {
        self.tx
            .execute("DELETE FROM caldav_server_object WHERE id = ?1", [id.0])?;
        Ok(())
    }
}
