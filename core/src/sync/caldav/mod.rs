//! The CalDAV part of the Sync Engine (ADR 0015): CalDAV's Incoming Sync
//! ([`incoming`]), its writes of the Mirror, the rules that read the
//! Mirror for it: handing a UID's Event Rows to its next Corresponding
//! Server Object ([`settle`]) and recognising a Server Object that moved
//! ([`moved`]), and the CalDAV resources it doesn't mirror, the Ignored
//! Resources: remembering them ([`ignore`]) and their Log lines
//! ([`log_ignored`]).

pub(crate) mod incoming;

use chrono_tz::Tz;

use super::WRITE_PERMISSION;
use crate::event::{self, Loaded, ObjectId, ServerObject};
use crate::import::{Holds, read_caldav_resource};
use crate::journal::JournalTransaction;
use crate::{CalendarId, ImportedEvent, Result};

/// Whether `events`, a CalDAV Server Object's Events of its UID
/// ([`Holds::OneUid`]), hold the Event that isn't an Exception: the single
/// Event or the Recurring Event.
fn holds_non_exception(events: &[ImportedEvent]) -> bool {
    events.iter().any(|event| event.recurrence_id().is_none())
}

/// Gives the Event Rows of `uid`, a UID of the Calendar `calendar_id` whose
/// Corresponding Server Object went, to its next Corresponding Server
/// Object, which is propagated to them as [`event::propagate_to_event_rows`]
/// writes them: its Events of the UID, read from its mirrored data by
/// [`read_caldav_resource`] with floating times pinned to `zone`. A UID no Server
/// Object holds any more loses its Event Rows. Should the mirrored data no
/// longer read, the Event Rows stay as they are, as for a Server Object
/// FreeCal can't read.
fn settle(
    journal_transaction: &mut JournalTransaction,
    calendar_id: CalendarId,
    uid: &str,
    zone: Tz,
) -> Result<()> {
    let Some(corresponding_server_object) = journal_transaction
        .database_transaction()
        .corresponding_server_object(calendar_id, uid)?
    else {
        return Loaded::of_uid(journal_transaction, calendar_id, uid)?
            .delete_all(journal_transaction);
    };
    let Some(object) = journal_transaction
        .database_transaction()
        .server_object(corresponding_server_object)?
    else {
        return Ok(());
    };
    match read_caldav_resource(&object, zone) {
        Holds::OneUid { events, .. } => event::propagate_to_event_rows(
            journal_transaction,
            calendar_id,
            corresponding_server_object,
            uid,
            &events,
        ),
        Holds::Unreadable | Holds::Nothing { .. } => Ok(()),
    }
}

/// The most UIDs a Log line of [`log_ignored`] lists.
const LOGGED_UIDS: usize = 5;

/// The most characters of each UID a Log line of [`log_ignored`] holds.
const LOGGED_UID_LENGTH: usize = 200;

/// Writes to the Log that the Sync Engine ignored the CalDAV resource at
/// `href` of the Calendar `calendar_id`, which holds the several UIDs
/// `uids`, as RFC 4791 §4.1 forbids (ADR 0015).
fn log_ignored(calendar_id: CalendarId, href: &str, uids: &[String]) {
    log::warn!("{}", ignored_line(calendar_id, href, uids));
}

/// The Log line of [`log_ignored`]. The address and the UIDs are untrusted,
/// so each is escaped, which keeps a newline in them from starting another
/// line, and the line is kept short: the first [`LOGGED_UIDS`] UIDs, each cut
/// to [`LOGGED_UID_LENGTH`] characters, and a count of the others.
fn ignored_line(calendar_id: CalendarId, href: &str, uids: &[String]) -> String {
    let listed: Vec<String> = uids
        .iter()
        .take(LOGGED_UIDS)
        .map(|uid| {
            let cut: String = uid.chars().take(LOGGED_UID_LENGTH).collect();
            format!("{cut:?}")
        })
        .collect();
    let others = match uids.len().saturating_sub(LOGGED_UIDS) {
        0 => String::new(),
        others => format!(" and {others} more"),
    };
    format!(
        "Ignored the CalDAV resource {href:?} of Calendar {}: it holds several UIDs, \
         which RFC 4791 §4.1 forbids: {}{others}",
        calendar_id.0,
        listed.join(", "),
    )
}

/// The CalDAV Server Object of `moved_from` that holds `uid`, taken out of
/// it; none if there is none.
fn moved(
    journal_transaction: &JournalTransaction,
    uid: &str,
    moved_from: &mut Vec<ObjectId>,
) -> Result<Option<ObjectId>> {
    for (index, &object) in moved_from.iter().enumerate() {
        if journal_transaction
            .database_transaction()
            .held_uid(object)?
            .as_deref()
            == Some(uid)
        {
            return Ok(Some(moved_from.remove(index)));
        }
    }
    Ok(None)
}

/// Makes `object`, as the server holds it now, the Mirror's version of the
/// CalDAV Server Object `over` in a Calendar, address included, or of a new
/// one, and returns its ID. It holds the UID `uid`, and the Event of it
/// that isn't an Exception when `holds_non_exception`. An Ignored Resource
/// at its address is forgotten in the same Database Transaction. Written in the
/// Database Transaction of the Incoming Sync's `journal_transaction`, as
/// only an Incoming Sync knows what the server holds, which checked that the
/// Calendar is writable.
fn mirror(
    journal_transaction: &JournalTransaction,
    calendar_id: CalendarId,
    object: &ServerObject,
    over: Option<ObjectId>,
    uid: &str,
    holds_non_exception: bool,
) -> Result<ObjectId> {
    journal_transaction.database_transaction().write_object(
        WRITE_PERMISSION,
        calendar_id,
        object,
        over,
        uid,
        holds_non_exception,
    )
}

/// Records that FreeCal couldn't read what the server holds now at `href` in
/// a Calendar: the Server Object there keeps the last version it could, and
/// its Event Rows stay as they are.
fn unreadable(
    journal_transaction: &JournalTransaction,
    calendar_id: CalendarId,
    href: &str,
) -> Result<()> {
    journal_transaction
        .database_transaction()
        .mark_unreadable(WRITE_PERMISSION, calendar_id, href)
}

/// Removes the Server Object `id` from the Mirror, as an Incoming Sync
/// finds it gone from the server. The Event Rows of the UID it was the
/// Corresponding Server Object of stay, for the Incoming Sync to give to
/// their next Corresponding Server Object or delete.
fn remove_object(journal_transaction: &JournalTransaction, id: ObjectId) -> Result<()> {
    journal_transaction
        .database_transaction()
        .delete_object(WRITE_PERMISSION, id)
}

/// Remembers `object`, a CalDAV resource of a Calendar that an Incoming
/// Sync fetched and doesn't mirror, as an Ignored Resource with its ETag,
/// so that it isn't fetched again until it changes. The Mirror holds
/// nothing at its address any more.
fn ignore(
    journal_transaction: &JournalTransaction,
    calendar_id: CalendarId,
    object: &ServerObject,
) -> Result<()> {
    journal_transaction.database_transaction().ignore_resource(
        WRITE_PERMISSION,
        calendar_id,
        object,
    )
}

/// Forgets the Ignored Resource at `href` in a Calendar, if there is one,
/// as an Incoming Sync finds it gone from the server or no longer breaking
/// a rule. Mirroring a resource forgets it too ([`mirror`]).
fn forget_ignored(
    journal_transaction: &JournalTransaction,
    calendar_id: CalendarId,
    href: &str,
) -> Result<()> {
    journal_transaction
        .database_transaction()
        .forget_ignored_resource(WRITE_PERMISSION, calendar_id, href)
}

/// The Sync Engine's Mirror writes, for other modules' tests to set the
/// Mirror up with.
#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    /// [`mirror`](super::mirror).
    pub(crate) fn mirror(
        journal_transaction: &JournalTransaction,
        calendar_id: CalendarId,
        object: &ServerObject,
        over: Option<ObjectId>,
        uid: &str,
        holds_non_exception: bool,
    ) -> Result<ObjectId> {
        super::mirror(
            journal_transaction,
            calendar_id,
            object,
            over,
            uid,
            holds_non_exception,
        )
    }

    /// [`remove_object`](super::remove_object).
    pub(crate) fn remove_object(
        journal_transaction: &JournalTransaction,
        id: ObjectId,
    ) -> Result<()> {
        super::remove_object(journal_transaction, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_log_line_of_a_caldav_resource_holding_several_uids_escapes_and_cuts_what_the_server_gave()
     {
        let long = "x".repeat(LOGGED_UID_LENGTH + 1);
        let uids: Vec<String> = ["a\nforged line", long.as_str(), "c", "d", "e", "f", "g"]
            .map(str::to_owned)
            .into();

        let line = ignored_line(CalendarId(7), "/two\r\n.ics", &uids);

        assert_eq!(
            line,
            format!(
                "Ignored the CalDAV resource \"/two\\r\\n.ics\" of Calendar 7: it holds several \
                 UIDs, which RFC 4791 §4.1 forbids: \"a\\nforged line\", \"{}\", \"c\", \"d\", \
                 \"e\" and 2 more",
                "x".repeat(LOGGED_UID_LENGTH)
            )
        );
        assert!(!line.contains('\n') && !line.contains('\r'));
    }
}
