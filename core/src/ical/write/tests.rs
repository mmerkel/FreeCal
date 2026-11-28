use chrono::{DateTime, Utc};

use std::collections::HashSet;

use super::*;
use crate::event::{RecurrenceId, StoredWhen};
use crate::guests;
use crate::ical::read::{ReadFile, read_events};
use crate::ical::server_texts::*;
use crate::import::ImportedEvent;
use crate::unimplemented::tests::{parameter, property};
use crate::unimplemented::{Entry, UnimplementedFields};

const STAMP: &str = "DTSTAMP:20261108T120000Z\r\n";

fn now() -> DateTime<Utc> {
    "2026-11-28T12:00:00Z".parse().unwrap()
}

/// A text with one `VEVENT` of UID `a`, starting on 5 October 2026 at 7:00
/// UTC, with `lines` after its start.
fn vevent(lines: &str) -> String {
    format!(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Test//EN\r\nBEGIN:VEVENT\r\nUID:a\r\n{STAMP}\
         DTSTART:20261005T070000Z\r\n{lines}END:VEVENT\r\nEND:VCALENDAR\r\n"
    )
}

fn read(text: &str) -> ReadFile {
    read_events("test.ics", text.as_bytes(), Tz::UTC).unwrap()
}

/// Every Event `text` holds, Exceptions after the others.
fn events_of(text: &str) -> Vec<ImportedEvent> {
    let read = read(text);
    read.events.into_iter().chain(read.exceptions).collect()
}

/// An Event read, as managed fields.
fn fields_of(event: &ImportedEvent) -> EventFields {
    EventFields {
        title: event.title.clone(),
        when: StoredWhen::imported(&event.when),
        location: event.location.clone(),
        description: event.description.clone(),
        description_html: event.description_html,
        zone_pinned: event.zone_pinned,
        rule: event.rule.clone(),
        exdates: event.exdates.clone(),
        recurrence_id: event.recurrence_id.clone(),
    }
}

/// The resource of `uid` with `events`, each as fields and tree, of which
/// nothing is left out.
fn write(uid: &str, events: &[(EventFields, UnimplementedFields)]) -> String {
    let pairs: Vec<(&EventFields, &UnimplementedFields)> =
        events.iter().map(|(fields, tree)| (fields, tree)).collect();
    let written = write_resource(uid, &pairs, now()).unwrap();
    assert_eq!(written.left_out, [], "{}", written.text);
    written.text
}

/// The resource of the Events read from `text`, all of one UID.
fn rewritten(text: &str) -> String {
    let events = events_of(text);
    let pairs: Vec<_> = events
        .iter()
        .map(|event| (fields_of(event), event.unimplemented.clone()))
        .collect();
    write(&events[0].uid, &pairs)
}

/// What the reader reads from an Event of a text, compared: the managed
/// fields the text holds (the recurrence ID by Occurrence, ADR 0012) and
/// the tree's canonical form.
type Compared = (
    String,
    String,
    StoredWhen,
    [String; 3],
    bool,
    Option<String>,
    String,
    Option<String>,
    String,
);

fn compared(events: &[(EventFields, UnimplementedFields)], uid: &str) -> Vec<Compared> {
    let recurring = events
        .iter()
        .find(|(fields, _)| fields.recurrence_id.is_none())
        .map(|(fields, _)| fields.when.to_imported().unwrap());
    let mut compared: Vec<Compared> = events
        .iter()
        .map(|(fields, tree)| {
            let id = fields.recurrence_id.as_ref();
            (
                id.map(|id| id.normal_form(None).into_string())
                    .unwrap_or_default(),
                uid.to_owned(),
                fields.when.clone(),
                [
                    fields.title.clone(),
                    fields.location.clone(),
                    fields.description.clone(),
                ],
                fields.description_html,
                fields.rule.clone(),
                fields.exdates.clone(),
                id.map(|id| id.normal_form(recurring.as_ref()).into_string()),
                tree.canonical(),
            )
        })
        .collect();
    // Stable, as the writer's order.
    compared.sort_by(|a, b| a.0.cmp(&b.0));
    compared
}

/// That `events` of the UID `uid` are written as well-formed text that
/// reads back to the same managed fields and equal trees; the text.
fn assert_round_trip(uid: &str, events: &[(EventFields, UnimplementedFields)]) -> String {
    let text = write(uid, events);
    assert_well_formed(&text);
    let read = events_of(&text);
    for event in &read {
        assert_eq!(event.uid, uid, "{text}");
    }
    let read: Vec<_> = read
        .iter()
        .map(|event| (fields_of(event), event.unimplemented.clone()))
        .collect();
    assert_eq!(compared(&read, uid), compared(events, uid), "{text}");
    text
}

/// That the Events of `text`, of one UID, read, written and read again,
/// give equal fields and trees.
fn assert_text_round_trip(text: &str) -> String {
    let events = events_of(text);
    let pairs: Vec<_> = events
        .iter()
        .map(|event| (fields_of(event), event.unimplemented.clone()))
        .collect();
    assert_round_trip(&events[0].uid, &pairs)
}

/// The content lines of `text`, unfolded.
fn content_lines(text: &str) -> Vec<String> {
    icalendar::parser::unfold(text)
        .split("\r\n")
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// That `text` is iCalendar as FreeCal writes it: CRLF line ends, lines of
/// at most 75 octets with no control characters, one `VCALENDAR` whose
/// components nest.
fn assert_well_formed(text: &str) {
    assert!(text.ends_with("\r\n"));
    let body = text.strip_suffix("\r\n").unwrap();
    for line in body.split("\r\n") {
        assert!(line.len() <= 75, "{line:?}");
        assert!(
            !line.chars().any(|c| c.is_control() && c != '\t'),
            "{line:?}"
        );
    }
    assert!(!body.replace("\r\n", "").contains(['\r', '\n']));
    let mut open: Vec<String> = Vec::new();
    let mut tops = 0;
    for line in content_lines(text) {
        if let Some(kind) = line.strip_prefix("BEGIN:") {
            if open.is_empty() {
                tops += 1;
            }
            open.push(kind.to_owned());
        } else if let Some(kind) = line.strip_prefix("END:") {
            assert_eq!(open.pop().as_deref(), Some(kind), "{text}");
        } else {
            assert!(!open.is_empty(), "{line:?} outside every component");
        }
    }
    assert!(open.is_empty() && tops == 1, "{text}");
}

fn entry(name: &str, value: EntryValue) -> Entry {
    Entry {
        name: name.to_owned(),
        value,
    }
}

fn whole(name: &str, parameters: Vec<Parameter>, value: &str) -> Entry {
    let value = value.to_owned();
    entry(name, EntryValue::Property { parameters, value })
}

fn component(name: &str, properties: Vec<Property>, components: Vec<Component>) -> Component {
    Component {
        name: name.to_owned(),
        properties,
        components,
    }
}

/// A timed Event in Berlin on 12 October 2026, 9:00 to 10:00.
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

fn exception(recurrence_id: &str, title: &str) -> EventFields {
    let wall = recurrence_id.parse().unwrap();
    EventFields {
        title: title.to_owned(),
        recurrence_id: Some(RecurrenceId::of_original(Time::Wall(
            wall,
            Some(Tz::Europe__Berlin),
        ))),
        ..fields()
    }
}

#[test]
fn a_single_event_is_one_vevent_in_one_vcalendar() {
    let text = assert_round_trip("a", &[(fields(), UnimplementedFields::default())]);

    let lines = content_lines(&text);
    assert_eq!(
        lines[..3],
        [
            "BEGIN:VCALENDAR",
            "VERSION:2.0",
            "PRODID:-//FreeCal//FreeCal//EN"
        ]
    );
    let vevent = lines
        .iter()
        .position(|line| line == "BEGIN:VEVENT")
        .unwrap();
    assert_eq!(
        lines[vevent..],
        [
            "BEGIN:VEVENT",
            "UID:a",
            "DTSTAMP:20261128T120000Z",
            "LAST-MODIFIED:20261128T120000Z",
            "DTSTART;TZID=Europe/Berlin:20261012T090000",
            "DTEND;TZID=Europe/Berlin:20261012T100000",
            "SUMMARY:Yoga",
            "LOCATION:Room 1",
            "DESCRIPTION:Bring a mat",
            "END:VEVENT",
            "END:VCALENDAR",
        ]
    );
}

#[test]
fn a_recurring_event_comes_first_then_its_exceptions_by_occurrence() {
    let recurring = EventFields {
        rule: Some("FREQ=DAILY;COUNT=5".to_owned()),
        exdates: "2026-10-13T09:00:00".to_owned(),
        ..fields()
    };
    let later = exception("2026-10-15T09:00:00", "Later");
    let earlier = exception("2026-10-14T09:00:00", "Earlier");
    let tree = UnimplementedFields::default;
    let events = [(later, tree()), (recurring, tree()), (earlier, tree())];

    let text = assert_round_trip("a", &events);

    let lines = content_lines(&text);
    let titles: Vec<&str> = lines
        .iter()
        .filter_map(|line| line.strip_prefix("SUMMARY:"))
        .collect();
    assert_eq!(titles, ["Yoga", "Earlier", "Later"]);
    assert!(text.contains("RRULE:FREQ=DAILY;COUNT=5\r\n"));
    assert!(text.contains("EXDATE;TZID=Europe/Berlin:20261013T090000\r\n"));
    assert!(text.contains("RECURRENCE-ID;TZID=Europe/Berlin:20261014T090000\r\n"));
}

#[test]
fn exceptions_without_their_recurring_event_are_written_alone() {
    let events = [
        (
            exception("2026-10-15T09:00:00", "Later"),
            UnimplementedFields::default(),
        ),
        (
            exception("2026-10-14T09:00:00", "Earlier"),
            UnimplementedFields::default(),
        ),
    ];

    let text = assert_round_trip("a", &events);

    assert!(!text.contains("RRULE"));
    assert_eq!(text.matches("BEGIN:VEVENT").count(), 2);
}

/// Each kind of property value, parameter and component that the reader's
/// tests cover, in an Event of their own.
const KINDS: &[&str] = &[
    "CLASS:PRIVATE\r\nSTATUS:TENTATIVE\r\nGEO:37.386013;-122.082932\r\nPRIORITY:5\r\n\
     URL:https://example.org/a?b=c;d\r\nCREATED:20260101T000000Z\r\n\
     RDATE:20261010T070000Z,20261011T070000Z\r\nEXRULE:FREQ=WEEKLY;COUNT=2\r\n\
     X-MICROSOFT-CDO-BUSYSTATUS:BUSY\r\nCOMMENT:a\\, b\\; c\\nd\r\nCATEGORIES:one,two\r\n\
     ATTACH;ENCODING=BASE64;VALUE=BINARY:SGVsbG8=\r\nX-FLAG;VALUE=BOOLEAN:TRUE\r\n\
     RDATE;VALUE=PERIOD:20261012T070000Z/PT1H\r\n",
    "ATTENDEE;CN=\"Doe, Jane\";MEMBER=\"mailto:a@example.org\",\"mailto:b@example.org\";\
     RSVP=TRUE;X-NUM-GUESTS=0:mailto:jane@example.org\r\n\
     CONTACT;ALTREP=\"cid:part1@example.org\":Jim Dolittle\r\n",
    "BEGIN:VALARM\r\nACTION:DISPLAY\r\nTRIGGER:-PT15M\r\nDESCRIPTION:Soon\r\nEND:VALARM\r\n\
     BEGIN:VALARM\r\nACTION:AUDIO\r\nTRIGGER:-PT5M\r\nEND:VALARM\r\n\
     BEGIN:X-OUTER\r\nX-A:1\r\nBEGIN:X-INNER\r\nX-B:2\r\nEND:X-INNER\r\nEND:X-OUTER\r\n\
     BEGIN:VLOCATION\r\nNAME:Room\r\nEND:VLOCATION\r\n",
    "SUMMARY;LANGUAGE=de;X-P=\"1\":Title\r\nLOCATION;LANGUAGE=de:Room\r\n\
     DESCRIPTION;X-P=a:Notes\r\nDTEND;X-P=1:20261005T080000Z\r\n\
     RRULE;X-P=1:FREQ=DAILY;COUNT=3\r\nEXDATE;X-P=1:20261006T070000Z\r\n",
    "DURATION;X-P=1:PT1H30M\r\n",
    "SUMMARY:First\r\nSUMMARY;LANGUAGE=de:Second\r\nDTSTART:20261006T070000Z\r\n\
     RRULE:FREQ=DAILY\r\nRRULE:FREQ=WEEKLY\r\n",
    "RRULE:FREQ=FORTNIGHTLY\r\nEXDATE;TZID=Europe/Berlin:20261006T090000\r\n",
    "EXDATE:20261006T070000Z\r\n",
    "RRULE:FREQ=DAILY;COUNT=3\r\nEXDATE:20261006T070000Z\r\nEXDATE:tomorrow\r\n",
    "SEQUENCE:3\r\n",
    "SUMMARY;X-P=1:\r\nX-EMPTY;X-Q:value\r\n",
    "x-lower;x-p=Mixed:a\\,b,c\r\nX-ALT-DESC;FMTTYPE=text/html:<p>a\\, b</p>\r\n",
];

#[test]
fn every_kind_of_value_parameter_and_component_reads_back_equal() {
    for lines in KINDS {
        let text = vevent(lines);
        let events = events_of(&text);
        assert!(
            !events[0].unimplemented.is_empty() || events[0].title.is_empty(),
            "{lines}"
        );
        assert_text_round_trip(&text);
    }
}

#[test]
fn a_tree_made_as_given_reads_back_equal() {
    let alarm = component(
        "VALARM",
        vec![
            property("ACTION", Vec::new(), "DISPLAY"),
            property(
                "DESCRIPTION",
                vec![parameter("LANGUAGE", &["de"])],
                "a\\,b,c",
            ),
        ],
        vec![component(
            "X-INNER",
            vec![property("X-B", Vec::new(), "2")],
            Vec::new(),
        )],
    );
    let tree = UnimplementedFields::new(vec![
        whole("ical:X-ESCAPED", Vec::new(), "a\\,b,c\\nd\\\\"),
        whole(
            "ical:ATTENDEE",
            vec![
                parameter("CN", &["Doe, Jane"]),
                parameter("MEMBER", &["mailto:a@example.org", "mailto:b@example.org"]),
                parameter("X-NONE", &[]),
                parameter("X-COLON", &["a:b"]),
            ],
            "mailto:jane@example.org",
        ),
        whole("ical:ATTENDEE", Vec::new(), "mailto:joe@example.org"),
        whole("ical:SEQUENCE", Vec::new(), "4"),
        entry(
            "ical:SUMMARY",
            EntryValue::Parameters(vec![parameter("LANGUAGE", &["de"])]),
        ),
        entry(
            "ical:DTSTART",
            EntryValue::Parameters(vec![parameter("X-P", &["1"])]),
        ),
        entry(
            "ical:UID",
            EntryValue::Parameters(vec![parameter("X-U", &["u"])]),
        ),
        entry(
            "ical:VALARM",
            EntryValue::Component {
                properties: alarm.properties.clone(),
                components: alarm.components.clone(),
            },
        ),
        entry(
            "ical:VCALENDAR",
            EntryValue::Component {
                properties: vec![
                    property("X-WR-CALNAME", Vec::new(), "Work\\, mostly"),
                    property("CALSCALE", Vec::new(), "JULIAN"),
                ],
                components: vec![component(
                    "VTODO",
                    vec![property("UID", Vec::new(), "todo")],
                    Vec::new(),
                )],
            },
        ),
    ]);
    let text = assert_round_trip("a", &[(fields(), tree)]);

    assert!(text.contains("X-ESCAPED:a\\,b,c\\nd\\\\\r\n"), "{text}");
    assert!(text.contains("SEQUENCE:4\r\n"));
    let level = text.find("X-WR-CALNAME").unwrap();
    assert!(level < text.find("BEGIN:VEVENT").unwrap());
    assert!(text.find("BEGIN:VTODO").unwrap() < text.find("BEGIN:VEVENT").unwrap());
}

#[test]
fn the_server_texts_read_write_and_read_again_equal() {
    for server in SERVER_TEXTS {
        assert_text_round_trip(server);
    }
}

#[test]
fn a_sequence_is_written_back_unchanged_and_a_new_event_has_none() {
    let text = rewritten(&vevent("SEQUENCE:7\r\n"));
    assert_eq!(text.matches("SEQUENCE").count(), 1);
    assert!(text.contains("SEQUENCE:7\r\n"));

    let new = write("a", &[(fields(), UnimplementedFields::default())]);
    assert!(!new.contains("SEQUENCE"));
}

#[test]
fn the_same_input_gives_the_same_text_in_any_order() {
    for server in SERVER_TEXTS {
        let events = events_of(server);
        let pairs: Vec<_> = events
            .iter()
            .map(|event| (fields_of(event), event.unimplemented.clone()))
            .collect();
        let mut reversed = pairs.clone();
        reversed.reverse();
        let uid = &events[0].uid;
        assert_eq!(write(uid, &pairs), write(uid, &pairs));
        assert_eq!(write(uid, &reversed), write(uid, &pairs), "{server}");
    }
}

#[test]
fn an_end_read_from_duration_is_a_dtend() {
    for given in [
        "DURATION:PT1H30M\r\n",
        "DURATION;X-P=1:PT1H30M\r\n",
        "DURATION:PT1H30M\r\nDURATION:PT2H\r\n",
    ] {
        let text = rewritten(&vevent(given));
        assert!(text.contains("DTEND:20261005T083000Z\r\n"), "{text}");
        assert!(!text.contains("DURATION"), "{text}");
    }

    let both = rewritten(&vevent("DTEND:20261005T080000Z\r\nDURATION:PT2H\r\n"));
    assert!(both.contains("DTEND:20261005T080000Z\r\n"), "{both}");
    assert!(!both.contains("DURATION"), "{both}");
}

#[test]
fn the_vtimezones_are_those_the_times_need() {
    for server in SERVER_TEXTS {
        let sent = rewritten(server);
        let lines = content_lines(&sent);
        let zones: Vec<&str> = (1..lines.len())
            .filter(|&at| lines[at - 1] == "BEGIN:VTIMEZONE")
            .map(|at| lines[at].as_str())
            .collect();
        let expected: &[&str] = match server {
            NEXTCLOUD | RADICALE_NEXTCLOUD => &["TZID:Europe/Berlin", "TZID:America/New_York"],
            ICLOUD => &["TZID:America/New_York"],
            EXCHANGE => &["TZID:Europe/Berlin"],
            _ => &[],
        };
        assert_eq!(zones, expected, "{server}");
    }
}

#[test]
fn a_flight_gets_the_vtimezone_of_each_end_and_a_utc_time_none() {
    let flight = EventFields {
        when: StoredWhen {
            all_day: false,
            start: "2026-10-15T10:00:00".to_owned(),
            end: "2026-10-15T13:00:00".to_owned(),
            start_zone: Some("Europe/Berlin".to_owned()),
            end_zone: Some("America/New_York".to_owned()),
        },
        ..fields()
    };
    let text = assert_round_trip("a", &[(flight, UnimplementedFields::default())]);
    assert!(text.contains("DTSTART;TZID=Europe/Berlin:20261015T100000\r\n"));
    assert!(text.contains("DTEND;TZID=America/New_York:20261015T130000\r\n"));
    assert!(text.contains("BEGIN:VTIMEZONE\r\nTZID:Europe/Berlin\r\n"));
    assert!(text.contains("BEGIN:VTIMEZONE\r\nTZID:America/New_York\r\n"));

    let utc = rewritten(&vevent("DTEND:20261005T080000Z\r\n"));
    assert!(utc.contains("DTSTART:20261005T070000Z\r\n"));
    assert!(!utc.contains("VTIMEZONE"));
}

#[test]
fn a_tzid_of_the_tree_gets_its_vtimezone() {
    let text = rewritten(&vevent(
        "RRULE:FREQ=FORTNIGHTLY\r\nEXDATE;TZID=W. Europe Standard Time:20261006T090000\r\n",
    ));
    assert!(
        text.contains("BEGIN:VTIMEZONE\r\nTZID:W. Europe Standard Time\r\n"),
        "{text}"
    );
}

#[test]
fn a_tzid_of_the_tree_gets_a_vtimezone_from_its_own_time() {
    let text = rewritten(&vevent(
        "RRULE:FREQ=FORTNIGHTLY\r\nEXDATE;TZID=Europe/Berlin:20100301T090000\r\n",
    ));
    assert!(text.contains("DTSTART:20091231T"), "{text}");
}

/// A text whose tree gives a time in `Custom`, a zone only its `VTIMEZONE`
/// defines.
const CUSTOM_ZONE: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Test//EN\r\n\
     BEGIN:VTIMEZONE\r\nTZID:Unused\r\nBEGIN:STANDARD\r\nDTSTART:19700101T000000\r\n\
     TZOFFSETFROM:+0300\r\nTZOFFSETTO:+0300\r\nEND:STANDARD\r\nEND:VTIMEZONE\r\n\
     BEGIN:VTIMEZONE\r\nTZID:Custom\r\nBEGIN:STANDARD\r\nDTSTART:19700101T000000\r\n\
     TZOFFSETFROM:+0130\r\nTZOFFSETTO:+0130\r\nEND:STANDARD\r\nEND:VTIMEZONE\r\n\
     BEGIN:VTIMEZONE\r\nTZID:Europe/Berlin\r\nBEGIN:STANDARD\r\n\
     DTSTART:19700101T000000\r\nTZOFFSETFROM:+0100\r\nTZOFFSETTO:+0100\r\nEND:STANDARD\r\n\
     END:VTIMEZONE\r\n\
     BEGIN:VEVENT\r\nUID:a\r\nDTSTAMP:20260101T000000Z\r\nDTSTART:20261005T070000Z\r\n\
     RRULE:FREQ=FORTNIGHTLY\r\nEXDATE;TZID=Custom:20261006T090000\r\n\
     X-AT;TZID=Europe/Berlin:20261007T090000\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

#[test]
fn a_tzid_freecal_doesnt_know_keeps_its_vtimezone() {
    let events = events_of(CUSTOM_ZONE);
    let kept: Vec<_> = events[0]
        .unimplemented
        .entries()
        .iter()
        .filter(|entry| entry.name == "ical:VTIMEZONE")
        .collect();
    assert_eq!(
        kept.len(),
        1,
        "only Custom: Unused isn't used, Berlin is known"
    );

    let text = assert_text_round_trip(CUSTOM_ZONE);
    assert_eq!(text.matches("TZID:Custom\r\n").count(), 1, "{text}");
    assert!(
        text.find("TZID:Custom\r\n") < text.find("BEGIN:VEVENT"),
        "at the calendar level: {text}"
    );
    assert!(!text.contains("Unused"), "{text}");
    assert_eq!(text.matches("TZID:Europe/Berlin\r\n").count(), 1, "{text}");
}

#[test]
fn a_vtimezone_gives_the_offset_changes_from_the_year_of_its_first_time() {
    let from = "2026-10-12T09:00:00".parse().unwrap();
    let text = vtimezone_text("Europe/Berlin", Tz::Europe__Berlin, from);
    let lines = content_lines(&text);
    let observance = |kind: &str, dtstart: &str| {
        let at = (1..lines.len())
            .find(|&at| lines[at - 1] == format!("BEGIN:{kind}") && lines[at] == dtstart)
            .unwrap_or_else(|| panic!("{kind} {dtstart} in {text}"));
        lines[at..]
            .iter()
            .take_while(|line| **line != format!("END:{kind}"))
            .cloned()
            .collect::<Vec<_>>()
    };
    let initial = observance("STANDARD", "DTSTART:20251231T010000");
    assert!(initial.contains(&"TZOFFSETTO:+0100".to_owned()));
    let summer = observance("DAYLIGHT", "DTSTART:20260329T020000");
    assert!(summer[1].starts_with("RDATE:20270328T020000,20280326T020000,"));
    assert!(summer.contains(&"TZOFFSETFROM:+0100".to_owned()));
    assert!(summer.contains(&"TZOFFSETTO:+0200".to_owned()));
    assert!(summer.contains(&"TZNAME:CEST".to_owned()));
    let winter = observance("STANDARD", "DTSTART:20261025T030000");
    assert!(winter.contains(&"TZOFFSETFROM:+0200".to_owned()));
    assert!(winter.contains(&"TZOFFSETTO:+0100".to_owned()));

    let tokyo = content_lines(&vtimezone_text("Asia/Tokyo", Tz::Asia__Tokyo, from));
    let standard = tokyo.iter().filter(|line| *line == "BEGIN:STANDARD");
    assert_eq!(standard.count(), 1);
    assert!(tokyo.contains(&"TZOFFSETTO:+0900".to_owned()));
}

#[test]
fn long_lines_are_folded_between_characters_and_end_with_crlf() {
    let long = EventFields {
        title: format!("{}🎂", "x".repeat(73)),
        description: "Grüße aus Köln, ".repeat(20),
        ..fields()
    };
    let tree = UnimplementedFields::new(vec![whole("ical:X-LONG", Vec::new(), &"ä".repeat(60))]);

    let text = assert_round_trip("a", &[(long, tree)]);

    let folded = format!("SUMMARY:{}\r\n {}🎂\r\n", "x".repeat(67), "x".repeat(6));
    assert!(text.contains(&folded), "{text}");
    assert!(!text.replace("\r\n", "").contains('\n'));
}

/// The hostile test Event (spec: "Untrusted content") in every managed
/// field, its UID too, and hostile entries in its tree: what is written is
/// well-formed, holds no attendee, and reads back to the text it can hold.
#[test]
fn hostile_fields_and_entries_cant_change_the_structure() {
    let hostile: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/hostile-event.json")).unwrap();
    let text = |key: &str| hostile[key].as_str().unwrap().to_owned();
    let breakout = "x\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nATTENDEE:mailto:bad@example.com\u{7}";
    let uid = format!("{}\r\nATTENDEE:mailto:uid@example.com", text("title"));
    let fields = EventFields {
        title: text("title"),
        location: text("location"),
        description: text("description"),
        rule: Some("FREQ=DAILY\r\nATTENDEE:mailto:rule@example.com".to_owned()),
        exdates: "<b>,2026-10-21\r\nATTENDEE:x".to_owned(),
        ..fields()
    };
    let tree = UnimplementedFields::new(vec![
        whole(
            "ical:X-A",
            vec![parameter("X-P", &[breakout, "a\"q"])],
            breakout,
        ),
        whole("ical:BEGIN", Vec::new(), "VEVENT"),
        whole("ical:END", Vec::new(), "VEVENT"),
        whole("ical:X-B:ATTENDEE", Vec::new(), "mailto:name@example.com"),
        whole("ical:X-\u{7}C", Vec::new(), "c"),
        whole("ical:DTSTAMP", Vec::new(), "20200101T000000Z"),
        entry(
            "ical:SUMMARY",
            EntryValue::Parameters(vec![parameter("X-Q\r\nATTENDEE", &["v"])]),
        ),
        entry(
            "ical:VALARM",
            EntryValue::Component {
                properties: vec![property("ACTION", Vec::new(), breakout)],
                components: vec![component("END:VALARM", Vec::new(), Vec::new())],
            },
        ),
        entry(
            "ical:VCALENDAR",
            EntryValue::Component {
                properties: vec![property("X-D", Vec::new(), breakout)],
                components: vec![component(
                    "VEVENT",
                    vec![property("UID", Vec::new(), "injected")],
                    Vec::new(),
                )],
            },
        ),
    ]);

    let written = write_resource(&uid, &[(&fields, &tree)], now()).unwrap();
    let sent = written.text;

    let left_out: HashSet<LeftOut> = written.left_out.into_iter().collect();
    assert_eq!(
        left_out,
        HashSet::from([
            LeftOut::ControlCharacter,
            LeftOut::Quote,
            LeftOut::NotAName,
            LeftOut::Structure,
            LeftOut::SetByWriter,
        ]),
        "every kind is reported"
    );
    assert_well_formed(&sent);
    assert_eq!(guests::attendees(&sent), Vec::<String>::new(), "{sent}");
    assert!(!sent.contains("FREQ=DAILY"));
    assert!(!sent.contains("EXDATE"));
    assert!(!sent.contains("injected"));
    let read = events_of(&sent);
    assert_eq!(read.len(), 1, "{sent}");
    assert_eq!(read[0].uid, uid.replace("\r\n", "\n"));
    assert_eq!(read[0].title, text("title").replace("\r\n", "\n"));
    assert_eq!(read[0].location, text("location"));
    assert_eq!(
        read[0].description,
        text("description").replace("\r\n", "\n")
    );
}

/// A text with the calendar-level content `X-WR-CALNAME` and a `VTODO`, and
/// a `VEVENT` of UID `a` starting on 5 October 2026 at 7:00 UTC for each of
/// `events`, with those lines after its start.
fn calendar(events: &[&str]) -> String {
    let mut text =
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Test//EN\r\nX-WR-CALNAME:Work\r\n".to_owned();
    for lines in events {
        text.push_str(&format!(
            "BEGIN:VEVENT\r\nUID:a\r\n{STAMP}DTSTART:20261005T070000Z\r\n{lines}END:VEVENT\r\n"
        ));
    }
    text.push_str("BEGIN:VTODO\r\nUID:t\r\nEND:VTODO\r\nEND:VCALENDAR\r\n");
    text
}

#[test]
fn exceptions_and_revisions_read_write_and_read_again_equal() {
    let texts = [
        // A recurrence ID with unknown parameters, and a rule on an
        // Exception, which the reader keeps whole.
        calendar(&[
            "RRULE:FREQ=DAILY;COUNT=5\r\n",
            "RECURRENCE-ID;RANGE=THISANDFUTURE;X-P=1:20261006T070000Z\r\nRRULE:FREQ=WEEKLY\r\n",
        ]),
        // The calendar level on every revision of the Event.
        calendar(&["SEQUENCE:1\r\n", "SEQUENCE:2\r\n"]),
        // Only Exceptions: the calendar level on the earliest, each revision.
        calendar(&[
            "RECURRENCE-ID:20261007T070000Z\r\n",
            "RECURRENCE-ID:20261006T070000Z\r\nSEQUENCE:1\r\n",
            "RECURRENCE-ID:20261006T070000Z\r\nSEQUENCE:2\r\n",
        ]),
    ];
    for text in &texts {
        let written = assert_text_round_trip(text);
        assert_eq!(written.matches("X-WR-CALNAME").count(), 1, "{written}");
        assert_eq!(written.matches("BEGIN:VTODO").count(), 1, "{written}");
    }
}

#[test]
fn an_entry_without_the_ical_prefix_isnt_written() {
    let tree = UnimplementedFields::new(vec![
        whole("google:X-COLOR", Vec::new(), "tomato"),
        whole("X-BARE", Vec::new(), "bare"),
        whole("ical:X-KEPT", Vec::new(), "kept"),
    ]);

    let text = write("a", &[(fields(), tree)]);

    assert!(text.contains("X-KEPT:kept\r\n"), "{text}");
    assert!(!text.contains("tomato"), "{text}");
    assert!(!text.contains("bare"), "{text}");
}

#[test]
fn a_vevent_nested_in_a_component_stays_in_the_tree() {
    let text = assert_text_round_trip(&vevent(
        "BEGIN:VALARM\r\nACTION:DISPLAY\r\nTRIGGER:-PT15M\r\n\
         BEGIN:VEVENT\r\nUID:inner\r\nDTSTART:20261005T070000Z\r\nEND:VEVENT\r\nEND:VALARM\r\n",
    ));

    assert_eq!(events_of(&text).len(), 1, "{text}");
    assert!(text.contains("UID:inner\r\n"), "{text}");
}

#[test]
fn what_the_reader_never_puts_where_it_is_is_reported() {
    let tree = UnimplementedFields::new(vec![
        entry(
            "ical:RRULE",
            EntryValue::Parameters(vec![parameter("X-P", &["1"])]),
        ),
        entry(
            "ical:X-NOT-MANAGED",
            EntryValue::Parameters(vec![parameter("X-P", &["1"])]),
        ),
    ]);
    let events = [(&fields(), &tree)];

    let written = write_resource("a", &events, now()).unwrap();

    assert_eq!(written.left_out, [LeftOut::Misplaced], "{}", written.text);
}

#[test]
fn a_calendar_level_unlike_the_first_events_is_reported() {
    let level = |name: &str| {
        UnimplementedFields::new(vec![entry(
            "ical:VCALENDAR",
            EntryValue::Component {
                properties: vec![property("X-WR-CALNAME", Vec::new(), name)],
                components: Vec::new(),
            },
        )])
    };
    let (first, same, other) = (level("Work"), level("Work"), level("Home"));
    let exception = exception("2026-10-06T07:00:00", "Later");

    let alike = write_resource("a", &[(&fields(), &first), (&exception, &same)], now());
    let unlike = write_resource("a", &[(&fields(), &first), (&exception, &other)], now());

    assert_eq!(alike.unwrap().left_out, []);
    let unlike = unlike.unwrap();
    assert_eq!(unlike.left_out, [LeftOut::Misplaced]);
    assert!(!unlike.text.contains("Home"), "{}", unlike.text);
}

#[test]
fn a_duration_or_the_parameters_of_a_time_in_the_tree_are_set_by_the_writer() {
    let tree = UnimplementedFields::new(vec![
        whole("ical:DURATION", Vec::new(), "PT5H"),
        entry(
            "ical:DTSTART",
            EntryValue::Parameters(vec![parameter("TZID", &["Asia/Tokyo"])]),
        ),
    ]);
    let events = [(&fields(), &tree)];

    let written = write_resource("a", &events, now()).unwrap();

    assert_eq!(written.left_out, [LeftOut::SetByWriter]);
    assert!(!written.text.contains("DURATION"), "{}", written.text);
    assert!(!written.text.contains("Tokyo"), "{}", written.text);
}
