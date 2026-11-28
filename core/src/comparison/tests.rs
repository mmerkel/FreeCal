use std::collections::BTreeMap;

use chrono_tz::Tz;

use super::*;
use crate::ical::read::Time;
use crate::unimplemented::tests::{parameter, property};
use crate::unimplemented::{Component, Entry, EntryValue};

fn fields() -> EventFields {
    EventFields {
        title: "Yoga".to_owned(),
        when: StoredWhen {
            all_day: false,
            start: "2026-10-12T09:00:00".to_owned(),
            end: "2026-10-12T10:00:00".to_owned(),
            start_zone: Some("Europe/Berlin".to_owned()),
            end_zone: Some("Europe/Berlin".to_owned()),
        },
        location: "Room 1".to_owned(),
        description: "Bring a mat".to_owned(),
        description_html: false,
        zone_pinned: false,
        rule: None,
        exdates: String::new(),
        recurrence_id: None,
    }
}

fn content(fields: EventFields) -> EventVersionContent {
    EventVersionContent {
        fields,
        unimplemented: UnimplementedFields::default(),
    }
}

fn with(mut content: EventVersionContent, entry: Entry) -> EventVersionContent {
    content.unimplemented.push(entry);
    content
}

fn event() -> EventVersionContent {
    content(fields())
}

fn titled(title: &str) -> EventVersionContent {
    content(EventFields {
        title: title.to_owned(),
        ..fields()
    })
}

fn located(location: &str) -> EventVersionContent {
    content(EventFields {
        location: location.to_owned(),
        ..fields()
    })
}

fn starting(start: &str) -> EventVersionContent {
    content(EventFields {
        when: StoredWhen {
            start: start.to_owned(),
            ..fields().when
        },
        ..fields()
    })
}

fn alarm(trigger: &str) -> Entry {
    Entry {
        name: "ical:VALARM".to_owned(),
        value: EntryValue::Component {
            properties: vec![
                property("ACTION", Vec::new(), "DISPLAY"),
                property("TRIGGER", Vec::new(), trigger),
            ],
            components: Vec::<Component>::new(),
        },
    }
}

fn title_parameter(value: &str) -> Entry {
    Entry {
        name: "ical:SUMMARY".to_owned(),
        value: EntryValue::Parameters(vec![parameter("X-VOICE", &[value])]),
    }
}

fn versions(contents: impl IntoIterator<Item = EventVersionContent>) -> UidVersions {
    UidVersions::new(contents).expect("one version per Occurrence")
}

/// The comparison of one single Event; `None` is no version on that side.
fn single(
    base: Option<EventVersionContent>,
    ours: Option<EventVersionContent>,
    theirs: Option<EventVersionContent>,
    on_conflict: OnConflict,
) -> UidOutcome {
    compare_event(
        &versions(base),
        &versions(ours),
        &versions(theirs),
        on_conflict,
    )
}

/// The outcome of the single Event; panics on a Conflict.
fn decided(outcome: UidOutcome) -> OccurrenceOutcome {
    let UidOutcome::Decided(mut decided) = outcome else {
        panic!("a Conflict");
    };
    assert_eq!(decided.len(), 1);
    decided.remove(&None).expect("the Event")
}

fn combination(outcome: UidOutcome) -> EventVersionContent {
    match decided(outcome) {
        OccurrenceOutcome::Combined(combined) => *combined,
        other => panic!("not combined: {other:?}"),
    }
}

fn is_conflict(outcome: &UidOutcome) -> bool {
    matches!(outcome, UidOutcome::Conflict)
}

const REPORT: OnConflict = OnConflict::Report;

#[test]
fn an_event_neither_side_changed_is_unchanged() {
    let outcome = single(Some(event()), Some(event()), Some(event()), REPORT);
    assert!(matches!(decided(outcome), OccurrenceOutcome::Unchanged));
}

#[test]
fn an_edit_on_one_side_only_takes_that_side() {
    let ours = single(
        Some(event()),
        Some(titled("Pilates")),
        Some(event()),
        REPORT,
    );
    assert!(matches!(decided(ours), OccurrenceOutcome::Ours));
    let theirs = single(
        Some(event()),
        Some(event()),
        Some(titled("Pilates")),
        REPORT,
    );
    assert!(matches!(decided(theirs), OccurrenceOutcome::Theirs));
}

#[test]
fn a_creation_on_one_side_only_takes_that_side() {
    let ours = single(None, Some(event()), None, REPORT);
    assert!(matches!(decided(ours), OccurrenceOutcome::Ours));
    let theirs = single(None, None, Some(event()), REPORT);
    assert!(matches!(decided(theirs), OccurrenceOutcome::Theirs));
}

#[test]
fn a_deletion_on_one_side_only_takes_that_side() {
    let ours = single(Some(event()), None, Some(event()), REPORT);
    assert!(matches!(decided(ours), OccurrenceOutcome::Ours));
    let theirs = single(Some(event()), Some(event()), None, REPORT);
    assert!(matches!(decided(theirs), OccurrenceOutcome::Theirs));
}

#[test]
fn both_sides_to_the_same_result_agree() {
    let edited = single(
        Some(event()),
        Some(titled("Pilates")),
        Some(titled("Pilates")),
        REPORT,
    );
    assert!(matches!(decided(edited), OccurrenceOutcome::Agreed));
    let deleted = single(Some(event()), None, None, REPORT);
    assert!(matches!(decided(deleted), OccurrenceOutcome::Agreed));
    let created = single(None, Some(event()), Some(event()), REPORT);
    assert!(matches!(decided(created), OccurrenceOutcome::Agreed));
}

#[test]
fn a_deletion_against_any_change_is_a_conflict() {
    let alarm_added = with(event(), alarm("-PT5M"));
    assert!(is_conflict(&single(
        Some(event()),
        None,
        Some(alarm_added.clone()),
        REPORT
    )));
    assert!(is_conflict(&single(
        Some(event()),
        Some(alarm_added),
        None,
        REPORT
    )));
}

#[test]
fn two_different_creations_are_a_conflict() {
    let outcome = single(None, Some(titled("Pilates")), Some(located("Hall")), REPORT);
    assert!(is_conflict(&outcome));
}

#[test]
fn unimplemented_fields_changed_on_one_side_take_that_side() {
    let outcome = single(
        Some(event()),
        Some(event()),
        Some(with(event(), alarm("-PT5M"))),
        REPORT,
    );
    assert!(matches!(decided(outcome), OccurrenceOutcome::Theirs));
}

#[test]
fn unimplemented_fields_changed_differently_on_both_sides_are_a_conflict() {
    let outcome = single(
        Some(event()),
        Some(with(event(), alarm("-PT5M"))),
        Some(with(event(), alarm("-PT1H"))),
        REPORT,
    );
    assert!(is_conflict(&outcome));
}

#[test]
fn the_title_against_the_location_is_combined() {
    let combined = combination(single(
        Some(event()),
        Some(titled("Pilates")),
        Some(located("Hall")),
        REPORT,
    ));
    assert_eq!(combined.fields.title, "Pilates");
    assert_eq!(combined.fields.location, "Hall");
    assert_eq!(combined.fields.description, "Bring a mat");
}

#[test]
fn the_times_against_the_unimplemented_fields_are_combined() {
    let combined = combination(single(
        Some(event()),
        Some(starting("2026-10-12T08:00:00")),
        Some(with(event(), alarm("-PT5M"))),
        REPORT,
    ));
    let expected = with(starting("2026-10-12T08:00:00"), alarm("-PT5M"));
    assert_eq!(combined.canonical(), expected.canonical());
}

#[test]
fn the_times_against_the_title_are_a_conflict() {
    for (ours, theirs) in [
        (starting("2026-10-12T08:00:00"), titled("Pilates")),
        (titled("Pilates"), starting("2026-10-12T08:00:00")),
    ] {
        assert!(is_conflict(&single(
            Some(event()),
            Some(ours),
            Some(theirs),
            REPORT
        )));
    }
}

#[test]
fn the_same_group_changed_differently_on_both_sides_is_a_conflict() {
    let outcome = single(
        Some(event()),
        Some(titled("Pilates")),
        Some(titled("Tai chi")),
        REPORT,
    );
    assert!(is_conflict(&outcome));
}

#[test]
fn a_winning_side_decides_every_conflict() {
    let title_on_both = |win| {
        single(
            Some(event()),
            Some(titled("Pilates")),
            Some(titled("Tai chi")),
            OnConflict::Win(win),
        )
    };
    assert!(matches!(
        decided(title_on_both(Side::Ours)),
        OccurrenceOutcome::Ours
    ));
    assert!(matches!(
        decided(title_on_both(Side::Theirs)),
        OccurrenceOutcome::Theirs
    ));
    let deletion = single(
        Some(event()),
        None,
        Some(titled("Tai chi")),
        OnConflict::Win(Side::Theirs),
    );
    assert!(matches!(decided(deletion), OccurrenceOutcome::Theirs));
}

#[test]
fn a_winning_side_leaves_a_combination_combined() {
    let combined = combination(single(
        Some(event()),
        Some(titled("Pilates")),
        Some(located("Hall")),
        OnConflict::Win(Side::Theirs),
    ));
    assert_eq!(combined.fields.title, "Pilates");
}

// Recurring Events.

fn recurring() -> EventVersionContent {
    content(EventFields {
        rule: Some("FREQ=DAILY".to_owned()),
        ..fields()
    })
}

fn excluding(exdate: &str) -> EventVersionContent {
    content(EventFields {
        exdates: exdate.to_owned(),
        ..recurring().fields
    })
}

/// An Exception of [`recurring`] for the Occurrence at `wall`, in Berlin.
fn exception(wall: &str, title: &str) -> EventVersionContent {
    let original = Time::Wall(wall.parse().unwrap(), Some(Tz::Europe__Berlin));
    content(EventFields {
        title: title.to_owned(),
        recurrence_id: Some(RecurrenceId::of_original(original)),
        ..fields()
    })
}

const TUESDAY: &str = "2026-10-13T09:00:00";
const WEDNESDAY: &str = "2026-10-14T09:00:00";

fn key(wall: &str) -> Option<NormalForm> {
    let recurring = recurring().fields.when.to_imported().unwrap();
    let id = exception(wall, "").fields.recurrence_id.unwrap();
    Some(id.normal_form(Some(&recurring)))
}

fn series(exceptions: &[(&str, &str)]) -> UidVersions {
    let exceptions = exceptions
        .iter()
        .map(|(wall, title)| exception(wall, title));
    versions([recurring()].into_iter().chain(exceptions))
}

fn decided_all(outcome: UidOutcome) -> BTreeMap<Option<NormalForm>, OccurrenceOutcome> {
    match outcome {
        UidOutcome::Decided(decided) => decided,
        UidOutcome::Conflict => panic!("a Conflict"),
    }
}

#[test]
fn changes_to_different_exceptions_are_both_kept() {
    let base = series(&[(TUESDAY, "Yoga"), (WEDNESDAY, "Yoga")]);
    let ours = series(&[(TUESDAY, "Pilates"), (WEDNESDAY, "Yoga")]);
    let theirs = series(&[(TUESDAY, "Yoga"), (WEDNESDAY, "Tai chi")]);
    let decided = decided_all(compare_event(&base, &ours, &theirs, REPORT));
    assert!(matches!(decided[&None], OccurrenceOutcome::Unchanged));
    assert!(matches!(decided[&key(TUESDAY)], OccurrenceOutcome::Ours));
    assert!(matches!(
        decided[&key(WEDNESDAY)],
        OccurrenceOutcome::Theirs
    ));
}

#[test]
fn a_recurring_event_changed_on_one_side_alone_is_followed_for_every_occurrence() {
    // A change to the whole series that also took its Exception back.
    let base = series(&[(TUESDAY, "Pilates")]);
    let ours = versions([content(EventFields {
        title: "Tai chi".to_owned(),
        ..recurring().fields
    })]);
    let decided = decided_all(compare_event(&base, &ours, &base, REPORT));
    assert!(matches!(decided[&None], OccurrenceOutcome::Ours));
    assert!(matches!(decided[&key(TUESDAY)], OccurrenceOutcome::Ours));
}

#[test]
fn the_recurring_event_against_one_occurrence_is_a_conflict_of_the_uid() {
    let base = series(&[(TUESDAY, "Yoga")]);
    let whole = versions([
        content(EventFields {
            location: "Hall".to_owned(),
            ..recurring().fields
        }),
        exception(TUESDAY, "Yoga"),
    ]);
    let one = series(&[(TUESDAY, "Pilates")]);
    assert!(is_conflict(&compare_event(&base, &whole, &one, REPORT)));
    assert!(is_conflict(&compare_event(&base, &one, &whole, REPORT)));
}

#[test]
fn the_recurring_event_against_a_new_exception_is_a_conflict_of_the_uid() {
    let base = versions([recurring()]);
    let whole = versions([titled("Tai chi")]);
    let one = series(&[(TUESDAY, "Pilates")]);
    assert!(is_conflict(&compare_event(&base, &whole, &one, REPORT)));
}

#[test]
fn a_winner_takes_the_whole_uid_in_its_conflict() {
    let base = series(&[(TUESDAY, "Yoga")]);
    let whole = versions([excluding(WEDNESDAY), exception(TUESDAY, "Yoga")]);
    let one = series(&[(TUESDAY, "Pilates")]);
    let decided = decided_all(compare_event(
        &base,
        &whole,
        &one,
        OnConflict::Win(Side::Theirs),
    ));
    assert_eq!(decided.len(), 2);
    assert!(
        decided
            .values()
            .all(|outcome| matches!(outcome, OccurrenceOutcome::Theirs))
    );
}

#[test]
fn the_same_change_to_the_recurring_event_on_both_sides_leaves_exceptions_free() {
    let base = series(&[(TUESDAY, "Yoga")]);
    let renamed = || {
        content(EventFields {
            location: "Hall".to_owned(),
            ..recurring().fields
        })
    };
    let ours = versions([renamed(), exception(TUESDAY, "Pilates")]);
    let theirs = versions([renamed(), exception(TUESDAY, "Yoga")]);
    let decided = decided_all(compare_event(&base, &ours, &theirs, REPORT));
    assert!(matches!(decided[&None], OccurrenceOutcome::Agreed));
    assert!(matches!(decided[&key(TUESDAY)], OccurrenceOutcome::Ours));
}

#[test]
fn an_exception_removed_on_one_side_and_edited_on_the_other_is_a_conflict() {
    let base = series(&[(TUESDAY, "Yoga")]);
    let removed = series(&[]);
    let edited = series(&[(TUESDAY, "Pilates")]);
    assert!(is_conflict(&compare_event(
        &base, &removed, &edited, REPORT
    )));
}

#[test]
fn an_excluded_date_against_an_exception_is_a_conflict() {
    let base = series(&[(TUESDAY, "Yoga")]);
    let excluded = versions([excluding(WEDNESDAY), exception(TUESDAY, "Yoga")]);
    let edited = series(&[(TUESDAY, "Pilates")]);
    assert!(is_conflict(&compare_event(
        &base, &excluded, &edited, REPORT
    )));
}

#[test]
fn an_exceptions_title_against_an_alarm_on_it_is_combined() {
    let base = series(&[(TUESDAY, "Yoga")]);
    let ours = series(&[(TUESDAY, "Pilates")]);
    let theirs = versions([
        recurring(),
        with(exception(TUESDAY, "Yoga"), alarm("-PT5M")),
    ]);
    let mut decided = decided_all(compare_event(&base, &ours, &theirs, REPORT));
    let OccurrenceOutcome::Combined(combined) = decided.remove(&key(TUESDAY)).unwrap() else {
        panic!("not combined");
    };
    let expected = with(exception(TUESDAY, "Pilates"), alarm("-PT5M"));
    assert_eq!(combined.canonical(), expected.canonical());
}

// Entries go with their field.

#[test]
fn a_parameter_of_the_title_stays_with_its_title() {
    let base = with(event(), title_parameter("calm"));
    let ours = with(titled("Pilates"), title_parameter("loud"));
    let combined = combination(single(
        Some(base.clone()),
        Some(ours),
        Some(with(located("Hall"), title_parameter("calm"))),
        REPORT,
    ));
    let expected = with(
        content(EventFields {
            title: "Pilates".to_owned(),
            location: "Hall".to_owned(),
            ..fields()
        }),
        title_parameter("loud"),
    );
    assert_eq!(combined.canonical(), expected.canonical());
    // A parameter changed alone is a change to the title.
    let parameter_alone = single(
        Some(base.clone()),
        Some(with(event(), title_parameter("loud"))),
        Some(titled("Tai chi")),
        REPORT,
    );
    assert!(is_conflict(&parameter_alone));
}

#[test]
fn a_rule_freecal_cant_expand_against_the_times_is_a_conflict() {
    let rule = |value: &str| Entry {
        name: "ical:RRULE".to_owned(),
        value: EntryValue::Property {
            parameters: Vec::new(),
            value: value.to_owned(),
        },
    };
    let base = with(event(), rule("FREQ=DAILY;BYEASTER=1"));
    let outcome = single(
        Some(base),
        Some(with(event(), rule("FREQ=DAILY;BYEASTER=2"))),
        Some(with(
            starting("2026-10-12T08:00:00"),
            rule("FREQ=DAILY;BYEASTER=1"),
        )),
        REPORT,
    );
    assert!(is_conflict(&outcome));
}

#[test]
fn every_managed_property_has_its_group() {
    let groups = [
        ("ical:DTSTART", FieldGroup::Times),
        ("ical:DTEND", FieldGroup::Times),
        ("ical:DURATION", FieldGroup::Times),
        ("ical:RRULE", FieldGroup::Times),
        ("ical:EXDATE", FieldGroup::Times),
        ("ical:RECURRENCE-ID", FieldGroup::Times),
        ("ical:VTIMEZONE", FieldGroup::Times),
        ("ical:SUMMARY", FieldGroup::Title),
        ("ical:LOCATION", FieldGroup::Location),
        ("ical:DESCRIPTION", FieldGroup::Description),
        // Identity and what the writer makes fresh have no field.
        ("ical:UID", FieldGroup::Unimplemented),
        ("ical:SEQUENCE", FieldGroup::Unimplemented),
        // What FreeCal doesn't manage.
        ("ical:VALARM", FieldGroup::Unimplemented),
        ("ical:X-MOZ-GENERATION", FieldGroup::Unimplemented),
        ("ical:VCALENDAR", FieldGroup::Unimplemented),
        ("caldav:SUMMARY", FieldGroup::Unimplemented),
    ];
    for (name, group) in groups {
        assert_eq!(entry_group(name), group, "{name}");
    }
}

#[test]
fn two_versions_for_one_occurrence_are_refused() {
    let twice = [
        recurring(),
        exception(TUESDAY, "a"),
        exception(TUESDAY, "b"),
    ];
    assert!(UidVersions::new(twice).is_none());
}

#[test]
fn the_canonical_form_ignores_the_order_of_entries() {
    let one = with(with(event(), alarm("-PT5M")), title_parameter("calm"));
    let other = with(with(event(), title_parameter("calm")), alarm("-PT5M"));
    assert_eq!(one.canonical(), other.canonical());
}

#[test]
fn excluded_dates_compare_as_a_set() {
    let one = excluding("2026-10-13T09:00:00,2026-10-14T09:00:00");
    let other = excluding(" 2026-10-14T09:00:00,2026-10-13T09:00:00,2026-10-14T09:00:00,");
    assert_eq!(one.canonical(), other.canonical());
    assert_ne!(
        one.canonical(),
        excluding("2026-10-13T09:00:00").canonical()
    );
}
