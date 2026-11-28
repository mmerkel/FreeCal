//! The Event Comparison (ADR 0019, "Whether the server changed"; ADR 0020,
//! "Where both changed"): two sides of one UID compared with what both are
//! compared with, per Occurrence and by Field Group, with the outcome for
//! the whole UID. Incoming Sync and Undo both call [`compare_event`]; it
//! writes nothing.
//!
//! The sides are `base` (what both last agreed on), `ours` (the user's
//! current versions) and `theirs` (the fetched versions, or those a
//! taken-back Snapshot ended).
//!
//! **Changed** means a different canonical form ([`EventVersionContent`]),
//! a deletion and a creation included. Per Occurrence: changed on one side
//! only, that side's version; on both to the same result, they agree; on
//! both differently, combined by Field Group where no group changed on both
//! and the times changed against no group but the Unimplemented Fields,
//! otherwise a Conflict. A deletion against any change is a Conflict.
//!
//! **Recurring Events.** A change to the Recurring Event itself (any
//! managed field, its rule and excluded dates included), its creation or
//! its deletion counts for every Occurrence; an Exception's for its own.
//! So a change to the Recurring Event against a change to an Exception on
//! the other side is a Conflict of the whole UID, never combined: groups
//! combine within one Event Version only. A Conflict on any Occurrence is a
//! Conflict of the whole UID.
//!
//! **Entries go with their field.** What the Unimplemented Fields keep of a
//! managed property belongs to that field's group ([`entry_group`]), so a
//! combination never takes a field from one side and its entries from the
//! other, and a change to a rule FreeCal can't expand is a change to the
//! times.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use crate::event::rule::exdates_of;
use crate::event::{EventFields, NormalForm, RecurrenceId, StoredWhen};
use crate::unimplemented::{ICAL, UnimplementedFields};

/// What one Event Version holds that the comparison reads: the managed
/// fields and the Unimplemented Fields, without its row ID, Calendar or
/// Snapshots. Two are equal when their canonical forms are.
#[derive(Debug, Clone)]
pub(crate) struct EventVersionContent {
    pub(crate) fields: EventFields,
    pub(crate) unimplemented: UnimplementedFields,
}

impl EventVersionContent {
    /// The canonical form: equal for two contents exactly when they are
    /// the same version. The recurrence ID counts by its Normal Form, as
    /// obtained, and the tree by its canonical form.
    pub(crate) fn canonical(&self) -> String {
        let by_group = self.forms();
        let ByGroup {
            times,
            title,
            location,
            description,
            unimplemented,
        } = by_group;
        json!([times, title, location, description, unimplemented]).to_string()
    }

    /// The form of each Field Group: its managed fields and its entries.
    fn forms(&self) -> ByGroup<Value> {
        let Grouped {
            times,
            title,
            location,
            description,
        } = grouped(self.fields.clone());
        let entries = ByGroup::from_fn(|group| {
            let entries = self
                .unimplemented
                .entries()
                .iter()
                .filter(|entry| entry_group(&entry.name) == group)
                .cloned()
                .collect();
            UnimplementedFields::new(entries).canonical()
        });
        ByGroup {
            times: json!([times.form(), entries.times]),
            title: json!([title, entries.title]),
            location: json!([location, entries.location]),
            description: json!([description.text, description.html, entries.description]),
            unimplemented: json!([entries.unimplemented]),
        }
    }
}

/// The versions of one UID on one side, each under the Occurrence it is
/// for: none for the Event that isn't an Exception (the Recurring Event, or
/// a single Event), an Exception's recurrence ID in its Normal Form against
/// that side's Recurring Event otherwise.
#[derive(Debug, Clone, Default)]
pub(crate) struct UidVersions(BTreeMap<Option<NormalForm>, EventVersionContent>);

impl UidVersions {
    /// The versions of one UID on one side; none when two of them are for
    /// one Occurrence.
    pub(crate) fn new(versions: impl IntoIterator<Item = EventVersionContent>) -> Option<Self> {
        let versions: Vec<EventVersionContent> = versions.into_iter().collect();
        let recurring = versions
            .iter()
            .find(|version| version.fields.recurrence_id.is_none() && version.fields.rule.is_some())
            .and_then(|version| version.fields.when.to_imported().ok());
        let mut map = BTreeMap::new();
        for version in versions {
            let key = version
                .fields
                .recurrence_id
                .as_ref()
                .map(|id| id.normal_form(recurring.as_ref()));
            if map.insert(key, version).is_some() {
                return None;
            }
        }
        Some(Self(map))
    }

    fn get(&self, key: &Option<NormalForm>) -> Option<&EventVersionContent> {
        self.0.get(key)
    }
}

/// One side of the comparison that can win.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    Ours,
    Theirs,
}

/// What to do where both sides changed an Occurrence and can't be
/// combined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OnConflict {
    /// Report a Conflict of the UID.
    Report,
    /// Take that side's version, whole, wherever the comparison finds a
    /// Conflict; for "… all over the later changes" (ADR 0020).
    Win(Side),
}

/// The outcome of [`compare_event`] for one UID.
#[derive(Debug, Clone)]
pub(crate) enum UidOutcome {
    /// What each Occurrence any side has becomes, keyed as in
    /// [`UidVersions`].
    Decided(BTreeMap<Option<NormalForm>, OccurrenceOutcome>),
    /// Both sides changed the UID in ways that can't be combined.
    Conflict,
}

/// What one Occurrence becomes. Where it names a side, that side's version
/// is current, or none where that side has none.
#[derive(Debug, Clone)]
pub(crate) enum OccurrenceOutcome {
    /// Neither side changed it.
    Unchanged,
    /// Only `ours` changed it, or `ours` won.
    Ours,
    /// Only `theirs` changed it, or `theirs` won.
    Theirs,
    /// Both changed it to the same result: the caller keeps one of the two.
    Agreed,
    /// Both changed it in different Field Groups: each group from the side
    /// that changed it.
    Combined(Box<EventVersionContent>),
}

/// Compares `ours` and `theirs`, two sides of one UID, with `base`, what
/// both are compared with (module documentation).
pub(crate) fn compare_event(
    base: &UidVersions,
    ours: &UidVersions,
    theirs: &UidVersions,
    on_conflict: OnConflict,
) -> UidOutcome {
    let keys: BTreeSet<&Option<NormalForm>> = base
        .0
        .keys()
        .chain(ours.0.keys())
        .chain(theirs.0.keys())
        .collect();
    // A change one side made that the other didn't make too.
    let changed_alone = |side: &UidVersions, other: &UidVersions, key: &Option<NormalForm>| {
        !same(base.get(key), side.get(key)) && !same(other.get(key), side.get(key))
    };
    let whole = |side, other| changed_alone(side, other, &None);
    let one = |side, other| {
        keys.iter()
            .any(|key| key.is_some() && changed_alone(side, other, key))
    };
    if (whole(ours, theirs) && one(theirs, ours)) || (whole(theirs, ours) && one(ours, theirs)) {
        return conflict(&keys, on_conflict);
    }
    let mut decided = BTreeMap::new();
    for key in keys.iter().copied() {
        let outcome = occurrence(base.get(key), ours.get(key), theirs.get(key));
        let outcome = match (outcome, on_conflict) {
            (Some(outcome), _) => outcome,
            (None, OnConflict::Report) => return UidOutcome::Conflict,
            (None, OnConflict::Win(side)) => side.outcome(),
        };
        decided.insert(key.clone(), outcome);
    }
    UidOutcome::Decided(decided)
}

impl Side {
    fn outcome(self) -> OccurrenceOutcome {
        match self {
            Side::Ours => OccurrenceOutcome::Ours,
            Side::Theirs => OccurrenceOutcome::Theirs,
        }
    }
}

/// A Conflict of the whole UID: reported, or the winner's versions of
/// every Occurrence.
fn conflict(keys: &BTreeSet<&Option<NormalForm>>, on_conflict: OnConflict) -> UidOutcome {
    match on_conflict {
        OnConflict::Report => UidOutcome::Conflict,
        OnConflict::Win(side) => UidOutcome::Decided(
            keys.iter()
                .map(|key| ((*key).clone(), side.outcome()))
                .collect(),
        ),
    }
}

/// The outcome for one Occurrence; none for a Conflict.
fn occurrence(
    base: Option<&EventVersionContent>,
    ours: Option<&EventVersionContent>,
    theirs: Option<&EventVersionContent>,
) -> Option<OccurrenceOutcome> {
    if same(ours, theirs) {
        return Some(match same(base, ours) {
            true => OccurrenceOutcome::Unchanged,
            false => OccurrenceOutcome::Agreed,
        });
    }
    if same(base, ours) {
        return Some(OccurrenceOutcome::Theirs);
    }
    if same(base, theirs) {
        return Some(OccurrenceOutcome::Ours);
    }
    // Changed differently on both sides; a deletion against any change, or
    // two different creations, is a Conflict.
    combined(base?, ours?, theirs?).map(|combined| OccurrenceOutcome::Combined(Box::new(combined)))
}

fn same(one: Option<&EventVersionContent>, other: Option<&EventVersionContent>) -> bool {
    one.map(EventVersionContent::canonical) == other.map(EventVersionContent::canonical)
}

/// The combination of `ours` and `theirs`, each Field Group from the side
/// that changed it; none where both changed one group, or one the times
/// and the other another group but the Unimplemented Fields.
fn combined(
    base: &EventVersionContent,
    ours: &EventVersionContent,
    theirs: &EventVersionContent,
) -> Option<EventVersionContent> {
    let base_forms = base.forms();
    let ours_changed = base_forms.changed(&ours.forms());
    let theirs_changed = base_forms.changed(&theirs.forms());
    let times_against_shown = |times: &ByGroup<bool>, other: &ByGroup<bool>| {
        times.times && (other.title || other.location || other.description)
    };
    if ours_changed.any_with(&theirs_changed, |one, other| *one && *other)
        || times_against_shown(&ours_changed, &theirs_changed)
        || times_against_shown(&theirs_changed, &ours_changed)
    {
        return None;
    }
    let from =
        ByGroup::from_fn(
            |group| match (ours_changed.get(group), theirs_changed.get(group)) {
                (true, _) => ours,
                (_, true) => theirs,
                _ => base,
            },
        );
    Some(from.combination())
}

/// The Field Groups (`GLOSSARY.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FieldGroup {
    /// The start and end with their time zones, the repeat rule, the
    /// excluded dates and the recurrence ID.
    Times,
    Title,
    Location,
    Description,
    Unimplemented,
}

/// One value per Field Group. Built and read field by field, never with
/// `..`, so a group added to [`FieldGroup`] doesn't compile until each use
/// handles it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ByGroup<T> {
    times: T,
    title: T,
    location: T,
    description: T,
    unimplemented: T,
}

impl<T> ByGroup<T> {
    fn from_fn(mut value: impl FnMut(FieldGroup) -> T) -> Self {
        Self {
            times: value(FieldGroup::Times),
            title: value(FieldGroup::Title),
            location: value(FieldGroup::Location),
            description: value(FieldGroup::Description),
            unimplemented: value(FieldGroup::Unimplemented),
        }
    }

    fn get(&self, group: FieldGroup) -> &T {
        match group {
            FieldGroup::Times => &self.times,
            FieldGroup::Title => &self.title,
            FieldGroup::Location => &self.location,
            FieldGroup::Description => &self.description,
            FieldGroup::Unimplemented => &self.unimplemented,
        }
    }

    /// Whether `test` holds for any group's pair of values.
    fn any_with<U>(&self, other: &ByGroup<U>, test: impl Fn(&T, &U) -> bool) -> bool {
        let pairs = ByGroup::from_fn(|group| test(self.get(group), other.get(group)));
        let ByGroup {
            times,
            title,
            location,
            description,
            unimplemented,
        } = pairs;
        times || title || location || description || unimplemented
    }
}

impl ByGroup<Value> {
    /// Which groups `other` changed against these.
    fn changed(&self, other: &Self) -> ByGroup<bool> {
        ByGroup::from_fn(|group| self.get(group) != other.get(group))
    }
}

impl ByGroup<&EventVersionContent> {
    /// One content with each group from its source: the managed fields and
    /// the entries that go with them.
    fn combination(&self) -> EventVersionContent {
        let fields = ungrouped(Grouped {
            times: grouped(self.times.fields.clone()).times,
            title: grouped(self.title.fields.clone()).title,
            location: grouped(self.location.fields.clone()).location,
            description: grouped(self.description.fields.clone()).description,
        });
        let ByGroup {
            times,
            title,
            location,
            description,
            unimplemented,
        } = ByGroup::from_fn(|group| {
            self.get(group)
                .unimplemented
                .entries()
                .iter()
                .filter(move |entry| entry_group(&entry.name) == group)
                .cloned()
        });
        let entries = times
            .chain(title)
            .chain(location)
            .chain(description)
            .chain(unimplemented)
            .collect();
        EventVersionContent {
            fields,
            unimplemented: UnimplementedFields::new(entries),
        }
    }
}

/// The managed fields by Field Group.
struct Grouped {
    times: Times,
    title: String,
    location: String,
    description: Description,
}

/// The managed fields of the times.
struct Times {
    when: StoredWhen,
    zone_pinned: bool,
    rule: Option<String>,
    exdates: String,
    recurrence_id: Option<RecurrenceId>,
}

impl Times {
    fn form(&self) -> Value {
        let Self {
            when,
            zone_pinned,
            rule,
            exdates,
            recurrence_id,
        } = self;
        let StoredWhen {
            all_day,
            start,
            end,
            start_zone,
            end_zone,
        } = when;
        let recurrence_id = recurrence_id
            .as_ref()
            .map(|id| id.normal_form(None).into_string());
        // A set: their order and repeats in the stored text mean nothing.
        let exdates = exdates_of(
            exdates
                .split(',')
                .map(str::trim)
                .filter(|exdate| !exdate.is_empty())
                .map(str::to_owned),
        );
        json!([
            all_day,
            start,
            end,
            start_zone,
            end_zone,
            zone_pinned,
            rule,
            exdates,
            recurrence_id
        ])
    }
}

/// The managed fields of the description.
struct Description {
    text: String,
    html: bool,
}

/// The one function that gives each managed field its Field Group. It
/// destructures `EventFields` without `..`, so a managed field added later
/// doesn't compile until it has a group here: adding `colour` to
/// `EventFields` fails with "pattern does not mention field `colour`".
/// [`ungrouped`] puts them back.
fn grouped(fields: EventFields) -> Grouped {
    let EventFields {
        title,
        when,
        location,
        description,
        description_html,
        zone_pinned,
        rule,
        exdates,
        recurrence_id,
    } = fields;
    Grouped {
        times: Times {
            when,
            zone_pinned,
            rule,
            exdates,
            recurrence_id,
        },
        title,
        location,
        description: Description {
            text: description,
            html: description_html,
        },
    }
}

/// The managed fields of `grouped`, as [`grouped`] took them apart.
fn ungrouped(grouped: Grouped) -> EventFields {
    let Grouped {
        times:
            Times {
                when,
                zone_pinned,
                rule,
                exdates,
                recurrence_id,
            },
        title,
        location,
        description: Description { text, html },
    } = grouped;
    EventFields {
        title,
        when,
        location,
        description: text,
        description_html: html,
        zone_pinned,
        rule,
        exdates,
        recurrence_id,
    }
}

/// The Field Group of the entry of the Unimplemented Fields named `name`:
/// what they keep of a managed property goes with its field; everything
/// else, the content of what FreeCal doesn't manage, is the Unimplemented
/// Fields. Names are strings, so the compiler can't check this; a test
/// lists every managed property's name.
pub(crate) fn entry_group(name: &str) -> FieldGroup {
    let Some(name) = name.strip_prefix(ICAL) else {
        return FieldGroup::Unimplemented;
    };
    match name {
        // A rule FreeCal can't expand stays whole in the tree with its
        // excluded dates; a kept `VTIMEZONE` gives the times' zones.
        "DTSTART" | "DTEND" | "DURATION" | "RRULE" | "EXDATE" | "RECURRENCE-ID" | "VTIMEZONE" => {
            FieldGroup::Times
        }
        "SUMMARY" => FieldGroup::Title,
        "LOCATION" => FieldGroup::Location,
        "DESCRIPTION" => FieldGroup::Description,
        _ => FieldGroup::Unimplemented,
    }
}

#[cfg(test)]
mod tests;
