//! Undo per Event (ADR 0009). An undo picks Journal Event Modifications,
//! a Journal Transaction whole, one Event of an import's or an Incoming
//! Sync's, or a Calendar's since a given time, and undoes them newest first, each Event back to its previous
//! Event Version. An Event that changed since gets its later Journal Event
//! Modifications undone first, and these bring the rest of their own
//! Journal Transaction with them as picking it would. Events that can't be
//! undone are skipped with the reason, and the rest is undone.
//!
//! Before writing, the undo is planned: what it undoes, what it skips, and
//! whether it is a Dangerous Undo. A Dangerous Undo runs only once the user
//! approved exactly that danger.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use super::{
    JournalEventModification, JournalTransaction, JournalTransactionId, Recorded,
    TransactionOrigin, WRITE_PERMISSION, hash, record,
};
use crate::store::{Store, StoredEvent};
use crate::{CalendarId, CoreError, EventId, Result, mass_change_guard};

/// What an undo picks (new term: Undo Target).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum UndoTarget {
    /// A Journal Transaction whole, or what of it isn't undone yet.
    JournalTransaction { id: JournalTransactionId },
    /// One Event of an import's or an Incoming Sync's Journal Transaction,
    /// with the Event Rows of its UID that the Journal Transaction changed
    /// too: a Recurring Event with its Exceptions.
    #[serde(rename_all = "camelCase")]
    Event {
        journal_transaction: JournalTransactionId,
        event_id: EventId,
    },
    /// Every change made at `since` (Unix seconds) or later to an Event of
    /// the Calendar `calendar_id`, or that put one into it, each with what
    /// undoing it brings along: the rest of a user action's or an undo's
    /// Journal Transaction, the Event Rows of its UID in an import's or an
    /// Incoming Sync's.
    #[serde(rename_all = "camelCase")]
    CalendarSince { calendar_id: CalendarId, since: i64 },
}

impl UndoTarget {
    /// The Journal Transaction it picks from, none for a Calendar since a
    /// given time.
    fn journal_transaction(self) -> Option<JournalTransactionId> {
        match self {
            UndoTarget::JournalTransaction { id } => Some(id),
            UndoTarget::Event {
                journal_transaction,
                ..
            } => Some(journal_transaction),
            UndoTarget::CalendarSince { .. } => None,
        }
    }
}

/// Why an undo is a Dangerous Undo, which runs only once the user approved
/// it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DangerousUndo {
    /// The later Journal Transactions it also undoes, oldest first.
    pub later_journal_transactions: Vec<LaterJournalTransaction>,
    /// How many of the Events it undoes have a change from a server among
    /// the changes undone.
    pub server_changes: usize,
    /// Whether it changes more Events of a Calendar than the Mass-Change
    /// Guard's thresholds allow.
    pub exceeds_mass_change_guard: bool,
    /// How many Events it changes.
    pub events: usize,
}

/// A later Journal Transaction an undo also undoes, part or whole.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct LaterJournalTransaction {
    pub id: JournalTransactionId,
    /// When it was made, in Unix seconds.
    pub made_at: i64,
    /// The titles of the Events it undoes of it.
    pub titles: Vec<String>,
}

/// An Event an undo leaves as it is (new term: Skipped Event).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SkippedEvent {
    pub title: String,
    pub reason: SkipReason,
}

/// Why an undo skips an Event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SkipReason {
    /// Undoing it would write into this Read-only Calendar.
    #[serde(rename_all = "camelCase")]
    ReadOnlyCalendar { calendar_id: CalendarId },
    /// It has guests, so FreeCal doesn't change it.
    HasGuests,
    /// Its current version is in no Journal Event Modification, as only a
    /// program other than FreeCal can make it.
    NotInChangeJournal,
    /// It was changed together with the Event titled `title`, which is
    /// skipped, by a Journal Transaction undone only whole, or as part of
    /// the same Recurring Event.
    ChangedTogetherWith { title: String },
}

/// What an undo did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum UndoOutcome {
    /// It undid all it could, and skipped `skipped`.
    Undone { skipped: Vec<SkippedEvent> },
    /// It is a Dangerous Undo the user hasn't approved, so it changed
    /// nothing. Undoing it again with `danger` approved runs it, unless it
    /// has changed by then.
    Dangerous {
        danger: DangerousUndo,
        skipped: Vec<SkippedEvent>,
    },
}

/// Undoes `target` as a Journal Transaction with the Transaction Origin
/// undo, made at `made_at` (Unix seconds), unless it is a Dangerous Undo
/// other than `approved`. A whole Journal Transaction whose undo changed
/// nothing, every Event skipped, is marked so, and the Undo History passes
/// over it. Refused for a user action's Journal Transaction
/// picked one Event at a time ([`CoreError::UndoneOnlyWhole`]), for a
/// target undone already, and while an Event Version that isn't an
/// Exception would join another with its UID in its Calendar (ticket 104).
pub(crate) fn undo(
    store: &Store,
    target: UndoTarget,
    approved: Option<&DangerousUndo>,
    made_at: i64,
) -> Result<Recorded<UndoOutcome>> {
    record(
        store,
        TransactionOrigin::Undo,
        made_at,
        |journal_transaction| {
            journal_transaction.undoes = target.journal_transaction();
            let plan = Plan::make(journal_transaction, target)?;
            let skipped = plan.skipped_events();
            if let Some(danger) = plan.danger(journal_transaction)?
                && approved != Some(&danger)
            {
                return Ok(UndoOutcome::Dangerous { danger, skipped });
            }
            plan.run(journal_transaction)?;
            if let UndoTarget::JournalTransaction { id } = target
                && plan.undone.is_empty()
            {
                journal_transaction
                    .database_transaction
                    .mark_undo_changed_nothing(WRITE_PERMISSION, id)?;
            }
            Ok(UndoOutcome::Undone { skipped })
        },
    )
}

/// Whether a Journal Transaction of `origin` can be undone one Event at a
/// time: an import's and an Incoming Sync's Events are independent, while
/// a user action's or an undo's depend on each other, as when "this and
/// following" splits a Recurring Event in two (ADR 0009).
pub(super) fn undone_per_event(origin: TransactionOrigin) -> bool {
    matches!(
        origin,
        TransactionOrigin::Import | TransactionOrigin::IncomingSync
    )
}

/// An Event as the Change Journal tracks it: its ID and its UID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct TrackedEvent {
    id: EventId,
    uid: String,
}

impl TrackedEvent {
    fn of(modification: &JournalEventModification) -> Self {
        Self {
            id: modification.event_id,
            uid: modification.uid.clone(),
        }
    }

    /// The hash of the Event Version `modification` found, none if it made
    /// the Event.
    fn previous_hash(&self, modification: &JournalEventModification) -> Option<Vec<u8>> {
        modification
            .previous
            .as_ref()
            .map(|previous| hash(&self.uid, previous))
    }
}

/// What an undo will do.
struct Plan {
    /// The Journal Event Modifications it undoes, by ID, but those of
    /// skipped Events.
    undone: BTreeMap<i64, JournalEventModification>,
    /// The IDs of those the user picked; the others are later ones.
    picked: BTreeSet<i64>,
    /// The Events it skips, each with the title it has now and why.
    skipped: BTreeMap<TrackedEvent, SkippedEvent>,
    /// Each Event's current version, before the undo.
    current: HashMap<TrackedEvent, Option<StoredEvent>>,
}

/// Why planning stopped to skip an Event, and start again without it.
struct Skip(TrackedEvent, SkipReason);

impl Plan {
    /// Plans undoing `target`: its Journal Event Modifications not undone
    /// yet, with every later one that an Event's current version needs
    /// undone first, and the rest of those later ones' Journal Transactions
    /// as [`undone_per_event`] says. An Event that can't be undone is
    /// skipped, and planning starts again without it, so that it brings in
    /// no later Journal Event Modifications.
    fn make(journal_transaction: &JournalTransaction, target: UndoTarget) -> Result<Self> {
        let picked = picked(journal_transaction, target)?;
        let mut planner = Planner {
            journal_transaction,
            histories: HashMap::new(),
            current: HashMap::new(),
        };
        let mut skipped = BTreeMap::new();
        loop {
            match planner.close(&picked, &skipped)? {
                Ok(mut undone) => {
                    undone.retain(|_, modification| {
                        !skipped.contains_key(&TrackedEvent::of(modification))
                    });
                    let skipped = skipped
                        .into_iter()
                        .map(|(event, reason)| {
                            let title = planner.title(&event);
                            (event, SkippedEvent { title, reason })
                        })
                        .collect();
                    return Ok(Plan {
                        undone,
                        picked: picked
                            .into_iter()
                            .map(|modification| modification.id)
                            .collect(),
                        skipped,
                        current: planner.current,
                    });
                }
                Err(Skip(event, reason)) => {
                    skipped.insert(event, reason);
                }
            }
        }
    }

    fn skipped_events(&self) -> Vec<SkippedEvent> {
        self.skipped.values().cloned().collect()
    }

    /// Why the undo is a Dangerous Undo, none when it is benign: it also
    /// undoes later Journal Transactions, reverts changes from a server, or
    /// changes more Events of a Calendar than the Mass-Change Guard allows.
    fn danger(&self, journal_transaction: &JournalTransaction) -> Result<Option<DangerousUndo>> {
        let mut later: BTreeMap<JournalTransactionId, LaterJournalTransaction> = BTreeMap::new();
        let mut events: BTreeMap<TrackedEvent, BTreeSet<CalendarId>> = BTreeMap::new();
        let mut from_server = BTreeSet::new();
        for modification in self.undone.values() {
            let event = TrackedEvent::of(modification);
            let calendars = events.entry(event.clone()).or_default();
            calendars.extend(
                modification
                    .previous
                    .iter()
                    .map(|previous| previous.calendar_id),
            );
            calendars.extend(
                self.current[&event]
                    .iter()
                    .map(|current| current.calendar_id),
            );
            if modification.origin == TransactionOrigin::IncomingSync {
                from_server.insert(event.clone());
            }
            if !self.picked.contains(&modification.id) {
                let title = self.title(&event);
                let entry = later
                    .entry(modification.journal_transaction)
                    .or_insert_with(|| LaterJournalTransaction {
                        id: modification.journal_transaction,
                        made_at: modification.made_at,
                        titles: Vec::new(),
                    });
                if !entry.titles.contains(&title) {
                    entry.titles.push(title);
                }
            }
        }
        let mut per_calendar: BTreeMap<CalendarId, usize> = BTreeMap::new();
        for calendar in events.values().flatten() {
            *per_calendar.entry(*calendar).or_default() += 1;
        }
        let mut exceeds_mass_change_guard = false;
        for (calendar, changed) in per_calendar {
            let calendar_events = journal_transaction
                .database_transaction
                .calendar_event_row_count(calendar)?;
            exceeds_mass_change_guard |= mass_change_guard::exceeded(changed, calendar_events);
        }
        let danger = DangerousUndo {
            later_journal_transactions: later.into_values().collect(),
            server_changes: from_server.len(),
            exceeds_mass_change_guard,
            events: events.len(),
        };
        let dangerous = !danger.later_journal_transactions.is_empty()
            || danger.server_changes > 0
            || danger.exceeds_mass_change_guard;
        Ok(dangerous.then_some(danger))
    }

    /// The title of the Event `event`: the one it has now, else the one it
    /// had before the latest change this undo undoes.
    fn title(&self, event: &TrackedEvent) -> String {
        title(
            &self.current[event],
            self.undone
                .values()
                .filter(|modification| &TrackedEvent::of(modification) == event),
        )
    }

    /// Undoes the planned Journal Event Modifications newest first, so that
    /// each finds the Event Version it made.
    fn run(&self, journal_transaction: &mut JournalTransaction) -> Result<()> {
        for modification in self.undone.values().rev() {
            let current = journal_transaction
                .database_transaction
                .current_event(modification.event_id, &modification.uid)?;
            // The plan read the same Database Transaction, so this holds
            // unless the plan is wrong; it then refuses rather than overwrite.
            if current.map(|event| hash(&modification.uid, &event)) != modification.hash {
                return Err(CoreError::ChangedSinceJournalTransaction(
                    modification.journal_transaction,
                ));
            }
            match &modification.previous {
                None => journal_transaction.delete(modification.event_id)?,
                Some(previous) => journal_transaction.put_back(
                    modification.journal_transaction,
                    modification.event_id,
                    &modification.uid,
                    previous,
                )?,
            }
            journal_transaction
                .database_transaction
                .mark_undone(WRITE_PERMISSION, modification.id)?;
        }
        Ok(())
    }
}

/// The Journal Event Modifications `target` picks that aren't undone yet.
fn picked(
    journal_transaction: &JournalTransaction,
    target: UndoTarget,
) -> Result<Vec<JournalEventModification>> {
    let (id, modifications) = match target {
        UndoTarget::JournalTransaction { id } => (
            id,
            journal_transaction
                .database_transaction
                .journal_event_modifications(id)?,
        ),
        UndoTarget::Event {
            journal_transaction: id,
            event_id,
        } => {
            let modifications = journal_transaction
                .database_transaction
                .journal_event_modifications(id)?;
            if !undone_per_event(modifications[0].origin) {
                return Err(CoreError::UndoneOnlyWhole(id));
            }
            let uid = modifications
                .iter()
                .find(|modification| modification.event_id == event_id)
                .ok_or(CoreError::EventNotFound(event_id))?
                .uid
                .clone();
            let of_uid = modifications
                .into_iter()
                .filter(|modification| modification.uid == uid)
                .collect();
            (id, of_uid)
        }
        UndoTarget::CalendarSince { calendar_id, since } => {
            let mut picked = BTreeMap::new();
            for modification in journal_transaction
                .database_transaction
                .calendar_journal_event_modifications(calendar_id, since)?
            {
                if modification.undone || picked.contains_key(&modification.id) {
                    continue;
                }
                for with in brought_along(journal_transaction, &modification)? {
                    picked.entry(with.id).or_insert(with);
                }
            }
            return Ok(picked.into_values().collect());
        }
    };
    let picked: Vec<_> = modifications
        .into_iter()
        .filter(|modification| !modification.undone)
        .collect();
    if picked.is_empty() {
        return Err(CoreError::AlreadyUndone(id));
    }
    Ok(picked)
}

/// The Journal Event Modifications that undoing `modification` brings
/// along, itself included: those of its Journal Transaction not undone yet,
/// of its UID only where [`undone_per_event`].
fn brought_along(
    journal_transaction: &JournalTransaction,
    modification: &JournalEventModification,
) -> Result<Vec<JournalEventModification>> {
    Ok(journal_transaction
        .database_transaction
        .journal_event_modifications(modification.journal_transaction)?
        .into_iter()
        .filter(|with| {
            !with.undone && (!undone_per_event(modification.origin) || with.uid == modification.uid)
        })
        .collect())
}

/// The later Journal Event Modifications of `event`, newest first and all
/// before `before`, that lead back from its version `version` to the one
/// `to` made, each made the version the one before it found; none if no
/// such path exists. It tries every one that made `version`, as an undo
/// and the change it undid can both have made it.
fn path_back(
    event: &TrackedEvent,
    history: &[JournalEventModification],
    version: Option<Vec<u8>>,
    to: &JournalEventModification,
    before: i64,
) -> Option<Vec<JournalEventModification>> {
    if version == to.hash {
        return Some(Vec::new());
    }
    history
        .iter()
        .rev()
        .filter(|later| {
            !later.undone && later.id > to.id && later.id < before && later.hash == version
        })
        .find_map(|later| {
            let mut path = path_back(event, history, event.previous_hash(later), to, later.id)?;
            path.insert(0, later.clone());
            Some(path)
        })
}

/// The title of an Event whose version now is `current`, else the one it
/// had before the latest of `modifications`.
fn title<'a>(
    current: &Option<StoredEvent>,
    modifications: impl DoubleEndedIterator<Item = &'a JournalEventModification>,
) -> String {
    current
        .as_ref()
        .or_else(|| {
            modifications
                .rev()
                .find_map(|modification| modification.previous.as_ref())
        })
        .map(|event| event.fields.title.clone())
        .unwrap_or_default()
}

/// Reads what planning needs, each Event's Journal Event Modifications and
/// current version once.
struct Planner<'a, 'b> {
    journal_transaction: &'a JournalTransaction<'b>,
    histories: HashMap<TrackedEvent, Vec<JournalEventModification>>,
    current: HashMap<TrackedEvent, Option<StoredEvent>>,
}

impl Planner<'_, '_> {
    /// Closes `picked` over what undoing it needs first, leaving the Events
    /// in `skipped` alone: for each Event, newest first, every Journal Event
    /// Modification later than one to undo that the Event's current version
    /// needs undone to reach it, with the rest of its Journal Transaction as
    /// picking it would. Stops at the first Event that can't be undone.
    fn close(
        &mut self,
        picked: &[JournalEventModification],
        skipped: &BTreeMap<TrackedEvent, SkipReason>,
    ) -> Result<std::result::Result<BTreeMap<i64, JournalEventModification>, Skip>> {
        let mut undone: BTreeMap<i64, JournalEventModification> = picked
            .iter()
            .map(|modification| (modification.id, modification.clone()))
            .collect();
        loop {
            let mut events: BTreeMap<TrackedEvent, Vec<JournalEventModification>> = BTreeMap::new();
            for modification in undone.values() {
                events
                    .entry(TrackedEvent::of(modification))
                    .or_default()
                    .push(modification.clone());
            }
            let mut later = Vec::new();
            for (event, modifications) in events {
                if skipped.contains_key(&event) {
                    continue;
                }
                match self.walk(&event, &modifications)? {
                    Ok(walked) => later.extend(walked),
                    Err(reason) => return Ok(Err(Skip(event, reason))),
                }
            }
            if later.is_empty() {
                return Ok(match self.changed_together_with_skipped(&undone, skipped) {
                    Some(skip) => Err(skip),
                    None => Ok(undone),
                });
            }
            for modification in later {
                for with in brought_along(self.journal_transaction, &modification)? {
                    undone.entry(with.id).or_insert(with);
                }
            }
        }
    }

    /// Walks the Event `event` back from its current version through
    /// `modifications`, its Journal Event Modifications to undo in the order
    /// they were made: the later ones not undone yet that it needs undone
    /// first, newest first, or why it can't be undone, either because no
    /// Journal Event Modification made its current version or because a
    /// version on the way is in a Calendar the undo may not write.
    fn walk(
        &mut self,
        event: &TrackedEvent,
        modifications: &[JournalEventModification],
    ) -> Result<std::result::Result<Vec<JournalEventModification>, SkipReason>> {
        let current = self.current(event)?.clone();
        let history = self.history(event)?.clone();
        let mut version = current.as_ref().map(|current| hash(&event.uid, current));
        let mut calendars: Vec<CalendarId> =
            current.iter().map(|current| current.calendar_id).collect();
        let mut before = i64::MAX;
        let mut walked = Vec::new();
        for modification in modifications.iter().rev() {
            let Some(path) = path_back(event, &history, version.clone(), modification, before)
            else {
                return Ok(Err(SkipReason::NotInChangeJournal));
            };
            for later in path.into_iter().chain([modification.clone()]) {
                calendars.extend(later.previous.iter().map(|previous| previous.calendar_id));
                version = event.previous_hash(&later);
                before = later.id;
                if later.id != modification.id {
                    walked.push(later);
                }
            }
        }
        for calendar in calendars {
            match self
                .journal_transaction
                .event_writable(calendar, &event.uid)
            {
                Ok(()) => {}
                Err(CoreError::CalendarReadOnly(calendar_id)) => {
                    return Ok(Err(SkipReason::ReadOnlyCalendar { calendar_id }));
                }
                Err(CoreError::EventHasGuests) => return Ok(Err(SkipReason::HasGuests)),
                Err(error) => return Err(error),
            }
        }
        Ok(Ok(walked))
    }

    /// The first Event in `undone` that must be skipped with one in
    /// `skipped`: changed by the same Journal Transaction undone only whole,
    /// or of the same UID in it, as a Recurring Event goes with its
    /// Exceptions.
    fn changed_together_with_skipped(
        &self,
        undone: &BTreeMap<i64, JournalEventModification>,
        skipped: &BTreeMap<TrackedEvent, SkipReason>,
    ) -> Option<Skip> {
        undone
            .values()
            .filter(|with| skipped.contains_key(&TrackedEvent::of(with)))
            .find_map(|with| {
                undone
                    .values()
                    .find(|modification| {
                        modification.journal_transaction == with.journal_transaction
                            && (!undone_per_event(modification.origin)
                                || modification.uid == with.uid)
                            && !skipped.contains_key(&TrackedEvent::of(modification))
                    })
                    .map(|modification| {
                        let title = self.title(&TrackedEvent::of(with));
                        Skip(
                            TrackedEvent::of(modification),
                            SkipReason::ChangedTogetherWith { title },
                        )
                    })
            })
    }

    fn current(&mut self, event: &TrackedEvent) -> Result<&Option<StoredEvent>> {
        if !self.current.contains_key(event) {
            let current = self
                .journal_transaction
                .database_transaction
                .current_event(event.id, &event.uid)?;
            self.current.insert(event.clone(), current);
        }
        Ok(&self.current[event])
    }

    fn history(&mut self, event: &TrackedEvent) -> Result<&Vec<JournalEventModification>> {
        if !self.histories.contains_key(event) {
            let history = self
                .journal_transaction
                .database_transaction
                .event_journal_event_modifications(event.id, &event.uid)?;
            self.histories.insert(event.clone(), history);
        }
        Ok(&self.histories[event])
    }

    /// The title of the skipped Event `event`, whose current version and
    /// history [`Planner::walk`] read.
    fn title(&self, event: &TrackedEvent) -> String {
        title(&self.current[event], self.histories[event].iter())
    }
}
