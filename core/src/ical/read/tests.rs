use std::collections::BTreeSet;

use super::*;
use crate::ical::server_texts::SERVER_TEXTS;
use crate::unimplemented::tests::{parameter, property};

const STAMP: &str = "DTSTAMP:20261108T120000Z\r\n";

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

/// The one Event `text` holds.
fn one(text: &str) -> ImportedEvent {
    let read = read(text);
    let mut events = read.events.into_iter().chain(read.exceptions);
    let event = events.next().unwrap();
    assert!(events.next().is_none());
    event
}

/// The values of the entries named `name` in `event`'s tree.
fn entries<'e>(event: &'e ImportedEvent, name: &str) -> Vec<&'e EntryValue> {
    let entries = event.unimplemented.entries().iter();
    entries
        .filter(|entry| entry.name == name)
        .map(|entry| &entry.value)
        .collect()
}

fn whole(parameters: Vec<Parameter>, value: &str) -> EntryValue {
    EntryValue::Property {
        parameters,
        value: value.to_owned(),
    }
}

fn names(event: &ImportedEvent) -> Vec<&str> {
    let entries = event.unimplemented.entries().iter();
    entries.map(|entry| entry.name.as_str()).collect()
}

#[test]
fn every_kind_of_property_value_goes_into_the_tree_as_read() {
    let values = [
        ("CLASS", "PRIVATE"),
        ("STATUS", "TENTATIVE"),
        ("GEO", "37.386013;-122.082932"),
        ("PRIORITY", "5"),
        ("URL", "https://example.org/a?b=c;d"),
        ("CREATED", "20260101T000000Z"),
        ("RDATE", "20261010T070000Z,20261011T070000Z"),
        ("EXRULE", "FREQ=WEEKLY;COUNT=2"),
        ("X-MICROSOFT-CDO-BUSYSTATUS", "BUSY"),
        ("COMMENT", "a, b; c\nd"),
        ("CATEGORIES", "one,two"),
    ];
    let mut lines = String::new();
    for (name, value) in values {
        let written = value.replace('\n', "\\n");
        lines.push_str(&format!("{name}:{written}\r\n"));
    }
    lines.push_str("ATTACH;ENCODING=BASE64;VALUE=BINARY:SGVsbG8=\r\n");
    lines.push_str("X-FLAG;VALUE=BOOLEAN:TRUE\r\n");
    lines.push_str("RDATE;VALUE=PERIOD:20261012T070000Z/PT1H\r\n");

    let event = one(&vevent(&lines));

    for (name, value) in values {
        let name = format!("ical:{name}");
        let expected = whole(Vec::new(), value);
        assert_eq!(entries(&event, &name)[..1], [&expected], "{name}");
    }
    let binary = vec![
        parameter("ENCODING", &["BASE64"]),
        parameter("VALUE", &["BINARY"]),
    ];
    assert_eq!(entries(&event, "ical:ATTACH"), [&whole(binary, "SGVsbG8=")]);
    let boolean = vec![parameter("VALUE", &["BOOLEAN"])];
    assert_eq!(entries(&event, "ical:X-FLAG"), [&whole(boolean, "TRUE")]);
    let period = vec![parameter("VALUE", &["PERIOD"])];
    let rdates = entries(&event, "ical:RDATE");
    assert_eq!(rdates[1], &whole(period, "20261012T070000Z/PT1H"));
}

#[test]
fn parameters_are_kept_quoted_listed_and_x() {
    let event = one(&vevent(
        "ATTENDEE;CN=\"Doe, Jane\";MEMBER=\"mailto:a@example.org\",\"mailto:b@example.org\";\
         X-NUM-GUESTS=0;RSVP=TRUE:mailto:jane@example.org\r\n\
         CONTACT;ALTREP=\"cid:part1@example.org\":Jim Dolittle\r\n",
    ));

    let attendee = vec![
        parameter("CN", &["Doe, Jane"]),
        parameter("MEMBER", &["mailto:a@example.org", "mailto:b@example.org"]),
        parameter("RSVP", &["TRUE"]),
        parameter("X-NUM-GUESTS", &["0"]),
    ];
    assert_eq!(
        entries(&event, "ical:ATTENDEE"),
        [&whole(attendee, "mailto:jane@example.org")]
    );
    let altrep = vec![parameter("ALTREP", &["cid:part1@example.org"])];
    assert_eq!(
        entries(&event, "ical:CONTACT"),
        [&whole(altrep, "Jim Dolittle")]
    );
}

#[test]
fn components_go_into_the_tree_whole_alarms_nested_and_unknown() {
    let event = one(&vevent(
        "BEGIN:VALARM\r\nACTION:DISPLAY\r\nTRIGGER:-PT15M\r\nDESCRIPTION:Soon\r\nEND:VALARM\r\n\
         BEGIN:VALARM\r\nACTION:AUDIO\r\nTRIGGER:-PT5M\r\nEND:VALARM\r\n\
         BEGIN:X-OUTER\r\nX-A:1\r\nBEGIN:X-INNER\r\nX-B:2\r\nEND:X-INNER\r\nEND:X-OUTER\r\n\
         BEGIN:VLOCATION\r\nNAME:Room\r\nEND:VLOCATION\r\n",
    ));

    let alarm = |action: &str, trigger: &str, more: Vec<Property>| {
        let mut properties = vec![
            property("ACTION", Vec::new(), action),
            property("TRIGGER", Vec::new(), trigger),
        ];
        properties.extend(more);
        properties.sort_by(|a, b| a.name.cmp(&b.name));
        EntryValue::Component {
            properties,
            components: Vec::new(),
        }
    };
    let soon = vec![property("DESCRIPTION", Vec::new(), "Soon")];
    assert_eq!(
        entries(&event, "ical:VALARM"),
        [
            &alarm("DISPLAY", "-PT15M", soon),
            &alarm("AUDIO", "-PT5M", Vec::new())
        ]
    );
    let outer = EntryValue::Component {
        properties: vec![property("X-A", Vec::new(), "1")],
        components: vec![Component {
            name: "X-INNER".to_owned(),
            properties: vec![property("X-B", Vec::new(), "2")],
            components: Vec::new(),
        }],
    };
    assert_eq!(entries(&event, "ical:X-OUTER"), [&outer]);
    assert_eq!(entries(&event, "ical:VLOCATION").len(), 1);
    // The alarm's description isn't the Event's.
    assert_eq!(event.description, "");
}

#[test]
fn unknown_parameters_of_each_managed_property_go_into_the_tree_without_its_value() {
    let unknown = ";LANGUAGE=de;X-P=\"1\"";
    for (name, line) in [
        ("UID", "UID{p}:b"),
        ("SUMMARY", "SUMMARY{p}:Title"),
        ("LOCATION", "LOCATION{p}:Room"),
        ("DESCRIPTION", "DESCRIPTION{p}:Notes"),
        ("DTEND", "DTEND{p}:20261005T080000Z"),
        ("RRULE", "RRULE{p}:FREQ=DAILY;COUNT=3"),
        (
            "EXDATE",
            "RRULE:FREQ=DAILY;COUNT=3\r\nEXDATE{p}:20261006T070000Z",
        ),
    ] {
        let lines = format!("{}\r\n", line.replace("{p}", unknown));
        let text = vevent(&lines).replace("UID:a\r\n", "");
        let text = if name == "UID" {
            text
        } else {
            text.replace(STAMP, &format!("{STAMP}UID:a\r\n"))
        };
        let event = one(&text);
        let expected = EntryValue::Parameters(vec![
            parameter("LANGUAGE", &["de"]),
            parameter("X-P", &["1"]),
        ]);
        assert_eq!(names(&event), [format!("ical:{name}")], "{name}");
        assert_eq!(
            entries(&event, &format!("ical:{name}")),
            [&expected],
            "{name}"
        );
    }

    let start = one(&vevent("").replace("DTSTART:", &format!("DTSTART{unknown}:")));
    assert_eq!(names(&start), ["ical:DTSTART"]);
    let duration = one(&vevent(&format!("DURATION{unknown}:PT1H\r\n")));
    assert_eq!(names(&duration), ["ical:DURATION"]);
    assert!(
        matches!(duration.when, ImportedWhen::Timed { end, .. } if end.format("%H").to_string() == "08")
    );

    let exception = one(&vevent(&format!(
        "RECURRENCE-ID;RANGE=THISANDFUTURE{unknown}:20261005T070000Z\r\n"
    )));
    assert!(exception.recurrence_id.is_some());
    let range = EntryValue::Parameters(vec![
        parameter("LANGUAGE", &["de"]),
        parameter("RANGE", &["THISANDFUTURE"]),
        parameter("X-P", &["1"]),
    ]);
    assert_eq!(entries(&exception, "ical:RECURRENCE-ID"), [&range]);
}

#[test]
fn managed_values_are_not_repeated_in_the_tree() {
    let event = one(&vevent(
        "DTEND;TZID=Europe/Berlin:20261005T100000\r\nSUMMARY:Title\r\nLOCATION:Room\r\n\
         DESCRIPTION:Notes\r\nRRULE:FREQ=DAILY;COUNT=3\r\nEXDATE:20261006T070000Z\r\n\
         EXDATE:20261007T070000Z\r\nLAST-MODIFIED:20261108T120000Z\r\n",
    ));

    assert!(event.unimplemented.is_empty(), "{:?}", names(&event));
    assert_eq!(event.title, "Title");
    assert_eq!(event.exdates.split(',').count(), 2);
    assert!(read(&vevent("")).violations.is_empty());
}

#[test]
fn a_repeated_managed_property_goes_into_the_tree_whole_and_is_a_violation() {
    let read = read(&vevent(
        "SUMMARY:First\r\nSUMMARY;LANGUAGE=de:Second\r\nDTSTART:20261006T070000Z\r\n\
         RRULE:FREQ=DAILY\r\nRRULE:FREQ=WEEKLY\r\n",
    ));
    let event = &read.events[0];

    assert_eq!(event.title, "First");
    let language = vec![parameter("LANGUAGE", &["de"])];
    assert_eq!(entries(event, "ical:SUMMARY"), [&whole(language, "Second")]);
    assert_eq!(
        entries(event, "ical:DTSTART"),
        [&whole(Vec::new(), "20261006T070000Z")]
    );
    assert_eq!(
        entries(event, "ical:RRULE"),
        [&whole(Vec::new(), "FREQ=WEEKLY")]
    );
    assert_eq!(
        read.violations,
        [
            RfcViolation::Repeated("DTSTART"),
            RfcViolation::Repeated("RRULE"),
            RfcViolation::Repeated("SUMMARY"),
        ]
    );
}

#[test]
fn a_value_form_freecal_doesn_t_model_goes_into_the_tree_whole() {
    let event = one(&vevent(
        "RRULE:FREQ=FORTNIGHTLY\r\nEXDATE:20261006T070000Z\r\n",
    ));
    assert_eq!(event.rule, None);
    assert_eq!(event.exdates, "");
    assert_eq!(names(&event), ["ical:EXDATE", "ical:RRULE"]);

    let without_rule = one(&vevent("EXDATE:20261006T070000Z\r\n"));
    assert_eq!(names(&without_rule), ["ical:EXDATE"]);

    let unreadable_exdate = one(&vevent(
        "RRULE:FREQ=DAILY;COUNT=3\r\nEXDATE:20261006T070000Z\r\nEXDATE:tomorrow\r\n",
    ));
    assert_eq!(unreadable_exdate.exdates.split(',').count(), 1);
    assert_eq!(
        entries(&unreadable_exdate, "ical:EXDATE"),
        [&whole(Vec::new(), "tomorrow")]
    );

    let rule_on_exception = one(&vevent(
        "RECURRENCE-ID:20261005T070000Z\r\nRRULE:FREQ=DAILY\r\n",
    ));
    assert_eq!(names(&rule_on_exception), ["ical:RRULE"]);
}

#[test]
fn sequence_is_an_entry_and_tells_revisions_apart() {
    let event = one(&vevent("SEQUENCE:3\r\n"));
    assert_eq!(event.sequence, 3);
    assert_eq!(entries(&event, "ical:SEQUENCE"), [&whole(Vec::new(), "3")]);
}

#[test]
fn the_rfc_violations_noticed_come_back_with_the_result() {
    let violations = |text: String| read(&text).violations;
    assert_eq!(
        violations(vevent("").replace(STAMP, "").replace("UID:a\r\n", "")),
        [
            RfcViolation::Missing("UID"),
            RfcViolation::Missing("DTSTAMP")
        ]
    );
    assert_eq!(
        violations(vevent("DTEND:20261005T060000Z\r\n")),
        [RfcViolation::EndBeforeStart]
    );
    assert_eq!(
        violations(vevent("").replace(
            "DTSTART:20261005T070000Z",
            "DTSTART;VALUE=DATE:20261005\r\nDTEND;VALUE=DATE:20261005"
        )),
        [RfcViolation::EndBeforeStart]
    );
    let both = read(&vevent("DTEND:20261005T080000Z\r\nDURATION:PT2H\r\n"));
    assert_eq!(both.violations, [RfcViolation::EndAndDuration]);
    assert_eq!(names(&both.events[0]), ["ical:DURATION"]);
}

#[test]
fn a_text_freecal_can_t_read_is_refused_whole() {
    for text in [
        vevent("").replace("DTSTART:20261005T070000Z\r\n", ""),
        vevent("RECURRENCE-ID:soon\r\n"),
        "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nEND:VCALENDAR\r\n".to_owned(),
        "no calendar".to_owned(),
    ] {
        assert!(
            read_events("x.ics", text.as_bytes(), Tz::UTC).is_err(),
            "{text}"
        );
    }
}

/// `text` with its Events, `VEVENT`s each with the lines given after its
/// UID and start, and calendar-level content.
fn calendar(events: &[(&str, &str)]) -> String {
    let mut text = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Test//EN\r\nCALSCALE:GREGORIAN\r\n\
                    X-WR-CALNAME:Work\r\nBEGIN:VTIMEZONE\r\nTZID:Europe/Berlin\r\nEND:VTIMEZONE\r\n"
        .to_owned();
    for (uid, lines) in events {
        text.push_str(&format!(
            "BEGIN:VEVENT\r\nUID:{uid}\r\n{STAMP}DTSTART:20261005T070000Z\r\n{lines}END:VEVENT\r\n"
        ));
    }
    text.push_str("BEGIN:VTODO\r\nUID:t\r\nEND:VTODO\r\nEND:VCALENDAR\r\n");
    text
}

fn calendar_level() -> EntryValue {
    EntryValue::Component {
        properties: vec![property("X-WR-CALNAME", Vec::new(), "Work")],
        components: vec![Component {
            name: "VTODO".to_owned(),
            properties: vec![property("UID", Vec::new(), "t")],
            components: Vec::new(),
        }],
    }
}

/// The recurrence IDs (none for an Event that isn't an Exception) of the
/// Events of `read` that hold the calendar-level entry.
fn holders(read: &ReadFile) -> Vec<Option<String>> {
    let events = read.events.iter().chain(&read.exceptions);
    events
        .filter(|event| !entries(event, "ical:VCALENDAR").is_empty())
        .inspect(|event| assert_eq!(entries(event, "ical:VCALENDAR"), [&calendar_level()]))
        .map(|event| {
            let id = event.recurrence_id.as_ref();
            id.map(|id| id.normal_form(None).into_string())
        })
        .collect()
}

#[test]
fn calendar_level_content_sits_on_the_single_or_recurring_event_or_the_earliest_exception() {
    let single = read(&calendar(&[("a", "")]));
    assert_eq!(holders(&single), [None]);

    let recurring = read(&calendar(&[
        ("a", "RECURRENCE-ID:20261006T070000Z\r\n"),
        ("a", "RRULE:FREQ=DAILY;COUNT=3\r\n"),
    ]));
    assert_eq!(holders(&recurring), [None]);

    let exceptions = read(&calendar(&[
        ("a", "RECURRENCE-ID:20261007T070000Z\r\n"),
        ("a", "RECURRENCE-ID:20261006T070000Z\r\n"),
    ]));
    assert_eq!(
        holders(&exceptions),
        [Some("2026-10-06T07:00:00[UTC]".to_owned())]
    );

    let several_uids = read(&calendar(&[("a", ""), ("b", "")]));
    assert_eq!(holders(&several_uids), Vec::<Option<String>>::new());
}

#[test]
fn order_folding_letter_case_and_line_endings_make_no_difference() {
    let one_way = vevent(
        "SUMMARY;LANGUAGE=de:Title\r\nX-A:1\r\nX-B;X-P=a:2\r\nX-A:3\r\n\
         BEGIN:VALARM\r\nACTION:DISPLAY\r\nTRIGGER:-PT5M\r\nEND:VALARM\r\n",
    );
    let other_way = "BEGIN:VCALENDAR\nversion:2.0\nprodid:-//Other//EN\nBEGIN:VEVENT\n\
         begin:valarm\ntrigger:-PT5M\naction:DISPLAY\nend:valarm\n\
         x-b;x-p=\"a\":2\nx-a:1\nDTSTART:20261005T070000Z\nsummary;language=de:Ti\n tle\n\
         x-a:3\nuid:a\ndtstamp:20200101T000000Z\nEND:VEVENT\nEND:VCALENDAR\n";

    let one_way = one(&one_way);
    let other_way = one(other_way);

    assert_eq!(other_way.title, "Title");
    assert!(!one_way.unimplemented.is_empty());
    assert_eq!(
        one_way.unimplemented.canonical(),
        other_way.unimplemented.canonical()
    );
    let changed = one(&vevent(
        "SUMMARY;LANGUAGE=de:Title\r\nX-A:3\r\nX-B;X-P=a:2\r\nX-A:1\r\n",
    ));
    assert_ne!(changed.unimplemented, one_way.unimplemented);
}

/// Each part of `text` as a path of [`DROPPED`]'s form: `VCALENDAR.NAME`
/// for its calendar-level properties and components, `VEVENT.NAME` and
/// `VEVENT.NAME;PARAMETER` for its Events' properties, `VEVENT.NAME` for
/// their components.
fn paths_in(text: &str) -> BTreeSet<String> {
    read_vcalendars(text, |roots| {
        let mut paths = BTreeSet::new();
        for root in &roots {
            for property in root.properties.iter().map(tree_property) {
                paths.insert(format!("VCALENDAR.{}", property.name));
            }
            for component in root.components.iter().map(tree_component) {
                if component.name != "VEVENT" {
                    paths.insert(format!("VCALENDAR.{}", component.name));
                    continue;
                }
                for property in &component.properties {
                    property_paths(&mut paths, &property.name, &property.parameters);
                }
                for inner in &component.components {
                    paths.insert(format!("VEVENT.{}", inner.name));
                }
            }
        }
        paths
    })
    .unwrap()
}

fn property_paths(paths: &mut BTreeSet<String>, name: &str, parameters: &[Parameter]) {
    paths.insert(format!("VEVENT.{name}"));
    for parameter in parameters {
        paths.insert(format!("VEVENT.{name};{}", parameter.name));
    }
}

/// The paths, as [`paths_in`] gives them, of what `read`'s trees and
/// managed fields hold.
fn paths_kept(read: &ReadFile) -> BTreeSet<String> {
    let mut paths: BTreeSet<String> = MANAGED
        .iter()
        .map(|name| format!("VEVENT.{name}"))
        .collect();
    for event in read.events.iter().chain(&read.exceptions) {
        for entry in event.unimplemented.entries() {
            let name = entry.name.strip_prefix(ICAL).unwrap();
            match &entry.value {
                EntryValue::Component {
                    properties,
                    components,
                } if name == "VCALENDAR" => {
                    for property in properties {
                        paths.insert(format!("VCALENDAR.{}", property.name));
                    }
                    for component in components {
                        paths.insert(format!("VCALENDAR.{}", component.name));
                    }
                }
                EntryValue::Component { .. } => {
                    paths.insert(format!("VEVENT.{name}"));
                }
                EntryValue::Property { parameters, .. } => {
                    property_paths(&mut paths, name, parameters)
                }
                EntryValue::Parameters(parameters) => {
                    for parameter in parameters {
                        paths.insert(format!("VEVENT.{name};{}", parameter.name));
                    }
                }
            }
        }
    }
    paths
}

/// The properties whose values are managed fields.
const MANAGED: &[&str] = &[
    "UID",
    "SUMMARY",
    "LOCATION",
    "DESCRIPTION",
    "DTSTART",
    "DTEND",
    "RECURRENCE-ID",
    "RRULE",
    "EXDATE",
];

#[test]
fn the_server_texts_are_read_without_loss_but_what_dropped_lists() {
    let left_out: Vec<Vec<String>> = SERVER_TEXTS
        .iter()
        .map(|text| {
            let read = read(text);
            let kept = paths_kept(&read);
            paths_in(text).difference(&kept).cloned().collect()
        })
        .collect();

    let dropped: BTreeSet<&str> = DROPPED.iter().map(|(path, _)| *path).collect();
    for (text, left_out) in SERVER_TEXTS.iter().zip(&left_out) {
        for path in left_out {
            assert!(dropped.contains(path.as_str()), "{path} lost from\n{text}");
        }
    }
    let stamp = ["VEVENT.DTSTAMP", "VEVENT.LAST-MODIFIED"];
    let tzid = ["VEVENT.DTEND;TZID", "VEVENT.DTSTART;TZID"];
    let exceptions = ["VEVENT.EXDATE;TZID", "VEVENT.RECURRENCE-ID;TZID"];
    let zones = ["VCALENDAR.VTIMEZONE"];
    let calscale = ["VCALENDAR.CALSCALE"];
    let expected: [Vec<&str>; 7] = [
        [&calscale[..], &zones, &stamp, &tzid, &exceptions].concat(),
        [&calscale[..], &stamp].concat(),
        [&calscale[..], &zones, &stamp, &tzid].concat(),
        [&zones[..], &stamp[..1], &tzid].concat(),
        vec![
            "VEVENT.DTEND;VALUE",
            "VEVENT.DTSTAMP",
            "VEVENT.DTSTART;VALUE",
        ],
        [&calscale[..], &zones, &stamp, &tzid, &exceptions].concat(),
        [&calscale[..], &stamp].concat(),
    ];
    for ((text, left_out), expected) in SERVER_TEXTS.iter().zip(&left_out).zip(expected) {
        let mut expected: Vec<&str> = expected;
        expected.extend(["VCALENDAR.PRODID", "VCALENDAR.VERSION"]);
        expected.sort_unstable();
        assert_eq!(left_out, &expected, "{text}");
    }
}

#[test]
fn every_windows_name_maps_to_a_zone_freecal_knows() {
    for (windows, iana) in WINDOWS_ZONES {
        assert!(iana.parse::<Tz>().is_ok(), "{windows} → {iana}");
    }
}

#[test]
fn html_is_told_apart_from_plain_text_with_angle_brackets() {
    for html in [
        "Agenda<br>Notes",
        "<p>Agenda</p>",
        "see <a href=\"https://example.org\">here</a>",
        "<B>bold</B>",
        "line<br/>",
    ] {
        assert!(looks_like_html(html), "{html:?}");
    }
    for plain in [
        "if a < b and c > d",
        "Mail <someone@example.org>",
        "<placeholder>",
        "x <3 y",
        "trailing <",
    ] {
        assert!(!looks_like_html(plain), "{plain:?}");
    }
}
