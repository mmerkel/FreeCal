//! The CalDAV part of the Sync Engine (ADR 0015): CalDAV's Incoming Sync
//! ([`incoming`]), its writes of the Mirror, by Calendar and UID, its rules
//! for which resource keeps a UID: the fixed order among resources holding
//! one UID ([`fixed_order`]) and the Ignored Resources fetched again for a
//! takeover ([`ignored_to_fetch`]), the CalDAV resources it doesn't mirror,
//! the Ignored Resources: remembering them and their Log lines
//! ([`ignore`]), and the resources it can't read ([`unreadable`]).

pub(crate) mod incoming;

use std::collections::{HashMap, HashSet};

use super::WRITE_PERMISSION;
use crate::event::{Loaded, ServerObject};
use crate::journal::JournalTransaction;
use crate::{CalendarId, ImportedEvent, Result};

/// A Known Server Resource of a CalDAV Calendar as the CalDAV Sync Engine
/// keeps it: a Server Object or an Ignored Resource, at an address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KnownServerResource {
    /// The ETag of the version FreeCal holds; none if the server gave none
    /// or FreeCal couldn't read the server's version, so that it is fetched
    /// on every Incoming Sync (ADR 0015).
    pub etag: Option<String>,
    /// The UID it holds as FreeCal last read it: the one a Server Object
    /// keeps, or the one an Ignored Resource holds that a Server Object
    /// keeps; none for an Ignored Resource holding several or not read.
    pub uid: Option<String>,
    /// Whether it is an Ignored Resource.
    pub ignored: bool,
}

/// The addresses of the Ignored Resources among `known`, a Calendar's
/// Known Server Resources by address, to fetch with `asked`, the addresses
/// of those an Incoming Sync fetches or finds removed, for a takeover (ADR
/// 0015): those holding a UID that a Server Object among `asked` keeps,
/// whether or not it still keeps it once fetched. Which resource keeps the
/// UID is decided inside the Journal Transaction; fetching them in the same
/// request needs no second one, at the price of fetching them when the
/// Server Object still keeps its UID.
fn ignored_to_fetch(known: &HashMap<String, KnownServerResource>, asked: &[String]) -> Vec<String> {
    let asked_set: HashSet<&str> = asked.iter().map(String::as_str).collect();
    let kept: HashSet<&str> = asked
        .iter()
        .filter_map(|href| known.get(href))
        .filter(|resource| !resource.ignored)
        .filter_map(|resource| resource.uid.as_deref())
        .collect();
    let mut hrefs: Vec<String> = known
        .iter()
        .filter(|(href, resource)| {
            resource.ignored
                && resource
                    .uid
                    .as_deref()
                    .is_some_and(|uid| kept.contains(uid))
                && !asked_set.contains(href.as_str())
        })
        .map(|(href, _)| href.clone())
        .collect();
    hrefs.sort();
    hrefs
}

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
        .server_object(calendar_id, uid)?
        .is_some()
    {
        return Ok(());
    }
    Loaded::of_uid(journal_transaction, calendar_id, uid)?.delete_all(journal_transaction)
}

/// The most UIDs a Log line of [`ignore`] lists.
const LOGGED_UIDS: usize = 5;

/// The most characters of each UID a Log line of [`ignore`] holds.
const LOGGED_UID_LENGTH: usize = 200;

/// Why the Sync Engine ignored a CalDAV resource (ADR 0015).
#[derive(Debug, Clone, Copy)]
enum Ignored<'a> {
    /// It holds several UIDs, these.
    SeveralUids(&'a [String]),
    /// It holds the Ambiguous UID `uid`, which the CalDAV Server Object at
    /// `kept_by` keeps.
    UidKept { uid: &'a str, kept_by: &'a str },
}

/// The Log line of [`ignore`]. Addresses and UIDs are untrusted, so
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

/// Makes `object`, as the server holds it now, the Mirror's version of the
/// CalDAV Server Object of a Calendar and the UID `uid`, address included
/// (ADR 0015). An Ignored Resource at its address is forgotten in the same
/// Database Transaction. Written in the Database Transaction of the
/// Incoming Sync's `journal_transaction`, as only an Incoming Sync knows
/// what the server holds, which checked that the Calendar is writable.
fn mirror(
    journal_transaction: &JournalTransaction,
    calendar_id: CalendarId,
    object: &ServerObject,
    uid: &str,
) -> Result<()> {
    journal_transaction.database_transaction().write_object(
        WRITE_PERMISSION,
        calendar_id,
        object,
        uid,
    )
}

/// Records that FreeCal couldn't read what the server holds now at `href` in
/// a Calendar by forgetting its ETag, so that it is fetched on every
/// Incoming Sync until it reads (ADR 0015). A Server Object there keeps the
/// last version it could read, its UID and its Event Rows; an Ignored
/// Resource keeps its UID; a new resource becomes an Ignored Resource
/// holding no UID. Writes to the Log when it becomes unreadable, not on
/// each fetch while it stays so.
fn unreadable(
    journal_transaction: &JournalTransaction,
    calendar_id: CalendarId,
    href: &str,
) -> Result<()> {
    if journal_transaction.database_transaction().forget_etag(
        WRITE_PERMISSION,
        calendar_id,
        href,
    )? {
        log::warn!("{}", unreadable_line(calendar_id, href));
    }
    Ok(())
}

/// The Log line of [`unreadable`]. The address is untrusted, so it is
/// escaped, which keeps a newline in it from starting another line.
fn unreadable_line(calendar_id: CalendarId, href: &str) -> String {
    format!(
        "Couldn't read the CalDAV resource {href:?} of Calendar {}: it is fetched on every \
         Incoming Sync until it reads",
        calendar_id.0
    )
}

/// Removes the Server Object of a Calendar and the UID `uid` from the
/// Mirror, as an Incoming Sync finds it gone from the server or no longer
/// keeping its UID. The Event Rows of its UID stay, for the Incoming Sync
/// to propagate the Server Object taking over to, or delete
/// ([`delete_if_unkept`]).
fn remove_object(
    journal_transaction: &JournalTransaction,
    calendar_id: CalendarId,
    uid: &str,
) -> Result<()> {
    journal_transaction
        .database_transaction()
        .delete_object(WRITE_PERMISSION, calendar_id, uid)
}

/// Remembers `object`, a CalDAV resource of a Calendar that an Incoming
/// Sync fetched and doesn't mirror, as an Ignored Resource with its ETag,
/// so that it isn't fetched again until it changes, and with the UID it
/// holds that a Server Object keeps, so that it is fetched again when that
/// one stops keeping it; none if it holds several. Writes to the Log that
/// it is ignored, and `why`: RFC 4791 §4.1 forbids it. One fetched with an
/// unchanged ETag, as for a takeover that didn't happen, writes no Log
/// line. The Mirror holds nothing at its address any more.
fn ignore(
    journal_transaction: &JournalTransaction,
    calendar_id: CalendarId,
    object: &ServerObject,
    why: Ignored,
) -> Result<()> {
    let uid = match why {
        Ignored::SeveralUids(_) => None,
        Ignored::UidKept { uid, .. } => Some(uid),
    };
    if journal_transaction.database_transaction().ignore_resource(
        WRITE_PERMISSION,
        calendar_id,
        object,
        uid,
    )? {
        log::warn!("{}", ignored_line(calendar_id, &object.href, why));
    }
    Ok(())
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
        uid: &str,
    ) -> Result<()> {
        super::mirror(journal_transaction, calendar_id, object, uid)
    }

    /// [`remove_object`](super::remove_object).
    pub(crate) fn remove_object(
        journal_transaction: &JournalTransaction,
        calendar_id: CalendarId,
        uid: &str,
    ) -> Result<()> {
        super::remove_object(journal_transaction, calendar_id, uid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::{self, TransactionOrigin, record};
    use crate::store::Store;

    /// Not visible through the core interface, so not a seam test.
    #[test]
    fn deleting_its_calendar_deletes_an_ignored_resource() {
        let directory = tempfile::TempDir::new().unwrap();
        let store = Store::open(&directory.path().join("freecal.sqlite3")).unwrap();
        let calendar_id = store.writable_server_calendar();
        let object = ServerObject {
            href: "/two.ics".to_owned(),
            etag: Some("1".to_owned()),
            data: String::new(),
        };
        let uids = ["a".to_owned(), "b".to_owned()];
        record(
            &store,
            TransactionOrigin::IncomingSync,
            0,
            |journal_transaction| {
                ignore(
                    journal_transaction,
                    calendar_id,
                    &object,
                    Ignored::SeveralUids(&uids),
                )
            },
        )
        .unwrap();
        assert_eq!(store.known_server_resources(calendar_id).unwrap().len(), 1);

        journal::delete_calendar(&store, calendar_id).unwrap();

        assert!(
            store
                .known_server_resources(calendar_id)
                .unwrap()
                .is_empty()
        );
    }

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
    fn the_log_line_of_an_unreadable_caldav_resource_escapes_its_address() {
        let line = unreadable_line(CalendarId(7), "/late\r\n.ics");

        assert_eq!(
            line,
            "Couldn't read the CalDAV resource \"/late\\r\\n.ics\" of Calendar 7: it is \
             fetched on every Incoming Sync until it reads"
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
