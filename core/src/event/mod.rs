//! Events as the core loads and changes them. An Event is loaded as one
//! value, [`Loaded`], with the other Events of its Calendar that share its
//! UID: a Recurring Event comes with its rule and its Exceptions. The
//! operations that need them all live here, so that only this module pairs
//! Exceptions with their Occurrences, by recurrence IDs always normalised.
//! Every change the user makes to an Event goes through it: one that
//! doesn't repeat is changed as one Occurrence alone would be (`scope.rs`).
//!
//! Loading from the Local Store serves reads; changes load from a Change
//! Journal unit and write through it, never the Store (ADR 0009).

pub(crate) mod rule;
mod scope;
mod when;

use chrono::NaiveDate;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::import::rejoined_id;
use crate::journal::Unit;
use crate::store::{Store, StoredEvent, Uid, UidEvent};
use crate::{CalendarId, CoreError, ImportedEvent, ImportedWhen, Result, StructuredText};

pub use rule::{Frequency, Repeat, RepeatEnd};
pub use scope::Scope;
pub(crate) use scope::{Edit, edit_event, edit_occurrence};
pub(crate) use when::{EventFields, ObjectId, StoredRange, StoredWhen, instant, wall_text};
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
    /// Event that it changed; none for every other Event.
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
    /// Which Occurrence of a Recurring Event this is: its original start, as
    /// text that only the core reads. None for a single Event, and for an
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

/// Where Events are loaded from: the Local Store for reads, a Change
/// Journal unit for changes.
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

/// A stored Exception: its ID, the recurrence ID of the Occurrence it
/// changed, normalised, and its fields. A stored recurrence ID that isn't
/// one gets none, and so pairs with no Occurrence.
struct Exception {
    id: EventId,
    rid: Option<String>,
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
    /// The Events of a Calendar with the UID `uid`, as a sync or an import
    /// finds them.
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
        let exceptions = exceptions
            .into_iter()
            .map(|(id, fields)| Exception {
                id,
                rid: fields.rid(),
                fields,
            })
            .collect();
        let event = events
            .iter()
            .position(|(id, _)| Some(*id) == opened)
            .or((!events.is_empty()).then_some(0))
            .map(|index| events.swap_remove(index));
        Self {
            calendar_id,
            uid,
            event,
            exceptions,
        }
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
    /// Events with the UID, adding it if the Calendar has none (see [`Unit`]
    /// for the server object each gets). Exceptions are paired by the
    /// Occurrence they changed: those `incoming` has replace them or are
    /// added, the latest of several for one Occurrence. The others are
    /// deleted, but for Events of their own ([`Loaded::of_its_own`]), which
    /// stay, joined beforehand ([`Loaded::join`]) if they had no Recurring
    /// Event.
    /// ADR 0007's rules 2 and 3 apply.
    pub fn take(&self, unit: &mut Unit, incoming: &ImportedEvent) -> Result<()> {
        let exceptions = latest(incoming.exceptions.iter().map(|exception| {
            let rid = exception
                .recurrence_id
                .as_deref()
                .and_then(rule::recurrence_id);
            (rid, exception)
        }));
        match &self.event {
            Some((id, previous)) => unit.replace(
                *id,
                &StoredEvent {
                    calendar_id: self.calendar_id,
                    fields: fields_over(incoming, previous, None),
                },
            )?,
            None => {
                unit.create(
                    &self.uid,
                    &StoredEvent {
                        calendar_id: self.calendar_id,
                        fields: fields(incoming, None),
                    },
                )?;
            }
        }
        let mut left: Vec<_> = self.exceptions.iter().collect();
        for (rid, exception) in exceptions {
            let same = rid.and_then(|rid| {
                left.iter()
                    .position(|stored| stored.rid.as_ref() == Some(&rid))
            });
            let (previous, fields) = match same.map(|index| left.remove(index)) {
                Some(stored) => (
                    Some(stored.id),
                    fields_over(exception, &stored.fields, None),
                ),
                None => (None, fields(exception, None)),
            };
            self.write_exception(unit, previous, self, fields)?;
        }
        for stored in left {
            if !self.of_its_own(stored) {
                unit.delete(stored.id)?;
            }
        }
        Ok(())
    }

    /// Makes `incoming`, an Exception from a file without its Recurring
    /// Event, one of the Events with the UID: an Exception of their
    /// Recurring Event if they have one, otherwise an Event of its own
    /// (ticket 88). It replaces the Exception for the same Occurrence or is
    /// added, and the others stay. ADR 0007's rules 2 and 3 apply.
    pub fn take_exception(&self, unit: &mut Unit, incoming: &ImportedEvent) -> Result<()> {
        let rid = incoming
            .recurrence_id_in(self.recurring_when().as_ref())
            .as_deref()
            .and_then(rule::recurrence_id);
        let event = |fields: EventFields| StoredEvent {
            calendar_id: self.calendar_id,
            fields: EventFields {
                recurrence_id: rid.clone(),
                ..fields
            },
        };
        match rid.as_deref().and_then(|rid| self.exception_at(rid)) {
            Some(stored) => unit.replace(
                stored.id,
                &event(fields_over(incoming, &stored.fields, None)),
            ),
            None => unit
                .create(&self.uid, &event(fields(incoming, None)))
                .map(|_| ()),
        }
    }

    /// Pairs the Exceptions with the UID with a Recurring Event at
    /// `recurring` that they are about to get, if they have none yet: each
    /// that was an Event of its own, its recurrence ID in the zone it was
    /// given in, gets it normalised against the Recurring Event (ticket 88).
    pub fn join(&self, unit: &mut Unit, recurring: &ImportedWhen) -> Result<()> {
        if self.recurring().is_some() {
            return Ok(());
        }
        for exception in &self.exceptions {
            let joined = exception
                .fields
                .recurrence_id
                .as_deref()
                .and_then(|text| rejoined_id(text, recurring));
            if let Some(joined) = joined {
                unit.replace(
                    exception.id,
                    &StoredEvent {
                        calendar_id: self.calendar_id,
                        fields: EventFields {
                            recurrence_id: Some(joined),
                            ..exception.fields.clone()
                        },
                    },
                )?;
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
        unit: &mut Unit,
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
                unit.replace(id, &event)?;
                Ok(id)
            }
            Some(id) => {
                unit.delete(id)?;
                unit.create(&into.uid, &event)
            }
            None => unit.create(&into.uid, &event),
        }
    }

    /// Whether `exception`, one of these Exceptions, is an Event of its own:
    /// without their Recurring Event, or changing none of its Occurrences,
    /// such as one for a cancelled Occurrence (ticket 88).
    fn of_its_own(&self, exception: &Exception) -> bool {
        match (self.recurring(), &exception.rid) {
            (Some((_, fields)), Some(rid)) => {
                !matches!(rule::occurrence_at(fields, rid), Ok(Some(_)))
            }
            _ => true,
        }
    }

    /// The Exception that changed the Occurrence `rid`, normalised, if there
    /// is one.
    fn exception_at(&self, rid: &str) -> Option<&Exception> {
        self.exceptions
            .iter()
            .find(|exception| exception.rid.as_deref() == Some(rid))
    }

    /// Cancels the Occurrence `rid`, normalised, of the Recurring Event,
    /// which it must have: the Occurrence becomes a cancelled Exception, and
    /// an Exception that changed it is deleted.
    fn cancel(&self, unit: &mut Unit, rid: &str) -> Result<()> {
        let (id, fields) = self.recurring_event();
        for exception in &self.exceptions {
            if exception.rid.as_deref() == Some(rid) {
                unit.delete(exception.id)?;
            }
        }
        unit.replace(
            *id,
            &StoredEvent {
                calendar_id: self.calendar_id,
                fields: EventFields {
                    exdates: rule::with_exdate(&fields.exdates, rid),
                    ..fields.clone()
                },
            },
        )
    }

    /// Deletes the Events with the UID, the Event and every Exception.
    pub fn delete_all(&self, unit: &mut Unit) -> Result<()> {
        let exceptions = self.exceptions.iter().map(|exception| exception.id);
        for id in self.event.iter().map(|(id, _)| *id).chain(exceptions) {
            unit.delete(id)?;
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
            recurrence_id: fields.recurrence_id.clone(),
        })
    }

    /// Deletes the Event, and with a Recurring Event or one of its
    /// Exceptions, the whole Recurring Event with all its Exceptions. An
    /// Exception that is an Event of its own goes alone.
    pub fn delete(&self, unit: &mut Unit) -> Result<()> {
        match self.loaded.recurring() {
            Some(_) if !self.is_of_its_own() => self.loaded.delete_all(unit),
            _ => unit.delete(self.id()),
        }
    }
}

/// Cancels the Occurrence of the Event `id` with the recurrence ID
/// `recurrence_id`, as its [`Occurrence`] has them, and leaves the rest of
/// the Recurring Event in place: the Occurrence becomes a cancelled
/// Exception, and an Exception that changed it is deleted. The only
/// Occurrence of a single Event, or of an Exception whose Recurring Event is
/// gone, is the Event.
pub(crate) fn cancel_occurrence(unit: &mut Unit, id: EventId, recurrence_id: &str) -> Result<()> {
    let cancelled = rule::recurrence_id(recurrence_id).ok_or(CoreError::OccurrenceNotFound(id))?;
    let opened = Opened::load(unit, id)?;
    match opened.loaded.recurring() {
        Some(_) => opened.loaded.cancel(unit, &cancelled),
        None => unit.delete(id),
    }
}

/// Creates an Event from the editor's `draft`, with times in the Display
/// Time Zone `zone`: a Recurring Event when the draft repeats.
pub(crate) fn create_event(unit: &mut Unit, zone: Tz, draft: EventDraft) -> Result<EventId> {
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
            object_id: None,
        },
    };
    unit.create(&new_uid(), &event)
}

/// The Occurrences of shown Calendars on the dates `from` up to `to`
/// (exclusive), in the Display Time Zone `zone` and ordered by start.
/// Recurring Events are expanded here, in wall time in their Event Time
/// Zone, and their Exceptions stand in for the Occurrences they changed:
/// of several for one Occurrence, as two server objects may hold, the first
/// by ID. An Exception that is an Event of its own ([`Loaded::of_its_own`])
/// has no recurrence ID.
pub(crate) fn occurrences(
    store: &Store,
    from: NaiveDate,
    to: NaiveDate,
    zone: Tz,
) -> Result<Vec<Occurrence>> {
    let range = StoredRange::new(from, to, zone);
    let mut occurrences = Vec::new();
    for event in store.shown_events_in(&range)? {
        let mut recurrence_id = event.fields.recurrence_id.clone();
        if let Some(rid) = event.fields.rid() {
            let with_uid = store.events_with_uid(Uid::Of(event.id))?;
            if with_uid
                .iter()
                .find(|other| other.fields.rid().as_ref() == Some(&rid))
                .is_some_and(|first| first.id != event.id)
            {
                continue;
            }
            if Opened::among(with_uid, event.id)?.is_of_its_own() {
                recurrence_id = None;
            }
        }
        let expanded = match event.fields.rule {
            None => None,
            Some(_) => {
                let changed: Vec<String> = store
                    .events_with_uid(Uid::Of(event.id))?
                    .into_iter()
                    .filter_map(|exception| exception.fields.recurrence_id)
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

/// An Event a server object holds, as [`take_object`] writes it.
pub(crate) struct ObjectEvent<'a> {
    pub uid: &'a str,
    /// For an Exception, the recurrence ID, normalised, of the Occurrence
    /// of its Recurring Event that it changed.
    pub rid: Option<String>,
    pub incoming: &'a ImportedEvent,
}

/// Makes `events`, the Events the server object `object` of the Calendar
/// `calendar_id` holds now, the object's rows, and changes no other row
/// (ADR 0010): each replaces the object's row with its UID and recurrence
/// ID or is added, and the object's other rows are deleted, whatever their
/// UIDs. Of several with one UID and recurrence ID, the latest is kept.
/// ADR 0007's rules 2 and 3 apply.
pub(crate) fn take_object(
    unit: &mut Unit,
    calendar_id: CalendarId,
    object: ObjectId,
    events: &[ObjectEvent],
) -> Result<()> {
    let mut left = unit.events_in_object(object)?;
    let events = latest(
        events
            .iter()
            .map(|event| ((event.uid, event.rid.as_deref()), event.incoming)),
    );
    for ((uid, rid), incoming) in events {
        // A stored recurrence ID that isn't one pairs with nothing.
        let same = left.iter().position(|row| {
            let stored = match &row.fields.recurrence_id {
                Some(text) => rule::recurrence_id(text).map(Some),
                None => Some(None),
            };
            row.uid == uid
                && stored
                    .as_ref()
                    .is_some_and(|stored| stored.as_deref() == rid)
        });
        let event = |fields: EventFields| StoredEvent {
            calendar_id,
            fields: EventFields {
                recurrence_id: rid.map(str::to_owned),
                ..fields
            },
        };
        match same.map(|index| left.remove(index)) {
            Some(row) => unit.replace(
                row.id,
                &event(fields_over(incoming, &row.fields, Some(object))),
            )?,
            None => _ = unit.create(uid, &event(fields(incoming, Some(object))))?,
        }
    }
    for row in left {
        unit.delete(row.id)?;
    }
    Ok(())
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

/// An incoming version's fields, as the Local Store holds them, in the
/// server object `object`.
fn fields(incoming: &ImportedEvent, object: Option<ObjectId>) -> EventFields {
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
        object_id: object,
    }
}

/// An incoming version's fields as the new version of an Event that has
/// `previous`: one floating end takes the zone of the other (ADR 0007, rule
/// 3), so only a version floating at both keeps the zone pinned before
/// (rule 2).
fn fields_over(
    incoming: &ImportedEvent,
    previous: &EventFields,
    object: Option<ObjectId>,
) -> EventFields {
    let fields = fields(incoming, object);
    if incoming.all_floating && previous.zone_pinned {
        EventFields {
            when: fields.when.in_zones_of(&previous.when),
            ..fields
        }
    } else {
        fields
    }
}
