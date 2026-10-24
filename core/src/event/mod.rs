//! Events as the core loads and changes them. An Event is loaded as one
//! value, [`Loaded`], with the other Events of its Calendar that share its
//! UID: a Recurring Event comes with its rule and its Exceptions. The
//! operations that need them all live here, so that only this module pairs
//! Exceptions with their Occurrences, by recurrence IDs always normalised.
//!
//! Loading from the Local Store serves reads; changes load from a Change
//! Journal unit and write through it, never the Store (ADR 0009).

pub(crate) mod rule;
mod when;

use chrono::NaiveDate;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::journal::Unit;
use crate::store::{Store, StoredEvent, Uid, UidEvent};
use crate::{CalendarId, CoreError, ImportedEvent, Result, StructuredText};

pub use rule::{Frequency, Repeat, RepeatEnd};
pub(crate) use when::{EventFields, StoredRange, StoredWhen, instant, wall_text};
pub use when::{EventId, OriginalServerData, When};

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
    /// text that only the core reads. None for a single Event.
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

impl Source for Unit<'_> {
    fn events_with_uid(&self, uid: Uid) -> Result<Vec<UidEvent>> {
        Unit::events_with_uid(self, uid)
    }
}

/// A stored Event: its ID and fields.
type Stored = (EventId, EventFields);

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
    exceptions: Vec<Stored>,
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

    /// Whether the Calendar has an Event with the UID that isn't an
    /// Exception.
    pub fn exists(&self) -> bool {
        self.event.is_some()
    }

    /// The Recurring Event, none when the Event doesn't repeat or is gone.
    fn recurring(&self) -> Option<&Stored> {
        self.event
            .as_ref()
            .filter(|(_, fields)| fields.rule.is_some())
    }

    /// Makes `incoming`, a new version from a file or a server with its
    /// Exceptions, the Events with the UID, adding it if the Calendar has
    /// none. Exceptions are paired by the Occurrence they changed: those
    /// `incoming` has replace them or are added, and the others are
    /// deleted. ADR 0007's rules 2 and 3 apply.
    pub fn take(&self, unit: &mut Unit, incoming: &ImportedEvent) -> Result<()> {
        let Some((id, previous)) = &self.event else {
            return create(unit, self.calendar_id, &self.uid, incoming);
        };
        let new = |fields| StoredEvent {
            calendar_id: self.calendar_id,
            fields,
        };
        unit.replace(*id, &new(fields_over(incoming, previous)))?;
        let mut left: Vec<_> = self.exceptions.iter().collect();
        for exception in &incoming.exceptions {
            let same = left.iter().position(|(_, fields)| {
                same_recurrence_id(
                    fields.recurrence_id.as_deref(),
                    exception.recurrence_id.as_deref(),
                )
            });
            match same.map(|index| left.remove(index)) {
                Some((id, previous)) => {
                    unit.replace(*id, &new(fields_over(exception, previous)))?;
                }
                None => _ = unit.create(&self.uid, &new(fields(exception)))?,
            }
        }
        for (id, _) in left {
            unit.delete(*id)?;
        }
        Ok(())
    }

    /// The UID, the Event and the Exceptions, for `scope.rs` until ticket 84
    /// moves it into the module.
    pub fn into_parts(self) -> (String, Option<Stored>, Vec<Stored>) {
        (self.uid, self.event, self.exceptions)
    }

    /// Deletes the Events with the UID, the Event and every Exception.
    pub fn delete_all(&self, unit: &mut Unit) -> Result<()> {
        for (id, _) in self.event.iter().chain(&self.exceptions) {
            unit.delete(*id)?;
        }
        Ok(())
    }
}

/// An Event as the window gives it, loaded: a single Event, a Recurring
/// Event or one of its Exceptions.
pub(crate) struct Opened {
    id: EventId,
    fields: EventFields,
    loaded: Loaded,
}

impl Opened {
    pub fn load(source: &impl Source, id: EventId) -> Result<Self> {
        let events = source.events_with_uid(Uid::Of(id))?;
        let opened = events
            .iter()
            .find(|event| event.id == id)
            .ok_or(CoreError::EventNotFound(id))?;
        let (calendar_id, uid) = (opened.calendar_id, opened.uid.clone());
        let fields = opened.fields.clone();
        Ok(Self {
            id,
            fields,
            loaded: Loaded::sorted(calendar_id, uid, events, Some(id)),
        })
    }

    /// What it was loaded with, for `scope.rs` until ticket 84.
    pub fn into_loaded(self) -> Loaded {
        self.loaded
    }

    fn is_exception(&self) -> bool {
        self.fields.recurrence_id.is_some()
    }

    /// The Event as the window shows it, in the Display Time Zone `zone`.
    /// An Exception repeats with its Recurring Event, which the editor can
    /// change from it.
    pub fn to_event(&self, zone: Tz) -> Result<Event> {
        let fields = &self.fields;
        let repeats = match &self.loaded.event {
            Some((_, recurring)) if self.is_exception() => recurring,
            _ => fields,
        };
        let (repeat, custom_repeat) = rule::repeat_of(repeats);
        Ok(Event {
            id: self.id,
            calendar_id: self.loaded.calendar_id,
            title: fields.title.clone(),
            when: fields.when.to_when(zone)?,
            location: StructuredText::from_plain_text(&fields.location),
            description: StructuredText::from_description(
                &fields.description,
                fields.description_html,
            ),
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
    /// Exceptions, the whole Recurring Event with all its Exceptions.
    pub fn delete(&self, unit: &mut Unit) -> Result<()> {
        let whole = match (self.is_exception(), self.loaded.recurring()) {
            (true, Some(_)) => true,
            (true, None) => false,
            (false, _) => self.fields.rule.is_some(),
        };
        let mut deleted = vec![self.id];
        if whole {
            deleted.extend(self.loaded.recurring().map(|(id, _)| *id));
            deleted.extend(self.loaded.exceptions.iter().map(|(id, _)| *id));
        }
        deleted.sort_unstable_by_key(|id| id.0);
        deleted.dedup();
        deleted.into_iter().try_for_each(|id| unit.delete(id))
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
    let Some((recurring_id, fields)) = opened.loaded.recurring() else {
        return unit.delete(id);
    };
    for (exception, fields) in &opened.loaded.exceptions {
        if same_recurrence_id(fields.recurrence_id.as_deref(), Some(&cancelled)) {
            unit.delete(*exception)?;
        }
    }
    unit.replace(
        *recurring_id,
        &StoredEvent {
            calendar_id: opened.loaded.calendar_id,
            fields: EventFields {
                exdates: rule::with_exdate(&fields.exdates, &cancelled),
                ..fields.clone()
            },
        },
    )
}

/// The Occurrences of shown Calendars on the dates `from` up to `to`
/// (exclusive), in the Display Time Zone `zone` and ordered by start.
/// Recurring Events are expanded here, in wall time in their Event Time
/// Zone, and their Exceptions stand in for the Occurrences they changed.
pub(crate) fn occurrences(
    store: &Store,
    from: NaiveDate,
    to: NaiveDate,
    zone: Tz,
) -> Result<Vec<Occurrence>> {
    let range = StoredRange::new(from, to, zone);
    let mut occurrences = Vec::new();
    for event in store.shown_events_in(&range)? {
        let expanded = match event.fields.rule {
            None => None,
            Some(_) => {
                let changed: Vec<String> = Opened::load(store, event.id)?
                    .loaded
                    .exceptions
                    .into_iter()
                    .filter_map(|(_, fields)| fields.recurrence_id)
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
                occurrence(&event.fields.when, event.fields.recurrence_id.clone())?,
            )),
        }
    }
    Ok(Occurrence::ordered(occurrences))
}

/// Adds `incoming`, a version from a file or a server, with its Exceptions
/// to the Calendar `calendar_id`, under the UID `uid`.
pub(crate) fn create(
    unit: &mut Unit,
    calendar_id: CalendarId,
    uid: &str,
    incoming: &ImportedEvent,
) -> Result<()> {
    let new = |fields| StoredEvent {
        calendar_id,
        fields,
    };
    unit.create(uid, &new(fields(incoming)))?;
    for exception in &incoming.exceptions {
        unit.create(uid, &new(fields(exception)))?;
    }
    Ok(())
}

/// Whether two recurrence IDs name the same Occurrence, however each is
/// written. One that isn't a recurrence ID names none.
pub(crate) fn same_recurrence_id(a: Option<&str>, b: Option<&str>) -> bool {
    let normalised = |id: Option<&str>| id.and_then(rule::recurrence_id);
    normalised(a).is_some() && normalised(a) == normalised(b)
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
        server_data: incoming.server_data.clone(),
    }
}

/// An incoming version's fields as the new version of an Event that has
/// `previous`: one floating end takes the zone of the other (ADR 0007, rule
/// 3), so only a version floating at both keeps the zone pinned before
/// (rule 2).
fn fields_over(incoming: &ImportedEvent, previous: &EventFields) -> EventFields {
    let fields = fields(incoming);
    if incoming.all_floating && previous.zone_pinned {
        EventFields {
            when: fields.when.in_zones_of(&previous.when),
            ..fields
        }
    } else {
        fields
    }
}
