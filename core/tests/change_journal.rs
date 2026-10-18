//! The Change Journal: every change to an Event is a unit that undoes it
//! exactly, and an undo is a unit too, which redoes it.

mod support;

use freecal_core::{
    Calendar, CalendarId, Core, CoreError, Event, ExistingEvents, ImportFile, Signal, UnitId,
};
use support::{Harness, all_day, calendar, date, draft, event, ics, timed};

/// Every Event of the shown Calendars, with everything its details show.
fn contents(core: &Core) -> Vec<Event> {
    core.occurrences(date("2000-01-01"), date("2100-01-01"))
        .unwrap()
        .into_iter()
        .map(|occurrence| core.event(occurrence.event_id).unwrap())
        .collect()
}

fn the_one(core: &Core, title: &str) -> Event {
    let mut found: Vec<_> = contents(core)
        .into_iter()
        .filter(|event| event.title == title)
        .collect();
    assert_eq!(found.len(), 1, "one Event titled {title:?}");
    found.remove(0)
}

fn changed(calendar_ids: &[CalendarId]) -> Vec<Signal> {
    vec![Signal::OccurrencesChanged {
        calendar_ids: calendar_ids.to_vec(),
    }]
}

fn make_read_only(harness: &Harness, calendar: &Calendar) {
    harness.arrange_local_store(&format!(
        "UPDATE calendar SET read_only = 1 WHERE id = {}",
        calendar.id.0
    ));
}

fn import(
    core: &Core,
    calendar_id: CalendarId,
    contents: Vec<u8>,
    existing: ExistingEvents,
) -> UnitId {
    let preview = core
        .preview_import(&ImportFile::new("events.ics", contents), calendar_id)
        .unwrap();
    core.run_import(&preview, existing).unwrap().unit.unwrap()
}

const MONDAY: (&str, &str) = ("DTSTART:20261005T070000Z", "DTEND:20261005T080000Z");
const TUESDAY: (&str, &str) = ("DTSTART:20261006T070000Z", "DTEND:20261006T080000Z");

/// A Calendar `Home` with the Event `Dentist`, and no Signals recorded.
fn home_with_dentist(harness: &Harness, core: &Core) -> (Calendar, Event) {
    let home = calendar(core, "Home");
    let dentist = core
        .create_event(draft(
            home.id,
            "Dentist",
            timed("2026-10-05T09:00:00", "2026-10-05T10:00:00"),
        ))
        .unwrap()
        .value;
    harness.signals.clear();
    (home, dentist)
}

/// Checks that undoing `unit` brings back `before` and signals `calendars`.
fn undoes_exactly(
    harness: &Harness,
    core: &Core,
    unit: UnitId,
    before: &[Event],
    calendars: &[CalendarId],
) {
    assert_ne!(contents(core), before, "the change changed something");
    harness.signals.clear();

    let undone = core.undo_unit(unit).unwrap();

    assert!(undone.unit.is_some());
    assert_eq!(contents(core), before);
    assert_eq!(harness.signals.received(), changed(calendars));
}

mod every_change_is_a_unit_that_undo_reverses_exactly {
    use super::*;

    #[test]
    fn create() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");

        let created = core
            .create_event(draft(
                home.id,
                "Dentist",
                all_day("2026-10-05", "2026-10-06"),
            ))
            .unwrap();

        undoes_exactly(&harness, &core, created.unit.unwrap(), &[], &[home.id]);
    }

    #[test]
    fn edit() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let (home, dentist) = home_with_dentist(&harness, &core);
        let before = contents(&core);
        let mut edit = draft(home.id, "Orthodontist", dentist.when);
        edit.location = "Main Street 1".to_owned();

        let edited = core.edit_event(dentist.id, edit).unwrap();

        undoes_exactly(&harness, &core, edited.unit.unwrap(), &before, &[home.id]);
    }

    #[test]
    fn reschedule() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let (home, dentist) = home_with_dentist(&harness, &core);
        let before = contents(&core);

        let rescheduled = core
            .reschedule_event(dentist.id, all_day("2026-10-07", "2026-10-08"))
            .unwrap();

        undoes_exactly(
            &harness,
            &core,
            rescheduled.unit.unwrap(),
            &before,
            &[home.id],
        );
    }

    #[test]
    fn delete_brings_the_event_back_under_its_id() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let (home, dentist) = home_with_dentist(&harness, &core);
        let before = contents(&core);

        let deleted = core.delete_event(dentist.id).unwrap();

        undoes_exactly(&harness, &core, deleted.unit.unwrap(), &before, &[home.id]);
        assert_eq!(core.event(dentist.id).unwrap(), dentist);
    }

    #[test]
    fn import_that_adds_and_updates() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        import(
            &core,
            home.id,
            ics(&event("kept", MONDAY.0, MONDAY.1)),
            ExistingEvents::Skip,
        );
        core.create_event(draft(home.id, "Mine", all_day("2026-10-07", "2026-10-08")))
            .unwrap();
        let before = contents(&core);
        let file = ics(&[
            event("kept", TUESDAY.0, TUESDAY.1),
            event("new", MONDAY.0, MONDAY.1),
        ]
        .concat());

        let unit = import(&core, home.id, file, ExistingEvents::Update);

        undoes_exactly(&harness, &core, unit, &before, &[home.id]);
    }
}

#[test]
fn an_edit_moving_an_event_to_another_calendar_is_undone_into_the_first() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let (home, dentist) = home_with_dentist(&harness, &core);
    let work = calendar(&core, "Work");
    let before = contents(&core);

    let moved = core
        .edit_event(dentist.id, draft(work.id, "Dentist", dentist.when))
        .unwrap();

    assert_eq!(the_one(&core, "Dentist").calendar_id, work.id);
    undoes_exactly(
        &harness,
        &core,
        moved.unit.unwrap(),
        &before,
        &[work.id, home.id],
    );
}

#[test]
fn a_change_that_changes_nothing_is_no_unit_and_sends_no_signal() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let (home, dentist) = home_with_dentist(&harness, &core);
    import(
        &core,
        home.id,
        ics(&event("kept", MONDAY.0, MONDAY.1)),
        ExistingEvents::Skip,
    );
    harness.signals.clear();

    let edited = core
        .edit_event(dentist.id, draft(home.id, "Dentist", dentist.when))
        .unwrap();
    let rescheduled = core.reschedule_event(dentist.id, dentist.when).unwrap();
    let preview = core
        .preview_import(
            &ImportFile::new("events.ics", ics(&event("kept", MONDAY.0, MONDAY.1))),
            home.id,
        )
        .unwrap();
    let updated = core.run_import(&preview, ExistingEvents::Update).unwrap();

    assert_eq!(edited.unit, None);
    assert_eq!(edited.value, dentist);
    assert_eq!(rescheduled.unit, None);
    assert_eq!((updated.unit, updated.updated), (None, 1));
    assert_eq!(harness.signals.received(), []);
}

#[test]
fn undoing_the_undo_redoes_and_an_undone_unit_is_undone_once() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let (home, dentist) = home_with_dentist(&harness, &core);
    let deleted = core.delete_event(dentist.id).unwrap().unit.unwrap();
    let undo = core.undo_unit(deleted).unwrap().unit.unwrap();
    assert_eq!(contents(&core), std::slice::from_ref(&dentist));
    harness.signals.clear();

    let again = core.undo_unit(deleted);
    let redo = core.undo_unit(undo).unwrap();

    assert!(matches!(again, Err(CoreError::AlreadyUndone(id)) if id == deleted));
    assert_eq!(contents(&core), []);
    assert_eq!(harness.signals.received(), changed(&[home.id]));
    assert!(matches!(
        core.undo_unit(undo),
        Err(CoreError::AlreadyUndone(_))
    ));
    core.undo_unit(redo.unit.unwrap()).unwrap();
    assert_eq!(contents(&core), [dentist]);
}

#[test]
fn undo_is_refused_in_a_calendar_now_read_only() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let home = calendar(&core, "Home");
    let unit = import(
        &core,
        home.id,
        ics(&event("new", MONDAY.0, MONDAY.1)),
        ExistingEvents::Skip,
    );
    let imported = contents(&core);
    make_read_only(&harness, &home);
    harness.signals.clear();

    let refused = core.undo_unit(unit);

    assert!(matches!(refused, Err(CoreError::CalendarReadOnly(id)) if id == home.id));
    assert_eq!(contents(&core), imported);
    assert_eq!(harness.signals.received(), []);
}

/// Ticket 58's cases: an imported Event moved since, an updated one edited
/// or deleted since. Undo per Event comes with ticket 72.
mod an_event_changed_since_refuses_the_undo {
    use super::*;

    fn refused(change: impl FnOnce(&Core, &Calendar)) {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        import(
            &core,
            home.id,
            ics(&event("kept", MONDAY.0, MONDAY.1)),
            ExistingEvents::Skip,
        );
        let file = ics(&[
            event("kept", TUESDAY.0, TUESDAY.1),
            event("new", MONDAY.0, MONDAY.1),
        ]
        .concat());
        let unit = import(&core, home.id, file, ExistingEvents::Update);
        change(&core, &home);
        let since = contents(&core);
        harness.signals.clear();

        let refused = core.undo_unit(unit);

        assert!(matches!(refused, Err(CoreError::ChangedSinceUnit(id)) if id == unit));
        assert_eq!(contents(&core), since);
        assert_eq!(harness.signals.received(), []);
    }

    #[test]
    fn an_added_event_moved_to_another_calendar() {
        refused(|core, _| {
            let work = calendar(core, "Work");
            let new = the_one(core, "new");
            core.edit_event(new.id, draft(work.id, "new", new.when))
                .unwrap();
        });
    }

    #[test]
    fn an_updated_event_edited() {
        refused(|core, home| {
            let kept = the_one(core, "kept");
            core.edit_event(kept.id, draft(home.id, "Kept", kept.when))
                .unwrap();
        });
    }

    #[test]
    fn an_updated_event_deleted() {
        refused(|core, _| {
            core.delete_event(the_one(core, "kept").id).unwrap();
        });
    }
}

#[test]
fn a_deleted_event_comes_back_beside_one_that_got_its_id_since() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let (home, dentist) = home_with_dentist(&harness, &core);
    let deleted = core.delete_event(dentist.id).unwrap().unit.unwrap();
    // SQLite gives a new Event the ID of the deleted one.
    let made_since = core
        .create_event(draft(
            home.id,
            "Made since",
            all_day("2026-10-08", "2026-10-09"),
        ))
        .unwrap()
        .value;
    assert_eq!(made_since.id, dentist.id);

    core.undo_unit(deleted).unwrap();

    assert_eq!(core.event(made_since.id).unwrap(), made_since);
    let back = the_one(&core, "Dentist");
    assert_eq!(
        back,
        Event {
            id: back.id,
            ..dentist
        }
    );
}

#[test]
fn deleting_a_calendar_removes_the_units_of_its_events() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let home = calendar(&core, "Home");
    let unit = import(
        &core,
        home.id,
        ics(&event("new", MONDAY.0, MONDAY.1)),
        ExistingEvents::Skip,
    );
    core.delete_calendar(home.id).unwrap();
    harness.signals.clear();

    let refused = core.undo_unit(unit);

    assert!(matches!(refused, Err(CoreError::UnitNotFound(id)) if id == unit));
    assert_eq!(harness.signals.received(), []);
}

#[test]
fn an_event_moved_out_of_a_calendar_deleted_since_is_not_moved_back() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let (home, dentist) = home_with_dentist(&harness, &core);
    let work = calendar(&core, "Work");
    let moved = core
        .edit_event(dentist.id, draft(work.id, "Dentist", dentist.when))
        .unwrap();
    core.delete_calendar(home.id).unwrap();
    let since = contents(&core);
    harness.signals.clear();

    let refused = core.undo_unit(moved.unit.unwrap());

    assert!(matches!(refused, Err(CoreError::CalendarNotFound(id)) if id == home.id));
    assert_eq!(contents(&core), since);
    assert_eq!(harness.signals.received(), []);
}
