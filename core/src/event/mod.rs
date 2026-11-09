//! Events as the core loads and changes them. An Event is loaded as one
//! value, [`Loaded`], with the other Events of its Calendar that share its
//! UID: a Recurring Event comes with its rule and its Exceptions. The
//! operations that need them all live here, so that only this module pairs
//! Exceptions with their Occurrences, comparing recurrence IDs by Occurrence
//! against the Recurring Event (ADR 0012).
//! Every change the user makes to an Event goes through it: one that
//! doesn't repeat is changed as one Occurrence alone would be (`scope.rs`).
//!
//! Loading from the Local Store serves reads; changes load from a Journal
//! Transaction and write through it, never the Store (ADR 0009).

mod recurrence_id;
pub(crate) mod rule;
mod scope;
mod when;

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::journal::JournalTransaction;
use crate::store::{KeptUnsentChange, RangeEvent, Store, StoredEvent, Uid, UidEvent};
use crate::sync::SendFailure;
use crate::{CalendarId, CoreError, ImportedEvent, ImportedWhen, Result, StructuredText};

pub(crate) use recurrence_id::{NormalForm, RecurrenceId, in_normal_form};
pub use rule::{Frequency, Repeat, RepeatEnd};
pub(crate) use scope::{Edit, edit_event, edit_occurrence, exception_counts};
pub use scope::{ExceptionCounts, Scope};
pub(crate) use when::{EventFields, StoredRange, StoredWhen, instant};
pub use when::{EventId, ServerObject, When};

/// What the user gives an Event when creating or editing it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct EventDraft {
    pub calendar_id: CalendarId,
    pub title: String,
    pub when: When,
    pub location: String,
    /// The plain-text description. When it equals the plain-text form of an
    /// Event's description, the original is kept byte for byte.
    pub description: String,
    /// The repeat rule, none for an Event that doesn't repeat. When it
    /// equals an Event's `repeat`, its stored rule is kept as it is.
    pub repeat: Option<Repeat>,
}

/// An Event with everything its details and editor show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: EventId,
    pub calendar_id: CalendarId,
    pub title: String,
    pub when: When,
    pub location: StructuredText,
    pub description: StructuredText,
    /// The zone FreeCal gave the Event when it arrived with a time of day but
    /// no time zone (ADR 0007), which its details mention.
    pub pinned_zone: Option<String>,
    /// A Recurring Event's repeat rule as the editor shows it, for an
    /// Exception its Recurring Event's; none for an Event that doesn't
    /// repeat, and for a rule the editor can't show, such as an hourly one.
    pub repeat: Option<Repeat>,
    /// Whether the repeat rule has more than `repeat` shows, such as the
    /// weekdays of a weekly rule. Those are kept while the frequency stays.
    pub custom_repeat: bool,
    /// For an Exception, the recurrence ID of the Occurrence of its Recurring
    /// Event that it changed, in the Normal Form, or as stored when it isn't
    /// one; none for every other Event.
    pub recurrence_id: Option<String>,
    /// Whether it has guests, attendees other than the user, which makes
    /// it read-only (spec: "Attendees"). For an Exception, whether its
    /// Recurring Event has, as they are one Event on the server.
    pub has_guests: bool,
    /// Its Unsent Change, for "Not yet synced: <reason>" (spec: "Event
    /// details"); none when it has none.
    pub unsent_change: Option<UnsentChange>,
}

/// The Unsent Change of an Event as the window gets it: since when it
/// waits, and why the Sender's last try to send it failed. A change made
/// after that try, or a Send Failure that can't be read, has none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct UnsentChange {
    /// An instant, in RFC 3339 UTC.
    #[cfg_attr(test, ts(type = "string"))]
    pub since: DateTime<Utc>,
    pub send_failure: Option<SendFailure>,
}

impl UnsentChange {
    /// The Unsent Change of the Event Row whose Occurrence is `key`: none
    /// for an Event Row that isn't an Exception, otherwise its recurrence
    /// ID in the Normal Form. It is found among `kept`, the Unsent Changes
    /// of its Calendar and UID, comparing by Occurrence against the
    /// Recurring Event at `recurring` (ADR 0012). A `since` that can't be
    /// read counts as the Unix epoch, so that the change still shows, as
    /// one that waited long.
    fn of_event_row(
        kept: &[KeptUnsentChange],
        key: Option<&NormalForm>,
        recurring: Option<&ImportedWhen>,
    ) -> Option<Self> {
        let found = kept.iter().find(|kept| {
            occurrence_key(kept.recurrence_id.as_ref(), recurring)
                .is_some_and(|kept_key| kept_key.as_ref() == key)
        })?;
        Some(Self {
            since: found
                .since
                .and_then(|since| DateTime::from_timestamp(since, 0))
                .unwrap_or(DateTime::UNIX_EPOCH),
            send_failure: found
                .send_failure
                .as_deref()
                .and_then(SendFailure::from_stored),
        })
    }
}

/// One dated appearance of an Event in the grid. A single Event has exactly
/// one; Recurring Events get one per date they fall on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Occurrence {
    /// The Event it belongs to: its Recurring Event, or the Exception that
    /// changed it.
    pub event_id: EventId,
    pub calendar_id: CalendarId,
    pub title: String,
    pub when: When,
    /// Which Occurrence of a Recurring Event this is: its original start in
    /// the Normal Form (ADR 0012), as text that only the core reads. The
    /// commands that take it find the Occurrence's Exception by comparing
    /// by Occurrence. None for a single Event, and for an
    /// Exception that is an Event of its own: without its Recurring Event,
    /// or changing none of its Occurrences.
    pub recurrence_id: Option<String>,
    /// Whether its Event has guests, which makes it read-only (spec:
    /// "Attendees"), as [`Event::has_guests`] says.
    pub has_guests: bool,
    /// Since when its Event has an Unsent Change, an instant in RFC 3339
    /// UTC, for the dashed outline (spec: "Event chips stay sparse"); none
    /// when it has none. It is its Event's [`Event::unsent_change`]: an
    /// Occurrence that an Exception changed has the Exception's, any other
    /// Occurrence of a Recurring Event the Recurring Event's.
    #[cfg_attr(test, ts(type = "string | null"))]
    pub unsent_since: Option<DateTime<Utc>>,
}

impl Occurrence {
    /// Orders Occurrences, each given with its derived UTC start, by the
    /// date they start on in the Display Time Zone and then by when they
    /// happen, which differs from their wall time in the repeated hour.
    fn ordered(mut occurrences: Vec<(i64, Occurrence)>) -> Vec<Occurrence> {
        occurrences.sort_by(|(a_utc, a), (b_utc, b)| {
            (a.when.start_date(), a_utc, a.event_id.0, &a.recurrence_id).cmp(&(
                b.when.start_date(),
                b_utc,
                b.event_id.0,
                &b.recurrence_id,
            ))
        });
        occurrences
            .into_iter()
            .map(|(_, occurrence)| occurrence)
            .collect()
    }
}

/// Where Events are loaded from: the Local Store for reads, a Journal
/// Transaction for changes.
pub(crate) trait Source {
    fn events_with_uid(&self, uid: Uid) -> Result<Vec<UidEvent>>;
}

impl Source for Store {
    fn events_with_uid(&self, uid: Uid) -> Result<Vec<UidEvent>> {
        Store::events_with_uid(self, uid)
    }
}

/// A stored Event: its ID and fields.
type Stored = (EventId, EventFields);

/// A stored Exception: its ID, the Normal Form of the recurrence ID of the
/// Occurrence it changed, computed once against the Recurring Event it is
/// loaded with (ADR 0012), and its fields, which hold the recurrence ID as
/// obtained. A stored recurrence ID that isn't one gets none, and so pairs
/// with no Occurrence.
struct Exception {
    id: EventId,
    rid: Option<NormalForm>,
    fields: EventFields,
}

/// The Events of a Calendar with one UID, sorted as GLOSSARY has them: the
/// Event that isn't an Exception, a Recurring Event when it has a rule, and
/// the Exceptions. With a Recurring Event they are its own; an Exception
/// whose Recurring Event is gone is an Event of its own.
pub(crate) struct Loaded {
    calendar_id: CalendarId,
    uid: String,
    /// The Event that isn't an Exception, the only one with the UID in its
    /// Calendar, as the Local Store keeps it (ticket 104).
    event: Option<Stored>,
    /// Every Exception with the UID, in the order they were added.
    exceptions: Vec<Exception>,
}

/// An incoming Event with the stored Event it is paired with, if any: its
/// ID and fields.
type Paired<'a, 'b> = (&'b ImportedEvent, Option<(EventId, &'a EventFields)>);

/// Incoming Events paired with the stored Events with their UID
/// ([`Loaded::pair`]).
struct Pairing<'a, 'b> {
    /// Each incoming Event, the latest of several for one Occurrence, with
    /// the stored Event for the same Occurrence.
    paired: Vec<Paired<'a, 'b>>,
    /// The stored Event that isn't an Exception, when no incoming Event
    /// paired with it.
    event_left: Option<EventId>,
    /// The stored Exceptions no incoming Event paired with.
    exceptions_left: Vec<&'a Exception>,
}

/// Which recurrence ID an incoming Event written over a stored one keeps:
/// the form its Server Object holds (ADR 0012). An import keeps the stored
/// one, so that a file giving it in another form leaves no Unsent Change in
/// a form the Server Object doesn't hold; Propagate takes the Server
/// Object's, the incoming one.
#[derive(Clone, Copy)]
enum Kept {
    Stored,
    Incoming,
}

impl Loaded {
    /// The Events of a Calendar with the UID `uid`, as an Incoming Sync or an
    /// import finds them.
    pub fn of_uid(source: &impl Source, calendar_id: CalendarId, uid: &str) -> Result<Self> {
        let events = source.events_with_uid(Uid::In(calendar_id, uid))?;
        Ok(Self::sorted(calendar_id, uid.to_owned(), events))
    }

    /// `events`, those of a Calendar with one UID, sorted.
    fn sorted(calendar_id: CalendarId, uid: String, events: Vec<UidEvent>) -> Self {
        let (exceptions, mut events): (Vec<_>, Vec<_>) = events
            .into_iter()
            .map(|event| (event.id, event.fields))
            .partition(|(_, fields)| fields.recurrence_id.is_some());
        let event = events.pop();
        debug_assert!(events.is_empty(), "one Event that isn't an Exception");
        let mut loaded = Self {
            calendar_id,
            uid,
            event,
            exceptions: Vec::new(),
        };
        let recurring = loaded.recurring_when();
        loaded.exceptions = exceptions
            .into_iter()
            .map(|(id, fields)| Exception {
                id,
                rid: normal_form(&fields, recurring.as_ref()),
                fields,
            })
            .collect();
        loaded
    }

    /// The Normal Form of the recurrence ID `text`, as the window gives an
    /// Occurrence's, against the Recurring Event; none when it isn't one.
    fn rid_of(&self, text: &str) -> Option<NormalForm> {
        RecurrenceId::obtained(text).normal_form(self.recurring_when().as_ref())
    }

    /// Whether the Calendar has an Event with the UID, an Exception without
    /// its Recurring Event too.
    pub fn exists(&self) -> bool {
        self.event.is_some() || !self.exceptions.is_empty()
    }

    /// The Recurring Event, none when the Event doesn't repeat or is gone.
    fn recurring(&self) -> Option<&Stored> {
        self.event
            .as_ref()
            .filter(|(_, fields)| fields.rule.is_some())
    }

    /// The Event that isn't an Exception, of Events known to have one: the
    /// Recurring Event, for the changes only a Recurring Event has, or in a
    /// scope the Event its changed Occurrences became, which may not repeat
    /// any more.
    fn recurring_event(&self) -> &Stored {
        self.event
            .as_ref()
            .expect("Events loaded with the Event that isn't an Exception")
    }

    /// Makes `incoming`, a new version from a file with its Exceptions, the
    /// Events with the UID, adding it if the Calendar has none. Exceptions
    /// are paired by Occurrence, against `incoming`'s Recurring Event
    /// ([`Loaded::pair`]): those `incoming` has replace them, keeping their
    /// recurrence IDs as stored, or are added. The others are deleted, but
    /// for Events of their own ([`Loaded::of_its_own`]), which stay and join
    /// `incoming` without being written, as their recurrence IDs stay as
    /// obtained (ADR 0012).
    pub fn take(
        &self,
        journal_transaction: &mut JournalTransaction,
        incoming: &ImportedEvent,
    ) -> Result<()> {
        let recurring = incoming.rule.as_ref().map(|_| &incoming.when);
        let pairing = self.pair(
            std::iter::once(incoming).chain(&incoming.exceptions),
            recurring,
        );
        self.write_paired(journal_transaction, &pairing.paired, Kept::Stored)?;
        for stored in pairing.exceptions_left {
            if !self.of_its_own(stored) {
                journal_transaction.delete(stored.id)?;
            }
        }
        Ok(())
    }

    /// Makes `incoming`, an Exception from a file without its Recurring
    /// Event, one of the Events with the UID: an Exception of their
    /// Recurring Event if they have one, otherwise an Event of its own
    /// (ticket 88). It replaces the Exception for the same Occurrence
    /// ([`Loaded::pair`]), keeping its recurrence ID as stored, or is added,
    /// and the others stay.
    pub fn take_exception(
        &self,
        journal_transaction: &mut JournalTransaction,
        incoming: &ImportedEvent,
    ) -> Result<()> {
        let pairing = self.pair([incoming], self.recurring_when().as_ref());
        self.write_paired(journal_transaction, &pairing.paired, Kept::Stored)
    }

    /// Pairs `incoming`, new versions of Events with the UID, with the
    /// stored ones by Occurrence, against the Recurring Event at `recurring`
    /// (ADR 0012): an Event that isn't an Exception with the stored one, an
    /// Exception with the stored Exception for the same Occurrence, the
    /// first by ID of several. Of several incoming for one Occurrence, the
    /// latest is kept ([`latest`]); a recurrence ID that isn't one pairs with
    /// nothing. Each Normal Form is computed once.
    fn pair<'a, 'b>(
        &'a self,
        incoming: impl IntoIterator<Item = &'b ImportedEvent>,
        recurring: Option<&ImportedWhen>,
    ) -> Pairing<'a, 'b> {
        let mut event = self.event.as_ref();
        let mut exceptions: Vec<_> = self
            .exceptions
            .iter()
            .map(|stored| (normal_form(&stored.fields, recurring), stored))
            .collect();
        let incoming = incoming.into_iter().map(|incoming| {
            (
                occurrence_key(incoming.recurrence_id.as_ref(), recurring),
                incoming,
            )
        });
        let paired = latest(incoming)
            .into_iter()
            .map(|(occurrence, incoming)| {
                let stored = match occurrence {
                    Some(None) => event.take().map(|(id, fields)| (*id, fields)),
                    Some(Some(rid)) => exceptions
                        .iter()
                        .position(|(stored, _)| stored.as_ref() == Some(&rid))
                        .map(|index| exceptions.remove(index).1)
                        .map(|stored| (stored.id, &stored.fields)),
                    None => None,
                };
                (incoming, stored)
            })
            .collect();
        Pairing {
            paired,
            event_left: event.map(|(id, _)| *id),
            exceptions_left: exceptions.into_iter().map(|(_, stored)| stored).collect(),
        }
    }

    /// Writes each of `paired`'s incoming Events over the stored Event it
    /// is paired with, keeping the recurrence ID `kept` (ADR 0012), or adds
    /// it. ADR 0007's rules 2 and 3 apply.
    fn write_paired(
        &self,
        journal_transaction: &mut JournalTransaction,
        paired: &[Paired<'_, '_>],
        kept: Kept,
    ) -> Result<()> {
        let event = |fields| StoredEvent {
            calendar_id: self.calendar_id,
            fields,
        };
        for (incoming, stored) in paired {
            match stored {
                Some((id, stored)) => {
                    let rid = match kept {
                        Kept::Stored => stored.recurrence_id.clone(),
                        Kept::Incoming => incoming.recurrence_id.clone(),
                    };
                    journal_transaction.replace(*id, &event(fields_over(incoming, stored, rid)))?
                }
                None => _ = journal_transaction.create(&self.uid, &event(fields(incoming)))?,
            }
        }
        Ok(())
    }

    /// The Recurring Event's time, none when the Event doesn't repeat, is
    /// gone, or its time can't be read.
    pub fn recurring_when(&self) -> Option<ImportedWhen> {
        let (_, fields) = self.recurring()?;
        fields.when.to_imported().ok()
    }

    /// Writes `fields` as an Exception of `into`, these Events or those they
    /// became, in place of `previous`, one of their stored Exceptions, if
    /// given. Returns its ID.
    fn write_exception(
        &self,
        journal_transaction: &mut JournalTransaction,
        previous: Option<EventId>,
        into: &Loaded,
        fields: EventFields,
    ) -> Result<EventId> {
        let event = StoredEvent {
            calendar_id: into.calendar_id,
            fields,
        };
        match previous {
            Some(id) if into.uid == self.uid => {
                journal_transaction.replace(id, &event)?;
                Ok(id)
            }
            Some(id) => {
                journal_transaction.delete(id)?;
                journal_transaction.create(&into.uid, &event)
            }
            None => journal_transaction.create(&into.uid, &event),
        }
    }

    /// Whether `exception`, one of these Exceptions, is an Event of its own:
    /// without their Recurring Event, or changing none of its Occurrences,
    /// such as one for a cancelled Occurrence (ticket 88).
    fn of_its_own(&self, exception: &Exception) -> bool {
        !matches!(self.slot_of(exception), Ok(Some(_)))
    }

    /// The Recurring Event, with the time its rule gives the Occurrence that
    /// `exception`, one of these Exceptions, changed; none when it is an
    /// Event of its own ([`Loaded::of_its_own`]).
    fn slot_of(&self, exception: &Exception) -> Result<Option<(&EventFields, StoredWhen)>> {
        let (Some((_, recurring)), Some(rid)) = (self.recurring(), &exception.rid) else {
            return Ok(None);
        };
        Ok(rule::occurrence_at(recurring, rid.as_str())?.map(|slot| (recurring, slot)))
    }

    /// The Exception that changed the Occurrence `rid`, in the Normal Form,
    /// if there is one: of several, the first by ID.
    fn exception_at(&self, rid: &NormalForm) -> Option<&Exception> {
        self.exceptions
            .iter()
            .find(|exception| exception.rid.as_ref() == Some(rid))
    }

    /// Cancels the Occurrence `rid`, in the Normal Form, of the Recurring Event,
    /// which it must have: the Occurrence becomes a cancelled Exception, and
    /// an Exception that changed it is deleted.
    fn cancel(&self, journal_transaction: &mut JournalTransaction, rid: &NormalForm) -> Result<()> {
        let (id, fields) = self.recurring_event();
        for exception in &self.exceptions {
            if exception.rid.as_ref() == Some(rid) {
                journal_transaction.delete(exception.id)?;
            }
        }
        journal_transaction.replace(
            *id,
            &StoredEvent {
                calendar_id: self.calendar_id,
                fields: EventFields {
                    exdates: rule::with_exdate(&fields.exdates, rid.as_str()),
                    ..fields.clone()
                },
            },
        )
    }

    /// Deletes the Events with the UID, the Event and every Exception.
    pub fn delete_all(&self, journal_transaction: &mut JournalTransaction) -> Result<()> {
        let exceptions = self.exceptions.iter().map(|exception| exception.id);
        for id in self.event.iter().map(|(id, _)| *id).chain(exceptions) {
            journal_transaction.delete(id)?;
        }
        Ok(())
    }

    /// The UID these Events share.
    pub fn uid(&self) -> &str {
        &self.uid
    }

    /// Pairs `server`, the Events of the UID's Server Object as read, with
    /// these Event Rows by Occurrence ([`Loaded::pair`]), against the
    /// Recurring Event the Event Rows hold, or else the one `server` holds
    /// (ADR 0012), as the CalDAV resource to send is built from them. When
    /// the Event Rows' Recurring Event moved from the time `server` holds,
    /// its Exceptions' Occurrences moved by as much, so `server`'s are
    /// paired where they moved to ([`moved_with`]) and their `VEVENT`s keep
    /// what FreeCal doesn't show. With no Server Object, `server` is empty
    /// and every Event Row is left.
    pub fn pair_with_server<'a>(&'a self, server: &[ImportedEvent]) -> ServerPairing<'a> {
        let held = server
            .iter()
            .find(|event| event.recurrence_id.is_none() && event.rule.is_some())
            .map(|event| event.when);
        let recurring = self.recurring_when().or(held);
        let moved: Option<Vec<ImportedEvent>> = match (&held, self.recurring()) {
            (Some(from), Some((_, to))) if Some(*from) != recurring => Some(
                server
                    .iter()
                    .map(|event| moved_with(event, from, &to.when))
                    .collect(),
            ),
            _ => None,
        };
        let server = moved.as_deref().unwrap_or(server);
        let pairing = self.pair(server, recurring.as_ref());
        let mut paired = vec![None; server.len()];
        for (incoming, stored) in pairing.paired {
            let index = server
                .iter()
                .position(|event| std::ptr::eq(event, incoming));
            if let (Some(index), Some((_, fields))) = (index, stored) {
                paired[index] = Some(fields);
            }
        }
        let event_left = pairing
            .event_left
            .and_then(|_| self.event.as_ref().map(|(_, fields)| fields));
        let exceptions_left = pairing.exceptions_left.into_iter().map(|left| &left.fields);
        ServerPairing {
            paired,
            left: event_left.into_iter().chain(exceptions_left).collect(),
            recurring,
        }
    }
}

/// `event`, an Event of a Server Object whose Recurring Event is at `from`,
/// with its recurrence ID moved as the Recurring Event moved to `to`, as
/// moving a Recurring Event's time moves its Exceptions' (ADR 0012); as it
/// is when it has no recurrence ID that is one.
fn moved_with(event: &ImportedEvent, from: &ImportedWhen, to: &StoredWhen) -> ImportedEvent {
    let moved = occurrence_key(event.recurrence_id.as_ref(), Some(from))
        .flatten()
        .and_then(|was| rule::moved_id(was.as_str(), &StoredWhen::imported(from), to));
    ImportedEvent {
        recurrence_id: moved
            .map(RecurrenceId::obtained)
            .or(event.recurrence_id.clone()),
        ..event.clone()
    }
}

/// The Events of a UID's Server Object paired with its Event Rows by
/// Occurrence ([`Loaded::pair_with_server`]).
pub(crate) struct ServerPairing<'a> {
    /// For each Event of the Server Object, in its order, the Event Row for
    /// its Occurrence; none when the Event Rows hold none, or when another
    /// Event of the Server Object, a later revision of the same Occurrence
    /// ([`latest`]), took it.
    pub paired: Vec<Option<&'a EventFields>>,
    /// The Event Rows no Event of the Server Object paired with: the Event
    /// that isn't an Exception first, then the Exceptions in the order they
    /// were added.
    pub left: Vec<&'a EventFields>,
    /// The time of the Recurring Event they were paired against, none when
    /// neither holds one.
    pub recurring: Option<ImportedWhen>,
}

/// The fields Propagate would give `row`, an Event Row, from `incoming`,
/// its Server Object's Event it pairs with, recurrence ID as the Server
/// Object holds it: those of `row`'s fields that differ from these differ
/// from what the Server Object holds, recurrence IDs by Occurrence (ADR
/// 0012).
pub(crate) fn as_propagated(incoming: &ImportedEvent, row: &EventFields) -> EventFields {
    fields_over(incoming, row, incoming.recurrence_id.clone())
}

/// An Event as the window gives it, loaded: a single Event, a Recurring
/// Event or one of its Exceptions.
pub(crate) struct Opened {
    loaded: Loaded,
    /// Which of `loaded`'s Exceptions it is, by index; none when it is
    /// their Event.
    exception: Option<usize>,
}

impl Opened {
    pub fn load(source: &impl Source, id: EventId) -> Result<Self> {
        Self::among(source.events_with_uid(Uid::Of(id))?, id)
    }

    /// The Event `id` of `events`, those with its UID.
    fn among(events: Vec<UidEvent>, id: EventId) -> Result<Self> {
        let opened = events
            .iter()
            .find(|event| event.id == id)
            .ok_or(CoreError::EventNotFound(id))?;
        let (calendar_id, uid) = (opened.calendar_id, opened.uid.clone());
        let loaded = Loaded::sorted(calendar_id, uid, events);
        let exception = loaded
            .exceptions
            .iter()
            .position(|exception| exception.id == id);
        Ok(Self { loaded, exception })
    }

    /// The opened Exception, none when it isn't one.
    fn exception(&self) -> Option<&Exception> {
        self.exception.map(|index| &self.loaded.exceptions[index])
    }

    fn is_exception(&self) -> bool {
        self.exception.is_some()
    }

    /// Whether it is an Exception that is an Event of its own
    /// ([`Loaded::of_its_own`]).
    fn is_of_its_own(&self) -> bool {
        self.exception()
            .is_some_and(|exception| self.loaded.of_its_own(exception))
    }

    fn id(&self) -> EventId {
        match self.exception() {
            Some(exception) => exception.id,
            None => self.event().0,
        }
    }

    fn fields(&self) -> &EventFields {
        match self.exception() {
            Some(exception) => &exception.fields,
            None => &self.event().1,
        }
    }

    /// The opened Event, of one that isn't an Exception: `loaded`'s Event,
    /// as it is loaded with it.
    fn event(&self) -> &Stored {
        self.loaded
            .event
            .as_ref()
            .expect("the opened Event loaded as the Event")
    }

    /// The Event as the window shows it, in the Display Time Zone `zone`,
    /// with whether `store` finds guests for its UID. An Exception repeats
    /// with its Recurring Event, which the editor can change from it, unless
    /// it is an Event of its own.
    pub fn to_event(&self, store: &Store, zone: Tz) -> Result<Event> {
        let fields = self.fields();
        let repeats = match &self.loaded.event {
            Some((_, recurring)) if self.is_exception() && !self.is_of_its_own() => recurring,
            _ => fields,
        };
        let (repeat, custom_repeat) = rule::repeat_of(repeats);
        Ok(Event {
            id: self.id(),
            calendar_id: self.loaded.calendar_id,
            title: fields.title.clone(),
            when: fields.when.to_when(zone)?,
            location: StructuredText::from_plain_text(&fields.location),
            description: fields.description(),
            pinned_zone: fields
                .zone_pinned
                .then(|| fields.when.start_zone.clone())
                .flatten(),
            repeat,
            custom_repeat,
            recurrence_id: self.exception().and_then(|exception| {
                exception
                    .rid
                    .clone()
                    .map(NormalForm::into_string)
                    .or_else(|| {
                        let obtained = exception.fields.recurrence_id.as_ref();
                        obtained.map(|obtained| obtained.as_obtained().to_owned())
                    })
            }),
            has_guests: store.has_guests(self.loaded.calendar_id, &self.loaded.uid)?,
            unsent_change: self.unsent_change(store)?,
        })
    }

    /// The opened Event's Unsent Change, as `store` keeps it; none for an
    /// Exception whose recurrence ID can't be read, which pairs with
    /// nothing.
    pub fn unsent_change(&self, store: &Store) -> Result<Option<UnsentChange>> {
        let key = match self.exception() {
            None => None,
            Some(Exception { rid: Some(rid), .. }) => Some(rid),
            Some(Exception { rid: None, .. }) => return Ok(None),
        };
        let kept = store.unsent_changes_of(self.loaded.calendar_id, &self.loaded.uid)?;
        Ok(UnsentChange::of_event_row(
            &kept,
            key,
            self.loaded.recurring_when().as_ref(),
        ))
    }

    /// Deletes the Event, and with a Recurring Event or one of its
    /// Exceptions, the whole Recurring Event with all its Exceptions. An
    /// Exception that is an Event of its own goes alone.
    pub fn delete(&self, journal_transaction: &mut JournalTransaction) -> Result<()> {
        match self.loaded.recurring() {
            Some(_) if !self.is_of_its_own() => self.loaded.delete_all(journal_transaction),
            _ => journal_transaction.delete(self.id()),
        }
    }
}

/// Cancels the Occurrence of the Event `id` with the recurrence ID
/// `recurrence_id`, as its [`Occurrence`] has them, and leaves the rest of
/// the Recurring Event in place: the Occurrence becomes a cancelled
/// Exception, and an Exception that changed it is deleted. The only
/// Occurrence of a single Event, or of an Exception whose Recurring Event is
/// gone, is the Event.
pub(crate) fn cancel_occurrence(
    journal_transaction: &mut JournalTransaction,
    id: EventId,
    recurrence_id: &str,
) -> Result<()> {
    let opened = Opened::load(journal_transaction, id)?;
    let cancelled = opened
        .loaded
        .rid_of(recurrence_id)
        .ok_or(CoreError::OccurrenceNotFound(id))?;
    match opened.loaded.recurring() {
        Some(_) => opened.loaded.cancel(journal_transaction, &cancelled),
        None => journal_transaction.delete(id),
    }
}

/// Creates an Event from the editor's `draft`, with times in the Display
/// Time Zone `zone`: a Recurring Event when the draft repeats.
pub(crate) fn create_event(
    journal_transaction: &mut JournalTransaction,
    zone: Tz,
    draft: EventDraft,
) -> Result<EventId> {
    let when = StoredWhen::new(draft.when, zone)?;
    let rule = draft
        .repeat
        .map(|repeat| rule::rule_text(repeat, None, &when))
        .transpose()?;
    let event = StoredEvent {
        calendar_id: draft.calendar_id,
        fields: EventFields {
            title: draft.title,
            when,
            location: draft.location,
            description: draft.description,
            description_html: false,
            zone_pinned: false,
            rule,
            exdates: String::new(),
            recurrence_id: None,
        },
    };
    journal_transaction.create(&new_uid(), &event)
}

/// The Occurrences of shown Calendars on the dates `from` up to `to`
/// (exclusive), in the Display Time Zone `zone` and ordered by start.
/// Recurring Events are expanded here, in wall time in their Event Time
/// Zone, and their Exceptions stand in for the Occurrences they changed:
/// of several for one Occurrence, as another program may have written, the
/// first by ID, paired by Occurrence. An Occurrence's recurrence ID is in the
/// Normal Form; an Exception that is an Event of its own
/// ([`Loaded::of_its_own`]) has none. Each says since when its Event has an
/// Unsent Change, if it has one.
pub(crate) fn occurrences(
    store: &Store,
    from: NaiveDate,
    to: NaiveDate,
    zone: Tz,
) -> Result<Vec<Occurrence>> {
    let range = StoredRange::new(from, to, zone);
    let mut occurrences = Vec::new();
    // The Events with each UID, and their Unsent Changes, loaded once.
    let mut loaded = HashMap::new();
    let mut unsent = HashMap::new();
    for event in store.shown_events_in(&range)? {
        let mut recurrence_id = None;
        // The Occurrence of the Event Row, against the Recurring Event at
        // `recurring`, for its Unsent Change; none for an Exception whose
        // recurrence ID can't be read, which pairs with nothing.
        let mut key = Some(None);
        let mut recurring = None;
        if event.fields.recurrence_id.is_some() {
            let with_uid = with_uid_of(&mut loaded, store, &event)?;
            recurring = with_uid.recurring_when();
            let own = with_uid
                .exceptions
                .iter()
                .find(|exception| exception.id == event.id);
            key = own.and_then(|own| own.rid.clone()).map(Some);
            if let Some(own) = own {
                let first = own.rid.as_ref().and_then(|rid| with_uid.exception_at(rid));
                if first.is_some_and(|first| first.id != own.id) {
                    continue;
                }
                if !with_uid.of_its_own(own) {
                    recurrence_id = own.rid.clone().map(NormalForm::into_string);
                }
            }
        }
        let expanded = match event.fields.rule {
            None => None,
            Some(_) => {
                let changed: Vec<NormalForm> = with_uid_of(&mut loaded, store, &event)?
                    .exceptions
                    .iter()
                    .filter_map(|exception| exception.rid.clone())
                    .collect();
                rule::occurrences_in(&event.fields, &changed, &range)?
            }
        };
        let unsent_since = match key {
            Some(key) => {
                let kept = match unsent.entry((event.calendar_id, event.uid.clone())) {
                    Entry::Occupied(entry) => entry.into_mut(),
                    Entry::Vacant(entry) => {
                        entry.insert(store.unsent_changes_of(event.calendar_id, &event.uid)?)
                    }
                };
                UnsentChange::of_event_row(kept, key.as_ref(), recurring.as_ref())
                    .map(|unsent| unsent.since)
            }
            None => None,
        };
        let occurrence = |when: &StoredWhen, recurrence_id| -> Result<Occurrence> {
            Ok(Occurrence {
                event_id: event.id,
                calendar_id: event.calendar_id,
                title: event.fields.title.clone(),
                when: when.to_when(zone)?,
                recurrence_id,
                has_guests: event.has_guests,
                unsent_since,
            })
        };
        match expanded {
            Some(found) => {
                for (recurrence_id, when) in found {
                    let start_utc = when.instants()?.0;
                    occurrences.push((start_utc, occurrence(&when, Some(recurrence_id))?));
                }
            }
            None => occurrences.push((
                event.start_utc,
                occurrence(&event.fields.when, recurrence_id)?,
            )),
        }
    }
    Ok(Occurrence::ordered(occurrences))
}

/// The Events with the UID of `event`, from `loaded` if they are in it,
/// otherwise loaded from `store` into it.
fn with_uid_of<'a>(
    loaded: &'a mut HashMap<(CalendarId, String), Loaded>,
    store: &Store,
    event: &RangeEvent,
) -> Result<&'a Loaded> {
    Ok(match loaded.entry((event.calendar_id, event.uid.clone())) {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => entry.insert(Loaded::of_uid(store, event.calendar_id, &event.uid)?),
    })
}

/// Makes `events`, the Events of the UID `uid` that its Corresponding
/// Server Object in the Calendar `calendar_id` holds now (at least one),
/// the UID's Event Rows, and changes no other Event Row (ADR 0015). Each
/// Event replaces the Event Row for the same Occurrence ([`Loaded::pair`]),
/// against the Recurring Event the Server Object holds, or else the one the
/// Event Rows hold (ADR 0012), taking its recurrence ID as the Server Object
/// gives it, or is added; the UID's other Event Rows are deleted.
pub(crate) fn propagate_to_event_rows(
    journal_transaction: &mut JournalTransaction,
    calendar_id: CalendarId,
    uid: &str,
    events: &[ImportedEvent],
) -> Result<()> {
    let loaded = Loaded::of_uid(journal_transaction, calendar_id, uid)?;
    let held = events
        .iter()
        .find(|event| event.recurrence_id.is_none() && event.rule.is_some())
        .map(|event| event.when);
    let recurring = held.or_else(|| loaded.recurring_when());
    let pairing = loaded.pair(events, recurring.as_ref());
    loaded.write_paired(journal_transaction, &pairing.paired, Kept::Incoming)?;
    let exceptions_left = pairing.exceptions_left.iter().map(|stored| stored.id);
    for id in pairing.event_left.into_iter().chain(exceptions_left) {
        journal_transaction.delete(id)?;
    }
    Ok(())
}

/// Which Occurrence an Event with the recurrence ID `obtained` stands for,
/// of a Recurring Event at `recurring`: `Some(None)` for an Event that isn't
/// an Exception, the Normal Form for an Exception, and none for a
/// recurrence ID that isn't one, which pairs with nothing.
fn occurrence_key(
    obtained: Option<&RecurrenceId>,
    recurring: Option<&ImportedWhen>,
) -> Option<Option<NormalForm>> {
    match obtained {
        None => Some(None),
        Some(obtained) => obtained.normal_form(recurring).map(Some),
    }
}

/// The Normal Form of the recurrence ID of an Exception with `fields`
/// against a Recurring Event at `recurring`; none for an Event that isn't
/// an Exception, and for a recurrence ID that isn't one.
pub(crate) fn normal_form(
    fields: &EventFields,
    recurring: Option<&ImportedWhen>,
) -> Option<NormalForm> {
    fields.recurrence_id.as_ref()?.normal_form(recurring)
}

/// Of `versions`, each with the Occurrence it is for, the latest for each
/// Occurrence: the one with the highest `SEQUENCE`, the first of equals.
/// iCalendar allows several, revisions told apart by `SEQUENCE`, so this
/// keeps an Occurrence to one Exception where the schema doesn't.
pub(crate) fn latest<'a, K: PartialEq>(
    versions: impl IntoIterator<Item = (K, &'a ImportedEvent)>,
) -> Vec<(K, &'a ImportedEvent)> {
    let mut kept: Vec<(K, &ImportedEvent)> = Vec::new();
    for (key, version) in versions {
        match kept.iter_mut().find(|(known, _)| *known == key) {
            Some(known) if version.sequence > known.1.sequence => known.1 = version,
            Some(_) => {}
            None => kept.push((key, version)),
        }
    }
    kept
}

/// A new UID for an Event.
pub(crate) fn new_uid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// An incoming version's fields, as the Local Store holds them.
fn fields(incoming: &ImportedEvent) -> EventFields {
    EventFields {
        title: incoming.title.clone(),
        when: StoredWhen::imported(&incoming.when),
        location: incoming.location.clone(),
        description: incoming.description.clone(),
        description_html: incoming.description_html,
        zone_pinned: incoming.zone_pinned,
        rule: incoming.rule.clone(),
        exdates: incoming.exdates.clone(),
        recurrence_id: incoming.recurrence_id.clone(),
    }
}

/// An incoming version's fields as the new version of an Event that has
/// `previous`, for the same Occurrence, with the recurrence ID
/// `recurrence_id`: the form its Server Object holds (ADR 0012). An import
/// passes the stored one, so that a file giving it in another form leaves no
/// Unsent Change in a form the Server Object doesn't hold; Propagate
/// ([`propagate_to_event_rows`]) passes the Server Object's. One floating end
/// takes the zone of the other (ADR 0007, rule 3), so only a version floating
/// at both keeps the zone pinned before (rule 2).
fn fields_over(
    incoming: &ImportedEvent,
    previous: &EventFields,
    recurrence_id: Option<RecurrenceId>,
) -> EventFields {
    let fields = EventFields {
        recurrence_id,
        ..fields(incoming)
    };
    if incoming.all_floating && previous.zone_pinned {
        EventFields {
            when: fields.when.in_zones_of(&previous.when),
            ..fields
        }
    } else {
        fields
    }
}
