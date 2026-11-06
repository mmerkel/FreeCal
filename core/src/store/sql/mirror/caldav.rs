//! The Sync Engine's writes of CalDAV's Mirror table, `caldav_server_object`
//! (ADR 0015): a CalDAV Server Object's iCalendar text as
//! received, with the one UID it holds, which no other in its Calendar
//! holds; and of the Ignored Resources,
//! `caldav_ignored_resource`, which aren't part of the Mirror. Each takes a
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
    /// returns its ID. It fails if another Server Object of the Calendar
    /// holds `uid` (ADR 0015). An Ignored Resource at its address leaves
    /// `caldav_ignored_resource`, as an href is never both mirrored and
    /// ignored.
    pub(crate) fn write_object(
        &self,
        permission: &SyncEngineWritePermission,
        calendar_id: CalendarId,
        object: &ServerObject,
        over: Option<ObjectId>,
        uid: &str,
    ) -> Result<ObjectId> {
        self.forget_ignored_resource(permission, calendar_id, &object.href)?;
        match over {
            Some(id) => {
                self.tx.execute(
                    "UPDATE caldav_server_object SET href = ?2, etag = ?3, data = ?4, \
                     unreadable = 0, uid = ?5 WHERE id = ?1",
                    params![id.0, object.href, object.etag, object.data, uid],
                )?;
                Ok(id)
            }
            None => {
                self.tx.execute(
                    "INSERT INTO caldav_server_object \
                     (calendar_id, href, etag, data, uid) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![calendar_id.0, object.href, object.etag, object.data, uid],
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

    /// Remembers the CalDAV resource `object` of a Calendar as an Ignored
    /// Resource with its ETag and `uid`, the UID it holds that a Server
    /// Object keeps, none if it holds several, or updates both of the one
    /// at its address. The Mirror must hold nothing at its address.
    pub(crate) fn ignore_resource(
        &self,
        _: &SyncEngineWritePermission,
        calendar_id: CalendarId,
        object: &ServerObject,
        uid: Option<&str>,
    ) -> Result<()> {
        self.tx.execute(
            "INSERT INTO caldav_ignored_resource (calendar_id, href, etag, uid) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT (calendar_id, href) DO UPDATE SET etag = excluded.etag, uid = excluded.uid",
            params![calendar_id.0, object.href, object.etag, uid],
        )?;
        Ok(())
    }

    /// Forgets the Ignored Resource at `href` in a Calendar, if there is
    /// one.
    pub(crate) fn forget_ignored_resource(
        &self,
        _: &SyncEngineWritePermission,
        calendar_id: CalendarId,
        href: &str,
    ) -> Result<()> {
        self.tx.execute(
            "DELETE FROM caldav_ignored_resource WHERE calendar_id = ?1 AND href = ?2",
            params![calendar_id.0, href],
        )?;
        Ok(())
    }
}
