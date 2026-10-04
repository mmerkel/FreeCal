mod support;

use chrono::{NaiveDate, NaiveDateTime};
use freecal_core::{
    Calendar, CalendarId, Colour, Core, CoreError, EventDraft, EventId, Piece, Signal, When,
    link_allowed,
};
use support::Harness;

fn calendar(core: &Core, name: &str) -> Calendar {
    let local = core.list_accounts().unwrap()[0].id;
    core.create_calendar(local, name, Colour::parse("#3366cc").unwrap())
        .unwrap()
}

fn date(text: &str) -> NaiveDate {
    text.parse().unwrap()
}

fn time(text: &str) -> NaiveDateTime {
    text.parse().unwrap()
}

fn timed(start: &str, end: &str) -> When {
    When::Timed {
        start: time(start),
        end: time(end),
    }
}

fn all_day(start: &str, end: &str) -> When {
    When::AllDay {
        start: date(start),
        end: date(end),
    }
}

fn draft(calendar_id: CalendarId, title: &str, when: When) -> EventDraft {
    EventDraft {
        calendar_id,
        title: title.to_owned(),
        when,
        location: String::new(),
        description: String::new(),
    }
}

/// The titles of the Occurrences on the dates `from` up to `to` (exclusive).
fn titles(core: &Core, from: &str, to: &str) -> Vec<String> {
    core.occurrences(date(from), date(to))
        .unwrap()
        .into_iter()
        .map(|occurrence| occurrence.title)
        .collect()
}

fn occurrences_changed(ids: &[CalendarId]) -> Signal {
    Signal::OccurrencesChanged {
        calendar_ids: ids.to_vec(),
    }
}

#[test]
fn a_created_event_can_be_read_back_and_is_signalled() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let signals_before = harness.signals.received().len();

    let created = core
        .create_event(EventDraft {
            calendar_id: home.id,
            title: "Dentist".to_owned(),
            when: timed("2026-10-05T09:00:00", "2026-10-05T10:30:00"),
            location: "Main Street 1".to_owned(),
            description: "Bring the card".to_owned(),
        })
        .unwrap();

    assert_eq!(core.event(created.id).unwrap(), created);
    assert_eq!(created.calendar_id, home.id);
    assert_eq!(created.title, "Dentist");
    assert_eq!(
        created.when,
        timed("2026-10-05T09:00:00", "2026-10-05T10:30:00")
    );
    assert_eq!(created.location, "Main Street 1");
    assert_eq!(created.description.text, "Bring the card");
    assert_eq!(
        harness.signals.received()[signals_before..],
        [occurrences_changed(&[home.id])]
    );
}

#[test]
fn occurrences_are_the_events_overlapping_the_range_in_start_order() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    for (title, when) in [
        (
            "Before",
            timed("2026-10-04T22:00:00", "2026-10-05T00:00:00"),
        ),
        ("Late", timed("2026-10-05T18:00:00", "2026-10-05T19:00:00")),
        (
            "Overnight",
            timed("2026-10-04T23:00:00", "2026-10-05T01:00:00"),
        ),
        ("Holiday", all_day("2026-10-05", "2026-10-06")),
        ("Trip", all_day("2026-10-01", "2026-10-08")),
        (
            "Reminder",
            timed("2026-10-06T08:00:00", "2026-10-06T08:00:00"),
        ),
        ("After", all_day("2026-10-07", "2026-10-08")),
        ("Early", timed("2026-10-05T07:00:00", "2026-10-05T08:00:00")),
    ] {
        core.create_event(draft(home.id, title, when)).unwrap();
    }

    assert_eq!(
        titles(&core, "2026-10-05", "2026-10-07"),
        ["Trip", "Overnight", "Holiday", "Early", "Late", "Reminder"]
    );
}

#[test]
fn hidden_calendars_have_no_occurrences() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let work = calendar(&core, "Work");
    let day = all_day("2026-10-05", "2026-10-06");
    core.create_event(draft(home.id, "Home event", day))
        .unwrap();
    core.create_event(draft(work.id, "Work event", day))
        .unwrap();

    core.set_calendar_shown(work.id, false).unwrap();

    assert_eq!(titles(&core, "2026-10-05", "2026-10-06"), ["Home event"]);
}

#[test]
fn occurrences_are_in_the_display_time_zone() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    // Made at 09:00 in Berlin (UTC+2 in October).
    core.create_event(draft(
        home.id,
        "Call",
        timed("2026-10-05T09:00:00", "2026-10-05T10:00:00"),
    ))
    .unwrap();

    harness.clock.set_time_zone("America/New_York");

    let occurrences = core
        .occurrences(date("2026-10-05"), date("2026-10-06"))
        .unwrap();
    assert_eq!(
        occurrences[0].when,
        timed("2026-10-05T03:00:00", "2026-10-05T04:00:00")
    );
}

#[test]
fn the_range_is_taken_in_the_display_time_zone() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    // 23:30 in Berlin is still 5 October there, though 21:30 UTC.
    core.create_event(draft(
        home.id,
        "Late",
        timed("2026-10-05T23:30:00", "2026-10-05T23:45:00"),
    ))
    .unwrap();

    assert_eq!(titles(&core, "2026-10-05", "2026-10-06"), ["Late"]);
    assert!(titles(&core, "2026-10-06", "2026-10-07").is_empty());
}

#[test]
fn a_time_the_clocks_skip_is_read_as_an_hour_later() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");

    // On 29 March 2026, Berlin's clocks jump from 02:00 to 03:00.
    let created = core
        .create_event(draft(
            home.id,
            "Gap",
            timed("2026-03-29T02:30:00", "2026-03-29T04:00:00"),
        ))
        .unwrap();

    assert_eq!(
        created.when,
        timed("2026-03-29T03:30:00", "2026-03-29T04:00:00")
    );
}

#[test]
fn editing_changes_every_field_and_survives_a_relaunch() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let created = core
        .create_event(draft(
            home.id,
            "Lunch",
            timed("2026-10-05T12:00:00", "2026-10-05T13:00:00"),
        ))
        .unwrap();
    let signals_before = harness.signals.received().len();

    let edited = core
        .edit_event(
            created.id,
            EventDraft {
                calendar_id: home.id,
                title: "Long lunch".to_owned(),
                when: all_day("2026-10-06", "2026-10-08"),
                location: "Café".to_owned(),
                description: "With Sam".to_owned(),
            },
        )
        .unwrap();
    drop(core);
    let relaunch = harness.open();

    assert_eq!(relaunch.event(created.id).unwrap(), edited);
    assert_eq!(edited.title, "Long lunch");
    assert_eq!(edited.when, all_day("2026-10-06", "2026-10-08"));
    assert_eq!(edited.location, "Café");
    assert_eq!(edited.description.text, "With Sam");
    assert_eq!(
        harness.signals.received()[signals_before..],
        [occurrences_changed(&[home.id])]
    );
}

#[test]
fn moving_an_event_to_another_calendar_signals_both() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let work = calendar(&core, "Work");
    let day = all_day("2026-10-05", "2026-10-06");
    let created = core.create_event(draft(home.id, "Review", day)).unwrap();
    let signals_before = harness.signals.received().len();

    let moved = core
        .edit_event(created.id, draft(work.id, "Review", day))
        .unwrap();

    assert_eq!(moved.calendar_id, work.id);
    assert_eq!(
        harness.signals.received()[signals_before..],
        [occurrences_changed(&[home.id, work.id])]
    );
}

#[test]
fn deleting_an_event_removes_it_and_is_signalled() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let day = all_day("2026-10-05", "2026-10-06");
    let doomed = core.create_event(draft(home.id, "Doomed", day)).unwrap();
    core.create_event(draft(home.id, "Kept", day)).unwrap();
    let signals_before = harness.signals.received().len();

    core.delete_event(doomed.id).unwrap();

    assert_eq!(titles(&core, "2026-10-05", "2026-10-06"), ["Kept"]);
    assert!(matches!(
        core.event(doomed.id),
        Err(CoreError::EventNotFound(id)) if id == doomed.id
    ));
    assert_eq!(
        harness.signals.received()[signals_before..],
        [occurrences_changed(&[home.id])]
    );
}

#[test]
fn deleting_a_calendar_deletes_its_events() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let created = core
        .create_event(draft(home.id, "Gone", all_day("2026-10-05", "2026-10-06")))
        .unwrap();

    core.delete_calendar(home.id).unwrap();

    assert!(matches!(
        core.event(created.id),
        Err(CoreError::EventNotFound(_))
    ));
}

#[test]
fn an_event_cannot_end_before_it_starts() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let signals_before = harness.signals.received().len();

    for when in [
        timed("2026-10-05T10:00:00", "2026-10-05T09:59:00"),
        all_day("2026-10-05", "2026-10-05"),
    ] {
        assert!(matches!(
            core.create_event(draft(home.id, "Backwards", when)),
            Err(CoreError::EndBeforeStart)
        ));
    }
    assert!(titles(&core, "2026-10-01", "2026-10-10").is_empty());
    assert_eq!(harness.signals.received().len(), signals_before);
}

#[test]
fn events_of_read_only_calendars_cannot_be_changed() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let work = calendar(&core, "Work");
    let day = all_day("2026-10-05", "2026-10-06");
    let in_home = core.create_event(draft(home.id, "Mine", day)).unwrap();
    let in_work = core.create_event(draft(work.id, "Theirs", day)).unwrap();
    harness.arrange_local_store(&format!(
        "UPDATE calendar SET read_only = 1 WHERE id = {}",
        work.id.0
    ));
    let signals_before = harness.signals.received().len();

    assert!(core.list_calendars().unwrap()[1].read_only);
    let refused = [
        core.create_event(draft(work.id, "New", day)).map(|_| ()),
        core.edit_event(in_work.id, draft(work.id, "Changed", day))
            .map(|_| ()),
        core.edit_event(in_home.id, draft(work.id, "Mine", day))
            .map(|_| ()),
        core.edit_event(in_work.id, draft(home.id, "Theirs", day))
            .map(|_| ()),
        core.delete_event(in_work.id),
    ];

    for result in refused {
        assert!(matches!(result, Err(CoreError::CalendarReadOnly(id)) if id == work.id));
    }
    assert_eq!(
        titles(&core, "2026-10-05", "2026-10-06"),
        ["Mine", "Theirs"]
    );
    assert_eq!(harness.signals.received().len(), signals_before);
}

#[test]
fn changing_a_missing_event_or_calendar_is_refused_without_a_signal() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let day = all_day("2026-10-05", "2026-10-06");
    let created = core.create_event(draft(home.id, "Here", day)).unwrap();
    let signals_before = harness.signals.received().len();
    let missing = EventId(999);

    assert!(matches!(
        core.create_event(draft(CalendarId(999), "Lost", day)),
        Err(CoreError::CalendarNotFound(_))
    ));
    assert!(matches!(
        core.edit_event(created.id, draft(CalendarId(999), "Lost", day)),
        Err(CoreError::CalendarNotFound(_))
    ));
    assert!(matches!(
        core.edit_event(missing, draft(home.id, "Lost", day)),
        Err(CoreError::EventNotFound(_))
    ));
    assert!(matches!(
        core.delete_event(missing),
        Err(CoreError::EventNotFound(_))
    ));
    assert_eq!(harness.signals.received().len(), signals_before);
}

#[test]
fn an_untouched_time_keeps_the_time_zone_it_was_made_in() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let created = core
        .create_event(draft(
            home.id,
            "Call",
            timed("2026-10-05T09:00:00", "2026-10-05T10:00:00"),
        ))
        .unwrap();
    harness.clock.set_time_zone("America/New_York");
    let shown_in_new_york = core.event(created.id).unwrap().when;

    core.edit_event(created.id, draft(home.id, "Call Berlin", shown_in_new_york))
        .unwrap();
    harness.clock.set_time_zone("Europe/Berlin");

    assert_eq!(
        core.event(created.id).unwrap().when,
        timed("2026-10-05T09:00:00", "2026-10-05T10:00:00")
    );
}

/// Arranges an Event whose description came in as HTML, as from Google or an
/// import, which the interface can't create yet.
fn arrange_html_description(harness: &Harness, id: EventId, html: &str) {
    harness.arrange_local_store(&format!(
        "UPDATE event SET description = '{}', description_html = 1 WHERE id = {}",
        html.replace('\'', "''"),
        id.0
    ));
}

#[test]
fn an_untouched_description_keeps_its_original_bytes() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let day = all_day("2026-10-05", "2026-10-06");
    let created = core.create_event(draft(home.id, "Talk", day)).unwrap();
    arrange_html_description(
        &harness,
        created.id,
        "<b>Slides</b>: <a href=\"https://example.com/s\">here</a>",
    );
    let before = core.event(created.id).unwrap().description;

    let mut unchanged = draft(home.id, "Talk, renamed", day);
    unchanged.description = before.text.clone();
    core.edit_event(created.id, unchanged).unwrap();

    let after = core.event(created.id).unwrap().description;
    assert_eq!(after, before);
    assert_eq!(
        after.paragraphs,
        vec![vec![
            Piece::Text {
                text: "Slides: ".to_owned()
            },
            Piece::Link {
                text: "here".to_owned(),
                url: "https://example.com/s".to_owned()
            },
        ]]
    );
}

#[test]
fn an_edited_description_is_written_back_as_plain_text() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let day = all_day("2026-10-05", "2026-10-06");
    let created = core.create_event(draft(home.id, "Talk", day)).unwrap();
    arrange_html_description(&harness, created.id, "<b>Slides</b>");

    let mut edited = draft(home.id, "Talk", day);
    edited.description = "Slides <b>soon</b>".to_owned();
    core.edit_event(created.id, edited).unwrap();

    assert_eq!(
        core.event(created.id).unwrap().description.paragraphs,
        vec![vec![Piece::Text {
            text: "Slides <b>soon</b>".to_owned()
        }]]
    );
}

#[test]
fn an_unreadable_stored_time_is_an_error_not_a_crash() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let created = core
        .create_event(draft(
            home.id,
            "Broken",
            timed("2026-10-05T09:00:00", "2026-10-05T10:00:00"),
        ))
        .unwrap();
    harness.arrange_local_store(&format!(
        "UPDATE event SET end_at = '2026-10-05T99:00' WHERE id = {}",
        created.id.0
    ));

    assert!(matches!(
        core.event(created.id),
        Err(CoreError::UnreadableEventTime(_))
    ));
    assert!(matches!(
        core.occurrences(date("2026-10-05"), date("2026-10-06")),
        Err(CoreError::UnreadableEventTime(_))
    ));
}

mod hostile {
    use super::*;

    fn fixture() -> serde_json::Value {
        serde_json::from_str(include_str!("../../fixtures/hostile-event.json")).unwrap()
    }

    fn field(name: &str) -> String {
        fixture()[name].as_str().unwrap().to_owned()
    }

    fn create(core: &Core) -> EventId {
        let home = calendar(core, "Home");
        core.create_event(EventDraft {
            calendar_id: home.id,
            title: field("title"),
            when: timed("2026-10-05T09:00:00", "2026-10-05T10:00:00"),
            location: field("location"),
            description: field("description"),
        })
        .unwrap()
        .id
    }

    fn links(paragraphs: &[Vec<Piece>]) -> Vec<String> {
        paragraphs
            .iter()
            .flatten()
            .filter_map(|piece| match piece {
                Piece::Link { url, .. } => Some(url.clone()),
                Piece::Text { .. } => None,
            })
            .collect()
    }

    fn assert_only_allowed_links(paragraphs: &[Vec<Piece>]) {
        for url in links(paragraphs) {
            assert!(link_allowed(&url), "{url:?} must not be a link");
        }
    }

    #[test]
    fn its_fields_are_stored_and_returned_as_plain_text() {
        let harness = Harness::new();
        let core = harness.open();

        let id = create(&core);

        let event = core.event(id).unwrap();
        assert_eq!(event.title, field("title"));
        assert_eq!(event.location, field("location"));
        assert_eq!(
            core.occurrences(date("2026-10-05"), date("2026-10-06"))
                .unwrap()[0]
                .title,
            field("title")
        );
        assert_eq!(
            serde_json::to_value(&event.description).unwrap(),
            fixture()["expected"]["plainText"]
        );
        assert_only_allowed_links(&event.description.paragraphs);
    }

    #[test]
    fn its_description_read_as_html_has_only_allowed_links() {
        let harness = Harness::new();
        let core = harness.open();
        let id = create(&core);

        arrange_html_description(&harness, id, &field("description"));

        let description = core.event(id).unwrap().description;
        assert_eq!(
            serde_json::to_value(&description).unwrap(),
            fixture()["expected"]["html"]
        );
        assert_only_allowed_links(&description.paragraphs);
        assert!(links(&description.paragraphs).contains(&"https://example.com/agenda".to_owned()));
    }
}
