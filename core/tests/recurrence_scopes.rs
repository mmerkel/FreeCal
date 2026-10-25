//! Changing an Occurrence of a Recurring Event with a recurrence scope: this
//! event, this and following events, or all events.

mod support;

use freecal_core::{
    CalendarId, Core, CoreError, EventDraft, EventId, ExceptionCounts, ExistingEvents, Frequency,
    ImportFile, Occurrence, Repeat, RepeatEnd, Scope, When,
};
use support::{Harness, all_day, calendar, draft, ics, on, timed, weekly, yoga};

fn occurrences(core: &Core, from: &str, to: &str) -> Vec<Occurrence> {
    core.occurrences(from.parse().unwrap(), to.parse().unwrap())
        .unwrap()
}

/// Each Occurrence in October and November as its title and time.
fn shown(core: &Core) -> Vec<(String, When)> {
    occurrences(core, "2026-10-01", "2026-12-01")
        .into_iter()
        .map(|occurrence| (occurrence.title, occurrence.when))
        .collect()
}

fn rid(occurrence: &Occurrence) -> &str {
    occurrence.recurrence_id.as_deref().unwrap()
}

#[test]
fn moving_this_event_changes_only_that_occurrence() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let id = yoga(&core, home.id, RepeatEnd::Count { count: 3 });
    let second = on(&core, "2026-10-12");

    let moved = core
        .reschedule_occurrence(
            id,
            rid(&second),
            Scope::This,
            false,
            timed("2026-10-13T08:00:00", "2026-10-13T09:00:00"),
        )
        .unwrap();

    assert_eq!(
        shown(&core),
        [
            (
                "Yoga".into(),
                timed("2026-10-05T18:00:00", "2026-10-05T19:00:00")
            ),
            (
                "Yoga".into(),
                timed("2026-10-13T08:00:00", "2026-10-13T09:00:00")
            ),
            (
                "Yoga".into(),
                timed("2026-10-19T18:00:00", "2026-10-19T19:00:00")
            ),
        ]
    );
    let exception = on(&core, "2026-10-13");
    assert_ne!(exception.event_id, id);
    assert_eq!(moved.value, exception);
    assert_eq!(
        core.event(exception.event_id).unwrap().recurrence_id,
        second.recurrence_id
    );

    core.undo_unit(moved.unit.unwrap()).unwrap();
    assert_eq!(on(&core, "2026-10-12"), second);
}

/// The draft the editor starts from for an Occurrence: its Event's fields
/// with the Occurrence's time.
fn draft_of(core: &Core, occurrence: &Occurrence) -> EventDraft {
    let event = core.event(occurrence.event_id).unwrap();
    EventDraft {
        calendar_id: event.calendar_id,
        title: event.title,
        when: occurrence.when,
        location: event.location.text,
        description: event.description.text,
        repeat: event.repeat,
    }
}

#[test]
fn editing_this_event_again_changes_its_exception_and_keeps_the_rest() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let id = yoga(&core, home.id, RepeatEnd::Count { count: 3 });
    let second = on(&core, "2026-10-12");
    let renamed = core
        .edit_occurrence(
            id,
            rid(&second),
            Scope::This,
            false,
            EventDraft {
                title: "Yoga outside".into(),
                location: "Park".into(),
                ..draft_of(&core, &second)
            },
        )
        .unwrap()
        .value;

    let moved = core
        .edit_occurrence(
            renamed.event_id,
            rid(&renamed),
            Scope::This,
            false,
            EventDraft {
                when: timed("2026-10-12T17:00:00", "2026-10-12T18:00:00"),
                ..draft_of(&core, &renamed)
            },
        )
        .unwrap()
        .value;

    assert_eq!(moved.event_id, renamed.event_id);
    assert_eq!(
        shown(&core),
        [
            (
                "Yoga".into(),
                timed("2026-10-05T18:00:00", "2026-10-05T19:00:00")
            ),
            (
                "Yoga outside".into(),
                timed("2026-10-12T17:00:00", "2026-10-12T18:00:00")
            ),
            (
                "Yoga".into(),
                timed("2026-10-19T18:00:00", "2026-10-19T19:00:00")
            ),
        ]
    );
    let exception = core.event(moved.event_id).unwrap();
    assert_eq!(exception.location.text, "Park");
    // An Exception shows the repeat rule of its Recurring Event.
    assert_eq!(
        exception.repeat,
        Some(weekly(RepeatEnd::Count { count: 3 }))
    );
}

#[test]
fn this_event_in_another_calendar_leaves_the_recurring_event() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let work = calendar(&core, "Work");
    let id = yoga(&core, home.id, RepeatEnd::Count { count: 3 });
    let second = on(&core, "2026-10-12");

    let moved = core
        .edit_occurrence(
            id,
            rid(&second),
            Scope::This,
            false,
            EventDraft {
                calendar_id: work.id,
                ..draft_of(&core, &second)
            },
        )
        .unwrap()
        .value;

    assert_eq!(
        (moved.calendar_id, moved.recurrence_id.as_deref()),
        (work.id, None)
    );
    assert_eq!(on(&core, "2026-10-12"), moved);
    let single = core.event(moved.event_id).unwrap();
    assert_eq!((single.repeat, single.recurrence_id), (None, None));
    assert_eq!(shown(&core).len(), 3);
}

#[test]
fn this_event_cant_change_the_repeat_rule() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let id = yoga(&core, home.id, RepeatEnd::Count { count: 3 });
    let second = on(&core, "2026-10-12");

    let refused = core.edit_occurrence(
        id,
        rid(&second),
        Scope::This,
        false,
        EventDraft {
            repeat: None,
            ..draft_of(&core, &second)
        },
    );

    assert!(matches!(refused, Err(CoreError::InvalidRepeat)));
    assert_eq!(shown(&core).len(), 3);
}

#[test]
fn a_recurrence_id_the_recurring_event_doesnt_have_is_refused() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let id = yoga(&core, home.id, RepeatEnd::Count { count: 3 });
    let when = timed("2026-10-13T18:00:00", "2026-10-13T19:00:00");

    for scope in [Scope::This, Scope::ThisAndFollowing, Scope::All] {
        for recurrence_id in ["2026-10-13T18:00:00", "2026-10-26T18:00:00", "<b>"] {
            assert!(matches!(
                core.reschedule_occurrence(id, recurrence_id, scope, false, when),
                Err(CoreError::OccurrenceNotFound(_))
            ));
        }
    }
}

/// Yoga as [`yoga`] with four Occurrences, the second renamed, the third
/// moved to 20:00 and the fourth cancelled.
fn yoga_with_exceptions(core: &Core, calendar_id: CalendarId) -> EventId {
    let id = yoga(core, calendar_id, RepeatEnd::Count { count: 4 });
    let second = on(core, "2026-10-12");
    core.edit_occurrence(
        id,
        rid(&second),
        Scope::This,
        false,
        EventDraft {
            title: "Yoga outside".into(),
            ..draft_of(core, &second)
        },
    )
    .unwrap();
    let third = on(core, "2026-10-19");
    core.reschedule_occurrence(
        id,
        rid(&third),
        Scope::This,
        false,
        timed("2026-10-19T20:00:00", "2026-10-19T21:00:00"),
    )
    .unwrap();
    let fourth = on(core, "2026-10-26");
    core.delete_occurrence(id, rid(&fourth)).unwrap();
    id
}

#[test]
fn moving_all_events_moves_every_occurrence_by_as_much_and_keeps_the_exceptions() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let id = yoga_with_exceptions(&core, home.id);
    let first = on(&core, "2026-10-05");

    // A day later and an hour earlier.
    let moved = core
        .reschedule_occurrence(
            id,
            rid(&first),
            Scope::All,
            false,
            timed("2026-10-06T17:00:00", "2026-10-06T18:00:00"),
        )
        .unwrap();

    assert_eq!(
        shown(&core),
        [
            (
                "Yoga".into(),
                timed("2026-10-06T17:00:00", "2026-10-06T18:00:00")
            ),
            (
                "Yoga outside".into(),
                timed("2026-10-13T17:00:00", "2026-10-13T18:00:00")
            ),
            // Moved on its own, so it stays where it was moved to.
            (
                "Yoga".into(),
                timed("2026-10-19T20:00:00", "2026-10-19T21:00:00")
            ),
        ]
    );
    assert_eq!(moved.value, on(&core, "2026-10-06"));

    core.undo_unit(moved.unit.unwrap()).unwrap();
    assert_eq!(shown(&core)[0].1, first.when);
    assert_eq!(shown(&core).len(), 3);
}

#[test]
fn editing_all_events_from_an_exception_changes_only_what_was_changed() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let id = yoga_with_exceptions(&core, home.id);
    let outside = on(&core, "2026-10-12");

    let edited = core
        .edit_occurrence(
            outside.event_id,
            rid(&outside),
            Scope::All,
            false,
            EventDraft {
                location: "Gym".into(),
                // 30 minutes longer.
                when: timed("2026-10-12T18:00:00", "2026-10-12T19:30:00"),
                ..draft_of(&core, &outside)
            },
        )
        .unwrap()
        .value;

    assert_eq!(
        shown(&core),
        [
            (
                "Yoga".into(),
                timed("2026-10-05T18:00:00", "2026-10-05T19:30:00")
            ),
            (
                "Yoga outside".into(),
                timed("2026-10-12T18:00:00", "2026-10-12T19:30:00")
            ),
            (
                "Yoga".into(),
                timed("2026-10-19T20:00:00", "2026-10-19T21:00:00")
            ),
        ]
    );
    assert_eq!(edited, on(&core, "2026-10-12"));
    for occurrence in occurrences(&core, "2026-10-01", "2026-12-01") {
        assert_eq!(
            core.event(occurrence.event_id).unwrap().location.text,
            "Gym"
        );
    }
    assert_eq!(core.event(id).unwrap().title, "Yoga");
}

#[test]
fn all_events_into_another_calendar_takes_the_exceptions_along() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let work = calendar(&core, "Work");
    yoga_with_exceptions(&core, home.id);
    let before = shown(&core);
    let third = on(&core, "2026-10-19");

    core.edit_occurrence(
        third.event_id,
        rid(&third),
        Scope::All,
        false,
        EventDraft {
            calendar_id: work.id,
            ..draft_of(&core, &third)
        },
    )
    .unwrap();

    assert_eq!(shown(&core), before);
    for occurrence in occurrences(&core, "2026-10-01", "2026-12-01") {
        assert_eq!(occurrence.calendar_id, work.id);
    }
}

#[test]
fn a_new_repeat_rule_for_all_events_keeps_the_exceptions_on_dates_it_still_has() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let id = yoga_with_exceptions(&core, home.id);
    let first = on(&core, "2026-10-05");

    // Every two weeks: 5 and 19 October, 2 and 16 November.
    core.edit_occurrence(
        id,
        rid(&first),
        Scope::All,
        false,
        EventDraft {
            repeat: Some(Repeat {
                interval: 2,
                ..weekly(RepeatEnd::Count { count: 4 })
            }),
            ..draft_of(&core, &first)
        },
    )
    .unwrap();

    assert_eq!(
        shown(&core),
        [
            (
                "Yoga".into(),
                timed("2026-10-05T18:00:00", "2026-10-05T19:00:00")
            ),
            (
                "Yoga".into(),
                timed("2026-10-19T20:00:00", "2026-10-19T21:00:00")
            ),
            (
                "Yoga".into(),
                timed("2026-11-02T18:00:00", "2026-11-02T19:00:00")
            ),
            (
                "Yoga".into(),
                timed("2026-11-16T18:00:00", "2026-11-16T19:00:00")
            ),
        ]
    );
}

#[test]
fn all_events_without_a_repeat_rule_is_the_first_event_alone() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let id = yoga_with_exceptions(&core, home.id);
    let third = on(&core, "2026-10-19");

    let single = core
        .edit_occurrence(
            third.event_id,
            rid(&third),
            Scope::All,
            false,
            EventDraft {
                repeat: None,
                ..draft_of(&core, &third)
            },
        )
        .unwrap()
        .value;

    assert_eq!(
        shown(&core),
        [(
            "Yoga".into(),
            timed("2026-10-05T18:00:00", "2026-10-05T19:00:00")
        )]
    );
    assert_eq!((single.event_id, single.recurrence_id), (id, None));
}

#[test]
fn moving_this_and_following_events_splits_the_recurring_event() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let id = yoga_with_exceptions(&core, home.id);
    let second = on(&core, "2026-10-12");

    // From the second on, a day later and an hour earlier.
    let moved = core
        .reschedule_occurrence(
            second.event_id,
            rid(&second),
            Scope::ThisAndFollowing,
            false,
            timed("2026-10-13T17:00:00", "2026-10-13T18:00:00"),
        )
        .unwrap();

    assert_eq!(
        shown(&core),
        [
            (
                "Yoga".into(),
                timed("2026-10-05T18:00:00", "2026-10-05T19:00:00")
            ),
            (
                "Yoga outside".into(),
                timed("2026-10-13T17:00:00", "2026-10-13T18:00:00")
            ),
            (
                "Yoga".into(),
                timed("2026-10-19T20:00:00", "2026-10-19T21:00:00")
            ),
        ]
    );
    assert_eq!(moved.value, on(&core, "2026-10-13"));
    let first = core.event(id).unwrap();
    assert_eq!(
        first.repeat.unwrap().end,
        RepeatEnd::Until {
            date: "2026-10-05".parse().unwrap()
        }
    );
    let following = occurrences(&core, "2026-10-10", "2026-12-01");
    let new = following[0].event_id;
    assert_ne!(new, id);
    assert_eq!(
        core.event(new).unwrap().repeat.map(|repeat| repeat.end),
        Some(RepeatEnd::Count { count: 3 })
    );

    core.undo_unit(moved.unit.unwrap()).unwrap();
    assert_eq!(on(&core, "2026-10-12"), second);
    assert_eq!(shown(&core).len(), 3);
}

#[test]
fn editing_this_and_following_events_changes_only_those() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let work = calendar(&core, "Work");
    yoga(&core, home.id, RepeatEnd::Never);
    let third = on(&core, "2026-10-19");

    core.edit_occurrence(
        third.event_id,
        rid(&third),
        Scope::ThisAndFollowing,
        false,
        EventDraft {
            title: "Pilates".into(),
            calendar_id: work.id,
            ..draft_of(&core, &third)
        },
    )
    .unwrap();

    let titles: Vec<_> = occurrences(&core, "2026-10-01", "2026-11-10")
        .into_iter()
        .map(|occurrence| (occurrence.title, occurrence.calendar_id))
        .collect();
    assert_eq!(
        titles,
        [
            ("Yoga".into(), home.id),
            ("Yoga".into(), home.id),
            ("Pilates".into(), work.id),
            ("Pilates".into(), work.id),
            ("Pilates".into(), work.id),
            ("Pilates".into(), work.id),
        ]
    );
}

#[test]
fn this_and_following_from_the_first_occurrence_changes_all_events() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let id = yoga(&core, home.id, RepeatEnd::Count { count: 3 });
    let first = on(&core, "2026-10-05");

    let moved = core
        .reschedule_occurrence(
            id,
            rid(&first),
            Scope::ThisAndFollowing,
            false,
            timed("2026-10-05T17:00:00", "2026-10-05T18:00:00"),
        )
        .unwrap()
        .value;

    assert_eq!(moved.event_id, id);
    assert_eq!(
        occurrences(&core, "2026-10-01", "2026-12-01")
            .into_iter()
            .map(|occurrence| (occurrence.event_id, occurrence.when))
            .collect::<Vec<_>>(),
        [
            (id, timed("2026-10-05T17:00:00", "2026-10-05T18:00:00")),
            (id, timed("2026-10-12T17:00:00", "2026-10-12T18:00:00")),
            (id, timed("2026-10-19T17:00:00", "2026-10-19T18:00:00")),
        ]
    );
}

#[test]
fn editing_the_whole_recurring_event_takes_its_exceptions_along_or_removes_them() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let work = calendar(&core, "Work");
    let id = yoga_with_exceptions(&core, home.id);
    let event = core.event(id).unwrap();
    let whole = EventDraft {
        calendar_id: event.calendar_id,
        title: event.title,
        when: event.when,
        location: event.location.text,
        description: event.description.text,
        repeat: event.repeat,
    };

    core.edit_event(
        id,
        EventDraft {
            calendar_id: work.id,
            ..whole.clone()
        },
    )
    .unwrap();
    assert!(
        occurrences(&core, "2026-10-01", "2026-12-01")
            .iter()
            .all(|occurrence| occurrence.calendar_id == work.id)
    );

    core.edit_event(
        id,
        EventDraft {
            calendar_id: work.id,
            repeat: None,
            ..whole
        },
    )
    .unwrap();
    assert_eq!(
        shown(&core),
        [(
            "Yoga".into(),
            timed("2026-10-05T18:00:00", "2026-10-05T19:00:00")
        )]
    );
}

#[test]
fn an_all_day_recurring_event_splits_with_its_cancelled_days() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let id = core
        .create_event(EventDraft {
            repeat: Some(Repeat {
                frequency: Frequency::Daily,
                interval: 1,
                end: RepeatEnd::Count { count: 5 },
            }),
            ..draft(home.id, "Course", all_day("2026-10-05", "2026-10-06"))
        })
        .unwrap()
        .value
        .id;
    core.delete_occurrence(id, "2026-10-08").unwrap();
    let third = on(&core, "2026-10-07");

    // From the third day on, two days later.
    core.reschedule_occurrence(
        id,
        rid(&third),
        Scope::ThisAndFollowing,
        false,
        all_day("2026-10-09", "2026-10-10"),
    )
    .unwrap();

    let days: Vec<_> = shown(&core).into_iter().map(|(_, when)| when).collect();
    assert_eq!(
        days,
        [
            all_day("2026-10-05", "2026-10-06"),
            all_day("2026-10-06", "2026-10-07"),
            all_day("2026-10-09", "2026-10-10"),
            all_day("2026-10-11", "2026-10-12"),
        ]
    );
}

fn import(core: &Core, calendar_id: CalendarId, body: &str) {
    let file = ImportFile::new("events.ics", ics(body));
    let preview = core.preview_import(&file, calendar_id).unwrap();
    core.run_import(&preview, ExistingEvents::Update).unwrap();
}

#[test]
fn moving_all_events_to_another_weekday_moves_the_weekdays_of_the_rule() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    // Mondays and Wednesdays at 18:00 from 5 October, six times.
    import(
        &core,
        home.id,
        "BEGIN:VEVENT\nUID:yoga\nSUMMARY:Yoga\n\
         DTSTART;TZID=Europe/Berlin:20261005T180000\nDTEND;TZID=Europe/Berlin:20261005T190000\n\
         RRULE:FREQ=WEEKLY;BYDAY=MO,WE;COUNT=6\nEND:VEVENT\n",
    );
    let wednesday = on(&core, "2026-10-14");
    core.edit_occurrence(
        wednesday.event_id,
        rid(&wednesday),
        Scope::This,
        false,
        EventDraft {
            title: "Yoga outside".into(),
            ..draft_of(&core, &wednesday)
        },
    )
    .unwrap();
    let cancelled = on(&core, "2026-10-19");
    core.delete_occurrence(cancelled.event_id, rid(&cancelled))
        .unwrap();
    let first = on(&core, "2026-10-05");

    // A day later: Tuesdays and Thursdays.
    core.reschedule_occurrence(
        first.event_id,
        rid(&first),
        Scope::All,
        false,
        timed("2026-10-06T18:00:00", "2026-10-06T19:00:00"),
    )
    .unwrap();

    let days: Vec<_> = shown(&core)
        .into_iter()
        .map(|(title, when)| match when {
            When::Timed { start, .. } => (title, start.date().to_string()),
            When::AllDay { start, .. } => (title, start.to_string()),
        })
        .collect();
    assert_eq!(
        days,
        [
            ("Yoga".into(), "2026-10-06".into()),
            ("Yoga".into(), "2026-10-08".into()),
            ("Yoga".into(), "2026-10-13".into()),
            ("Yoga outside".into(), "2026-10-15".into()),
            ("Yoga".into(), "2026-10-22".into()),
        ]
    );
}

#[test]
fn a_rule_freecal_cant_read_keeps_its_exceptions_through_an_edit() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let id = yoga(&core, home.id, RepeatEnd::Count { count: 3 });
    let second = on(&core, "2026-10-12");
    core.reschedule_occurrence(
        id,
        rid(&second),
        Scope::This,
        false,
        timed("2026-10-12T20:00:00", "2026-10-12T21:00:00"),
    )
    .unwrap();
    drop(core);
    harness.arrange_local_store(
        "UPDATE event SET rrule = 'FREQ=WEEKLY;BYDAY=<img src=x>' WHERE rrule IS NOT NULL",
    );
    let core = harness.open();
    let event = core.event(id).unwrap();

    core.edit_event(
        id,
        EventDraft {
            calendar_id: event.calendar_id,
            title: "Pilates".into(),
            when: event.when,
            location: event.location.text,
            description: event.description.text,
            repeat: event.repeat,
        },
    )
    .unwrap();

    assert_eq!(
        shown(&core),
        [
            (
                "Pilates".into(),
                timed("2026-10-05T18:00:00", "2026-10-05T19:00:00")
            ),
            // Kept at its own time, with the title it shares.
            (
                "Pilates".into(),
                timed("2026-10-12T20:00:00", "2026-10-12T21:00:00")
            ),
        ]
    );
}

#[test]
fn all_events_at_a_time_set_from_a_moved_exception_are_at_that_time() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    yoga_with_exceptions(&core, home.id);
    // Moved on its own from 18:00 to 20:00.
    let moved = on(&core, "2026-10-19");

    let edited = core
        .reschedule_occurrence(
            moved.event_id,
            rid(&moved),
            Scope::All,
            false,
            timed("2026-10-19T19:00:00", "2026-10-19T20:00:00"),
        )
        .unwrap()
        .value;

    assert_eq!(
        shown(&core),
        [
            (
                "Yoga".into(),
                timed("2026-10-05T19:00:00", "2026-10-05T20:00:00")
            ),
            (
                "Yoga outside".into(),
                timed("2026-10-12T19:00:00", "2026-10-12T20:00:00")
            ),
            (
                "Yoga".into(),
                timed("2026-10-19T19:00:00", "2026-10-19T20:00:00")
            ),
        ]
    );
    assert_eq!(edited, on(&core, "2026-10-19"));
}

#[test]
fn exception_counts_are_the_exceptions_with_values_of_their_own_in_each_scope() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let id = yoga_with_exceptions(&core, home.id);
    let first = on(&core, "2026-10-05");
    let outside = on(&core, "2026-10-12");
    let moved = on(&core, "2026-10-19");

    // The renamed one and the moved one; the cancelled one has no values.
    assert_eq!(
        core.exception_counts(id, rid(&first)).unwrap(),
        ExceptionCounts {
            this_and_following: 2,
            all: 2
        }
    );
    // The Exception being changed takes the change anyway.
    assert_eq!(
        core.exception_counts(outside.event_id, rid(&outside))
            .unwrap(),
        ExceptionCounts {
            this_and_following: 1,
            all: 1
        }
    );
    assert_eq!(
        core.exception_counts(moved.event_id, rid(&moved)).unwrap(),
        ExceptionCounts {
            this_and_following: 0,
            all: 1
        }
    );
}

#[test]
fn an_exception_with_the_values_of_its_recurring_event_isnt_counted() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    // The second Occurrence as it was, and the third with a description.
    import(
        &core,
        home.id,
        "BEGIN:VEVENT\nUID:yoga\nSUMMARY:Yoga\n\
         DTSTART;TZID=Europe/Berlin:20261005T180000\nDTEND;TZID=Europe/Berlin:20261005T190000\n\
         RRULE:FREQ=WEEKLY;COUNT=3\nEND:VEVENT\n\
         BEGIN:VEVENT\nUID:yoga\nSUMMARY:Yoga\nRECURRENCE-ID;TZID=Europe/Berlin:20261012T180000\n\
         DTSTART;TZID=Europe/Berlin:20261012T180000\nDTEND;TZID=Europe/Berlin:20261012T190000\n\
         END:VEVENT\n\
         BEGIN:VEVENT\nUID:yoga\nSUMMARY:Yoga\nDESCRIPTION:Bring a mat\n\
         RECURRENCE-ID;TZID=Europe/Berlin:20261019T180000\n\
         DTSTART;TZID=Europe/Berlin:20261019T180000\nDTEND;TZID=Europe/Berlin:20261019T190000\n\
         END:VEVENT\n",
    );
    let first = on(&core, "2026-10-05");

    assert_eq!(
        core.exception_counts(first.event_id, rid(&first)).unwrap(),
        ExceptionCounts {
            this_and_following: 1,
            all: 1
        }
    );
}

#[test]
fn exceptions_too_for_all_events_take_the_change_and_shift_by_as_much() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let id = yoga_with_exceptions(&core, home.id);
    let first = on(&core, "2026-10-05");

    // Renamed, an hour later, and 30 minutes longer.
    let edited = core
        .edit_occurrence(
            id,
            rid(&first),
            Scope::All,
            true,
            EventDraft {
                title: "Pilates".into(),
                when: timed("2026-10-05T19:00:00", "2026-10-05T20:30:00"),
                ..draft_of(&core, &first)
            },
        )
        .unwrap();

    assert_eq!(
        shown(&core),
        [
            (
                "Pilates".into(),
                timed("2026-10-05T19:00:00", "2026-10-05T20:30:00")
            ),
            (
                "Pilates".into(),
                timed("2026-10-12T19:00:00", "2026-10-12T20:30:00")
            ),
            (
                "Pilates".into(),
                timed("2026-10-19T21:00:00", "2026-10-19T22:30:00")
            ),
        ]
    );

    core.undo_unit(edited.unit.unwrap()).unwrap();
    let titles: Vec<_> = shown(&core).into_iter().map(|(title, _)| title).collect();
    assert_eq!(titles, ["Yoga", "Yoga outside", "Yoga"]);
    assert_eq!(
        on(&core, "2026-10-19").when,
        timed("2026-10-19T20:00:00", "2026-10-19T21:00:00")
    );
}

#[test]
fn exceptions_too_for_this_and_following_changes_only_those_from_it_on() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    yoga_with_exceptions(&core, home.id);
    // One before the change, with a location of its own.
    let first = on(&core, "2026-10-05");
    core.edit_occurrence(
        first.event_id,
        rid(&first),
        Scope::This,
        false,
        EventDraft {
            location: "Park".into(),
            ..draft_of(&core, &first)
        },
    )
    .unwrap();
    let outside = on(&core, "2026-10-12");

    // From the renamed one on: another location, an hour earlier.
    core.edit_occurrence(
        outside.event_id,
        rid(&outside),
        Scope::ThisAndFollowing,
        true,
        EventDraft {
            location: "Gym".into(),
            when: timed("2026-10-12T17:00:00", "2026-10-12T18:00:00"),
            ..draft_of(&core, &outside)
        },
    )
    .unwrap();

    let shown: Vec<_> = occurrences(&core, "2026-10-01", "2026-12-01")
        .into_iter()
        .map(|occurrence| {
            let location = core.event(occurrence.event_id).unwrap().location.text;
            (occurrence.title, occurrence.when, location)
        })
        .collect();
    assert_eq!(
        shown,
        [
            (
                "Yoga".into(),
                timed("2026-10-05T18:00:00", "2026-10-05T19:00:00"),
                "Park".into()
            ),
            (
                "Yoga outside".into(),
                timed("2026-10-12T17:00:00", "2026-10-12T18:00:00"),
                "Gym".into()
            ),
            (
                "Yoga".into(),
                timed("2026-10-19T19:00:00", "2026-10-19T20:00:00"),
                "Gym".into()
            ),
        ]
    );
}

#[test]
fn exceptions_too_unchecked_leaves_their_own_values() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    yoga_with_exceptions(&core, home.id);
    let outside = on(&core, "2026-10-12");

    core.edit_occurrence(
        outside.event_id,
        rid(&outside),
        Scope::ThisAndFollowing,
        false,
        EventDraft {
            title: "Pilates".into(),
            when: timed("2026-10-12T17:00:00", "2026-10-12T18:00:00"),
            ..draft_of(&core, &outside)
        },
    )
    .unwrap();

    assert_eq!(
        shown(&core),
        [
            (
                "Yoga".into(),
                timed("2026-10-05T18:00:00", "2026-10-05T19:00:00")
            ),
            (
                "Pilates".into(),
                timed("2026-10-12T17:00:00", "2026-10-12T18:00:00")
            ),
            // The title it shared follows; the time it was moved to stays.
            (
                "Pilates".into(),
                timed("2026-10-19T20:00:00", "2026-10-19T21:00:00")
            ),
        ]
    );
}

#[test]
fn an_all_day_recurring_event_made_timed_takes_its_exceptions_and_cancelled_days_along() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let id = core
        .create_event(EventDraft {
            repeat: Some(Repeat {
                frequency: Frequency::Daily,
                interval: 1,
                end: RepeatEnd::Count { count: 4 },
            }),
            ..draft(home.id, "Course", all_day("2026-10-05", "2026-10-06"))
        })
        .unwrap()
        .value
        .id;
    let second = on(&core, "2026-10-06");
    core.edit_occurrence(
        id,
        rid(&second),
        Scope::This,
        false,
        EventDraft {
            title: "Course outside".into(),
            ..draft_of(&core, &second)
        },
    )
    .unwrap();
    core.delete_occurrence(id, "2026-10-07").unwrap();
    let first = on(&core, "2026-10-05");

    // It gains the Event Time Zone: the Display Time Zone.
    core.edit_occurrence(
        id,
        rid(&first),
        Scope::All,
        false,
        EventDraft {
            when: timed("2026-10-05T09:00:00", "2026-10-05T12:00:00"),
            ..draft_of(&core, &first)
        },
    )
    .unwrap();

    let shown: Vec<_> = occurrences(&core, "2026-10-01", "2026-12-01")
        .into_iter()
        .map(|occurrence| (occurrence.title, occurrence.when, occurrence.recurrence_id))
        .collect();
    assert_eq!(
        shown,
        [
            (
                "Course".into(),
                timed("2026-10-05T09:00:00", "2026-10-05T12:00:00"),
                Some("2026-10-05T09:00:00".into())
            ),
            (
                "Course outside".into(),
                timed("2026-10-06T09:00:00", "2026-10-06T12:00:00"),
                Some("2026-10-06T09:00:00".into())
            ),
            (
                "Course".into(),
                timed("2026-10-08T09:00:00", "2026-10-08T12:00:00"),
                Some("2026-10-08T09:00:00".into())
            ),
        ]
    );
}

#[test]
fn exceptions_too_keep_their_length_where_the_shift_would_end_them_before_they_start() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let id = yoga(&core, home.id, RepeatEnd::Count { count: 2 });
    let second = on(&core, "2026-10-12");
    core.reschedule_occurrence(
        id,
        rid(&second),
        Scope::This,
        false,
        timed("2026-10-12T18:30:00", "2026-10-12T18:45:00"),
    )
    .unwrap();
    let first = on(&core, "2026-10-05");

    // 45 minutes shorter: the Exception's end would come before its start.
    core.reschedule_occurrence(
        id,
        rid(&first),
        Scope::All,
        true,
        timed("2026-10-05T18:00:00", "2026-10-05T18:15:00"),
    )
    .unwrap();

    assert_eq!(
        shown(&core),
        [
            (
                "Yoga".into(),
                timed("2026-10-05T18:00:00", "2026-10-05T18:15:00")
            ),
            (
                "Yoga".into(),
                timed("2026-10-12T18:30:00", "2026-10-12T18:45:00")
            ),
        ]
    );
}
