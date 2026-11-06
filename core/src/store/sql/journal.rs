//! The writes of the tables the Change Journal guards (ADR 0011): Event
//! Rows (`event`), Journal Transactions and their Journal Event
//! Modifications, Unsent Changes, and deleting Calendars and accounts. Each
//! takes a [`JournalWritePermission`], which only [`crate::journal`] can
//! make, and is
//! a method of a [`DatabaseTransaction`].

use rusqlite::params;

use super::EVENT_FIELDS;
use crate::event::RecurrenceId;
use crate::journal::{JournalTransactionId, JournalWritePermission};
use crate::store::{CombinedEventKey, DatabaseTransaction, StoredEvent};
use crate::{AccountId, CalendarId, CoreError, EventId, Result};

impl DatabaseTransaction<'_> {
    /// The one way an Event Row is written: as a new Event when `id` is none,
    /// otherwise over the Event `id` or as a new one with that ID.
    pub(crate) fn write_event(
        &self,
        _: &JournalWritePermission,
        id: Option<EventId>,
        uid: &str,
        event: &StoredEvent,
        (start_utc, end_utc): (i64, i64),
    ) -> Result<EventId> {
        let fields = &event.fields;
        self.tx.execute(
            "INSERT INTO event (id, uid, calendar_id, title, all_day, start_at, end_at, \
             start_zone, end_zone, start_utc, end_utc, location, description, \
             description_html, zone_pinned, rrule, exdates, recurrence_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, \
               ?18) \
             ON CONFLICT (id) DO UPDATE SET uid = excluded.uid, \
               calendar_id = excluded.calendar_id, title = excluded.title, \
               all_day = excluded.all_day, start_at = excluded.start_at, \
               end_at = excluded.end_at, start_zone = excluded.start_zone, \
               end_zone = excluded.end_zone, start_utc = excluded.start_utc, \
               end_utc = excluded.end_utc, location = excluded.location, \
               description = excluded.description, \
               description_html = excluded.description_html, \
               zone_pinned = excluded.zone_pinned, rrule = excluded.rrule, \
               exdates = excluded.exdates, recurrence_id = excluded.recurrence_id",
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
                fields.rule,
                fields.exdates,
                fields.recurrence_id.as_ref().map(RecurrenceId::as_obtained),
            ],
        )?;
        Ok(id.unwrap_or(EventId(self.tx.last_insert_rowid())))
    }

    /// Deletes the Event Row `id`.
    pub(crate) fn delete_event(&self, _: &JournalWritePermission, id: EventId) -> Result<()> {
        self.tx.execute("DELETE FROM event WHERE id = ?1", [id.0])?;
        Ok(())
    }

    /// Adds the Combined Event Key `combined_event_key` of an Event to the
    /// Unsent Changes, as changed at `since`, if its Calendar is a server
    /// Calendar; one already there keeps its `since`.
    pub(crate) fn add_unsent_change(
        &self,
        _: &JournalWritePermission,
        combined_event_key: &CombinedEventKey,
        since: i64,
    ) -> Result<()> {
        let CombinedEventKey {
            calendar_id,
            event_key,
        } = combined_event_key;
        self.tx.execute(
            "INSERT INTO unsent_change (calendar_id, uid, recurrence_id, since) \
             SELECT id, ?2, ?3, ?4 FROM calendar WHERE id = ?1 AND url IS NOT NULL \
             ON CONFLICT DO NOTHING",
            params![
                calendar_id.0,
                event_key.uid,
                event_key
                    .recurrence_id
                    .as_ref()
                    .map(RecurrenceId::as_obtained),
                since
            ],
        )?;
        Ok(())
    }

    /// Stores a new Journal Transaction, as `origin` records it, made at
    /// `made_at`.
    pub(crate) fn insert_journal_transaction(
        &self,
        _: &JournalWritePermission,
        origin: &str,
        made_at: i64,
    ) -> Result<JournalTransactionId> {
        self.tx.execute(
            "INSERT INTO journal_transaction (origin, made_at) VALUES (?1, ?2)",
            params![origin, made_at],
        )?;
        Ok(JournalTransactionId(self.tx.last_insert_rowid()))
    }

    /// Stores a Journal Event Modification of `journal_transaction`: the Event
    /// `id` with the UID `uid` went from `previous`, none when it was created,
    /// to an Event Version with the hash `hash`, none when it was deleted.
    pub(crate) fn insert_journal_event_modification(
        &self,
        _: &JournalWritePermission,
        journal_transaction: JournalTransactionId,
        id: EventId,
        uid: &str,
        previous: Option<&StoredEvent>,
        hash: Option<Vec<u8>>,
    ) -> Result<()> {
        let fields = previous.map(|event| &event.fields);
        let when = fields.map(|fields| &fields.when);
        self.tx.execute(
            &format!(
                "INSERT INTO journal_event_modification \
                 (journal_transaction_id, event_id, uid, hash, calendar_id, {EVENT_FIELDS}) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, \
                   ?16, ?17, ?18)"
            ),
            params![
                journal_transaction.0,
                id.0,
                uid,
                hash,
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
                fields.and_then(|fields| fields.rule.as_ref()),
                fields.map(|fields| &fields.exdates),
                fields
                    .and_then(|fields| fields.recurrence_id.as_ref())
                    .map(RecurrenceId::as_obtained),
            ],
        )?;
        Ok(())
    }

    /// Marks every Journal Event Modification of `journal_transaction` undone.
    pub(crate) fn mark_undone(
        &self,
        _: &JournalWritePermission,
        journal_transaction: JournalTransactionId,
    ) -> Result<()> {
        self.tx.execute(
            "UPDATE journal_event_modification SET undone = 1 WHERE journal_transaction_id = ?1",
            [journal_transaction.0],
        )?;
        Ok(())
    }

    /// Deletes a Calendar's Events with their Journal Event Modifications, the
    /// Journal Event Modifications that would put an Event back into it with
    /// that Event's earlier Journal Event Modifications, and the Journal
    /// Transactions left without Journal Event Modifications.
    pub(crate) fn delete_calendar_events(
        &self,
        _: &JournalWritePermission,
        id: CalendarId,
    ) -> Result<()> {
        self.tx.execute(
            "DELETE FROM journal_event_modification WHERE EXISTS (SELECT 1 FROM event \
             WHERE event.calendar_id = ?1 AND event.id = journal_event_modification.event_id \
               AND event.uid = journal_event_modification.uid)",
            [id.0],
        )?;
        self.tx.execute(
            "DELETE FROM journal_event_modification WHERE id <= (SELECT MAX(back.id) \
             FROM journal_event_modification AS back WHERE back.calendar_id = ?1 \
               AND back.event_id = journal_event_modification.event_id \
               AND back.uid = journal_event_modification.uid)",
            [id.0],
        )?;
        self.tx
            .execute("DELETE FROM event WHERE calendar_id = ?1", [id.0])?;
        self.tx.execute(
            "DELETE FROM journal_transaction WHERE NOT EXISTS \
             (SELECT 1 FROM journal_event_modification \
              WHERE journal_transaction_id = journal_transaction.id)",
            [],
        )?;
        Ok(())
    }

    /// Writes the Event `id`'s derived UTC instants, which the Change Journal
    /// doesn't record.
    pub(crate) fn write_instants(
        &self,
        _: &JournalWritePermission,
        id: EventId,
        (start_utc, end_utc): (i64, i64),
    ) -> Result<()> {
        self.tx.execute(
            "UPDATE event SET start_utc = ?2, end_utc = ?3 WHERE id = ?1",
            params![id.0, start_utc, end_utc],
        )?;
        Ok(())
    }

    /// Deletes the Calendar row `id`, once the Journal has deleted its
    /// Events. Its Server Objects go with it, by the schema's cascade.
    pub(crate) fn delete_calendar_row(
        &self,
        _: &JournalWritePermission,
        id: CalendarId,
    ) -> Result<()> {
        match self
            .tx
            .execute("DELETE FROM calendar WHERE id = ?1", [id.0])?
        {
            0 => Err(CoreError::CalendarNotFound(id)),
            _ => Ok(()),
        }
    }

    /// Deletes the Account row `id`, once the Journal has deleted its
    /// Calendars.
    pub(crate) fn delete_account_row(
        &self,
        _: &JournalWritePermission,
        id: AccountId,
    ) -> Result<()> {
        self.tx
            .execute("DELETE FROM account WHERE id = ?1", [id.0])?;
        Ok(())
    }
}
