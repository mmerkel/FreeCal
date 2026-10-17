mod support;

use freecal_core::{
    Calendar, CalendarId, Core, CoreError, Event, EventId, ExistingEvents, ImportFile,
    ImportOutcome, ImportPreview, Signal, link_allowed,
};
use support::{Harness, calendar, date, event, ics, timed};

fn preview(core: &Core, calendar_id: CalendarId, contents: Vec<u8>) -> ImportPreview {
    core.preview_import(&ImportFile::new("events.ics", contents), calendar_id)
        .unwrap()
}

fn import(
    core: &Core,
    calendar_id: CalendarId,
    contents: Vec<u8>,
    existing: ExistingEvents,
) -> ImportOutcome {
    core.run_import(&preview(core, calendar_id, contents), existing)
        .unwrap()
}

/// Every Event of the shown Calendars, with everything its details show.
fn contents(core: &Core) -> Vec<Event> {
    core.occurrences(date("2000-01-01"), date("2100-01-01"))
        .unwrap()
        .into_iter()
        .map(|occurrence| core.event(occurrence.event_id).unwrap())
        .collect()
}

fn titled(core: &Core, title: &str) -> Vec<Event> {
    contents(core)
        .into_iter()
        .filter(|event| event.title == title)
        .collect()
}

fn the_one(core: &Core, title: &str) -> Event {
    let mut found = titled(core, title);
    assert_eq!(found.len(), 1, "one Event titled {title:?}");
    found.remove(0)
}

const MONDAY: (&str, &str) = ("DTSTART:20261005T070000Z", "DTEND:20261005T080000Z");
const TUESDAY: (&str, &str) = ("DTSTART:20261006T070000Z", "DTEND:20261006T080000Z");

/// A Calendar holding the imported Event `kept`, with no Signals recorded.
fn calendar_with_kept(harness: &Harness, core: &Core) -> Calendar {
    let home = calendar(core, "Home");
    import(
        core,
        home.id,
        ics(&event("kept", MONDAY.0, MONDAY.1)),
        ExistingEvents::Skip,
    );
    harness.signals.clear();
    home
}

/// `kept` moved to Tuesday, and a new Event `new`.
fn kept_moved_and_new() -> Vec<u8> {
    ics(&[
        event("kept", TUESDAY.0, TUESDAY.1),
        event("new", MONDAY.0, MONDAY.1),
    ]
    .concat())
}

#[test]
fn new_events_are_stored_with_their_fields_and_uids() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let home = calendar(&core, "Home");
    let contents = ics(
        "BEGIN:VEVENT\nUID:dentist\nSUMMARY:Dentist\nLOCATION:Main Street 1\n\
         DESCRIPTION:Bring the card\\, please\nDTSTART;TZID=Europe/Berlin:20261005T090000\n\
         DTEND;TZID=Europe/Berlin:20261005T103000\nEND:VEVENT\n\
         BEGIN:VEVENT\nUID:holiday\nSUMMARY:Holiday\nDTSTART;VALUE=DATE:20261005\n\
         DTEND;VALUE=DATE:20261007\nEND:VEVENT\n",
    );
    harness.signals.clear();

    let outcome = import(&core, home.id, contents.clone(), ExistingEvents::Skip);

    assert_eq!(
        (outcome.imported, outcome.updated, outcome.skipped),
        (2, 0, 0)
    );
    let dentist = the_one(&core, "Dentist");
    assert_eq!(dentist.calendar_id, home.id);
    assert_eq!(
        dentist.when,
        timed("2026-10-05T09:00:00", "2026-10-05T10:30:00")
    );
    assert_eq!(dentist.location.text, "Main Street 1");
    assert_eq!(dentist.description.text, "Bring the card, please");
    assert_eq!(dentist.pinned_zone, None);
    assert_eq!(
        the_one(&core, "Holiday").when,
        support::all_day("2026-10-05", "2026-10-07")
    );
    // Their UIDs are the file's: importing it again finds them.
    let again = preview(&core, home.id, contents);
    assert_eq!((again.new_events, again.existing_events), (0, 2));
    assert_eq!(
        harness.signals.received(),
        [Signal::OccurrencesChanged {
            calendar_ids: vec![home.id]
        }]
    );
}

#[test]
fn update_changes_events_the_calendar_has_and_adds_the_others() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let home = calendar_with_kept(&harness, &core);
    let kept = the_one(&core, "kept");

    let outcome = import(&core, home.id, kept_moved_and_new(), ExistingEvents::Update);

    assert_eq!(
        (outcome.imported, outcome.updated, outcome.skipped),
        (1, 1, 0)
    );
    let updated = the_one(&core, "kept");
    assert_eq!(updated.id, kept.id);
    assert_eq!(
        updated.when,
        timed("2026-10-06T09:00:00", "2026-10-06T10:00:00")
    );
    assert_eq!(titled(&core, "new").len(), 1);
}

#[test]
fn skip_leaves_events_the_calendar_has_as_they_are() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let home = calendar_with_kept(&harness, &core);
    let kept = the_one(&core, "kept");

    let outcome = import(&core, home.id, kept_moved_and_new(), ExistingEvents::Skip);

    assert_eq!(
        (outcome.imported, outcome.updated, outcome.skipped),
        (1, 0, 1)
    );
    assert_eq!(the_one(&core, "kept"), kept);
    assert_eq!(titled(&core, "new").len(), 1);
}

#[test]
fn copy_adds_events_the_calendar_has_as_new_events_beside_them() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let home = calendar_with_kept(&harness, &core);
    let kept = the_one(&core, "kept");

    let outcome = import(&core, home.id, kept_moved_and_new(), ExistingEvents::Copy);

    assert_eq!(
        (outcome.imported, outcome.updated, outcome.skipped),
        (2, 0, 0)
    );
    let both = titled(&core, "kept");
    assert_eq!(both.len(), 2);
    assert!(both.contains(&kept));
    assert!(both.iter().any(|copy| copy.id != kept.id
        && copy.when == timed("2026-10-06T09:00:00", "2026-10-06T10:00:00")));
}

#[test]
fn an_import_into_a_read_only_calendar_is_refused_and_stores_nothing() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let home = calendar(&core, "Home");
    let preview = preview(&core, home.id, kept_moved_and_new());
    harness.arrange_local_store(&format!(
        "UPDATE calendar SET read_only = 1 WHERE id = {}",
        home.id.0
    ));
    harness.signals.clear();

    let refused = core.run_import(&preview, ExistingEvents::Update);

    assert!(matches!(refused, Err(CoreError::CalendarReadOnly(id)) if id == home.id));
    assert_eq!(contents(&core), []);
    assert_eq!(harness.signals.received(), []);
}

#[test]
fn undoing_an_import_restores_the_calendar_exactly() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let home = calendar_with_kept(&harness, &core);
    let other = calendar(&core, "Other");
    core.create_event(support::draft(
        home.id,
        "Mine",
        timed("2026-10-07T09:00:00", "2026-10-07T10:00:00"),
    ))
    .unwrap();
    core.create_event(support::draft(
        other.id,
        "Elsewhere",
        timed("2026-10-07T09:00:00", "2026-10-07T10:00:00"),
    ))
    .unwrap();
    let before = contents(&core);
    let file = kept_moved_and_new();
    let previewed = preview(&core, home.id, file.clone());
    let outcome = core.run_import(&previewed, ExistingEvents::Update).unwrap();
    assert_ne!(contents(&core), before);
    harness.signals.clear();

    core.undo_import(outcome.id).unwrap();

    assert_eq!(contents(&core), before);
    assert_eq!(preview(&core, home.id, file), previewed);
    assert_eq!(
        harness.signals.received(),
        [Signal::OccurrencesChanged {
            calendar_ids: vec![home.id]
        }]
    );
}

#[test]
fn an_import_is_undone_once() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let home = calendar(&core, "Home");
    let outcome = import(&core, home.id, kept_moved_and_new(), ExistingEvents::Skip);
    core.undo_import(outcome.id).unwrap();
    harness.signals.clear();

    let again = core.undo_import(outcome.id);

    assert!(matches!(again, Err(CoreError::ImportNotFound(id)) if id == outcome.id));
    assert_eq!(harness.signals.received(), []);
}

#[test]
fn undo_puts_back_an_updated_event_deleted_since_and_spares_events_made_since() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let home = calendar_with_kept(&harness, &core);
    let other = calendar(&core, "Other");
    let kept = the_one(&core, "kept");
    let file = ics(&[
        event("kept", TUESDAY.0, TUESDAY.1),
        event("moved", MONDAY.0, MONDAY.1),
        event("deleted", MONDAY.0, MONDAY.1),
    ]
    .concat());
    let outcome = import(&core, home.id, file, ExistingEvents::Update);
    let moved = the_one(&core, "moved");
    let deleted = the_one(&core, "deleted");
    core.edit_event(moved.id, support::draft(other.id, "moved", moved.when))
        .unwrap();
    core.delete_event(kept.id).unwrap();
    core.delete_event(deleted.id).unwrap();
    // SQLite gives a new Event the ID of the deleted one.
    let made_since = core
        .create_event(support::draft(
            home.id,
            "Made since",
            timed("2026-10-08T09:00:00", "2026-10-08T10:00:00"),
        ))
        .unwrap();
    assert_eq!(made_since.id, deleted.id);
    harness.signals.clear();

    core.undo_import(outcome.id).unwrap();

    assert_eq!(the_one(&core, "kept"), kept);
    assert_eq!(titled(&core, "moved"), []);
    assert_eq!(core.event(made_since.id).unwrap(), made_since);
    assert_eq!(
        harness.signals.received(),
        [Signal::OccurrencesChanged {
            calendar_ids: vec![home.id, other.id]
        }]
    );
}

#[test]
fn undoing_an_import_into_a_calendar_now_read_only_is_refused() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let home = calendar(&core, "Home");
    let outcome = import(&core, home.id, kept_moved_and_new(), ExistingEvents::Skip);
    let imported = contents(&core);
    harness.arrange_local_store(&format!(
        "UPDATE calendar SET read_only = 1 WHERE id = {}",
        home.id.0
    ));

    let refused = core.undo_import(outcome.id);

    assert!(matches!(refused, Err(CoreError::CalendarReadOnly(id)) if id == home.id));
    assert_eq!(contents(&core), imported);
}

#[test]
fn undo_is_refused_while_an_event_it_changes_is_in_a_read_only_calendar() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let home = calendar(&core, "Home");
    let locked = calendar(&core, "Locked");
    let outcome = import(&core, home.id, kept_moved_and_new(), ExistingEvents::Skip);
    let new = the_one(&core, "new");
    core.edit_event(new.id, support::draft(locked.id, "new", new.when))
        .unwrap();
    harness.arrange_local_store(&format!(
        "UPDATE calendar SET read_only = 1 WHERE id = {}",
        locked.id.0
    ));
    let imported = contents(&core);

    let refused = core.undo_import(outcome.id);

    assert!(matches!(refused, Err(CoreError::CalendarReadOnly(id)) if id == locked.id));
    assert_eq!(contents(&core), imported);
}

#[test]
fn an_import_that_changes_nothing_sends_no_signal() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let home = calendar_with_kept(&harness, &core);

    let outcome = import(
        &core,
        home.id,
        ics(&event("kept", TUESDAY.0, TUESDAY.1)),
        ExistingEvents::Skip,
    );

    assert_eq!(outcome.skipped, 1);
    assert_eq!(harness.signals.received(), []);
}

mod pinned_zones {
    use super::*;

    const FLOATING: (&str, &str) = ("DTSTART:20261005T090000", "DTEND:20261005T100000");

    fn pinned_event(core: &Core) -> (Calendar, EventId) {
        let home = calendar(core, "Home");
        import(
            core,
            home.id,
            ics(&event("floating", FLOATING.0, FLOATING.1)),
            ExistingEvents::Skip,
        );
        (home, the_one(core, "floating").id)
    }

    #[test]
    fn an_event_with_a_pinned_zone_says_which() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();

        let (_, id) = pinned_event(&core);

        let event = core.event(id).unwrap();
        assert_eq!(event.pinned_zone.as_deref(), Some("Europe/Berlin"));
        assert_eq!(
            event.when,
            timed("2026-10-05T09:00:00", "2026-10-05T10:00:00")
        );
    }

    #[test]
    fn the_mark_goes_when_the_user_changes_the_time_and_stays_otherwise() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let (home, renamed) = pinned_event(&core);
        let other = import(
            &core,
            home.id,
            ics(&event("rescheduled", FLOATING.0, FLOATING.1)),
            ExistingEvents::Skip,
        );
        assert_eq!(other.imported, 1);
        let rescheduled = the_one(&core, "rescheduled").id;
        let when = core.event(renamed).unwrap().when;

        core.edit_event(renamed, support::draft(home.id, "Renamed", when))
            .unwrap();
        core.reschedule_event(
            rescheduled,
            timed("2026-10-05T11:00:00", "2026-10-05T12:00:00"),
        )
        .unwrap();

        assert_eq!(
            core.event(renamed).unwrap().pinned_zone.as_deref(),
            Some("Europe/Berlin")
        );
        assert_eq!(core.event(rescheduled).unwrap().pinned_zone, None);
    }

    #[test]
    fn an_update_still_floating_keeps_the_zone_pinned_before() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let (home, id) = pinned_event(&core);
        harness.clock.set_time_zone("America/New_York");

        import(
            &core,
            home.id,
            ics(&event(
                "floating",
                "DTSTART:20261005T110000",
                "DTEND:20261005T120000",
            )),
            ExistingEvents::Update,
        );

        let event = core.event(id).unwrap();
        assert_eq!(event.pinned_zone.as_deref(), Some("Europe/Berlin"));
        // 11:00 in Berlin is 5:00 in New York.
        assert_eq!(
            event.when,
            timed("2026-10-05T05:00:00", "2026-10-05T06:00:00")
        );
    }

    #[test]
    fn an_update_floating_at_one_end_takes_the_zone_of_the_other() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let (home, id) = pinned_event(&core);
        harness.clock.set_time_zone("America/New_York");

        import(
            &core,
            home.id,
            ics(&event(
                "floating",
                "DTSTART:20261005T110000",
                "DTEND;TZID=America/New_York:20261005T120000",
            )),
            ExistingEvents::Update,
        );

        let event = core.event(id).unwrap();
        assert_eq!(event.pinned_zone.as_deref(), Some("America/New_York"));
        assert_eq!(
            event.when,
            timed("2026-10-05T11:00:00", "2026-10-05T12:00:00")
        );
    }

    #[test]
    fn an_update_with_a_zone_loses_the_mark() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let (home, id) = pinned_event(&core);

        import(
            &core,
            home.id,
            ics(&event(
                "floating",
                "DTSTART;TZID=America/New_York:20261005T090000",
                "DTEND;TZID=America/New_York:20261005T100000",
            )),
            ExistingEvents::Update,
        );

        let event = core.event(id).unwrap();
        assert_eq!(event.pinned_zone, None);
        assert_eq!(
            event.when,
            timed("2026-10-05T15:00:00", "2026-10-05T16:00:00")
        );
    }
}

mod hostile {
    use super::*;
    use icalendar::{Component, EventLike};

    fn fixture() -> serde_json::Value {
        serde_json::from_str(include_str!("../../fixtures/hostile-event.json")).unwrap()
    }

    fn field(name: &str) -> String {
        fixture()[name].as_str().unwrap().to_owned()
    }

    #[test]
    fn it_is_stored_and_read_back_as_text_and_allowed_links() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let file = icalendar::Calendar::new()
            .push(
                icalendar::Event::new()
                    .uid("hostile")
                    .summary(&field("title"))
                    .location(&field("location"))
                    .description(&field("description"))
                    .starts(support::time("2026-10-05T09:00:00"))
                    .ends(support::time("2026-10-05T10:00:00"))
                    .done(),
            )
            .done()
            .to_string()
            .into_bytes();

        let outcome = import(&core, home.id, file, ExistingEvents::Skip);

        assert_eq!(outcome.imported, 1);
        let event = the_one(&core, &field("title"));
        assert_eq!(event.location.text, field("location"));
        assert_eq!(
            serde_json::to_value(&event.description).unwrap(),
            fixture()["expected"]["html"]
        );
        assert_eq!(
            serde_json::to_value(&event.location).unwrap(),
            fixture()["expected"]["location"]
        );
        for piece in event
            .description
            .paragraphs
            .iter()
            .chain(&event.location.paragraphs)
            .flatten()
        {
            if let freecal_core::Piece::Link { url, .. } = piece {
                assert!(link_allowed(url), "{url:?} must not be a link");
            }
        }
    }
}
