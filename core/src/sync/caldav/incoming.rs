//! The CalDAV Incoming Sync: what changed in a Calendar on its server,
//! asked for before the Journal Transaction ([`changes`]), then brought
//! into its local copy in one Journal Transaction, with its Mirror writes
//! in the same Database Transaction. Fetched resources are matched with the
//! Known Server Resources by address, and a CalDAV Server Object is its
//! Calendar and UID: an Ambiguous UID is kept by one at a time (ADR 0015).

use std::collections::HashMap;

use chrono_tz::Tz;

use super::{
    Ignored, KnownServerResource, delete_if_unkept, fixed_order, forget_ignored, ignore,
    ignored_to_fetch, mirror, remove_object, unreadable,
};
use crate::event::{self, ServerObject};
use crate::import::{Holds, read_caldav_resource};
use crate::journal::{JournalTransaction, TransactionOrigin};
use crate::remote::{Changes, KnownVersions, Remote};
use crate::{CalendarId, ImportedEvent, Result};

/// Asks `remote` what changed in the Calendar at `url` since `token`, given
/// `known`, its Known Server Resources by address. The Ignored Resources
/// that a takeover may need ([`ignored_to_fetch`](super::ignored_to_fetch))
/// are fetched in the same request as the rest. Runs before the Journal
/// Transaction, as no network request runs inside a Database Transaction.
pub(crate) fn changes(
    remote: &dyn Remote,
    known: &HashMap<String, KnownServerResource>,
    url: &str,
    token: Option<&str>,
    display: Tz,
) -> Result<Changes> {
    let versions: KnownVersions = known
        .iter()
        .map(|(href, resource)| (href.clone(), resource.etag.clone()))
        .collect();
    remote.changes(url, token, &versions, display, &|asked| {
        ignored_to_fetch(known, asked)
    })
}

/// Brings `changes` into the Calendar `calendar_id` in `journal_transaction`,
/// as [`mirror_and_propagate`] does it, and records its new sync state with
/// them.
pub(crate) fn incoming_sync(
    journal_transaction: &mut JournalTransaction,
    calendar_id: CalendarId,
    changes: &Changes,
) -> Result<()> {
    mirror_and_propagate(
        journal_transaction,
        calendar_id,
        &changes.changed,
        &changes.removed,
        changes.zone,
    )?;
    journal_transaction.synced(calendar_id, changes.state.token.as_deref())
}

/// A fetched CalDAV resource holding the Events `events` of one UID: a
/// candidate to keep it.
#[derive(Clone, Copy)]
struct Candidate<'a> {
    object: &'a ServerObject,
    events: &'a [ImportedEvent],
}

impl Candidate<'_> {
    fn fixed_order(&self) -> (bool, &str) {
        fixed_order(&self.object.href, self.events)
    }
}

/// Mirrors and propagates the resources `fetched` into the Calendar
/// `calendar_id` in `journal_transaction`, their floating times pinned to
/// `zone`, and removes those at the addresses `removed`. A removed Server
/// Object, or one that no longer holds the UID it kept, goes from the
/// Mirror first, and the UID's Event Rows with it unless a resource takes
/// over. The resources holding one UID, an Ambiguous UID when there are
/// several, compete to keep it as [`keep`] decides; the one keeping it is
/// mirrored, and propagated to the UID's Event Rows as
/// [`event::propagate_to_event_rows`] writes them, and the others are
/// Ignored Resources. A resource holding nothing FreeCal keeps isn't
/// mirrored; one holding several UIDs, which RFC 4791 §4.1 forbids, is an
/// Ignored Resource ([`ignore`]), and an Ignored Resource that now holds
/// nothing, or is removed, is forgotten. One FreeCal can't read loses its
/// ETag, so that it is fetched again ([`unreadable`]): a Server Object
/// keeps its last readable version, UID and Event Rows, an Ignored Resource
/// its UID, and a new one becomes an Ignored Resource. A Recurring Event
/// that arrives joins the Calendar's Exceptions that had none without
/// writing them, as they keep their recurrence IDs as obtained (ADR 0012).
pub(crate) fn mirror_and_propagate(
    journal_transaction: &mut JournalTransaction,
    calendar_id: CalendarId,
    fetched: &[ServerObject],
    removed: &[String],
    zone: Tz,
) -> Result<()> {
    journal_transaction.writable(calendar_id)?;
    // First every Server Object that goes or stops keeping its UID goes, so
    // that the UID is free for another before any is mirrored, and its
    // address for another UID.
    let mut lost = Vec::new();
    for href in removed {
        forget_ignored(journal_transaction, calendar_id, href)?;
        lost.extend(remove_at(journal_transaction, calendar_id, href, |_| true)?);
    }
    let read: Vec<(&ServerObject, Holds)> = fetched
        .iter()
        .map(|object| (object, read_caldav_resource(object, zone)))
        .collect();
    let mut by_uid: Vec<(&str, Vec<Candidate>)> = Vec::new();
    for (object, holds) in &read {
        let href = &object.href;
        lost.extend(remove_at(journal_transaction, calendar_id, href, |uid| {
            !holds.keeps(uid)
        })?);
        match holds {
            Holds::Unreadable => unreadable(journal_transaction, calendar_id, href)?,
            Holds::NoEvents => forget_ignored(journal_transaction, calendar_id, href)?,
            Holds::SeveralUids(uids) => {
                ignore(
                    journal_transaction,
                    calendar_id,
                    object,
                    Ignored::SeveralUids(uids),
                )?;
            }
            Holds::OneUid { uid, events } => {
                let candidate = Candidate { object, events };
                match by_uid.iter_mut().find(|(held, _)| held == uid) {
                    Some((_, candidates)) => candidates.push(candidate),
                    None => by_uid.push((uid, vec![candidate])),
                }
            }
        }
    }
    for (uid, candidates) in by_uid {
        keep(journal_transaction, calendar_id, uid, &candidates, zone)?;
    }
    for uid in lost {
        delete_if_unkept(journal_transaction, calendar_id, &uid)?;
    }
    Ok(())
}

/// Removes the Server Object at `href` in the Calendar `calendar_id` from
/// the Mirror if `goes` says so of the UID it keeps, and returns that UID.
fn remove_at(
    journal_transaction: &JournalTransaction,
    calendar_id: CalendarId,
    href: &str,
    goes: impl Fn(&str) -> bool,
) -> Result<Option<String>> {
    let uid = journal_transaction
        .database_transaction()
        .uid_at(calendar_id, href)?
        .filter(|uid| goes(uid));
    if let Some(uid) = &uid {
        remove_object(journal_transaction, calendar_id, uid)?;
    }
    Ok(uid)
}

/// Decides which of `candidates`, the fetched resources holding `uid` in
/// the Calendar `calendar_id`, keeps the UID, an Ambiguous UID when they
/// are several or a Server Object holds it already, mirrors it and
/// propagates it to the UID's Event Rows, and makes the others Ignored
/// Resources holding `uid` (ADR 0015). The Server Object keeping `uid`
/// keeps it as long as it holds it: when it is a candidate, it keeps it,
/// and when it is unchanged, no candidate does. With no Server Object
/// keeping `uid`, as when it went in this Incoming Sync, removed, moved to
/// another address or holding something else now, the first candidate by
/// the [`fixed_order`] keeps it. In a First Sync, every Server Object was
/// mirrored in the same Incoming Sync, so the one keeping `uid` competes
/// with the candidates by the fixed order, and becomes an Ignored Resource
/// if one comes first.
fn keep(
    journal_transaction: &mut JournalTransaction,
    calendar_id: CalendarId,
    uid: &str,
    candidates: &[Candidate],
    zone: Tz,
) -> Result<()> {
    let first = candidates
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.fixed_order().cmp(&b.fixed_order()))
        .map(|(index, _)| index);
    let held = journal_transaction
        .database_transaction()
        .server_object(calendar_id, uid)?;
    let keeper = match held {
        None => first,
        Some(held) => {
            if let Some(index) = candidates
                .iter()
                .position(|candidate| candidate.object.href == held.href)
            {
                Some(index)
            } else if journal_transaction.origin() == TransactionOrigin::FirstSync
                && let Holds::OneUid { events, .. } = read_caldav_resource(&held, zone)
                && let Some(index) = first.filter(|&index| {
                    candidates[index].fixed_order() < fixed_order(&held.href, &events)
                })
            {
                remove_object(journal_transaction, calendar_id, uid)?;
                let kept_by = &candidates[index].object.href;
                ignore(
                    journal_transaction,
                    calendar_id,
                    &held,
                    Ignored::UidKept { uid, kept_by },
                )?;
                Some(index)
            } else {
                // It stays and keeps the UID.
                return ignore_others(
                    journal_transaction,
                    calendar_id,
                    uid,
                    candidates,
                    &held.href,
                );
            }
        }
    };
    let Some(index) = keeper else {
        return Ok(());
    };
    let Candidate { object, events } = candidates[index];
    mirror(journal_transaction, calendar_id, object, uid)?;
    event::propagate_to_event_rows(journal_transaction, calendar_id, uid, events)?;
    ignore_others(
        journal_transaction,
        calendar_id,
        uid,
        candidates,
        &object.href,
    )
}

/// Makes each of `candidates` but the one at `kept_by`, which keeps `uid`,
/// an Ignored Resource holding `uid`.
fn ignore_others(
    journal_transaction: &JournalTransaction,
    calendar_id: CalendarId,
    uid: &str,
    candidates: &[Candidate],
    kept_by: &str,
) -> Result<()> {
    for candidate in candidates {
        if candidate.object.href != kept_by {
            ignore(
                journal_transaction,
                calendar_id,
                candidate.object,
                Ignored::UidKept { uid, kept_by },
            )?;
        }
    }
    Ok(())
}
