mod support;

use freecal_core::{
    CalendarId, Core, CoreError, EventDraft, EventId, ExistingEvents, Frequency, ImportFile,
    Occurrence, Repeat, RepeatEnd, Signal, When,
};
use support::{Harness, all_day, calendar, date, draft, event, ics, timed};

fn repeat(frequency: Frequency, interval: u16, end: RepeatEnd) -> Repeat {
    Repeat {
        frequency,
        interval,
        end,
    }
}

fn count(count: u32) -> RepeatEnd {
    RepeatEnd::Count { count }
}

fn until(text: &str) -> RepeatEnd {
    RepeatEnd::Until { date: date(text) }
}

fn recurring(calendar_id: CalendarId, title: &str, when: When, repeat: Repeat) -> EventDraft {
    EventDraft {
        repeat: Some(repeat),
        ..draft(calendar_id, title, when)
    }
}

fn create(core: &Core, draft: EventDraft) -> EventId {
    core.create_event(draft).unwrap().value.id
}

fn occurrences(core: &Core, from: &str, to: &str) -> Vec<Occurrence> {
    core.occurrences(date(from), date(to)).unwrap()
}

/// When each Occurrence on the dates `from` up to `to` (exclusive) starts,
/// in the Display Time Zone.
fn starts(core: &Core, from: &str, to: &str) -> Vec<String> {
    occurrences(core, from, to)
        .into_iter()
        .map(|occurrence| match occurrence.when {
            When::AllDay { start, .. } => start.to_string(),
            When::Timed { start, .. } => start.to_string(),
        })
        .collect()
}

fn import(core: &Core, calendar_id: CalendarId, body: &str, existing: ExistingEvents) {
    let file = ImportFile::new("events.ics", ics(body));
    let preview = core.preview_import(&file, calendar_id).unwrap();
    core.run_import(&preview, existing).unwrap();
}

#[test]
fn a_recurring_event_has_an_occurrence_on_every_date_it_falls_on() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let id = create(
        &core,
        recurring(
            home.id,
            "Yoga",
            timed("2026-10-05T18:00:00", "2026-10-05T19:00:00"),
            repeat(Frequency::Weekly, 1, count(3)),
        ),
    );

    let found = occurrences(&core, "2026-10-01", "2026-11-01");

    assert_eq!(
        found
            .iter()
            .map(|occurrence| (
                occurrence.event_id,
                occurrence.when,
                occurrence.title.as_str()
            ))
            .collect::<Vec<_>>(),
        [
            (
                id,
                timed("2026-10-05T18:00:00", "2026-10-05T19:00:00"),
                "Yoga"
            ),
            (
                id,
                timed("2026-10-12T18:00:00", "2026-10-12T19:00:00"),
                "Yoga"
            ),
            (
                id,
                timed("2026-10-19T18:00:00", "2026-10-19T19:00:00"),
                "Yoga"
            ),
        ]
    );
    let ids: Vec<_> = found
        .iter()
        .map(|occurrence| occurrence.recurrence_id.clone().unwrap())
        .collect();
    assert_eq!(ids.len(), 3);
    assert!(ids[0] != ids[1] && ids[1] != ids[2]);
}

#[test]
fn only_the_occurrences_in_the_range_are_given_and_a_single_event_has_no_recurrence_id() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    create(
        &core,
        recurring(
            home.id,
            "Standup",
            timed("2026-10-05T09:00:00", "2026-10-05T09:15:00"),
            repeat(Frequency::Daily, 1, RepeatEnd::Never),
        ),
    );
    create(
        &core,
        draft(
            home.id,
            "Lunch",
            timed("2026-10-08T12:00:00", "2026-10-08T13:00:00"),
        ),
    );

    let found = occurrences(&core, "2026-10-07", "2026-10-09");

    assert_eq!(
        found
            .iter()
            .map(|occurrence| (
                occurrence.title.as_str(),
                occurrence.recurrence_id.is_some()
            ))
            .collect::<Vec<_>>(),
        [("Standup", true), ("Standup", true), ("Lunch", false)]
    );
}

#[test]
fn a_weekly_meeting_stays_at_its_wall_time_when_the_clocks_change() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    // Berlin's clocks go back on 25 October 2026.
    create(
        &core,
        recurring(
            home.id,
            "Team",
            timed("2026-10-19T09:00:00", "2026-10-19T10:00:00"),
            repeat(Frequency::Weekly, 1, RepeatEnd::Never),
        ),
    );

    assert_eq!(
        starts(&core, "2026-10-19", "2026-11-03"),
        [
            "2026-10-19 09:00:00",
            "2026-10-26 09:00:00",
            "2026-11-02 09:00:00"
        ]
    );
}

#[test]
fn occurrences_are_expanded_in_the_event_time_zone_and_shown_in_the_display_time_zone() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    // 9:00 in Berlin; New York changes its clocks a week after Berlin.
    create(
        &core,
        recurring(
            home.id,
            "Team",
            timed("2026-10-19T09:00:00", "2026-10-19T10:00:00"),
            repeat(Frequency::Weekly, 1, count(3)),
        ),
    );

    harness.clock.set_time_zone("America/New_York");

    assert_eq!(
        starts(&core, "2026-10-01", "2026-11-10"),
        [
            "2026-10-19 03:00:00",
            "2026-10-26 04:00:00",
            "2026-11-02 03:00:00"
        ]
    );
}

#[test]
fn an_interval_skips_dates_and_until_ends_on_its_date() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    create(
        &core,
        recurring(
            home.id,
            "Run",
            timed("2026-10-05T07:00:00", "2026-10-05T08:00:00"),
            repeat(Frequency::Daily, 2, until("2026-10-11")),
        ),
    );

    assert_eq!(
        starts(&core, "2026-10-01", "2026-11-01"),
        [
            "2026-10-05 07:00:00",
            "2026-10-07 07:00:00",
            "2026-10-09 07:00:00",
            "2026-10-11 07:00:00"
        ]
    );
}

#[test]
fn all_day_events_repeat_monthly_and_yearly() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    create(
        &core,
        recurring(
            home.id,
            "Rent",
            all_day("2026-10-31", "2026-11-01"),
            repeat(Frequency::Monthly, 1, count(3)),
        ),
    );
    create(
        &core,
        recurring(
            home.id,
            "Birthday",
            all_day("2026-10-20", "2026-10-21"),
            repeat(Frequency::Yearly, 1, until("2028-12-31")),
        ),
    );

    // RFC 5545: a month without a 31st has no Occurrence.
    assert_eq!(
        starts(&core, "2026-10-01", "2029-12-31"),
        [
            "2026-10-20",
            "2026-10-31",
            "2026-12-31",
            "2027-01-31",
            "2027-10-20",
            "2028-10-20"
        ]
    );
    assert_eq!(
        occurrences(&core, "2026-12-31", "2027-01-01")[0].when,
        all_day("2026-12-31", "2027-01-01")
    );
}

#[test]
fn a_recurring_event_without_an_end_still_has_occurrences_years_later() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    create(
        &core,
        recurring(
            home.id,
            "Standup",
            timed("2026-10-05T09:00:00", "2026-10-05T09:15:00"),
            repeat(Frequency::Weekly, 1, RepeatEnd::Never),
        ),
    );

    assert_eq!(
        starts(&core, "2036-10-06", "2036-10-07"),
        ["2036-10-06 09:00:00"]
    );
}

#[test]
fn the_repeat_rule_reads_back_and_an_edit_of_other_fields_keeps_it() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let rule = repeat(Frequency::Weekly, 2, until("2026-12-31"));
    let created = core
        .create_event(recurring(
            home.id,
            "Yoga",
            timed("2026-10-05T18:00:00", "2026-10-05T19:00:00"),
            rule,
        ))
        .unwrap()
        .value;
    assert_eq!(created.repeat, Some(rule));
    assert!(!created.custom_repeat);
    assert_eq!(created.recurrence_id, None);

    let edited = core
        .edit_event(
            created.id,
            recurring(
                home.id,
                "Yin yoga",
                timed("2026-10-05T18:00:00", "2026-10-05T19:00:00"),
                rule,
            ),
        )
        .unwrap();

    assert_eq!(edited.value.repeat, Some(rule));
    assert_eq!(starts(&core, "2026-10-01", "2026-10-31").len(), 2);
}

#[test]
fn an_edit_can_change_or_remove_the_repeat_rule() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let when = timed("2026-10-05T18:00:00", "2026-10-05T19:00:00");
    let id = create(
        &core,
        recurring(
            home.id,
            "Yoga",
            when,
            repeat(Frequency::Weekly, 1, count(5)),
        ),
    );

    let daily = repeat(Frequency::Daily, 1, count(2));
    core.edit_event(id, recurring(home.id, "Yoga", when, daily))
        .unwrap();
    assert_eq!(core.event(id).unwrap().repeat, Some(daily));
    assert_eq!(starts(&core, "2026-10-01", "2026-11-01").len(), 2);

    core.edit_event(id, draft(home.id, "Yoga", when)).unwrap();
    assert_eq!(core.event(id).unwrap().repeat, None);
    assert_eq!(starts(&core, "2026-10-01", "2026-11-01").len(), 1);
}

#[test]
fn a_repeat_rule_without_any_occurrence_is_refused() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let when = timed("2026-10-05T18:00:00", "2026-10-05T19:00:00");

    for rule in [
        repeat(Frequency::Daily, 0, RepeatEnd::Never),
        repeat(Frequency::Daily, 1, count(0)),
        repeat(Frequency::Daily, 1, until("2026-10-04")),
    ] {
        assert!(
            matches!(
                core.create_event(recurring(home.id, "Yoga", when, rule)),
                Err(CoreError::InvalidRepeat)
            ),
            "{rule:?}"
        );
    }
    assert!(starts(&core, "2026-10-01", "2026-11-01").is_empty());
}

#[test]
fn deleting_one_occurrence_cancels_it_and_leaves_the_rest_and_can_be_undone() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let id = create(
        &core,
        recurring(
            home.id,
            "Yoga",
            timed("2026-10-05T18:00:00", "2026-10-05T19:00:00"),
            repeat(Frequency::Weekly, 1, count(3)),
        ),
    );
    let second = occurrences(&core, "2026-10-12", "2026-10-13").remove(0);
    harness.signals.clear();

    let deleted = core
        .delete_occurrence(id, second.recurrence_id.as_deref().unwrap())
        .unwrap();

    assert_eq!(
        starts(&core, "2026-10-01", "2026-11-01"),
        ["2026-10-05 18:00:00", "2026-10-19 18:00:00"]
    );
    assert!(core.event(id).unwrap().repeat.is_some());
    assert_eq!(
        harness.signals.received(),
        [Signal::OccurrencesChanged {
            calendar_ids: vec![home.id]
        }]
    );

    core.undo_unit(deleted.unit.unwrap()).unwrap();
    assert_eq!(starts(&core, "2026-10-01", "2026-11-01").len(), 3);
}

#[test]
fn deleting_an_occurrence_with_a_recurrence_id_the_event_doesnt_have_is_refused() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let id = create(
        &core,
        recurring(
            home.id,
            "Yoga",
            timed("2026-10-05T18:00:00", "2026-10-05T19:00:00"),
            repeat(Frequency::Weekly, 1, count(3)),
        ),
    );

    assert!(matches!(
        core.delete_occurrence(id, "<script>"),
        Err(CoreError::OccurrenceNotFound(_))
    ));
    assert_eq!(starts(&core, "2026-10-01", "2026-11-01").len(), 3);
}

#[test]
fn deleting_a_recurring_event_deletes_all_its_occurrences() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let id = create(
        &core,
        recurring(
            home.id,
            "Yoga",
            timed("2026-10-05T18:00:00", "2026-10-05T19:00:00"),
            repeat(Frequency::Weekly, 1, RepeatEnd::Never),
        ),
    );

    core.delete_event(id).unwrap();

    assert!(starts(&core, "2026-10-01", "2027-11-01").is_empty());
}

/// A weekly meeting from a file: Mondays at 9:00 in Berlin until the last
/// one on 2 November, given in UTC as Google writes it, without 19 October,
/// and with 26 October moved to 11:00.
const WEEKLY_FROM_A_FILE: &str = "BEGIN:VEVENT\nUID:team\nSUMMARY:Team\n\
     DTSTART;TZID=Europe/Berlin:20261005T090000\nDTEND;TZID=Europe/Berlin:20261005T100000\n\
     RRULE:FREQ=WEEKLY;UNTIL=20261102T080000Z\n\
     EXDATE;TZID=Europe/Berlin:20261019T090000\nEND:VEVENT\n\
     BEGIN:VEVENT\nUID:team\nSUMMARY:Team (moved)\n\
     RECURRENCE-ID;TZID=Europe/Berlin:20261026T090000\n\
     DTSTART;TZID=Europe/Berlin:20261026T110000\nDTEND;TZID=Europe/Berlin:20261026T120000\n\
     END:VEVENT\n";

#[test]
fn an_import_carries_the_rule_the_cancelled_occurrences_and_the_changed_ones() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");

    import(&core, home.id, WEEKLY_FROM_A_FILE, ExistingEvents::Update);

    let found = occurrences(&core, "2026-10-01", "2026-11-30");
    assert_eq!(
        found
            .iter()
            .map(|occurrence| (occurrence.title.as_str(), occurrence.when))
            .collect::<Vec<_>>(),
        [
            ("Team", timed("2026-10-05T09:00:00", "2026-10-05T10:00:00")),
            ("Team", timed("2026-10-12T09:00:00", "2026-10-12T10:00:00")),
            (
                "Team (moved)",
                timed("2026-10-26T11:00:00", "2026-10-26T12:00:00")
            ),
            ("Team", timed("2026-11-02T09:00:00", "2026-11-02T10:00:00")),
        ]
    );
    let moved = core.event(found[2].event_id).unwrap();
    assert_eq!(moved.repeat, None);
    assert_eq!(moved.recurrence_id, found[2].recurrence_id);
    assert!(moved.recurrence_id.is_some());
    assert_eq!(
        core.event(found[0].event_id).unwrap().repeat,
        Some(repeat(Frequency::Weekly, 1, until("2026-11-02")))
    );
}

#[test]
fn deleting_a_changed_occurrence_doesnt_bring_back_the_original() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    import(&core, home.id, WEEKLY_FROM_A_FILE, ExistingEvents::Update);
    let moved = occurrences(&core, "2026-10-26", "2026-10-27").remove(0);

    core.delete_occurrence(moved.event_id, moved.recurrence_id.as_deref().unwrap())
        .unwrap();

    assert!(starts(&core, "2026-10-26", "2026-10-27").is_empty());
    assert_eq!(starts(&core, "2026-10-01", "2026-11-30").len(), 3);
}

#[test]
fn deleting_the_whole_recurring_event_from_a_changed_occurrence_deletes_it_all() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    import(&core, home.id, WEEKLY_FROM_A_FILE, ExistingEvents::Update);
    let moved = occurrences(&core, "2026-10-26", "2026-10-27").remove(0);

    core.delete_event(moved.event_id).unwrap();

    assert!(starts(&core, "2026-10-01", "2026-11-30").is_empty());
}

#[test]
fn an_import_update_replaces_the_changed_occurrences_and_the_undo_restores_them() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    import(&core, home.id, WEEKLY_FROM_A_FILE, ExistingEvents::Update);
    let again = WEEKLY_FROM_A_FILE
        .replace("Team (moved)", "Team (moved again)")
        .replace("20261026T110000", "20261026T140000")
        .replace("20261026T120000", "20261026T150000");
    let file = ImportFile::new("events.ics", ics(&again));
    let preview = core.preview_import(&file, home.id).unwrap();
    assert_eq!((preview.new_events, preview.existing_events), (0, 1));

    let outcome = core.run_import(&preview, ExistingEvents::Update).unwrap();

    let moved = occurrences(&core, "2026-10-26", "2026-10-27");
    assert_eq!(
        moved
            .iter()
            .map(|occurrence| (occurrence.title.as_str(), occurrence.when))
            .collect::<Vec<_>>(),
        [(
            "Team (moved again)",
            timed("2026-10-26T14:00:00", "2026-10-26T15:00:00")
        )]
    );
    assert_eq!(starts(&core, "2026-10-01", "2026-11-30").len(), 4);

    core.undo_unit(outcome.unit.unwrap()).unwrap();
    assert_eq!(
        occurrences(&core, "2026-10-26", "2026-10-27")[0].title,
        "Team (moved)"
    );
}

#[test]
fn a_custom_rule_from_a_file_keeps_its_weekdays_while_its_frequency_stays() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    // Every 2 weeks on Sunday and Tuesday.
    import(
        &core,
        home.id,
        "BEGIN:VEVENT\nUID:swim\nSUMMARY:Swim\nDTSTART;VALUE=DATE:20261004\n\
         RRULE:FREQ=WEEKLY;INTERVAL=2;BYDAY=SU,TU;WKST=SU\nEND:VEVENT\n",
        ExistingEvents::Update,
    );
    let id = occurrences(&core, "2026-10-04", "2026-10-05")[0].event_id;
    let event = core.event(id).unwrap();
    assert_eq!(
        event.repeat,
        Some(repeat(Frequency::Weekly, 2, RepeatEnd::Never))
    );
    assert!(event.custom_repeat);
    assert_eq!(
        starts(&core, "2026-10-01", "2026-10-31"),
        ["2026-10-04", "2026-10-06", "2026-10-18", "2026-10-20"]
    );

    core.edit_event(
        id,
        recurring(
            home.id,
            "Swim",
            all_day("2026-10-04", "2026-10-05"),
            repeat(Frequency::Weekly, 1, count(4)),
        ),
    )
    .unwrap();

    assert!(core.event(id).unwrap().custom_repeat);
    assert_eq!(
        starts(&core, "2026-10-01", "2026-10-31"),
        ["2026-10-04", "2026-10-06", "2026-10-11", "2026-10-13"]
    );
}

#[test]
fn a_rule_freecal_cant_read_leaves_the_event_as_its_first_occurrence() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    import(
        &core,
        home.id,
        &event(
            "odd",
            "DTSTART:20261005T070000Z\nRRULE:FREQ=FORTNIGHTLY",
            "DTEND:20261005T080000Z",
        ),
        ExistingEvents::Update,
    );

    let found = occurrences(&core, "2026-10-01", "2026-12-01");

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].recurrence_id, None);
    assert_eq!(core.event(found[0].event_id).unwrap().repeat, None);
}

#[test]
fn a_stored_rule_freecal_cant_read_shows_the_first_occurrence_rather_than_failing() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    create(
        &core,
        recurring(
            home.id,
            "Yoga",
            timed("2026-10-05T18:00:00", "2026-10-05T19:00:00"),
            repeat(Frequency::Weekly, 1, count(3)),
        ),
    );
    drop(core);
    harness.arrange_local_store("UPDATE event SET rrule = 'FREQ=WEEKLY;BYDAY=<img src=x>'");
    let core = harness.open();

    assert_eq!(
        starts(&core, "2026-10-01", "2026-11-01"),
        ["2026-10-05 18:00:00"]
    );
}

#[test]
fn a_rule_that_repeats_every_second_from_long_ago_doesnt_stall_the_grid() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    import(
        &core,
        home.id,
        &event(
            "busy",
            "DTSTART:20000101T000000Z\nRRULE:FREQ=SECONDLY",
            "DTEND:20000101T000001Z",
        ),
        ExistingEvents::Update,
    );

    occurrences(&core, "2026-10-01", "2026-11-01");
}
