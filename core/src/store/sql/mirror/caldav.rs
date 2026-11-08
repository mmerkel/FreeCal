//! The Sync Engine's writes of CalDAV's Mirror table, `caldav_server_object`
//! (ADR 0015): a CalDAV Server Object's iCalendar text as
//! received, with the one UID it holds, which no other in its Calendar
//! holds; and of the Ignored Resources,
//! `caldav_ignored_resource`, which aren't part of the Mirror. Each takes a
//! [`SyncEngineWritePermission`], which only [`crate::sync`] can make, and is
//! a method of a [`DatabaseTransaction`]. The CalDAV Sync Engine's reads of
//! those tables sit here too, methods of [`Store`]. The Corresponding Server
//! Object, which the Provider-neutral Sender and Conflicts read (ADR 0014),
//! stays with the shared readers.

use std::collections::HashMap;

use rusqlite::{OptionalExtension, params};

use super::super::readable;
use crate::event::ServerObject;
use crate::guests;
use crate::store::{DatabaseTransaction, Store};
use crate::sync::SyncEngineWritePermission;
use crate::sync::caldav::KnownServerResource;
use crate::{CalendarId, Result};

impl Store {
    /// The Known Server Resources of a CalDAV Calendar, by their address as
    /// the server gave it: its Server Objects, each with the UID it keeps,
    /// and its Ignored Resources, each with the UID it holds that a Server
    /// Object keeps, none for one holding several or not read.
    pub fn known_server_resources(
        &self,
        calendar_id: CalendarId,
    ) -> Result<HashMap<String, KnownServerResource>> {
        let mut stmt = self.conn.prepare(
            "SELECT href, etag, uid, FALSE FROM caldav_server_object \
             WHERE calendar_id = ?1 \
             UNION ALL SELECT href, etag, uid, TRUE FROM caldav_ignored_resource \
             WHERE calendar_id = ?1",
        )?;
        let known = readable(stmt.query_map([calendar_id.0], |row| {
            Ok((
                row.get(0)?,
                KnownServerResource {
                    etag: row.get(1)?,
                    uid: row.get(2)?,
                    ignored: row.get(3)?,
                },
            ))
        })?)?;
        Ok(known.into_iter().collect())
    }

    /// The UID that the CalDAV Server Object at `href` in a Calendar keeps,
    /// if one is there.
    pub fn uid_at(&self, calendar_id: CalendarId, href: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT uid FROM caldav_server_object WHERE calendar_id = ?1 AND href = ?2",
                params![calendar_id.0, href],
                |row| row.get(0),
            )
            .optional()?)
    }
}

impl DatabaseTransaction<'_> {
    /// Writes `object` as the CalDAV Server Object of a Calendar and the
    /// UID `uid`, address included, over the one there is (ADR 0015), with
    /// the attendees of its text (see [`guests`]), so that they always
    /// match it. It fails if another Server Object of the Calendar is at
    /// its address. An Ignored Resource at its address leaves
    /// `caldav_ignored_resource`, as an href is never both mirrored and
    /// ignored.
    pub(crate) fn write_object(
        &self,
        permission: &SyncEngineWritePermission,
        calendar_id: CalendarId,
        object: &ServerObject,
        uid: &str,
    ) -> Result<()> {
        self.forget_ignored_resource(permission, calendar_id, &object.href)?;
        self.tx.execute(
            "INSERT INTO caldav_server_object (calendar_id, uid, href, etag, data, attendees) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT (calendar_id, uid) DO UPDATE \
             SET href = excluded.href, etag = excluded.etag, data = excluded.data, \
             attendees = excluded.attendees",
            params![
                calendar_id.0,
                uid,
                object.href,
                object.etag,
                object.data,
                guests::stored(&guests::attendees(&object.data))
            ],
        )?;
        Ok(())
    }

    /// The ETag kept at `href` in a Calendar in `table`, a CalDAV Server
    /// Object's or an Ignored Resource's: none when it holds nothing there,
    /// `Some(None)` when it holds no ETag.
    fn etag_at(
        &self,
        table: &'static str,
        calendar_id: CalendarId,
        href: &str,
    ) -> Result<Option<Option<String>>> {
        Ok(self
            .tx
            .query_row(
                &format!("SELECT etag FROM {table} WHERE calendar_id = ?1 AND href = ?2"),
                params![calendar_id.0, href],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Forgets the ETag of the Known Server Resource at `href` in a
    /// Calendar, as FreeCal can't read what the server holds there, so that
    /// it is fetched on every Incoming Sync until it reads (ADR 0015). A
    /// CalDAV Server Object keeps its text and UID, and an Ignored Resource
    /// its UID; a resource FreeCal doesn't know yet becomes an Ignored
    /// Resource with neither ETag nor UID. Returns whether it became
    /// unreadable: it is new, or its ETag was known.
    pub(crate) fn forget_etag(
        &self,
        _: &SyncEngineWritePermission,
        calendar_id: CalendarId,
        href: &str,
    ) -> Result<bool> {
        let at = params![calendar_id.0, href];
        if let Some(etag) = self.etag_at("caldav_server_object", calendar_id, href)? {
            self.tx.execute(
                "UPDATE caldav_server_object SET etag = NULL WHERE calendar_id = ?1 AND href = ?2",
                at,
            )?;
            return Ok(etag.is_some());
        }
        let ignored = self.etag_at("caldav_ignored_resource", calendar_id, href)?;
        self.tx.execute(
            "INSERT INTO caldav_ignored_resource (calendar_id, href, etag, uid) \
             VALUES (?1, ?2, NULL, NULL) \
             ON CONFLICT (calendar_id, href) DO UPDATE SET etag = NULL",
            at,
        )?;
        Ok(ignored.is_none_or(|etag| etag.is_some()))
    }

    /// Forgets the ETags of the Ignored Resources of a Calendar holding the
    /// UID `uid`, so that the next Incoming Sync fetches them, as one fetches
    /// every Known Server Resource without an ETag (ADR 0015).
    pub(crate) fn forget_etags_holding(
        &self,
        _: &SyncEngineWritePermission,
        calendar_id: CalendarId,
        uid: &str,
    ) -> Result<()> {
        self.tx.execute(
            "UPDATE caldav_ignored_resource SET etag = NULL WHERE calendar_id = ?1 AND uid = ?2",
            params![calendar_id.0, uid],
        )?;
        Ok(())
    }

    /// Deletes the CalDAV Server Object of a Calendar and the UID `uid`;
    /// Event Rows aren't written.
    pub(crate) fn delete_object(
        &self,
        _: &SyncEngineWritePermission,
        calendar_id: CalendarId,
        uid: &str,
    ) -> Result<()> {
        self.tx.execute(
            "DELETE FROM caldav_server_object WHERE calendar_id = ?1 AND uid = ?2",
            params![calendar_id.0, uid],
        )?;
        Ok(())
    }

    /// Remembers the CalDAV resource `object` of a Calendar as an Ignored
    /// Resource with its ETag and `uid`, the UID it holds that a Server
    /// Object keeps, none if it holds several, or updates both of the one
    /// at its address. The Mirror must hold nothing at its address. Returns
    /// whether it changed: it is new, or was remembered with another ETag
    /// or none.
    pub(crate) fn ignore_resource(
        &self,
        _: &SyncEngineWritePermission,
        calendar_id: CalendarId,
        object: &ServerObject,
        uid: Option<&str>,
    ) -> Result<bool> {
        let remembered = self.etag_at("caldav_ignored_resource", calendar_id, &object.href)?;
        self.tx.execute(
            "INSERT INTO caldav_ignored_resource (calendar_id, href, etag, uid) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT (calendar_id, href) DO UPDATE SET etag = excluded.etag, uid = excluded.uid",
            params![calendar_id.0, object.href, object.etag, uid],
        )?;
        let unchanged = remembered
            .flatten()
            .is_some_and(|etag| object.etag == Some(etag));
        Ok(!unchanged)
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
