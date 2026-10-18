//! Recurrence scopes: a change to one Occurrence of a Recurring Event made
//! to that Occurrence alone, which makes it an Exception, to it and the
//! Occurrences after it, which splits the Recurring Event in two, or to all
//! of them.
//!
//! A change is what differs between the Occurrence as it was and what the
//! user saved. Applied to the other Occurrences, it changes only the fields
//! the user changed, so that a new title leaves the times alone, and an
//! Exception keeps what was changed in it on its own: a field it shares with
//! its Recurring Event follows the change, one it changed stays.

use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::event::StoredWhen;
use crate::journal::Unit;
use crate::recurrence;
use crate::store::{EventFields, StoredEvent};
use crate::{CalendarId, CoreError, EventDraft, EventId, Repeat, Result, StructuredText, When};

/// Which Occurrences of a Recurring Event a change to one of them changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum Scope {
    /// Only this Occurrence, which becomes an Exception.
    This,
    /// This Occurrence and every later one, which become a Recurring Event
    /// of their own.
    ThisAndFollowing,
    /// Every Occurrence: the whole Recurring Event.
    All,
}

/// What the user changed an Occurrence to: a draft from the editor, or a new
/// time from a drag or a resize, with everything else as it was.
pub(crate) enum Edit {
    Draft(EventDraft),
    Reschedule(When),
}

/// The Occurrence that the changed one is now: its Event, title, time and
/// recurrence ID, none for a single Event.
pub(crate) struct Edited {
    pub id: EventId,
    pub calendar_id: CalendarId,
    pub title: String,
    pub when: StoredWhen,
    pub recurrence_id: Option<String>,
}

/// Changes the Occurrence of the Event `id` with the recurrence ID
/// `recurrence_id`, as its [`crate::Occurrence`] has them, in `scope`. Times
/// in `change` are in the Display Time Zone `zone`.
pub(crate) fn edit(
    unit: &mut Unit,
    zone: Tz,
    id: EventId,
    recurrence_id: &str,
    scope: Scope,
    change: Edit,
) -> Result<Edited> {
    let not_found = || CoreError::OccurrenceNotFound(id);
    let rid = recurrence::recurrence_id(recurrence_id).ok_or_else(not_found)?;
    let opened = unit.find_event(id)?;
    let calendar_id = opened.calendar_id;
    let uid = unit.event_uid(id)?;
    let is_rid =
        |text: Option<&str>| text.and_then(recurrence::recurrence_id).as_ref() == Some(&rid);
    let (recurring, exception) = match opened.fields.recurrence_id.as_deref() {
        Some(own) if is_rid(Some(own)) => (
            unit.find_event_by_uid(calendar_id, &uid)?,
            Some((id, opened.fields)),
        ),
        Some(_) => return Err(not_found()),
        None => {
            let exception = unit
                .exceptions(calendar_id, &uid)?
                .into_iter()
                .find(|(_, fields)| is_rid(fields.recurrence_id.as_deref()));
            (Some((id, opened.fields)), exception)
        }
    };
    let Some((recurring_id, fields)) = recurring.filter(|(_, fields)| fields.rule.is_some()) else {
        // An Exception whose Recurring Event is gone is an Event of its own.
        let (id, fields) = exception.ok_or_else(not_found)?;
        let opened = Opened {
            exception: Some(id),
            fields,
            slot: None,
        };
        return alone(unit, zone, calendar_id, None, &opened, &rid, change);
    };
    let recurring = Recurring {
        id: recurring_id,
        uid: &uid,
        calendar_id,
        fields: &fields,
    };
    let slot = recurrence::occurrence_at(&fields, &rid)?;
    let opened = match exception {
        Some((id, fields)) => Opened {
            exception: Some(id),
            fields,
            slot,
        },
        None => Opened {
            exception: None,
            fields: EventFields {
                when: slot.clone().ok_or_else(not_found)?,
                rule: None,
                exdates: String::new(),
                recurrence_id: Some(rid.clone()),
                ..fields.clone()
            },
            slot,
        },
    };
    match scope {
        Scope::This => alone(
            unit,
            zone,
            calendar_id,
            Some(&recurring),
            &opened,
            &rid,
            change,
        ),
        Scope::ThisAndFollowing => following(unit, zone, &recurring, &opened, &rid, change),
        Scope::All => all(unit, zone, &recurring, &opened, &rid, change),
    }
}

/// Changes an Event of a Recurring Event as a whole: an Exception alone, a
/// Recurring Event with all its Occurrences, from its first date.
pub(crate) fn edit_whole(unit: &mut Unit, zone: Tz, id: EventId, change: Edit) -> Result<Edited> {
    let event = unit.find_event(id)?;
    if let Some(own) = event.fields.recurrence_id {
        return edit(unit, zone, id, &own, Scope::This, change);
    }
    let uid = unit.event_uid(id)?;
    let recurring = Recurring {
        id,
        uid: &uid,
        calendar_id: event.calendar_id,
        fields: &event.fields,
    };
    let start = event.fields.when.start.clone();
    let opened = Opened {
        exception: None,
        fields: EventFields {
            rule: None,
            exdates: String::new(),
            recurrence_id: Some(start.clone()),
            ..event.fields.clone()
        },
        slot: None,
    };
    all(unit, zone, &recurring, &opened, &start, change)
}

/// A Recurring Event: the one an Occurrence belongs to, as it was, or the
/// one it became.
struct Recurring<'a> {
    id: EventId,
    uid: &'a str,
    calendar_id: CalendarId,
    fields: &'a EventFields,
}

/// The Occurrence being changed: an Exception, or one the rule gives, as
/// the fields an Exception for it would start with.
struct Opened {
    exception: Option<EventId>,
    fields: EventFields,
    /// Its time as the rule gives it, where an Exception was before it was
    /// changed; none when the rule doesn't give it.
    slot: Option<StoredWhen>,
}

impl Recurring<'_> {
    fn changes(&self, change: Edit, opened: &Opened, zone: Tz) -> Result<Changes> {
        let repeat = recurrence::repeat_of(self.fields).0;
        Changes::of(change, opened, self.calendar_id, repeat, zone)
    }

    /// The Recurring Event that the Occurrences from `first`, the time of
    /// one of them, on become with `changes`; `rule` is their rule.
    fn changed(
        &self,
        changes: &Changes,
        first: &StoredWhen,
        rule: Option<String>,
    ) -> Result<EventFields> {
        let mut new = EventFields {
            when: first.clone(),
            ..self.fields.clone()
        };
        changes.follow(&mut new, self.fields);
        if let Some((from, to)) = &changes.when {
            new.when = recurrence::shifted(first, from, to)?;
            new.zone_pinned = false;
        }
        // The days the rule names move with its first Occurrence.
        let moved =
            |rule: Option<&str>| rule.map(|text| recurrence::rule_moved(text, first, &new.when));
        new.rule = match changes.repeat {
            None => moved(rule.as_deref()),
            Some(None) => None,
            Some(Some(repeat)) => Some(recurrence::rule_text(
                repeat,
                moved(self.fields.rule.as_deref()).as_deref(),
                &new.when,
            )?),
        };
        if recurrence::gives_none(&new) {
            return Err(CoreError::InvalidRepeat);
        }
        Ok(new)
    }

    /// Writes the Exceptions whose recurrence IDs `carried` says as the
    /// Exceptions of `new`, the Recurring Event that this one became from
    /// the Occurrence `first` on. Each keeps what it changed on its own and
    /// follows `changes` in the rest; the one that was changed, `opened`,
    /// takes them all. One whose Occurrence `new` doesn't have is deleted;
    /// with a rule FreeCal can't read, each is kept. Returns the new ID of
    /// `opened`, if it is kept.
    fn carry_exceptions(
        &self,
        unit: &mut Unit,
        carried: impl Fn(&str) -> bool,
        changes: &Changes,
        first: &StoredWhen,
        new: &Recurring,
        opened: Option<EventId>,
    ) -> Result<Option<EventId>> {
        let mut kept = None;
        for (id, fields) in unit.exceptions(self.calendar_id, self.uid)? {
            let Some(rid) = fields.recurrence_id.as_deref().filter(|rid| carried(rid)) else {
                continue;
            };
            let moved = recurrence::moved_id(rid, first, &new.fields.when);
            // The Exception's new place, none when it is gone, and `Some(None)`
            // when it can't be told.
            let slot = match (&new.fields.rule, &moved) {
                (Some(_), Some(moved)) if recurrence::readable(new.fields) => {
                    recurrence::occurrence_at(new.fields, moved)?.map(Some)
                }
                (Some(_), Some(_)) => Some(None),
                _ => None,
            };
            let (Some(moved), Some(slot)) = (moved, slot) else {
                unit.delete(id)?;
                continue;
            };
            let mut exception = if Some(id) == opened {
                changes.applied(&fields)
            } else {
                let mut exception = fields.clone();
                changes.follow(&mut exception, self.fields);
                // One at its Occurrence's time moves with it.
                if let Some(slot) = slot
                    && recurrence::occurrence_at(self.fields, rid)?.as_ref() == Some(&fields.when)
                {
                    exception.when = slot;
                }
                exception
            };
            exception.recurrence_id = Some(moved);
            let event = StoredEvent {
                calendar_id: new.calendar_id,
                fields: exception,
            };
            let written = if new.uid == self.uid {
                unit.replace(id, &event)?;
                id
            } else {
                unit.delete(id)?;
                unit.create(new.uid, &event)?
            };
            if Some(id) == opened {
                kept = Some(written);
            }
        }
        Ok(kept)
    }
}

/// Changes every Occurrence: the whole Recurring Event. Its cancelled
/// Occurrences and Exceptions move with it.
fn all(
    unit: &mut Unit,
    zone: Tz,
    recurring: &Recurring,
    opened: &Opened,
    rid: &str,
    change: Edit,
) -> Result<Edited> {
    let changes = recurring.changes(change, opened, zone)?;
    let was = &recurring.fields.when;
    let mut fields = recurring.changed(&changes, was, recurring.fields.rule.clone())?;
    fields.exdates = match fields.rule {
        Some(_) => recurrence::exdates_where(
            &recurring.fields.exdates,
            |_| true,
            |id| recurrence::moved_id(id, was, &fields.when),
        ),
        None => String::new(),
    };
    let new = Recurring {
        calendar_id: changes.calendar_id.unwrap_or(recurring.calendar_id),
        fields: &fields,
        ..*recurring
    };
    unit.replace(
        new.id,
        &StoredEvent {
            calendar_id: new.calendar_id,
            fields: fields.clone(),
        },
    )?;
    let kept = recurring.carry_exceptions(unit, |_| true, &changes, was, &new, opened.exception)?;
    now_at(
        unit,
        &new,
        kept,
        recurrence::moved_id(rid, was, &fields.when),
    )
}

/// Changes the Occurrence `rid` and every later one: the Recurring Event
/// ends before it, and they become a Recurring Event of their own, with the
/// cancelled Occurrences and Exceptions among them. From the first
/// Occurrence on, that is all of them.
fn following(
    unit: &mut Unit,
    zone: Tz,
    recurring: &Recurring,
    opened: &Opened,
    rid: &str,
    change: Edit,
) -> Result<Edited> {
    let not_found = || CoreError::OccurrenceNotFound(recurring.id);
    let slot = opened.slot.as_ref().ok_or_else(not_found)?;
    let Some(ended) = recurrence::rule_until(recurring.fields, rid) else {
        return all(unit, zone, recurring, opened, rid, change);
    };
    let changes = recurring.changes(change, opened, zone)?;
    let carried = |id: &str| recurrence::is_from(id, rid);
    let mut fields =
        recurring.changed(&changes, slot, recurrence::rule_from(recurring.fields, rid))?;
    fields.exdates = match fields.rule {
        Some(_) => recurrence::exdates_where(&recurring.fields.exdates, carried, |id| {
            recurrence::moved_id(id, slot, &fields.when)
        }),
        None => String::new(),
    };
    unit.replace(
        recurring.id,
        &StoredEvent {
            calendar_id: recurring.calendar_id,
            fields: EventFields {
                rule: Some(ended),
                exdates: recurrence::exdates_where(
                    &recurring.fields.exdates,
                    |id| !carried(id),
                    |id| Some(id.to_owned()),
                ),
                ..recurring.fields.clone()
            },
        },
    )?;
    let uid = uuid::Uuid::new_v4().to_string();
    let calendar_id = changes.calendar_id.unwrap_or(recurring.calendar_id);
    let id = unit.create(
        &uid,
        &StoredEvent {
            calendar_id,
            fields: fields.clone(),
        },
    )?;
    let new = Recurring {
        id,
        uid: &uid,
        calendar_id,
        fields: &fields,
    };
    let kept = recurring.carry_exceptions(unit, carried, &changes, slot, &new, opened.exception)?;
    now_at(
        unit,
        &new,
        kept,
        recurrence::moved_id(rid, slot, &fields.when),
    )
}

/// The Occurrence that the changed one is now: the Exception `kept`, or the
/// Occurrence `rid` of `new`, or its first when it has no such Occurrence
/// any more.
fn now_at(
    unit: &Unit,
    new: &Recurring,
    kept: Option<EventId>,
    rid: Option<String>,
) -> Result<Edited> {
    if let Some(kept) = kept {
        let event = unit.find_event(kept)?;
        let recurrence_id = event.fields.recurrence_id.clone();
        return Ok(edited(kept, event, recurrence_id));
    }
    let fields = new.fields;
    let slot = match (&fields.rule, &rid) {
        (Some(_), Some(rid)) => recurrence::occurrence_at(fields, rid)?,
        _ => None,
    };
    let recurrence_id = match &slot {
        Some(_) => rid,
        None if fields.rule.is_some() => recurrence::occurrence_at(fields, &fields.when.start)?
            .map(|_| fields.when.start.clone()),
        None => None,
    };
    let event = StoredEvent {
        calendar_id: new.calendar_id,
        fields: EventFields {
            when: slot.unwrap_or_else(|| fields.when.clone()),
            ..fields.clone()
        },
    };
    Ok(edited(new.id, event, recurrence_id))
}

/// What the user changed: each field that differs from the Occurrence as it
/// was, none for one left as it was.
struct Changes {
    title: Option<String>,
    location: Option<String>,
    description: Option<String>,
    /// The Occurrence's time before, as its rule gives it, and after. An
    /// Exception moved to 20:00 and changed to 19:00 for all Occurrences
    /// moves them all to 19:00.
    when: Option<(StoredWhen, StoredWhen)>,
    calendar_id: Option<CalendarId>,
    /// The new repeat rule of the Recurring Event, none for an Event that
    /// doesn't repeat any more.
    repeat: Option<Option<Repeat>>,
}

impl Changes {
    /// What `change` changes about the Occurrence `opened` of the Event in
    /// `calendar_id`, whose Recurring Event repeats by `repeat`.
    fn of(
        change: Edit,
        opened: &Opened,
        calendar_id: CalendarId,
        repeat: Option<Repeat>,
        zone: Tz,
    ) -> Result<Self> {
        let was = &opened.fields;
        let description = description_text(was);
        let draft = match change {
            Edit::Draft(draft) => draft,
            Edit::Reschedule(when) => EventDraft {
                calendar_id,
                title: was.title.clone(),
                when,
                location: was.location.clone(),
                description: description.clone(),
                repeat,
            },
        };
        let when = was.when.edit(draft.when, zone)?;
        let from = opened.slot.as_ref().unwrap_or(&was.when);
        let changed = |was: &String, now: String| (*was != now).then_some(now);
        Ok(Self {
            title: changed(&was.title, draft.title),
            location: changed(&was.location, draft.location),
            description: changed(&description, draft.description),
            when: (when != was.when).then(|| (from.clone(), when)),
            calendar_id: (draft.calendar_id != calendar_id).then_some(draft.calendar_id),
            repeat: (draft.repeat != repeat).then_some(draft.repeat),
        })
    }

    fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.location.is_none()
            && self.description.is_none()
            && self.when.is_none()
            && self.calendar_id.is_none()
            && self.repeat.is_none()
    }

    /// Changes the fields `fields` shares with `was`, the Event as it was
    /// before; with `was` the Occurrence that was changed itself, all of
    /// them. Leaves the time alone.
    fn follow(&self, fields: &mut EventFields, was: &EventFields) {
        if let Some(title) = &self.title
            && fields.title == was.title
        {
            fields.title.clone_from(title);
        }
        if let Some(location) = &self.location
            && fields.location == was.location
        {
            fields.location.clone_from(location);
        }
        if let Some(description) = &self.description
            && (&fields.description, fields.description_html)
                == (&was.description, was.description_html)
        {
            fields.description.clone_from(description);
            fields.description_html = false;
        }
    }

    /// The Occurrence that was changed, as it is now.
    fn applied(&self, opened: &EventFields) -> EventFields {
        let mut fields = opened.clone();
        self.follow(&mut fields, opened);
        if let Some((_, when)) = &self.when {
            fields.when = when.clone();
            // A time the user changed is theirs, no longer a pinned one.
            fields.zone_pinned = false;
        }
        fields
    }
}

/// Changes the Occurrence `opened` of an Event in `calendar_id` alone. It
/// becomes an Exception of `recurring`, its Recurring Event if it has one,
/// or an Event of its own when it moves to another Calendar, as an Exception
/// lives in its Recurring Event's Calendar.
fn alone(
    unit: &mut Unit,
    zone: Tz,
    calendar_id: CalendarId,
    recurring: Option<&Recurring>,
    opened: &Opened,
    rid: &str,
    change: Edit,
) -> Result<Edited> {
    let repeat = recurring.and_then(|recurring| recurrence::repeat_of(recurring.fields).0);
    let changes = Changes::of(change, opened, calendar_id, repeat, zone)?;
    if changes.repeat.is_some() {
        // An Exception can't repeat.
        return Err(CoreError::InvalidRepeat);
    }
    let fields = changes.applied(&opened.fields);
    match (recurring, changes.calendar_id) {
        (Some(recurring), Some(target)) => {
            if let Some(exception) = opened.exception {
                unit.delete(exception)?;
            }
            cancel(unit, recurring, rid)?;
            let event = StoredEvent {
                calendar_id: target,
                fields: EventFields {
                    recurrence_id: None,
                    ..fields
                },
            };
            let id = unit.create(&uuid::Uuid::new_v4().to_string(), &event)?;
            Ok(edited(id, event, None))
        }
        (Some(recurring), None) if changes.is_empty() && opened.exception.is_none() => Ok(edited(
            recurring.id,
            StoredEvent {
                calendar_id,
                fields: EventFields {
                    when: opened.fields.when.clone(),
                    ..recurring.fields.clone()
                },
            },
            Some(rid.to_owned()),
        )),
        (recurring, target) => {
            let event = StoredEvent {
                calendar_id: target.unwrap_or(calendar_id),
                fields,
            };
            let id = match (opened.exception, recurring) {
                (Some(id), _) => {
                    unit.replace(id, &event)?;
                    id
                }
                (None, Some(recurring)) => unit.create(recurring.uid, &event)?,
                (None, None) => {
                    unreachable!("an Occurrence without a Recurring Event is an Exception")
                }
            };
            let recurrence_id = event.fields.recurrence_id.clone();
            Ok(edited(id, event, recurrence_id))
        }
    }
}

/// Cancels the Occurrence `rid` of `recurring`.
fn cancel(unit: &mut Unit, recurring: &Recurring, rid: &str) -> Result<()> {
    unit.replace(
        recurring.id,
        &StoredEvent {
            calendar_id: recurring.calendar_id,
            fields: EventFields {
                exdates: recurrence::with_exdate(&recurring.fields.exdates, rid),
                ..recurring.fields.clone()
            },
        },
    )
}

fn edited(id: EventId, event: StoredEvent, recurrence_id: Option<String>) -> Edited {
    Edited {
        id,
        calendar_id: event.calendar_id,
        title: event.fields.title,
        when: event.fields.when,
        recurrence_id,
    }
}

/// The plain text of an Event's description, as the editor shows it.
fn description_text(fields: &EventFields) -> String {
    StructuredText::from_description(&fields.description, fields.description_html).text
}
