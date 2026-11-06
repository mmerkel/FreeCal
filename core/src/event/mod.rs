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

use chrono::NaiveDate;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::journal::JournalTransaction;
use crate::store::{RangeEvent, Store, StoredEvent, Uid, UidEvent};
use crate::{CalendarId, CoreError, ImportedEvent, ImportedWhen, Result, StructuredText};

pub(crate) use recurrence_id::{NormalForm, RecurrenceId, in_normal_form};
pub use rule::{Frequency, Repeat, RepeatEnd};
pub(crate) use scope::{Edit, edit_event, edit_occurrence, exception_counts};
pub use scope::{ExceptionCounts, Scope};
pub(crate) use when::{EventFields, ObjectId, StoredRange, StoredWhen, instant};
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
    /// The Event that isn't an Exception: the one the window gave, otherwise
    /// the first by ID, should the Calendar have several with the UID.
    event: Option<Stored>,
    /// Every Exception with the UID, in the order they were added.
    exceptions: Vec<Exception>,
}

impl Loaded {
    /// The Events of a Calendar with the UID `uid`, as an Incoming Sync or an
    /// import finds them.
    pub fn of_uid(source: &impl Source, calendar_id: CalendarId, uid: &str) -> Result<Self> {
        let events = source.events_with_uid(Uid::In(calendar_id, uid))?;
        Ok(Self::sorted(calendar_id, uid.to_owned(), events, None))
    }

    /// `events` sorted, with `opened`, if it isn't an Exception, as the
    /// Event.
    fn sorted(
        calendar_id: CalendarId,
        uid: String,
        events: Vec<UidEvent>,
        opened: Option<EventId>,
    ) -> Self {
        let (exceptions, mut events): (Vec<_>, Vec<_>) = events
            .into_iter()
            .map(|event| (event.id, event.fields))
            .partition(|(_, fields)| fields.recurrence_id.is_some());
        let event = events
            .iter()
            .position(|(id, _)| Some(*id) == opened)
            .or((!events.is_empty()).then_some(0))
            .map(|index| events.swap_remove(index));
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
    /// are paired by Occurrence, against `incoming`'s Recurring Event: those
    /// `incoming` has replace them or are added, the latest of several for
    /// one Occurrence. The others are deleted, but for Events of their own
    /// ([`Loaded::of_its_own`]), which stay and join `incoming` without being
    /// written, as their recurrence IDs stay as obtained (ADR 0012). ADR 0007's
    /// rules 2 and 3 apply.
    pub fn take(
        &self,
        journal_transaction: &mut JournalTransaction,
        incoming: &ImportedEvent,
    ) -> Result<()> {
        let recurring = incoming.rule.as_ref().map(|_| &incoming.when);
        let exceptions = latest(incoming.exceptions.iter().map(|exception| {
            let obtained = exception.recurrence_id.as_ref();
            (
                obtained.and_then(|obtained| obtained.normal_form(recurring)),
                exception,
            )
        }));
        match &self.event {
            Some((id, previous)) => journal_transaction.replace(
                *id,
                &StoredEvent {
                    calendar_id: self.calendar_id,
                    fields: fields_over(incoming, previous, previous.recurrence_id.clone()),
                },
            )?,
            None => {
                journal_transaction.create(
                    &self.uid,
                    &StoredEvent {
                        calendar_id: self.calendar_id,
                        fields: fields(incoming),
                    },
                )?;
            }
        }
        let mut left: Vec<_> = self
            .exceptions
            .iter()
            .map(|stored| (normal_form(&stored.fields, recurring), stored))
            .collect();
        for (key, exception) in exceptions {
            let same = key
                .as_ref()
                .and_then(|key| left.iter().position(|(rid, _)| rid.as_ref() == Some(key)));
            let (previous, fields) = match same.map(|index| left.remove(index).1) {
                Some(stored) => {
                    let kept = stored.fields.recurrence_id.clone();
                    (
                        Some(stored.id),
                        fields_over(exception, &stored.fields, kept),
                    )
                }
                None => (None, fields(exception)),
            };
            self.write_exception(journal_transaction, previous, self, fields)?;
        }
        for (_, stored) in left {
            if !self.of_its_own(stored) {
                journal_transaction.delete(stored.id)?;
            }
        }
        Ok(())
    }

    /// Makes `incoming`, an Exception from a file without its Recurring
    /// Event, one of the Events with the UID: an Exception of their
    /// Recurring Event if they have one, otherwise an Event of its own
    /// (ticket 88). It replaces the Exception for the same Occurrence or is
    /// added, and the others stay. ADR 0007's rules 2 and 3 apply.
    pub fn take_exception(
        &self,
        journal_transaction: &mut JournalTransaction,
        incoming: &ImportedEvent,
    ) -> Result<()> {
        let rid = incoming
            .recurrence_id
            .as_ref()
            .and_then(|obtained| obtained.normal_form(self.recurring_when().as_ref()));
        match rid.as_ref().and_then(|rid| self.exception_at(rid)) {
            Some(stored) => journal_transaction.replace(
                stored.id,
                &StoredEvent {
                    calendar_id: self.calendar_id,
                    fields: fields_over(
                        incoming,
                        &stored.fields,
                        stored.fields.recurrence_id.clone(),
                    ),
                },
            ),
            None => journal_transaction
                .create(
                    &self.uid,
                    &StoredEvent {
                        calendar_id: self.calendar_id,
                        fields: fields(incoming),
                    },
                )
                .map(|_| ()),
        }
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
        let loaded = Loaded::sorted(calendar_id, uid, events, Some(id));
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

    /// The Event as the window shows it, in the Display Time Zone `zone`.
    /// An Exception repeats with its Recurring Event, which the editor can
    /// change from it, unless it is an Event of its own.
    pub fn to_event(&self, zone: Tz) -> Result<Event> {
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
        })
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
/// ([`Loaded::of_its_own`]) has none.
pub(crate) fn occurrences(
    store: &Store,
    from: NaiveDate,
    to: NaiveDate,
    zone: Tz,
) -> Result<Vec<Occurrence>> {
    let range = StoredRange::new(from, to, zone);
    let mut occurrences = Vec::new();
    // The Events with each UID, loaded once.
    let mut loaded = HashMap::new();
    for event in store.shown_events_in(&range)? {
        let mut recurrence_id = None;
        if event.fields.recurrence_id.is_some() {
            let with_uid = with_uid_of(&mut loaded, store, &event)?;
            let own = with_uid
                .exceptions
                .iter()
                .find(|exception| exception.id == event.id);
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
        let occurrence = |when: &StoredWhen, recurrence_id| -> Result<Occurrence> {
            Ok(Occurrence {
                event_id: event.id,
                calendar_id: event.calendar_id,
                title: event.fields.title.clone(),
                when: when.to_when(zone)?,
                recurrence_id,
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

/// Makes `events`, the Events of the UID `uid` that the Server Object
/// `object` of the Calendar `calendar_id` holds now (at least one), the
/// UID's Event Rows, as `object` is its Corresponding Server Object, the
/// one CalDAV Server Object holding it, and changes no other Event Row
/// (ADR 0015): each Event replaces the Event Row for the
/// same Occurrence, or the one that isn't an Exception, or is added, and
/// the UID's other Event Rows are deleted. Exceptions are paired by Occurrence against the Recurring Event
/// `object` holds, or else the one the Event Rows hold (ADR 0012), and each
/// Event Row takes its recurrence ID as `object` gives it. Of several for one Occurrence, the
/// latest is kept. ADR 0007's rules 2 and 3 apply.
pub(crate) fn propagate_to_event_rows(
    journal_transaction: &mut JournalTransaction,
    calendar_id: CalendarId,
    object: ObjectId,
    uid: &str,
    events: &[ImportedEvent],
) -> Result<()> {
    debug_assert_eq!(
        journal_transaction
            .database_transaction()
            .corresponding_server_object(calendar_id, uid)?,
        Some(object)
    );
    let held = events
        .iter()
        .find(|event| event.recurrence_id.is_none() && event.rule.is_some())
        .map(|event| event.when);
    let event_rows = journal_transaction.events_with_uid(Uid::In(calendar_id, uid))?;
    // The Recurring Event of the Event Rows, chosen as `Loaded` chooses it:
    // the first Event Row that isn't an Exception, if it repeats.
    let recurring = held.or_else(|| {
        event_rows
            .iter()
            .find(|event_row| event_row.fields.recurrence_id.is_none())
            .filter(|event_row| event_row.fields.rule.is_some())
            .and_then(|event_row| event_row.fields.when.to_imported().ok())
    });
    let occurrence = |obtained: Option<&RecurrenceId>| occurrence_key(obtained, recurring.as_ref());
    let mut left: Vec<_> = event_rows
        .into_iter()
        .map(|event_row| {
            (
                occurrence(event_row.fields.recurrence_id.as_ref()),
                event_row,
            )
        })
        .collect();
    let latest = latest(
        events
            .iter()
            .map(|event| (occurrence(event.recurrence_id.as_ref()), event)),
    );
    for (key, incoming) in latest {
        let same = key.as_ref().and_then(|key| {
            left.iter()
                .position(|(occurrence, _)| occurrence.as_ref() == Some(key))
        });
        let event = |fields: EventFields| StoredEvent {
            calendar_id,
            fields,
        };
        match same.map(|index| left.remove(index).1) {
            Some(event_row) => journal_transaction.replace(
                event_row.id,
                &event(fields_over(
                    incoming,
                    &event_row.fields,
                    incoming.recurrence_id.clone(),
                )),
            )?,
            None => _ = journal_transaction.create(uid, &event(fields(incoming)))?,
        }
    }
    for (_, event_row) in left {
        journal_transaction.delete(event_row.id)?;
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
fn normal_form(fields: &EventFields, recurring: Option<&ImportedWhen>) -> Option<NormalForm> {
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
