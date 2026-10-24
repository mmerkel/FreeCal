//! Recurrence scopes: a change to one Occurrence of a Recurring Event made
//! to that Occurrence alone, which makes it an Exception, to it and the
//! Occurrences after it, which splits the Recurring Event in two, or to all
//! of them. An Event that doesn't repeat is changed as one Occurrence alone
//! would be.
//!
//! A change is what differs between the Occurrence as it was and what the
//! user saved. Applied to the other Occurrences, it changes only the fields
//! the user changed, so that a new title leaves the times alone, and an
//! Exception keeps what was changed in it on its own: a field it shares with
//! its Recurring Event follows the change, one it changed stays.

use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use super::{EventDraft, EventFields, Loaded, Opened, StoredWhen, rule, same_recurrence_id};
use crate::journal::Unit;
use crate::store::StoredEvent;
use crate::{CalendarId, CoreError, EventId, Repeat, Result, StructuredText, When};

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
pub(crate) fn edit_occurrence(
    unit: &mut Unit,
    zone: Tz,
    id: EventId,
    recurrence_id: &str,
    scope: Scope,
    change: Edit,
) -> Result<Edited> {
    let not_found = || CoreError::OccurrenceNotFound(id);
    let rid = rule::recurrence_id(recurrence_id).ok_or_else(not_found)?;
    let opened = Opened::load(unit, id)?;
    let exception = if opened.is_exception() {
        if !same_recurrence_id(opened.fields.recurrence_id.as_deref(), Some(&rid)) {
            return Err(not_found());
        }
        Some((opened.id, opened.fields.clone()))
    } else {
        opened.loaded.exception_at(&rid).cloned()
    };
    let loaded = &opened.loaded;
    let Some((_, fields)) = loaded.recurring() else {
        // An Exception whose Recurring Event is gone is an Event of its own.
        let (id, fields) = exception.ok_or_else(not_found)?;
        let changing = Changing {
            stored: Some(id),
            fields,
            slot: None,
        };
        return alone(unit, zone, loaded, &changing, &rid, change);
    };
    let slot = rule::occurrence_at(fields, &rid)?;
    let changing = match exception {
        Some((id, fields)) => Changing {
            stored: Some(id),
            fields,
            slot,
        },
        None => Changing {
            stored: None,
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
        Scope::This => alone(unit, zone, loaded, &changing, &rid, change),
        Scope::ThisAndFollowing => following(unit, zone, loaded, &changing, &rid, change),
        Scope::All => all(unit, zone, loaded, &changing, &rid, change),
    }
}

/// Changes an Event as a whole: a single Event or an Exception alone, a
/// Recurring Event with all its Occurrences, from its first date.
pub(crate) fn edit_event(unit: &mut Unit, zone: Tz, id: EventId, change: Edit) -> Result<Edited> {
    let opened = Opened::load(unit, id)?;
    if let Some(own) = &opened.fields.recurrence_id {
        return edit_occurrence(unit, zone, id, own, Scope::This, change);
    }
    let start = opened.fields.when.start.clone();
    let changing = match opened.loaded.recurring() {
        // An Event that doesn't repeat is changed as its start's Occurrence
        // alone would be.
        None => {
            let changing = Changing {
                stored: Some(id),
                fields: opened.fields.clone(),
                slot: None,
            };
            return alone(unit, zone, &opened.loaded, &changing, &start, change);
        }
        Some(_) => Changing {
            stored: None,
            fields: EventFields {
                rule: None,
                exdates: String::new(),
                recurrence_id: Some(start.clone()),
                ..opened.fields.clone()
            },
            slot: None,
        },
    };
    all(unit, zone, &opened.loaded, &changing, &start, change)
}

/// What is being changed: a stored Event, one that doesn't repeat or an
/// Exception, or an Occurrence the rule gives, as the fields an Exception
/// for it would start with.
struct Changing {
    stored: Option<EventId>,
    fields: EventFields,
    /// Its time as the rule gives it, where an Exception was before it was
    /// changed; none when the rule doesn't give it.
    slot: Option<StoredWhen>,
}

/// The Event that the changed Occurrences became, in the Calendar
/// `calendar_id` under the UID `uid`, with no Exceptions yet.
fn became(calendar_id: CalendarId, uid: String, id: EventId, fields: EventFields) -> Loaded {
    Loaded {
        calendar_id,
        uid,
        event: Some((id, fields)),
        exceptions: Vec::new(),
    }
}

/// The changes in a scope, on Events loaded with a Recurring Event, or on
/// the Recurring Event the changed Occurrences became.
impl Loaded {
    fn changes(&self, change: Edit, changing: &Changing, zone: Tz) -> Result<Changes> {
        let repeat = rule::repeat_of(&self.recurring_event().1).0;
        Changes::of(change, changing, self.calendar_id, repeat, zone)
    }

    /// The Recurring Event that the Occurrences from `first`, the time of
    /// one of them, on become with `changes`; `rule` is their rule.
    fn changed(
        &self,
        changes: &Changes,
        first: &StoredWhen,
        rule: Option<String>,
    ) -> Result<EventFields> {
        let (_, was) = self.recurring_event();
        let mut new = EventFields {
            when: first.clone(),
            ..was.clone()
        };
        changes.follow(&mut new, was);
        if let Some((from, to)) = &changes.when {
            new.when = rule::shifted(first, from, to)?;
            new.zone_pinned = false;
        }
        // The days the rule names move with its first Occurrence.
        let moved = |rule: Option<&str>| rule.map(|text| rule::rule_moved(text, first, &new.when));
        new.rule = match changes.repeat {
            None => moved(rule.as_deref()),
            Some(None) => None,
            Some(Some(repeat)) => Some(rule::rule_text(
                repeat,
                moved(was.rule.as_deref()).as_deref(),
                &new.when,
            )?),
        };
        if rule::gives_none(&new) {
            return Err(CoreError::InvalidRepeat);
        }
        Ok(new)
    }

    /// Writes the Exceptions whose recurrence IDs `carried` says as the
    /// Exceptions of `new`, the Recurring Event that this one became from
    /// the Occurrence `first` on. Each keeps what it changed on its own and
    /// follows `changes` in the rest; the one that was changed, `changing`,
    /// takes them all. One whose Occurrence `new` doesn't have is deleted;
    /// with a rule FreeCal can't read, each is kept. Returns the new ID of
    /// `changing`, if it is kept.
    fn carry_exceptions(
        &self,
        unit: &mut Unit,
        carried: impl Fn(&str) -> bool,
        changes: &Changes,
        first: &StoredWhen,
        new: &Loaded,
        changing: Option<EventId>,
    ) -> Result<Option<EventId>> {
        let (_, was) = self.recurring_event();
        let (_, now) = new.recurring_event();
        let mut kept = None;
        for (id, fields) in &self.exceptions {
            let Some(rid) = fields.recurrence_id.as_deref().filter(|rid| carried(rid)) else {
                continue;
            };
            let moved = rule::moved_id(rid, first, &now.when);
            // The Exception's new place, none when it is gone, and `Some(None)`
            // when it can't be told.
            let slot = match (&now.rule, &moved) {
                (Some(_), Some(moved)) if rule::readable(now) => {
                    rule::occurrence_at(now, moved)?.map(Some)
                }
                (Some(_), Some(_)) => Some(None),
                _ => None,
            };
            let (Some(moved), Some(slot)) = (moved, slot) else {
                unit.delete(*id)?;
                continue;
            };
            let mut exception = if Some(*id) == changing {
                changes.applied(fields)
            } else {
                let mut exception = fields.clone();
                changes.follow(&mut exception, was);
                // One at its Occurrence's time moves with it.
                if let Some(slot) = slot
                    && rule::occurrence_at(was, rid)?.as_ref() == Some(&fields.when)
                {
                    exception.when = slot;
                }
                exception
            };
            exception.recurrence_id = Some(moved);
            let written = self.write_exception(unit, Some(*id), new, exception)?;
            if Some(*id) == changing {
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
    loaded: &Loaded,
    changing: &Changing,
    rid: &str,
    change: Edit,
) -> Result<Edited> {
    let (id, recurring) = loaded.recurring_event();
    let changes = loaded.changes(change, changing, zone)?;
    let was = &recurring.when;
    let mut fields = loaded.changed(&changes, was, recurring.rule.clone())?;
    fields.exdates = match fields.rule {
        Some(_) => rule::exdates_where(
            &recurring.exdates,
            |_| true,
            |id| rule::moved_id(id, was, &fields.when),
        ),
        None => String::new(),
    };
    let calendar_id = changes.calendar_id.unwrap_or(loaded.calendar_id);
    unit.replace(
        *id,
        &StoredEvent {
            calendar_id,
            fields: fields.clone(),
        },
    )?;
    let moved = rule::moved_id(rid, was, &fields.when);
    let new = became(calendar_id, loaded.uid.clone(), *id, fields);
    let kept = loaded.carry_exceptions(unit, |_| true, &changes, was, &new, changing.stored)?;
    now_at(unit, &new, kept, moved)
}

/// Changes the Occurrence `rid` and every later one: the Recurring Event
/// ends before it, and they become a Recurring Event of their own, with the
/// cancelled Occurrences and Exceptions among them. From the first
/// Occurrence on, that is all of them.
fn following(
    unit: &mut Unit,
    zone: Tz,
    loaded: &Loaded,
    changing: &Changing,
    rid: &str,
    change: Edit,
) -> Result<Edited> {
    let (id, recurring) = loaded.recurring_event();
    let not_found = || CoreError::OccurrenceNotFound(*id);
    let slot = changing.slot.as_ref().ok_or_else(not_found)?;
    let Some(ended) = rule::rule_until(recurring, rid) else {
        return all(unit, zone, loaded, changing, rid, change);
    };
    let changes = loaded.changes(change, changing, zone)?;
    let carried = |id: &str| rule::is_from(id, rid);
    let mut fields = loaded.changed(&changes, slot, rule::rule_from(recurring, rid))?;
    fields.exdates = match fields.rule {
        Some(_) => rule::exdates_where(&recurring.exdates, carried, |id| {
            rule::moved_id(id, slot, &fields.when)
        }),
        None => String::new(),
    };
    unit.replace(
        *id,
        &StoredEvent {
            calendar_id: loaded.calendar_id,
            fields: EventFields {
                rule: Some(ended),
                exdates: rule::exdates_where(
                    &recurring.exdates,
                    |id| !carried(id),
                    |id| Some(id.to_owned()),
                ),
                ..recurring.clone()
            },
        },
    )?;
    let uid = uuid::Uuid::new_v4().to_string();
    let calendar_id = changes.calendar_id.unwrap_or(loaded.calendar_id);
    let id = unit.create(
        &uid,
        &StoredEvent {
            calendar_id,
            fields: fields.clone(),
        },
    )?;
    let moved = rule::moved_id(rid, slot, &fields.when);
    let new = became(calendar_id, uid, id, fields);
    let kept = loaded.carry_exceptions(unit, carried, &changes, slot, &new, changing.stored)?;
    now_at(unit, &new, kept, moved)
}

/// The Occurrence that the changed one is now: the Exception `kept`, or the
/// Occurrence `rid` of `new`, or its first when it has no such Occurrence
/// any more.
fn now_at(unit: &Unit, new: &Loaded, kept: Option<EventId>, rid: Option<String>) -> Result<Edited> {
    if let Some(kept) = kept {
        let event = unit.find_event(kept)?;
        let recurrence_id = event.fields.recurrence_id.clone();
        return Ok(edited(kept, event, recurrence_id));
    }
    let (id, fields) = new.recurring_event();
    let slot = match (&fields.rule, &rid) {
        (Some(_), Some(rid)) => rule::occurrence_at(fields, rid)?,
        _ => None,
    };
    let recurrence_id = match &slot {
        Some(_) => rid,
        None if fields.rule.is_some() => {
            rule::occurrence_at(fields, &fields.when.start)?.map(|_| fields.when.start.clone())
        }
        None => None,
    };
    let event = StoredEvent {
        calendar_id: new.calendar_id,
        fields: EventFields {
            when: slot.unwrap_or_else(|| fields.when.clone()),
            ..fields.clone()
        },
    };
    Ok(edited(*id, event, recurrence_id))
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
    /// What `change` changes about the Occurrence `changing` of the Event in
    /// `calendar_id`, whose Recurring Event repeats by `repeat`.
    fn of(
        change: Edit,
        changing: &Changing,
        calendar_id: CalendarId,
        repeat: Option<Repeat>,
        zone: Tz,
    ) -> Result<Self> {
        let was = &changing.fields;
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
        let from = changing.slot.as_ref().unwrap_or(&was.when);
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
    fn applied(&self, changing: &EventFields) -> EventFields {
        let mut fields = changing.clone();
        self.follow(&mut fields, changing);
        if let Some((_, when)) = &self.when {
            fields.when = when.clone();
            // A time the user changed is theirs, no longer a pinned one.
            fields.zone_pinned = false;
        }
        fields
    }
}

/// Changes `changing` of `loaded` alone: an Event that doesn't repeat,
/// which becomes a Recurring Event when the user gives it a repeat rule, or
/// an Occurrence of the Recurring Event if there is one, which becomes an
/// Exception, or an Event of its own when it moves to another Calendar, as
/// an Exception lives in its Recurring Event's Calendar.
fn alone(
    unit: &mut Unit,
    zone: Tz,
    loaded: &Loaded,
    changing: &Changing,
    rid: &str,
    change: Edit,
) -> Result<Edited> {
    let calendar_id = loaded.calendar_id;
    let recurring = loaded.recurring();
    let repeat = recurring.and_then(|(_, fields)| rule::repeat_of(fields).0);
    let changes = Changes::of(change, changing, calendar_id, repeat, zone)?;
    let mut fields = changes.applied(&changing.fields);
    match changes.repeat {
        None => {}
        Some(Some(repeat)) if recurring.is_none() && changing.fields.recurrence_id.is_none() => {
            fields.rule = Some(rule::rule_text(repeat, None, &fields.when)?);
        }
        // An Exception can't repeat.
        Some(_) => return Err(CoreError::InvalidRepeat),
    }
    match (recurring, changes.calendar_id) {
        (Some(_), Some(target)) => {
            loaded.cancel(unit, rid)?;
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
        (Some((id, recurring)), None) if changes.is_empty() && changing.stored.is_none() => {
            Ok(edited(
                *id,
                StoredEvent {
                    calendar_id,
                    fields: EventFields {
                        when: changing.fields.when.clone(),
                        ..recurring.clone()
                    },
                },
                Some(rid.to_owned()),
            ))
        }
        (_, target) => {
            let event = StoredEvent {
                calendar_id: target.unwrap_or(calendar_id),
                fields,
            };
            // Only a Recurring Event gives an Occurrence that isn't stored.
            let id = match changing.stored {
                Some(id) => {
                    unit.replace(id, &event)?;
                    id
                }
                None => unit.create(&loaded.uid, &event)?,
            };
            let recurrence_id = event.fields.recurrence_id.clone();
            Ok(edited(id, event, recurrence_id))
        }
    }
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
