//! Undo per Event (ADR 0009): an Event changed since has its later changes
//! undone first, an import is undone one Event at a time, Events that can't
//! be undone are skipped, and an undo says whether it is a Dangerous Undo.
//! Undoing changes pulled from a server is in `caldav_sync.rs`.

mod support;

use freecal_core::{
    Calendar, CalendarId, Core, CoreError, DangerousUndo, Event, ExistingEvents, ImportFile,
    JournalTransactionId, LaterJournalTransaction, RepeatEnd, Scope, Signal, SkipReason,
    SkippedEvent, UndoOutcome, UndoTarget,
};
use support::{Harness, UndoApproved, calendar, date, draft, event, ics, on, timed, yoga};

/// Every Event of the shown Calendars, one per Occurrence.
fn contents(core: &Core) -> Vec<Event> {
    core.occurrences(date("2000-01-01"), date("2100-01-01"))
        .unwrap()
        .into_iter()
        .map(|occurrence| core.event(&occurrence.combined_event_key).unwrap())
        .collect()
}

fn import(core: &Core, calendar_id: CalendarId, body: &str) -> JournalTransactionId {
    let preview = core
        .preview_import(&ImportFile::new("events.ics", ics(body)), calendar_id)
        .unwrap();
    core.run_import(&preview, ExistingEvents::Update)
        .unwrap()
        .journal_transaction
        .unwrap()
}

fn whole(id: JournalTransactionId) -> UndoTarget {
    UndoTarget::JournalTransaction { id }
}

fn one(journal_transaction: JournalTransactionId, event: &Event) -> UndoTarget {
    UndoTarget::Event {
        journal_transaction,
        combined_event_key: event.combined_event_key.clone(),
    }
}

fn undone(skipped: Vec<SkippedEvent>) -> UndoOutcome {
    UndoOutcome::Undone { skipped }
}

/// The danger of an undo that changes nothing until approved.
fn danger(core: &Core, target: UndoTarget) -> DangerousUndo {
    let before = contents(core);
    match core.undo(target, None).unwrap() {
        freecal_core::Changed {
            journal_transaction: None,
            value: UndoOutcome::Dangerous { danger, .. },
        } => {
            assert_eq!(contents(core), before, "a Dangerous Undo changes nothing");
            danger
        }
        other => panic!("expected a Dangerous Undo, got {other:?}"),
    }
}

/// The time the harness's clock stands at, as Journal Transactions record it.
fn now(harness: &Harness) -> i64 {
    use freecal_core::Clock;
    harness.clock.now().timestamp()
}

fn the_event(core: &Core, day: &str) -> Event {
    core.event(&on(core, day).combined_event_key).unwrap()
}

const MONDAY: (&str, &str) = ("DTSTART:20261005T070000Z", "DTEND:20261005T080000Z");
const TUESDAY: (&str, &str) = ("DTSTART:20261006T070000Z", "DTEND:20261006T080000Z");
const WEDNESDAY: (&str, &str) = ("DTSTART:20261007T070000Z", "DTEND:20261007T080000Z");

/// Ticket 58's cases: an import that adds `new` on Monday and updates
/// `kept` from Monday to Tuesday, then a change to one of them. Undoing the
/// import undoes that change first, as a Dangerous Undo naming it.
mod an_event_changed_since_has_its_later_changes_undone_first {
    use super::*;

    fn cascades(change: impl FnOnce(&Core, &Calendar) -> JournalTransactionId, title: &str) {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        import(&core, home.id, &event("kept", MONDAY.0, MONDAY.1));
        let before = contents(&core);
        let both = [
            event("kept", TUESDAY.0, TUESDAY.1),
            event("new", MONDAY.0, MONDAY.1),
        ]
        .concat();
        let imported = import(&core, home.id, &both);
        let later = change(&core, &home);
        let changed = contents(&core);
        harness.signals.clear();

        let danger = danger(&core, whole(imported));
        let ran = core.undo(whole(imported), Some(&danger)).unwrap();

        assert_eq!(
            danger,
            DangerousUndo {
                later_journal_transactions: vec![LaterJournalTransaction {
                    id: later,
                    made_at: now(&harness),
                    titles: vec![title.to_owned()],
                }],
                server_changes: 0,
                exceeds_mass_change_guard: false,
                events: 2,
            }
        );
        assert!(ran.journal_transaction.is_some());
        assert_eq!(ran.value, undone(vec![]));
        assert_eq!(contents(&core), before);
        assert!(matches!(
            core.undo(whole(later), None),
            Err(CoreError::AlreadyUndone(id)) if id == later
        ));
        // The undo of both is one Journal Transaction, which redoes both.
        core.undo_approved(ran.journal_transaction.unwrap())
            .unwrap();
        assert_eq!(contents(&core), changed);
    }

    #[test]
    fn an_added_event_moved_to_another_calendar() {
        cascades(
            |core, _| {
                let work = calendar(core, "Work");
                let new = the_event(core, "2026-10-05");
                core.edit_event(&new.combined_event_key, draft(work.id, "new", new.when))
                    .unwrap()
                    .journal_transaction
                    .unwrap()
            },
            "new",
        );
    }

    #[test]
    fn an_updated_event_edited() {
        cascades(
            |core, home| {
                let kept = the_event(core, "2026-10-06");
                core.edit_event(&kept.combined_event_key, draft(home.id, "Kept", kept.when))
                    .unwrap()
                    .journal_transaction
                    .unwrap()
            },
            "Kept",
        );
    }

    #[test]
    fn an_updated_event_deleted() {
        cascades(
            |core, _| {
                core.delete_event(&the_event(core, "2026-10-06").combined_event_key)
                    .unwrap()
                    .journal_transaction
                    .unwrap()
            },
            "kept",
        );
    }

    /// The later changes are undone newest first, each Event back through
    /// every version it had.
    #[test]
    fn through_several_later_changes_newest_first() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let imported = import(&core, home.id, &event("new", MONDAY.0, MONDAY.1));
        let new = the_event(&core, "2026-10-05");
        let renamed = core
            .edit_event(&new.combined_event_key, draft(home.id, "Renamed", new.when))
            .unwrap();
        let moved = core
            .edit_event(
                &new.combined_event_key,
                draft(home.id, "Renamed", the_event(&core, "2026-10-05").when),
            )
            .unwrap();
        let deleted = core.delete_event(&new.combined_event_key).unwrap();

        let danger = danger(&core, whole(imported));
        core.undo(whole(imported), Some(&danger)).unwrap();

        let later: Vec<_> = danger
            .later_journal_transactions
            .iter()
            .map(|later| later.id)
            .collect();
        let expected: Vec<_> = [renamed.journal_transaction, moved.journal_transaction]
            .into_iter()
            .chain([deleted.journal_transaction])
            .flatten()
            .collect();
        assert_eq!(later, expected);
        assert_eq!(contents(&core), []);
    }

    /// An Event whose later change was undone already is back where the
    /// undo needs it, so nothing more is undone and the undo is benign.
    #[test]
    fn not_a_later_change_undone_already() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let imported = import(&core, home.id, &event("new", MONDAY.0, MONDAY.1));
        let new = the_event(&core, "2026-10-05");
        let renamed = core
            .edit_event(&new.combined_event_key, draft(home.id, "Renamed", new.when))
            .unwrap();
        core.undo(whole(renamed.journal_transaction.unwrap()), None)
            .unwrap();

        let ran = core.undo(whole(imported), None).unwrap();

        assert_eq!(ran.value, undone(vec![]));
        assert_eq!(contents(&core), []);
    }

    /// The Event's version now was made both by an undo and by the change
    /// that undo's undone change followed; only the change leads back.
    #[test]
    fn past_an_undo_to_the_change_that_leads_back() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let imported = import(&core, home.id, &event("new", MONDAY.0, MONDAY.1));
        let new = the_event(&core, "2026-10-05");
        let one = core
            .edit_event(&new.combined_event_key, draft(home.id, "One", new.when))
            .unwrap()
            .journal_transaction
            .unwrap();
        let two = core
            .edit_event(&new.combined_event_key, draft(home.id, "Two", new.when))
            .unwrap()
            .journal_transaction
            .unwrap();
        core.undo(whole(two), None).unwrap();

        let danger = danger(&core, whole(imported));
        let ran = core.undo(whole(imported), Some(&danger)).unwrap();

        assert_eq!(ran.value, undone(vec![]));
        let later: Vec<_> = danger
            .later_journal_transactions
            .iter()
            .map(|later| later.id)
            .collect();
        assert_eq!(later, [one]);
        assert_eq!(contents(&core), []);
    }

    /// A Dangerous Undo approved for another danger than it has by then
    /// asks again.
    #[test]
    fn and_only_the_danger_approved_runs() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let imported = import(&core, home.id, &event("new", MONDAY.0, MONDAY.1));
        let new = the_event(&core, "2026-10-05");
        core.edit_event(&new.combined_event_key, draft(home.id, "Renamed", new.when))
            .unwrap();
        let approved = danger(&core, whole(imported));
        core.edit_event(
            &new.combined_event_key,
            draft(home.id, "Renamed again", new.when),
        )
        .unwrap();

        let asked = core.undo(whole(imported), Some(&approved)).unwrap();

        assert!(
            matches!(&asked.value, UndoOutcome::Dangerous { danger, .. } if danger.later_journal_transactions.len() == 2)
        );
        assert_eq!(asked.journal_transaction, None);
        assert_eq!(contents(&core)[0].title, "Renamed again");
    }
}

/// A user action's Journal Transaction is undone only whole, as when "this
/// and following" splits a Recurring Event in two: undoing one half would
/// make Occurrences vanish or appear twice.
mod a_split_by_this_and_following {
    use super::*;

    /// Yoga on Mondays in October, moved an hour later from the 19th on,
    /// which splits it in two; returns the split and both halves.
    fn split(core: &Core) -> (JournalTransactionId, Event, Event) {
        let home = calendar(core, "Home");
        yoga(core, home.id, RepeatEnd::Count { count: 4 });
        let third = on(core, "2026-10-19");
        let split = core
            .reschedule_occurrence(
                &third.combined_event_key,
                third.recurrence_id.as_deref().unwrap(),
                Scope::ThisAndFollowing,
                false,
                timed("2026-10-19T19:00:00", "2026-10-19T20:00:00"),
            )
            .unwrap()
            .journal_transaction
            .unwrap();
        let first = the_event(core, "2026-10-05");
        let following = the_event(core, "2026-10-19");
        assert_ne!(first.combined_event_key, following.combined_event_key);
        (split, first, following)
    }

    #[test]
    fn is_not_undone_from_either_half_alone() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let (split, first, following) = split(&core);

        for half in [&first, &following] {
            assert!(matches!(
                core.undo(one(split, half), None),
                Err(CoreError::UndoneOnlyWhole(id)) if id == split
            ));
        }
    }

    /// A later change of either half undoes the split whole with it.
    #[test]
    fn is_undone_whole_after_a_change_of_either_half() {
        for edited_half in ["2026-10-05", "2026-10-26"] {
            let harness = Harness::new();
            let core = harness.open_without_starting_calendars();
            let home = calendar(&core, "Home");
            yoga(&core, home.id, RepeatEnd::Count { count: 4 });
            let before = contents(&core);
            let third = on(&core, "2026-10-19");
            let split = core
                .reschedule_occurrence(
                    &third.combined_event_key,
                    third.recurrence_id.as_deref().unwrap(),
                    Scope::ThisAndFollowing,
                    false,
                    timed("2026-10-19T19:00:00", "2026-10-19T20:00:00"),
                )
                .unwrap()
                .journal_transaction
                .unwrap();
            let half = the_event(&core, edited_half);
            let renamed = core
                .edit_event(
                    &half.combined_event_key,
                    freecal_core::EventDraft {
                        repeat: half.repeat,
                        ..draft(half.calendar_id, "Pilates", half.when)
                    },
                )
                .unwrap()
                .journal_transaction
                .unwrap();

            let danger = danger(&core, whole(split));
            core.undo(whole(split), Some(&danger)).unwrap();

            assert_eq!(
                danger
                    .later_journal_transactions
                    .iter()
                    .map(|later| later.id)
                    .collect::<Vec<_>>(),
                [renamed],
                "{edited_half}"
            );
            assert_eq!(contents(&core), before, "{edited_half}");
        }
    }

    /// One half skipped skips the other: the split is undone only whole.
    #[test]
    fn is_skipped_whole_when_one_half_is_skipped() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let (split, first, following) = split(&core);
        let work = calendar(&core, "Work");
        core.edit_event(
            &following.combined_event_key,
            freecal_core::EventDraft {
                repeat: following.repeat,
                ..draft(work.id, "Yoga", following.when)
            },
        )
        .unwrap();
        core.set_calendar_read_only(work.id, true).unwrap();
        let before = contents(&core);

        let ran = core.undo(whole(split), None).unwrap();

        assert_eq!(ran.journal_transaction, None);
        assert_eq!(
            ran.value,
            undone(vec![
                SkippedEvent {
                    title: first.title,
                    reason: SkipReason::ChangedTogetherWith {
                        title: "Yoga".to_owned()
                    },
                },
                SkippedEvent {
                    title: "Yoga".to_owned(),
                    reason: SkipReason::ReadOnlyCalendar {
                        calendar_id: work.id
                    },
                },
            ])
        );
        assert_eq!(contents(&core), before);
    }

    /// Undoing the Recurring Event's creation after the split undoes the
    /// split whole first, so the following half goes too.
    #[test]
    fn goes_with_an_earlier_change_it_followed() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let created = core
            .create_event(freecal_core::EventDraft {
                repeat: Some(support::weekly(RepeatEnd::Count { count: 4 })),
                ..draft(
                    home.id,
                    "Yoga",
                    timed("2026-10-05T18:00:00", "2026-10-05T19:00:00"),
                )
            })
            .unwrap()
            .journal_transaction
            .unwrap();
        let third = on(&core, "2026-10-19");
        let split = core
            .reschedule_occurrence(
                &third.combined_event_key,
                third.recurrence_id.as_deref().unwrap(),
                Scope::ThisAndFollowing,
                false,
                timed("2026-10-19T19:00:00", "2026-10-19T20:00:00"),
            )
            .unwrap()
            .journal_transaction
            .unwrap();

        let danger = danger(&core, whole(created));
        core.undo(whole(created), Some(&danger)).unwrap();

        assert_eq!(danger.later_journal_transactions[0].id, split);
        assert_eq!(danger.events, 2);
        assert_eq!(contents(&core), []);
    }
}

/// An import's Journal Transaction is undone one Event at a time, a
/// Recurring Event with its Exceptions together, or whole.
mod one_event_of_an_import {
    use super::*;

    const RECURRING: &str = "BEGIN:VEVENT\nUID:yoga\nSUMMARY:Yoga\n\
        DTSTART:20261012T160000Z\nDTEND:20261012T170000Z\nRRULE:FREQ=WEEKLY;COUNT=3\nEND:VEVENT\n\
        BEGIN:VEVENT\nUID:yoga\nSUMMARY:Yoga late\nRECURRENCE-ID:20261019T160000Z\n\
        DTSTART:20261019T180000Z\nDTEND:20261019T190000Z\nEND:VEVENT\n";

    #[test]
    fn a_recurring_event_with_its_exceptions_then_the_rest() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let body = [
            event("standup", MONDAY.0, MONDAY.1),
            RECURRING.to_owned(),
            event("review", WEDNESDAY.0, WEDNESDAY.1),
        ]
        .concat();
        let imported = import(&core, home.id, &body);
        assert_eq!(contents(&core).len(), 5);
        let recurring = the_event(&core, "2026-10-12");
        harness.signals.clear();

        let ran = core.undo(one(imported, &recurring), None).unwrap();

        assert!(ran.journal_transaction.is_some());
        assert_eq!(ran.value, undone(vec![]));
        let titles: Vec<_> = contents(&core)
            .into_iter()
            .map(|event| event.title)
            .collect();
        assert_eq!(titles, ["standup", "review"]);
        assert_eq!(
            harness.signals.received(),
            [Signal::OccurrencesChanged {
                calendar_ids: vec![home.id]
            }]
        );
        assert!(matches!(
            core.undo(one(imported, &recurring), None),
            Err(CoreError::AlreadyUndone(id)) if id == imported
        ));

        core.undo(whole(imported), None).unwrap();

        assert_eq!(contents(&core), []);
        assert!(matches!(
            core.undo(whole(imported), None),
            Err(CoreError::AlreadyUndone(id)) if id == imported
        ));
    }

    #[test]
    fn of_an_exception_takes_its_recurring_event_along() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let imported = import(&core, home.id, RECURRING);
        let exception = the_event(&core, "2026-10-19");
        assert_eq!(exception.title, "Yoga late");

        core.undo(one(imported, &exception), None).unwrap();

        assert_eq!(contents(&core), []);
    }

    #[test]
    fn not_in_it_is_refused() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let imported = import(&core, home.id, &event("standup", MONDAY.0, MONDAY.1));
        let other = core
            .create_event(draft(
                home.id,
                "Other",
                timed("2026-10-06T09:00:00", "2026-10-06T10:00:00"),
            ))
            .unwrap()
            .value;

        assert!(matches!(
            core.undo(one(imported, &other), None),
            Err(CoreError::EventNotFound)
        ));
    }
}

mod skipped {
    use super::*;

    /// Ticket 58 refused the whole undo here.
    #[test]
    fn events_in_a_read_only_calendar_naming_it() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let imported = import(&core, home.id, &event("new", MONDAY.0, MONDAY.1));
        let before = contents(&core);
        core.set_calendar_read_only(home.id, true).unwrap();
        harness.signals.clear();

        let ran = core.undo(whole(imported), None).unwrap();

        assert_eq!(ran.journal_transaction, None);
        assert_eq!(
            ran.value,
            undone(vec![SkippedEvent {
                title: "new".to_owned(),
                reason: SkipReason::ReadOnlyCalendar {
                    calendar_id: home.id
                },
            }])
        );
        assert_eq!(contents(&core), before);
        assert_eq!(harness.signals.received(), []);
    }

    /// The rest is undone; the skipped Event brings in none of its later
    /// changes.
    #[test]
    fn an_event_moved_into_a_read_only_calendar_and_not_its_later_changes() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let work = calendar(&core, "Work");
        let body = [
            event("moved", MONDAY.0, MONDAY.1),
            event("stays", TUESDAY.0, TUESDAY.1),
        ]
        .concat();
        let imported = import(&core, home.id, &body);
        let moved = the_event(&core, "2026-10-05");
        core.edit_event(
            &moved.combined_event_key,
            draft(work.id, "moved", moved.when),
        )
        .unwrap();
        core.set_calendar_read_only(work.id, true).unwrap();

        let ran = core.undo(whole(imported), None).unwrap();

        assert_eq!(
            ran.value,
            undone(vec![SkippedEvent {
                title: "moved".to_owned(),
                reason: SkipReason::ReadOnlyCalendar {
                    calendar_id: work.id
                },
            }])
        );
        let titles: Vec<_> = contents(&core)
            .into_iter()
            .map(|event| event.title)
            .collect();
        assert_eq!(titles, ["moved"]);
    }

    /// Only a program other than FreeCal can change an Event so.
    #[test]
    fn an_event_whose_current_version_is_not_in_the_change_journal() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let body = [
            event("changed", MONDAY.0, MONDAY.1),
            event("other", TUESDAY.0, TUESDAY.1),
        ]
        .concat();
        let imported = import(&core, home.id, &body);
        harness.arrange_local_store(
            "UPDATE event SET title = 'Changed outside' WHERE uid = 'changed'",
        );

        let ran = core.undo(whole(imported), None).unwrap();

        assert_eq!(
            ran.value,
            undone(vec![SkippedEvent {
                title: "Changed outside".to_owned(),
                reason: SkipReason::NotInChangeJournal,
            }])
        );
        let titles: Vec<_> = contents(&core)
            .into_iter()
            .map(|event| event.title)
            .collect();
        assert_eq!(titles, ["Changed outside"]);
    }
}

/// More than 10 Events and more than 5% of a Calendar is past the
/// Mass-Change Guard's thresholds.
mod past_the_mass_change_guard {
    use super::*;

    fn events(count: usize) -> String {
        (0..count)
            .map(|day| {
                let day = 1 + day;
                event(
                    &format!("e{day}"),
                    &format!("DTSTART:202610{day:02}T070000Z"),
                    &format!("DTEND:202610{day:02}T080000Z"),
                )
            })
            .collect()
    }

    #[test]
    fn eleven_events_of_a_calendar_of_eleven_is_dangerous() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let imported = import(&core, home.id, &events(11));

        let danger = danger(&core, whole(imported));

        assert_eq!(
            danger,
            DangerousUndo {
                later_journal_transactions: vec![],
                server_changes: 0,
                exceeds_mass_change_guard: true,
                events: 11,
            }
        );
    }

    #[test]
    fn ten_events_is_benign() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let imported = import(&core, home.id, &events(10));

        let ran = core.undo(whole(imported), None).unwrap();

        assert_eq!(ran.value, undone(vec![]));
        assert_eq!(contents(&core), []);
    }
}

/// Journal Transactions are kept for 90 days, whether or not FreeCal is
/// restarted (ADR 0009).
mod kept_for_90_days {
    use super::*;

    fn created(core: &Core, home: &Calendar, title: &str) -> JournalTransactionId {
        core.create_event(draft(
            home.id,
            title,
            timed("2026-10-05T09:00:00", "2026-10-05T10:00:00"),
        ))
        .unwrap()
        .journal_transaction
        .unwrap()
    }

    #[test]
    fn and_removed_with_the_next_change_after() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let old = created(&core, &home, "Old");
        let kept = created(&core, &home, "Kept");

        harness.clock.set("2027-01-01T10:59:59+01:00");
        created(&core, &home, "Within");
        core.undo(whole(kept), None).unwrap();
        harness.clock.set("2027-01-01T11:00:01+01:00");
        created(&core, &home, "After");

        assert!(matches!(
            core.undo(whole(old), None),
            Err(CoreError::JournalTransactionNotFound(id)) if id == old
        ));
    }

    #[test]
    fn and_removed_when_the_local_store_opens_after() {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let home = calendar(&core, "Home");
        let old = created(&core, &home, "Old");
        drop(core);

        harness.clock.set("2027-01-02T12:00:00+01:00");
        let core = harness.open();

        assert!(matches!(
            core.undo(whole(old), None),
            Err(CoreError::JournalTransactionNotFound(id)) if id == old
        ));
    }
}
