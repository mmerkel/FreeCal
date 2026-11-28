//! What the user sees of the Change Journal (ticket 19): the timeline of
//! Journal Transactions, each with its Events, an Event's History, undoing
//! a Calendar since a given time, and the Undo History, what Ctrl+Z undoes
//! and Ctrl+Shift+Z redoes.

mod support;

use freecal_core::{
    Core, Event, ExistingEvents, ImportFile, JournalEventModificationKind, JournalTransactionId,
    ListedJournalEventModification, ListedJournalTransaction, RepeatEnd, Scope, Signal,
    TransactionOrigin, UndoHistoryStep, UndoOutcome, UndoTarget, UserAction,
};
use support::{Harness, UndoApproved, calendar, date, dentist, draft, event, ics, on, timed, yoga};

fn import(core: &Core, calendar_id: freecal_core::CalendarId, body: &str) -> JournalTransactionId {
    let preview = core
        .preview_import(&ImportFile::new("events.ics", ics(body)), calendar_id)
        .unwrap();
    core.run_import(&preview, ExistingEvents::Update)
        .unwrap()
        .journal_transaction
        .unwrap()
}

/// The time the harness's clock stands at, as Journal Transactions record it.
fn now(harness: &Harness) -> i64 {
    use freecal_core::Clock;
    harness.clock.now().timestamp()
}

fn whole(id: JournalTransactionId) -> UndoTarget {
    UndoTarget::JournalTransaction { id }
}

fn titles(core: &Core) -> Vec<String> {
    core.occurrences(date("2000-01-01"), date("2100-01-01"))
        .unwrap()
        .into_iter()
        .map(|occurrence| occurrence.title)
        .collect()
}

fn renamed(core: &Core, event: &Event, title: &str) -> JournalTransactionId {
    core.edit_event(
        &event.combined_event_key,
        draft(event.calendar_id, title, event.when),
    )
    .unwrap()
    .journal_transaction
    .unwrap()
}

const MONDAY: (&str, &str) = ("DTSTART:20261005T070000Z", "DTEND:20261005T080000Z");
const TUESDAY: (&str, &str) = ("DTSTART:20261006T070000Z", "DTEND:20261006T080000Z");

mod the_timeline {
    use super::*;

    #[test]
    fn lists_every_journal_transaction_newest_first() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let made = now(&harness);
        let created = core.create_event(dentist(home.id)).unwrap();
        let dentist = created.value;
        let created = created.journal_transaction.unwrap();
        let edited = renamed(&core, &dentist, "Dentist at 9");
        let imported = import(
            &core,
            home.id,
            &format!(
                "{}{}",
                event("a", MONDAY.0, MONDAY.1),
                event("b", TUESDAY.0, TUESDAY.1)
            ),
        );
        let undo = core
            .undo(whole(edited), None)
            .unwrap()
            .journal_transaction
            .unwrap();

        let listed = core.change_journal().unwrap();

        let expected = |id, origin, undoes, events, undone_events, titles: &[&str], per_event| {
            ListedJournalTransaction {
                id,
                origin,
                made_at: made,
                undoes,
                events,
                undone_events,
                calendar_ids: vec![home.id],
                titles: titles.iter().map(|title| (*title).to_owned()).collect(),
                undone_per_event: per_event,
                undo_changed_nothing: false,
            }
        };
        assert_eq!(
            listed,
            vec![
                expected(
                    undo,
                    TransactionOrigin::Undo,
                    Some(edited),
                    1,
                    0,
                    &["Dentist"],
                    false
                ),
                expected(
                    imported,
                    TransactionOrigin::Import,
                    None,
                    2,
                    0,
                    &["a", "b"],
                    true
                ),
                expected(
                    edited,
                    TransactionOrigin::UserAction {
                        action: UserAction::Edited
                    },
                    None,
                    1,
                    1,
                    &["Dentist at 9"],
                    false
                ),
                expected(
                    created,
                    TransactionOrigin::UserAction {
                        action: UserAction::Created
                    },
                    None,
                    1,
                    0,
                    &["Dentist"],
                    false
                ),
            ]
        );
    }

    #[test]
    fn a_move_to_another_calendar_names_both() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let work = calendar(&core, "Work");
        let dentist = core.create_event(dentist(home.id)).unwrap().value;

        let moved = core
            .edit_event(
                &dentist.combined_event_key,
                draft(work.id, "Dentist", dentist.when),
            )
            .unwrap()
            .journal_transaction
            .unwrap();

        let listed = core.change_journal().unwrap();
        assert_eq!(listed[0].id, moved);
        assert_eq!(listed[0].calendar_ids, vec![home.id, work.id]);
    }

    #[test]
    fn a_journal_transaction_expands_into_its_events() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let made = now(&harness);
        let imported = import(
            &core,
            home.id,
            &format!(
                "{}{}",
                event("a", MONDAY.0, MONDAY.1),
                event("b", TUESDAY.0, TUESDAY.1)
            ),
        );
        let a = on(&core, "2026-10-05").combined_event_key;
        let b = on(&core, "2026-10-06").combined_event_key;
        core.undo(
            UndoTarget::Event {
                journal_transaction: imported,
                combined_event_key: a.clone(),
            },
            None,
        )
        .unwrap();

        let listed = core.journal_transaction_events(imported).unwrap();

        let expected = |combined_event_key, title: &str, undone| ListedJournalEventModification {
            journal_transaction: imported,
            origin: TransactionOrigin::Import,
            made_at: made,
            title: title.to_owned(),
            calendar_id: home.id,
            kind: JournalEventModificationKind::Created,
            undone,
            undo: (!undone).then_some(UndoTarget::Event {
                journal_transaction: imported,
                combined_event_key,
            }),
        };
        assert_eq!(
            listed,
            vec![expected(a, "a", true), expected(b, "b", false)]
        );
    }

    #[test]
    fn a_user_actions_events_are_undone_with_the_whole_journal_transaction() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let dentist = core.create_event(dentist(home.id)).unwrap().value;

        let deleted = core
            .delete_event(&dentist.combined_event_key)
            .unwrap()
            .journal_transaction
            .unwrap();

        let listed = core.journal_transaction_events(deleted).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].kind, JournalEventModificationKind::Deleted);
        assert_eq!(listed[0].title, "Dentist");
        assert_eq!(listed[0].calendar_id, home.id);
        assert_eq!(listed[0].undo, Some(whole(deleted)));
    }

    #[test]
    fn an_unknown_journal_transaction_is_refused() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();

        let refused = core.journal_transaction_events(JournalTransactionId(99));

        assert!(matches!(
            refused,
            Err(freecal_core::CoreError::JournalTransactionNotFound(
                JournalTransactionId(99)
            ))
        ));
    }
}

mod an_events_history {
    use super::*;

    #[test]
    fn lists_its_journal_event_modifications_newest_first_and_no_other_events() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let created = core.create_event(dentist(home.id)).unwrap();
        let dentist = created.value;
        let created = created.journal_transaction.unwrap();
        core.create_event(draft(
            home.id,
            "Lunch",
            timed("2026-10-05T12:00:00", "2026-10-05T13:00:00"),
        ))
        .unwrap();
        let edited = renamed(&core, &dentist, "Dentist at 9");

        let history = core.event_history(&dentist.combined_event_key).unwrap();

        let kinds: Vec<_> = history
            .iter()
            .map(|listed| {
                (
                    listed.journal_transaction,
                    listed.kind,
                    listed.title.as_str(),
                )
            })
            .collect();
        assert_eq!(
            kinds,
            vec![
                (edited, JournalEventModificationKind::Edited, "Dentist at 9"),
                (created, JournalEventModificationKind::Created, "Dentist"),
            ]
        );
        assert_eq!(history[1].undo, Some(whole(created)));
    }

    #[test]
    fn of_a_recurring_event_has_its_exceptions() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let yoga = yoga(&core, home.id, RepeatEnd::Count { count: 3 });
        let occurrence = core
            .occurrences(date("2026-10-12"), date("2026-10-13"))
            .unwrap()
            .remove(0);
        let moved = core
            .reschedule_occurrence(
                &yoga,
                &occurrence.recurrence_id.clone().unwrap(),
                Scope::This,
                false,
                timed("2026-10-12T19:00:00", "2026-10-12T20:00:00"),
            )
            .unwrap()
            .journal_transaction
            .unwrap();

        let history = core.event_history(&yoga).unwrap();

        assert_eq!(history.len(), 2);
        assert_eq!(history[0].journal_transaction, moved);
    }

    #[test]
    fn an_undo_from_the_history_writes_the_previous_event_version_back() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let dentist = core.create_event(dentist(home.id)).unwrap().value;
        renamed(&core, &dentist, "Dentist at 9");
        harness.signals.clear();

        let edit = core
            .event_history(&dentist.combined_event_key)
            .unwrap()
            .remove(0);
        core.undo(edit.undo.unwrap(), None).unwrap();

        assert_eq!(core.event(&dentist.combined_event_key).unwrap(), dentist);
        assert_eq!(
            harness.signals.received_but_sync_status(),
            vec![Signal::OccurrencesChanged {
                calendar_ids: vec![home.id]
            }]
        );
        let history = core.event_history(&dentist.combined_event_key).unwrap();
        assert_eq!(history[0].origin, TransactionOrigin::Undo);
        assert!(history[1].undone);
        assert_eq!(history[1].undo, None);
    }
}

mod undoing_a_calendar_since_a_given_time {
    use super::*;

    #[test]
    fn undoes_every_change_to_its_events_since_then_and_no_other() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let work = calendar(&core, "Work");
        let dentist = core.create_event(dentist(home.id)).unwrap().value;
        harness.clock.set("2026-10-03T13:00:00+02:00");
        let since = now(&harness);
        renamed(&core, &dentist, "Dentist at 9");
        core.create_event(draft(
            home.id,
            "Lunch",
            timed("2026-10-05T12:00:00", "2026-10-05T13:00:00"),
        ))
        .unwrap();
        core.create_event(draft(
            work.id,
            "Standup",
            timed("2026-10-05T10:00:00", "2026-10-05T10:15:00"),
        ))
        .unwrap();
        harness.signals.clear();

        let undone = core
            .undo(
                UndoTarget::CalendarSince {
                    calendar_id: home.id,
                    since,
                },
                None,
            )
            .unwrap();

        assert_eq!(undone.value, UndoOutcome::Undone { skipped: vec![] });
        assert!(undone.journal_transaction.is_some());
        assert_eq!(titles(&core), vec!["Dentist", "Standup"]);
        assert_eq!(
            harness.signals.received_but_sync_status(),
            vec![Signal::OccurrencesChanged {
                calendar_ids: vec![home.id]
            }]
        );
        let listed = core.change_journal().unwrap();
        assert_eq!(listed[0].undoes, None);
    }

    #[test]
    fn a_user_action_with_events_in_another_calendar_is_undone_whole() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let work = calendar(&core, "Work");
        let dentist = core.create_event(dentist(home.id)).unwrap().value;
        harness.clock.set("2026-10-03T13:00:00+02:00");
        let since = now(&harness);
        core.edit_event(
            &dentist.combined_event_key,
            draft(work.id, "Dentist", dentist.when),
        )
        .unwrap();

        core.undo(
            UndoTarget::CalendarSince {
                calendar_id: work.id,
                since,
            },
            None,
        )
        .unwrap();

        assert_eq!(core.event(&dentist.combined_event_key).unwrap(), dentist);
    }

    #[test]
    fn with_nothing_since_then_changes_nothing() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        core.create_event(dentist(home.id)).unwrap();
        harness.clock.set("2026-10-03T13:00:00+02:00");
        harness.signals.clear();

        let undone = core
            .undo(
                UndoTarget::CalendarSince {
                    calendar_id: home.id,
                    since: now(&harness),
                },
                None,
            )
            .unwrap();

        assert_eq!(undone.journal_transaction, None);
        assert_eq!(undone.value, UndoOutcome::Undone { skipped: vec![] });
        assert_eq!(titles(&core), vec!["Dentist"]);
        assert_eq!(harness.signals.received(), vec![]);
    }

    #[test]
    fn many_events_ask_first() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let since = now(&harness);
        let body: String = (0..11)
            .map(|day| {
                event(
                    &format!("e{day}"),
                    &format!("DTSTART:202610{:02}T070000Z", day + 1),
                    &format!("DTEND:202610{:02}T080000Z", day + 1),
                )
            })
            .collect();
        import(&core, home.id, &body);
        let target = UndoTarget::CalendarSince {
            calendar_id: home.id,
            since,
        };

        let asked = core.undo(target.clone(), None).unwrap();

        let UndoOutcome::Dangerous { danger, .. } = asked.value else {
            panic!("expected a Dangerous Undo, got {asked:?}");
        };
        assert!(danger.exceeds_mass_change_guard);
        assert_eq!(titles(&core).len(), 11);
        core.undo(target, Some(&danger)).unwrap();
        assert_eq!(titles(&core), Vec::<String>::new());
    }
}

mod the_undo_history {
    use super::*;

    /// One step, as the Journal Transaction to undo and the user change it
    /// undoes or redoes.
    type Step = Option<(JournalTransactionId, Option<JournalTransactionId>)>;

    fn ids(step: Option<UndoHistoryStep>) -> Step {
        step.map(|step| {
            (
                step.journal_transaction,
                step.user_change.map(|user_change| user_change.id),
            )
        })
    }

    /// What Ctrl+Z and Ctrl+Shift+Z undo next, in that order.
    fn steps(core: &Core) -> (Step, Step) {
        let history = core.undo_history().unwrap();
        (ids(history.undo), ids(history.redo))
    }

    fn step(journal_transaction: JournalTransactionId, user_change: JournalTransactionId) -> Step {
        Some((journal_transaction, Some(user_change)))
    }

    #[test]
    fn outlasts_a_restart() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let created = core
            .create_event(dentist(home.id))
            .unwrap()
            .journal_transaction
            .unwrap();
        drop(core);

        let core = harness.open();

        assert_eq!(steps(&core), (step(created, created), None));
    }

    #[test]
    fn names_the_kind_of_each_user_action() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let created = core.create_event(dentist(home.id)).unwrap().value;
        let action = |core: &Core| match core.undo_history().unwrap().undo.unwrap().user_change {
            Some(ListedJournalTransaction {
                origin: TransactionOrigin::UserAction { action },
                ..
            }) => action,
            other => panic!("not a user action: {other:?}"),
        };
        assert_eq!(action(&core), UserAction::Created);

        renamed(&core, &created, "Dentist at 9");
        assert_eq!(action(&core), UserAction::Edited);

        core.reschedule_event(
            &created.combined_event_key,
            timed("2026-10-05T12:00:00", "2026-10-05T13:00:00"),
        )
        .unwrap();
        assert_eq!(action(&core), UserAction::Moved);

        core.delete_event(&created.combined_event_key).unwrap();
        assert_eq!(action(&core), UserAction::Deleted);
    }

    #[test]
    fn undoes_the_users_changes_newest_first_and_redoes_the_undos_back_to_front() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let created = core.create_event(dentist(home.id)).unwrap();
        let dentist = created.value;
        let created = created.journal_transaction.unwrap();
        let edited = renamed(&core, &dentist, "Dentist at 9");
        assert_eq!(steps(&core), (step(edited, edited), None));

        let undid_edit = core
            .undo(whole(edited), None)
            .unwrap()
            .journal_transaction
            .unwrap();
        let undid_create = core
            .undo(whole(created), None)
            .unwrap()
            .journal_transaction
            .unwrap();
        assert_eq!(steps(&core), (None, step(undid_create, created)));

        let redid_create = core
            .undo(whole(undid_create), None)
            .unwrap()
            .journal_transaction
            .unwrap();
        assert_eq!(
            steps(&core),
            (step(redid_create, created), step(undid_edit, edited))
        );

        let redid_edit = core
            .undo(whole(undid_edit), None)
            .unwrap()
            .journal_transaction
            .unwrap();
        assert_eq!(titles(&core), vec!["Dentist at 9"]);
        assert_eq!(steps(&core), (step(redid_edit, edited), None));

        // Undoing a redo again is an undo, which can be redone.
        let undid_again = core
            .undo(whole(redid_edit), None)
            .unwrap()
            .journal_transaction
            .unwrap();
        assert_eq!(
            steps(&core),
            (step(redid_create, created), step(undid_again, edited))
        );
    }

    #[test]
    fn a_new_change_ends_redo() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let created = core.create_event(dentist(home.id)).unwrap();
        let dentist = created.value;
        let edited = renamed(&core, &dentist, "Dentist at 9");
        core.undo(whole(edited), None).unwrap();

        let lunch = core
            .create_event(draft(
                home.id,
                "Lunch",
                timed("2026-10-05T12:00:00", "2026-10-05T13:00:00"),
            ))
            .unwrap()
            .journal_transaction
            .unwrap();

        assert_eq!(steps(&core), (step(lunch, lunch), None));
    }

    #[test]
    fn counts_an_undo_from_the_history_at_once() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let created = core.create_event(dentist(home.id)).unwrap();
        let dentist = created.value;
        let created = created.journal_transaction.unwrap();
        let edited = renamed(&core, &dentist, "Dentist at 9");

        let edit = core
            .event_history(&dentist.combined_event_key)
            .unwrap()
            .remove(0);
        let undone = core
            .undo(edit.undo.unwrap(), None)
            .unwrap()
            .journal_transaction
            .unwrap();

        assert_eq!(steps(&core), (step(created, created), step(undone, edited)));
    }

    #[test]
    fn has_imports_and_undoes_the_rest_of_one_partly_undone() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let imported = import(
            &core,
            home.id,
            &format!(
                "{}{}",
                event("a", MONDAY.0, MONDAY.1),
                event("b", TUESDAY.0, TUESDAY.1)
            ),
        );
        let a = core.journal_transaction_events(imported).unwrap()[0]
            .undo
            .clone()
            .unwrap();
        let undid_a = core.undo(a, None).unwrap().journal_transaction.unwrap();

        assert_eq!(
            steps(&core),
            (step(imported, imported), step(undid_a, imported))
        );
    }

    #[test]
    fn an_undo_of_a_calendar_since_a_time_is_redone_without_a_user_change() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let since = now(&harness);
        core.create_event(dentist(home.id)).unwrap();

        let undone = core
            .undo(
                UndoTarget::CalendarSince {
                    calendar_id: home.id,
                    since,
                },
                None,
            )
            .unwrap()
            .journal_transaction
            .unwrap();

        assert_eq!(steps(&core), (None, Some((undone, None))));
    }

    #[test]
    fn passes_over_a_change_whose_undo_changed_nothing() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let work = calendar(&core, "Work");
        let created = core
            .create_event(dentist(work.id))
            .unwrap()
            .journal_transaction
            .unwrap();
        let in_home = core
            .create_event(dentist(home.id))
            .unwrap()
            .journal_transaction
            .unwrap();
        core.set_calendar_read_only(home.id, true).unwrap();

        let undone = core.undo(whole(in_home), None).unwrap();

        assert_eq!(undone.journal_transaction, None);
        assert!(matches!(undone.value, UndoOutcome::Undone { ref skipped } if skipped.len() == 1));
        assert!(
            core.change_journal()
                .unwrap()
                .iter()
                .any(|listed| listed.id == in_home && listed.undo_changed_nothing)
        );
        drop(core);
        let core = harness.open();
        assert_eq!(steps(&core), (step(created, created), None));
    }

    #[test]
    fn approved_undo_runs_on_the_undo_history_step() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let created = core
            .create_event(dentist(home.id))
            .unwrap()
            .journal_transaction
            .unwrap();

        let step = core.undo_history().unwrap().undo.unwrap();
        core.undo_approved(step.journal_transaction).unwrap();

        assert_eq!(titles(&core), Vec::<String>::new());
        assert_eq!(steps(&core).1.unwrap().1, Some(created));
    }
}
