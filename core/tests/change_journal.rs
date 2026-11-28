//! The Change Journal: every change to an Event is a Journal Transaction
//! that undoes it exactly, and an undo is a Journal Transaction too, which
//! redoes it. Undo per Event is in `undo_per_event.rs`.

mod support;

use support::UndoApproved;

use freecal_core::{
    Calendar, CalendarId, Core, CoreError, Event, ExistingEvents, ImportFile, JournalTransactionId,
    RepeatEnd, Scope, Signal,
};
use support::{Harness, all_day, calendar, date, draft, event, ics, on, timed, yoga};

/// Every Event of the shown Calendars, with everything its details show.
fn contents(core: &Core) -> Vec<Event> {
    core.occurrences(date("2000-01-01"), date("2100-01-01"))
        .unwrap()
        .into_iter()
        .map(|occurrence| core.event(&occurrence.combined_event_key).unwrap())
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

fn import(
    core: &Core,
    calendar_id: CalendarId,
    contents: Vec<u8>,
    existing: ExistingEvents,
) -> JournalTransactionId {
    let preview = core
        .preview_import(&ImportFile::new("events.ics", contents), calendar_id)
        .unwrap();
    core.run_import(&preview, existing)
        .unwrap()
        .journal_transaction
        .unwrap()
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

/// Checks that undoing `journal_transaction` brings back `before` and
/// signals `calendars`.
fn undoes_exactly(
    harness: &Harness,
    core: &Core,
    journal_transaction: JournalTransactionId,
    before: &[Event],
    calendars: &[CalendarId],
) {
    assert_ne!(contents(core), before, "the change changed something");
    harness.signals.clear();

    let undone = core.undo_approved(journal_transaction).unwrap();

    assert!(undone.journal_transaction.is_some());
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

        undoes_exactly(
            &harness,
            &core,
            created.journal_transaction.unwrap(),
            &[],
            &[home.id],
        );
    }

    #[test]
    fn edit() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let (home, dentist) = home_with_dentist(&harness, &core);
        let before = contents(&core);
        let mut edit = draft(home.id, "Orthodontist", dentist.when);
        edit.location = "Main Street 1".to_owned();

        let edited = core.edit_event(&dentist.combined_event_key, edit).unwrap();

        undoes_exactly(
            &harness,
            &core,
            edited.journal_transaction.unwrap(),
            &before,
            &[home.id],
        );
    }

    #[test]
    fn reschedule() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let (home, dentist) = home_with_dentist(&harness, &core);
        let before = contents(&core);

        let rescheduled = core
            .reschedule_event(
                &dentist.combined_event_key,
                all_day("2026-10-07", "2026-10-08"),
            )
            .unwrap();

        undoes_exactly(
            &harness,
            &core,
            rescheduled.journal_transaction.unwrap(),
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

        let deleted = core.delete_event(&dentist.combined_event_key).unwrap();

        undoes_exactly(
            &harness,
            &core,
            deleted.journal_transaction.unwrap(),
            &before,
            &[home.id],
        );
        assert_eq!(core.event(&dentist.combined_event_key).unwrap(), dentist);
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

        let journal_transaction = import(&core, home.id, file, ExistingEvents::Update);

        undoes_exactly(&harness, &core, journal_transaction, &before, &[home.id]);
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
        .edit_event(
            &dentist.combined_event_key,
            draft(work.id, "Dentist", dentist.when),
        )
        .unwrap();

    assert_eq!(the_one(&core, "Dentist").calendar_id, work.id);
    undoes_exactly(
        &harness,
        &core,
        moved.journal_transaction.unwrap(),
        &before,
        &[work.id, home.id],
    );
}

#[test]
fn a_change_that_changes_nothing_is_no_journal_transaction_and_sends_no_signal() {
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
        .edit_event(
            &dentist.combined_event_key,
            draft(home.id, "Dentist", dentist.when),
        )
        .unwrap();
    let rescheduled = core
        .reschedule_event(&dentist.combined_event_key, dentist.when)
        .unwrap();
    let preview = core
        .preview_import(
            &ImportFile::new("events.ics", ics(&event("kept", MONDAY.0, MONDAY.1))),
            home.id,
        )
        .unwrap();
    let updated = core.run_import(&preview, ExistingEvents::Update).unwrap();

    assert_eq!(edited.journal_transaction, None);
    assert_eq!(edited.value, dentist);
    assert_eq!(rescheduled.journal_transaction, None);
    assert_eq!((updated.journal_transaction, updated.updated), (None, 1));
    assert_eq!(harness.signals.received(), []);
}

#[test]
fn undoing_the_undo_redoes_and_an_undone_journal_transaction_is_undone_once() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let (home, dentist) = home_with_dentist(&harness, &core);
    let deleted = core
        .delete_event(&dentist.combined_event_key)
        .unwrap()
        .journal_transaction
        .unwrap();
    let undo = core
        .undo_approved(deleted)
        .unwrap()
        .journal_transaction
        .unwrap();
    assert_eq!(contents(&core), std::slice::from_ref(&dentist));
    harness.signals.clear();

    let again = core.undo_approved(deleted);
    let redo = core.undo_approved(undo).unwrap();

    assert!(matches!(again, Err(CoreError::AlreadyUndone(id)) if id == deleted));
    assert_eq!(contents(&core), []);
    assert_eq!(harness.signals.received(), changed(&[home.id]));
    assert!(matches!(
        core.undo_approved(undo),
        Err(CoreError::AlreadyUndone(_))
    ));
    core.undo_approved(redo.journal_transaction.unwrap())
        .unwrap();
    assert_eq!(contents(&core), [dentist]);
}

/// Ticket 85: Exceptions share their Recurring Event's UID, so only an ID
/// that is never given out again tells a new Exception from a deleted one.
#[test]
fn a_deleted_exception_comes_back_under_its_id_beside_one_made_since() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let home = calendar(&core, "Home");
    let yoga = yoga(&core, home.id, RepeatEnd::Count { count: 3 });
    let first = on(&core, "2026-10-05");
    let moved = core
        .reschedule_occurrence(
            &yoga,
            first.recurrence_id.as_deref().unwrap(),
            Scope::This,
            false,
            timed("2026-10-05T08:00:00", "2026-10-05T09:00:00"),
        )
        .unwrap()
        .value;
    let exception = core.event(&moved.combined_event_key).unwrap();
    let deleted = core
        .delete_occurrence(
            &moved.combined_event_key,
            moved.recurrence_id.as_deref().unwrap(),
        )
        .unwrap()
        .journal_transaction
        .unwrap();
    let second = on(&core, "2026-10-12");
    let made_since = core
        .reschedule_occurrence(
            &yoga,
            second.recurrence_id.as_deref().unwrap(),
            Scope::This,
            false,
            timed("2026-10-12T08:00:00", "2026-10-12T09:00:00"),
        )
        .unwrap()
        .value;
    assert_ne!(made_since.combined_event_key, exception.combined_event_key);
    let made_since = core.event(&made_since.combined_event_key).unwrap();

    core.undo_approved(deleted).unwrap();

    assert_eq!(
        core.event(&exception.combined_event_key).unwrap(),
        exception
    );
    assert_eq!(
        core.event(&made_since.combined_event_key).unwrap(),
        made_since
    );
    assert_eq!(
        on(&core, "2026-10-05").combined_event_key,
        exception.combined_event_key
    );
}

#[test]
fn deleting_a_calendar_removes_the_journal_transactions_of_its_events() {
    let harness = Harness::new();
    let core = harness.open_without_starting_calendars();
    let home = calendar(&core, "Home");
    let journal_transaction = import(
        &core,
        home.id,
        ics(&event("new", MONDAY.0, MONDAY.1)),
        ExistingEvents::Skip,
    );
    core.delete_calendar(home.id).unwrap();
    harness.signals.clear();

    let refused = core.undo_approved(journal_transaction);

    assert!(
        matches!(refused, Err(CoreError::JournalTransactionNotFound(id)) if id == journal_transaction)
    );
    assert_eq!(harness.signals.received(), []);
}

/// The Journal Event Modifications of an Event moved out of a Calendar deleted
/// since can't be undone: undoing the move would put it back there, and its
/// earlier Journal Event Modifications are undone only through the move.
#[test]
fn deleting_a_calendar_removes_the_journal_event_modifications_putting_events_back() {
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
        .edit_event(
            &dentist.combined_event_key,
            draft(work.id, "Dentist", dentist.when),
        )
        .unwrap();
    let renamed = core
        .edit_event(
            &moved.value.combined_event_key,
            draft(work.id, "Orthodontist", dentist.when),
        )
        .unwrap();
    let gone = core
        .create_event(draft(home.id, "Gone", all_day("2026-10-07", "2026-10-08")))
        .unwrap()
        .value;
    let deleted = core.delete_event(&gone.combined_event_key).unwrap();
    core.delete_calendar(home.id).unwrap();
    harness.signals.clear();

    for journal_transaction in [
        created.journal_transaction,
        moved.journal_transaction,
        deleted.journal_transaction,
    ]
    .map(Option::unwrap)
    {
        let refused = core.undo_approved(journal_transaction);
        assert!(
            matches!(refused, Err(CoreError::JournalTransactionNotFound(id)) if id == journal_transaction)
        );
    }
    assert_eq!(harness.signals.received(), []);
    core.undo_approved(renamed.journal_transaction.unwrap())
        .unwrap();
    assert_eq!(the_one(&core, "Dentist").calendar_id, work.id);
}
