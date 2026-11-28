//! What the user sees of the Change Journal (ticket 19): the Change Journal
//! list, one timeline of Journal Transactions that expand into their
//! Events; an Event's History; and the Undo History, what Ctrl+Z undoes and
//! Ctrl+Shift+Z redoes, worked out from the Change Journal alone so that an
//! undo from the list or a History counts at once, and a restart changes
//! nothing.

use std::collections::HashMap;

use serde::Serialize;

use super::undo::{UndoTarget, undone_per_event};
use super::{JournalEventModification, JournalTransactionId, TransactionOrigin};
use crate::store::{CombinedEventKey, Store, StoredJournalTransaction};
use crate::{CalendarId, CoreError, EventId, Result};

/// How many titles of its Events a listed Journal Transaction names.
const TITLES_NAMED: usize = 3;

/// A Journal Transaction as the Change Journal list shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ListedJournalTransaction {
    pub id: JournalTransactionId,
    pub origin: TransactionOrigin,
    /// When it was made, in Unix seconds.
    pub made_at: i64,
    /// For an undo, the Journal Transaction it was asked to undo; none for
    /// one of a Calendar since a given time.
    pub undoes: Option<JournalTransactionId>,
    /// How many Events it changed, counted as Event Rows.
    pub events: usize,
    /// How many of those are undone.
    pub undone_events: usize,
    /// The Calendars it changed Events in, in the order first changed.
    pub calendar_ids: Vec<CalendarId>,
    /// The first few different titles of its Events, as it left them, or as
    /// they were before it deleted them.
    pub titles: Vec<String>,
    /// Whether its Events can be undone one at a time, as an import's and an
    /// Incoming Sync's; a user action's and an undo's are undone only whole
    /// (ADR 0009).
    pub undone_per_event: bool,
    /// Whether undoing it whole changed nothing, every Event skipped; the
    /// Undo History passes over it.
    pub undo_changed_nothing: bool,
}

/// What a Journal Event Modification did to its Event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum JournalEventModificationKind {
    Created,
    Edited,
    Deleted,
}

/// A Journal Event Modification as a Journal Transaction in the Change
/// Journal list, or an Event's History, shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ListedJournalEventModification {
    pub journal_transaction: JournalTransactionId,
    pub origin: TransactionOrigin,
    /// When its Journal Transaction was made, in Unix seconds.
    pub made_at: i64,
    /// The Event's title as the change left it, or as it was before the
    /// change deleted it.
    pub title: String,
    /// The Event's Calendar, likewise.
    pub calendar_id: CalendarId,
    pub kind: JournalEventModificationKind,
    pub undone: bool,
    /// What its Undo undoes, none once it is undone: the Event, with the
    /// Event Rows of its UID in the Journal Transaction, where the Journal
    /// Transaction is undone one Event at a time, else the whole Journal
    /// Transaction.
    pub undo: Option<UndoTarget>,
}

/// What Ctrl+Z and Ctrl+Shift+Z undo next, from the Undo History: the
/// user's own changes, newest first, and the undos made since their last
/// change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct UndoHistory {
    /// What Ctrl+Z undoes: the newest Journal Transaction that is a user
    /// action, an import or a redo, isn't wholly undone, and whose undo
    /// didn't change nothing.
    pub undo: Option<UndoHistoryStep>,
    /// What Ctrl+Shift+Z redoes, by undoing it: the newest undo made since
    /// the user's last change that isn't a redo and isn't undone.
    pub redo: Option<UndoHistoryStep>,
}

/// One step of the Undo History.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct UndoHistoryStep {
    /// The Journal Transaction to undo whole.
    pub journal_transaction: JournalTransactionId,
    /// The user action or import the step undoes or redoes, through the
    /// undos between, which a toast names; none for an undo of a Calendar
    /// since a given time, or of an Incoming Sync.
    pub user_change: Option<ListedJournalTransaction>,
}

/// The Undo History, worked out from `listed`, the Change Journal list.
pub(crate) fn undo_history(listed: &[ListedJournalTransaction]) -> UndoHistory {
    let by_id: HashMap<_, _> = listed
        .iter()
        .map(|journal_transaction| (journal_transaction.id, journal_transaction))
        .collect();
    let open = |journal_transaction: &ListedJournalTransaction| {
        journal_transaction.undone_events < journal_transaction.events
            && !journal_transaction.undo_changed_nothing
    };
    let step = |journal_transaction: &ListedJournalTransaction| UndoHistoryStep {
        journal_transaction: journal_transaction.id,
        user_change: user_change(journal_transaction, &by_id).cloned(),
    };
    let undo = listed
        .iter()
        .find(|journal_transaction| {
            open(journal_transaction)
                && (is_user_change(journal_transaction) || is_redo(journal_transaction, &by_id))
        })
        .map(step);
    let redo = listed
        .iter()
        .take_while(|journal_transaction| !is_user_change(journal_transaction))
        .find(|journal_transaction| {
            journal_transaction.origin == TransactionOrigin::Undo
                && open(journal_transaction)
                && !is_redo(journal_transaction, &by_id)
        })
        .map(step);
    UndoHistory { undo, redo }
}

/// Whether a Journal Transaction is one of the user's own changes, as
/// Ctrl+Z undoes them: a user action or an import.
fn is_user_change(journal_transaction: &ListedJournalTransaction) -> bool {
    matches!(
        journal_transaction.origin,
        TransactionOrigin::UserAction { .. } | TransactionOrigin::Import
    )
}

/// The undos from `journal_transaction` back through the Journal
/// Transactions each was asked to undo, itself included: none if it isn't
/// an undo.
fn undo_chain<'a>(
    journal_transaction: &'a ListedJournalTransaction,
    by_id: &HashMap<JournalTransactionId, &'a ListedJournalTransaction>,
) -> impl Iterator<Item = &'a ListedJournalTransaction> {
    std::iter::successors(Some(journal_transaction), |undo| {
        undo.undoes.and_then(|undoes| by_id.get(&undoes).copied())
    })
    .take_while(|undo| undo.origin == TransactionOrigin::Undo)
}

/// Whether an undo is a redo: it undid an undo that wasn't a redo itself.
fn is_redo(
    journal_transaction: &ListedJournalTransaction,
    by_id: &HashMap<JournalTransactionId, &ListedJournalTransaction>,
) -> bool {
    undo_chain(journal_transaction, by_id)
        .count()
        .is_multiple_of(2)
        && journal_transaction.origin == TransactionOrigin::Undo
}

/// The user action or import that `journal_transaction` is, or undoes or
/// redoes through the undos between.
fn user_change<'a>(
    journal_transaction: &'a ListedJournalTransaction,
    by_id: &HashMap<JournalTransactionId, &'a ListedJournalTransaction>,
) -> Option<&'a ListedJournalTransaction> {
    let first = undo_chain(journal_transaction, by_id)
        .last()
        .map_or(Some(journal_transaction.id), |undo| undo.undoes)?;
    by_id
        .get(&first)
        .copied()
        .filter(|first| is_user_change(first))
}

/// The Change Journal list: every Journal Transaction, newest first.
pub(crate) fn change_journal(store: &Store) -> Result<Vec<ListedJournalTransaction>> {
    Ok(store
        .journal_transactions()?
        .into_iter()
        .map(listed_journal_transaction)
        .collect())
}

fn listed_journal_transaction(stored: StoredJournalTransaction) -> ListedJournalTransaction {
    let mut calendar_ids = Vec::new();
    let mut titles: Vec<String> = Vec::new();
    for modification in &stored.modifications {
        for (calendar, _) in modification.previous.iter().chain(&modification.made) {
            if !calendar_ids.contains(calendar) {
                calendar_ids.push(*calendar);
            }
        }
        let title = modification
            .made
            .as_ref()
            .or(modification.previous.as_ref())
            .and_then(|(_, title)| title.clone())
            .unwrap_or_default();
        if titles.len() < TITLES_NAMED && !titles.contains(&title) {
            titles.push(title);
        }
    }
    ListedJournalTransaction {
        id: stored.id,
        origin: stored.origin,
        made_at: stored.made_at,
        undoes: stored.undoes,
        events: stored.modifications.len(),
        undone_events: stored
            .modifications
            .iter()
            .filter(|modification| modification.undone)
            .count(),
        calendar_ids,
        titles,
        undone_per_event: undone_per_event(stored.origin),
        undo_changed_nothing: stored.undo_changed_nothing,
    }
}

/// The Events of the Journal Transaction `id`, in the order they were
/// changed.
pub(crate) fn journal_transaction_events(
    store: &Store,
    id: JournalTransactionId,
) -> Result<Vec<ListedJournalEventModification>> {
    Ok(store
        .journal_event_modifications(id)?
        .into_iter()
        .filter_map(listed_journal_event_modification)
        .collect())
}

/// The History of the Event `id`: the Journal Event Modifications of the
/// Events with its UID, a Recurring Event with its Exceptions, newest first.
pub(crate) fn event_history(
    store: &Store,
    id: EventId,
) -> Result<Vec<ListedJournalEventModification>> {
    let (uid, _) = store.event_and_uid(id)?.ok_or(CoreError::EventNotFound)?;
    Ok(store
        .uid_journal_event_modifications(&uid)?
        .into_iter()
        .rev()
        .filter_map(listed_journal_event_modification)
        .collect())
}

/// How the list shows `modification`; none for one that neither found nor
/// made an Event Version, which FreeCal never records.
fn listed_journal_event_modification(
    modification: JournalEventModification,
) -> Option<ListedJournalEventModification> {
    let (kind, calendar_id, title) = match (&modification.previous, modification.made) {
        (None, Some((calendar, title))) => (JournalEventModificationKind::Created, calendar, title),
        (Some(_), Some((calendar, title))) => {
            (JournalEventModificationKind::Edited, calendar, title)
        }
        (Some(previous), None) => (
            JournalEventModificationKind::Deleted,
            previous.calendar_id,
            previous.fields.title.clone(),
        ),
        (None, None) => return None,
    };
    let undo = (!modification.undone).then(|| {
        if undone_per_event(modification.origin) {
            UndoTarget::Event {
                journal_transaction: modification.journal_transaction,
                combined_event_key: CombinedEventKey::without_recurrence_id(
                    calendar_id,
                    &modification.uid,
                ),
            }
        } else {
            UndoTarget::JournalTransaction {
                id: modification.journal_transaction,
            }
        }
    });
    Some(ListedJournalEventModification {
        journal_transaction: modification.journal_transaction,
        origin: modification.origin,
        made_at: modification.made_at,
        title,
        calendar_id,
        kind,
        undone: modification.undone,
        undo,
    })
}
