//! The Change Journal: every change to an Event is a unit that undoes it
//! exactly, and an undo is a unit too, which redoes it.

mod support;

use freecal_core::{
    Calendar, CalendarId, Core, CoreError, Event, EventDraft, ExistingEvents, Frequency,
    ImportFile, Occurrence, Repeat, RepeatEnd, Scope, Signal, UnitId,
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

/// The one Occurrence of a shown Calendar on `date`.
fn the_one_on(core: &Core, date: &str) -> Occurrence {
    let day = self::date(date);
    let mut found = core.occurrences(day, day.succ_opt().unwrap()).unwrap();
    assert_eq!(found.len(), 1, "one Occurrence on {date}");
    found.remove(0)
}

/// Ticket 85: Exceptions share their Recurring Event's UID, so only an ID
/// that is never given out again tells a new Exception from a deleted one.
#[test]
fn a_deleted_exception_comes_back_under_its_id_beside_one_made_since() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let home = calendar(&core, "Home");
    let yoga = EventDraft {
        repeat: Some(Repeat {
            frequency: Frequency::Weekly,
            interval: 1,
            end: RepeatEnd::Count { count: 3 },
        }),
        ..draft(
            home.id,
            "Yoga",
            timed("2026-10-05T18:00:00", "2026-10-05T19:00:00"),
        )
    };
    let yoga = core.create_event(yoga).unwrap().value;
    let first = the_one_on(&core, "2026-10-05");
    let moved = core
        .reschedule_occurrence(
            yoga.id,
            first.recurrence_id.as_deref().unwrap(),
            Scope::This,
            timed("2026-10-05T08:00:00", "2026-10-05T09:00:00"),
        )
        .unwrap()
        .value;
    let exception = core.event(moved.event_id).unwrap();
    let deleted = core
        .delete_occurrence(moved.event_id, moved.recurrence_id.as_deref().unwrap())
        .unwrap()
        .unit
        .unwrap();
    let second = the_one_on(&core, "2026-10-12");
    let made_since = core
        .reschedule_occurrence(
            yoga.id,
            second.recurrence_id.as_deref().unwrap(),
            Scope::This,
            timed("2026-10-12T08:00:00", "2026-10-12T09:00:00"),
        )
        .unwrap()
        .value;
    assert_ne!(made_since.event_id, exception.id);
    let made_since = core.event(made_since.event_id).unwrap();

    core.undo_unit(deleted).unwrap();

    assert_eq!(core.event(exception.id).unwrap(), exception);
    assert_eq!(core.event(made_since.id).unwrap(), made_since);
    assert_eq!(the_one_on(&core, "2026-10-05").event_id, exception.id);
}

/// The `event` table as it was before ticket 85, whose IDs SQLite gave out
/// again, and the Local Store before ticket 86's server objects, which come
/// after.
const BEFORE_TICKET_85: &str = "
    ALTER TABLE event RENAME TO event_85;
    CREATE TABLE event (
        id INTEGER PRIMARY KEY,
        calendar_id INTEGER NOT NULL REFERENCES calendar (id) ON DELETE CASCADE,
        uid TEXT NOT NULL,
        title TEXT NOT NULL,
        all_day INTEGER NOT NULL,
        start_at TEXT NOT NULL,
        end_at TEXT NOT NULL,
        start_zone TEXT,
        end_zone TEXT,
        start_utc INTEGER NOT NULL,
        end_utc INTEGER NOT NULL,
        location TEXT NOT NULL,
        description TEXT NOT NULL,
        description_html INTEGER NOT NULL DEFAULT 0,
        zone_pinned INTEGER NOT NULL DEFAULT 0,
        rrule TEXT,
        exdates TEXT NOT NULL DEFAULT '',
        recurrence_id TEXT,
        server_href TEXT,
        server_etag TEXT,
        server_data TEXT,
        CHECK (CASE WHEN all_day
            THEN start_zone IS NULL AND end_zone IS NULL
            ELSE start_zone IS NOT NULL AND end_zone IS NOT NULL END)
    );
    INSERT INTO event (id, calendar_id, uid, title, all_day, start_at, end_at, start_zone,
        end_zone, start_utc, end_utc, location, description, description_html, zone_pinned,
        rrule, exdates, recurrence_id)
    SELECT id, calendar_id, uid, title, all_day, start_at, end_at, start_zone,
        end_zone, start_utc, end_utc, location, description, description_html, zone_pinned,
        rrule, exdates, recurrence_id
    FROM event_85;
    DROP TABLE event_85;
    DROP TABLE server_object;
    ALTER TABLE journal_entry DROP COLUMN object_id;
    ALTER TABLE journal_entry ADD COLUMN server_href TEXT;
    ALTER TABLE journal_entry ADD COLUMN server_etag TEXT;
    ALTER TABLE journal_entry ADD COLUMN server_data TEXT;
    CREATE INDEX event_calendar_start ON event (calendar_id, start_utc);
    CREATE INDEX event_calendar_uid ON event (calendar_id, uid);
    PRAGMA user_version = 16;
";

#[test]
fn after_migrating_a_new_event_never_gets_an_id_the_journal_holds() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let (home, dentist) = home_with_dentist(&harness, &core);
    let lunch = core
        .create_event(draft(home.id, "Lunch", all_day("2026-10-06", "2026-10-07")))
        .unwrap()
        .value;
    let deleted = core.delete_event(lunch.id).unwrap().unit.unwrap();
    drop(core);
    harness.arrange_local_store(BEFORE_TICKET_85);
    let core = harness.open();

    let made_since = core
        .create_event(draft(
            home.id,
            "Made since",
            all_day("2026-10-08", "2026-10-09"),
        ))
        .unwrap()
        .value;

    assert!(made_since.id.0 > lunch.id.0);
    core.undo_unit(deleted).unwrap();
    assert_eq!(core.event(lunch.id).unwrap(), lunch);
    assert_eq!(the_one(&core, "Dentist"), dentist);
    assert_eq!(core.event(made_since.id).unwrap(), made_since);
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

/// The entries of an Event moved out of a Calendar deleted since can't be
/// undone: undoing the move would put it back there, and its earlier
/// entries are undone only through the move.
#[test]
fn deleting_a_calendar_removes_the_entries_that_would_put_an_event_back_into_it() {
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
    let dentist = created.value;
    let work = calendar(&core, "Work");
    let moved = core
        .edit_event(dentist.id, draft(work.id, "Dentist", dentist.when))
        .unwrap();
    let renamed = core
        .edit_event(dentist.id, draft(work.id, "Orthodontist", dentist.when))
        .unwrap();
    let gone = core
        .create_event(draft(home.id, "Gone", all_day("2026-10-07", "2026-10-08")))
        .unwrap()
        .value;
    let deleted = core.delete_event(gone.id).unwrap();
    core.delete_calendar(home.id).unwrap();
    harness.signals.clear();

    for unit in [created.unit, moved.unit, deleted.unit].map(Option::unwrap) {
        let refused = core.undo_unit(unit);
        assert!(matches!(refused, Err(CoreError::UnitNotFound(id)) if id == unit));
    }
    assert_eq!(harness.signals.received(), []);
    core.undo_unit(renamed.unit.unwrap()).unwrap();
    assert_eq!(the_one(&core, "Dentist").calendar_id, work.id);
}
