//! The CalDAV part of the Sync Engine (ADR 0015): CalDAV's Incoming Sync
//! ([`incoming`]), its writes of the Mirror, its rules for which
//! resource keeps a UID: the fixed order among resources holding one UID
//! ([`fixed_order`]) and recognising a Server Object that moved
//! ([`moved`]), and the CalDAV resources it doesn't mirror, the Ignored
//! Resources: remembering them ([`ignore`]) and their Log lines
//! ([`log_ignored`]).

pub(crate) mod incoming;

use super::WRITE_PERMISSION;
use crate::event::{Loaded, ObjectId, ServerObject};
use crate::journal::JournalTransaction;
use crate::{CalendarId, ImportedEvent, Result};

/// Where a CalDAV resource holding `events`, the Events of one UID, stands
/// in the fixed order among the resources of one Incoming Sync holding
/// that UID (ADR 0015): the one holding the single Event or the Recurring
/// Event first, then the lowest href, `href`, by byte comparison. The
/// lowest key comes first.
fn fixed_order<'a>(href: &'a str, events: &[ImportedEvent]) -> (bool, &'a str) {
    let only_exceptions = events.iter().all(|event| event.recurrence_id.is_some());
    (only_exceptions, href)
}

/// Deletes the Event Rows of `uid` in the Calendar `calendar_id` if no
/// CalDAV Server Object holds it any more, as an Incoming Sync found the
/// one keeping it gone and none taking over.
fn delete_if_unkept(
    journal_transaction: &mut JournalTransaction,
    calendar_id: CalendarId,
    uid: &str,
) -> Result<()> {
    if journal_transaction
        .database_transaction()
        .corresponding_server_object(calendar_id, uid)?
        .is_some()
    {
        return Ok(());
    }
    Loaded::of_uid(journal_transaction, calendar_id, uid)?.delete_all(journal_transaction)
}

/// The most UIDs a Log line of [`log_ignored`] lists.
const LOGGED_UIDS: usize = 5;

/// The most characters of each UID a Log line of [`log_ignored`] holds.
const LOGGED_UID_LENGTH: usize = 200;

/// Why the Sync Engine ignored a CalDAV resource (ADR 0015).
#[derive(Debug, Clone, Copy)]
enum Ignored<'a> {
    /// It holds several UIDs, these.
    SeveralUids(&'a [String]),
    /// It holds the UID `uid`, which the CalDAV Server Object at `kept_by`
    /// keeps.
    UidKept { uid: &'a str, kept_by: &'a str },
}

/// Writes to the Log that the Sync Engine ignored the CalDAV resource at
/// `href` of the Calendar `calendar_id`, and `why`: RFC 4791 §4.1 forbids
/// it (ADR 0015).
fn log_ignored(calendar_id: CalendarId, href: &str, why: Ignored) {
    log::warn!("{}", ignored_line(calendar_id, href, why));
}

/// The Log line of [`log_ignored`]. Addresses and UIDs are untrusted, so
/// each is escaped, which keeps a newline in them from starting another
/// line, and the line is kept short: the first [`LOGGED_UIDS`] UIDs, each
/// cut to [`LOGGED_UID_LENGTH`] characters, and a count of the others.
fn ignored_line(calendar_id: CalendarId, href: &str, why: Ignored) -> String {
    let quoted = |uid: &str| {
        let cut: String = uid.chars().take(LOGGED_UID_LENGTH).collect();
        format!("{cut:?}")
    };
    let reason = match why {
        Ignored::SeveralUids(uids) => {
            let listed: Vec<String> = uids
                .iter()
                .take(LOGGED_UIDS)
                .map(|uid| quoted(uid))
                .collect();
            let others = match uids.len().saturating_sub(LOGGED_UIDS) {
                0 => String::new(),
                others => format!(" and {others} more"),
            };
            format!(
                "it holds several UIDs, which RFC 4791 §4.1 forbids: {}{others}",
                listed.join(", ")
            )
        }
        Ignored::UidKept { uid, kept_by } => format!(
            "it holds the UID {} that the CalDAV resource {kept_by:?} holds, \
             which RFC 4791 §4.1 forbids",
            quoted(uid)
        ),
    };
    format!(
        "Ignored the CalDAV resource {href:?} of Calendar {}: {reason}",
        calendar_id.0
    )
}

/// Whether the CalDAV Server Object `object` is one of `moved_from`, the
/// Server Objects removed in the same Incoming Sync, so that a new resource
/// holding its UID is it moved to another address; taken out of
/// `moved_from` if so.
fn moved(object: ObjectId, moved_from: &mut Vec<ObjectId>) -> bool {
    let before = moved_from.len();
    moved_from.retain(|&removed| removed != object);
    moved_from.len() != before
}

/// Makes `object`, as the server holds it now, the Mirror's version of the
/// CalDAV Server Object `over` in a Calendar, address included, or of a new
/// one, and returns its ID. It holds the UID `uid`, which no other Server
/// Object of the Calendar may hold. An Ignored Resource
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
) -> Result<ObjectId> {
    journal_transaction.database_transaction().write_object(
        WRITE_PERMISSION,
        calendar_id,
        object,
        over,
        uid,
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
/// finds it gone from the server or no longer keeping its UID. The Event
/// Rows of its UID stay, for the Incoming Sync to propagate the Server
/// Object taking over to, or delete ([`delete_if_unkept`]).
fn remove_object(journal_transaction: &JournalTransaction, id: ObjectId) -> Result<()> {
    journal_transaction
        .database_transaction()
        .delete_object(WRITE_PERMISSION, id)
}

/// Remembers `object`, a CalDAV resource of a Calendar that an Incoming
/// Sync fetched and doesn't mirror, as an Ignored Resource with its ETag,
/// so that it isn't fetched again until it changes, and with `uid`, the
/// UID it holds that a Server Object keeps, so that it is fetched again
/// when that one stops keeping it; none if it holds several. The Mirror
/// holds nothing at its address any more.
fn ignore(
    journal_transaction: &JournalTransaction,
    calendar_id: CalendarId,
    object: &ServerObject,
    uid: Option<&str>,
) -> Result<()> {
    journal_transaction.database_transaction().ignore_resource(
        WRITE_PERMISSION,
        calendar_id,
        object,
        uid,
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
    ) -> Result<ObjectId> {
        super::mirror(journal_transaction, calendar_id, object, over, uid)
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

        let line = ignored_line(CalendarId(7), "/two\r\n.ics", Ignored::SeveralUids(&uids));

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

    #[test]
    fn the_log_line_of_a_caldav_resource_holding_a_uid_kept_by_another_escapes_and_cuts() {
        let long = format!("a\n{}", "x".repeat(LOGGED_UID_LENGTH));

        let line = ignored_line(
            CalendarId(7),
            "/copy\n.ics",
            Ignored::UidKept {
                uid: &long,
                kept_by: "/kept\n.ics",
            },
        );

        assert_eq!(
            line,
            format!(
                "Ignored the CalDAV resource \"/copy\\n.ics\" of Calendar 7: it holds the UID \
                 \"a\\n{}\" that the CalDAV resource \"/kept\\n.ics\" holds, which RFC 4791 \
                 §4.1 forbids",
                "x".repeat(LOGGED_UID_LENGTH - 2)
            )
        );
        assert!(!line.contains('\n'));
    }
}
