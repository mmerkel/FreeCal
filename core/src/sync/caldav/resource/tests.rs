use std::collections::HashSet;

use chrono::{DateTime, NaiveDateTime, Utc};
use chrono_tz::Tz;

use super::*;
use crate::event::{EventId, RecurrenceId, Source, StoredWhen};
use crate::import::{self, ExistingEvents, ImportFile};
use crate::journal::{self, TransactionOrigin};
use crate::store::{Store, Uid, UidEvent};
use crate::{CalendarId, guests};

/// Server texts in the forms servers and the clients writing to them
/// store, each with FreeCal's Properties among properties FreeCal doesn't
/// keep. Radicale's are as it returned them; the others follow what each
/// writes.
const NEXTCLOUD: &str = "BEGIN:VCALENDAR\r
PRODID:-//IDN nextcloud.com//Calendar app 4.6.0//EN\r
VERSION:2.0\r
CALSCALE:GREGORIAN\r
BEGIN:VTIMEZONE\r
TZID:Europe/Berlin\r
BEGIN:DAYLIGHT\r
TZOFFSETFROM:+0100\r
TZOFFSETTO:+0200\r
TZNAME:CEST\r
DTSTART:19700329T020000\r
RRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=-1SU\r
END:DAYLIGHT\r
BEGIN:STANDARD\r
TZOFFSETFROM:+0200\r
TZOFFSETTO:+0100\r
TZNAME:CET\r
DTSTART:19701025T030000\r
RRULE:FREQ=YEARLY;BYMONTH=10;BYDAY=-1SU\r
END:STANDARD\r
END:VTIMEZONE\r
BEGIN:VEVENT\r
CREATED:20261001T080000Z\r
DTSTAMP:20261001T080512Z\r
LAST-MODIFIED:20261001T080512Z\r
SEQUENCE:2\r
UID:5f1c2a9e-5d1b-4bd6-a1b2-nextcloud\r
DTSTART;TZID=Europe/Berlin:20261012T090000\r
DTEND;TZID=Europe/Berlin:20261012T093000\r
STATUS:CONFIRMED\r
SUMMARY:Standup\r
LOCATION:Room 4\r
DESCRIPTION:Daily sync\\, short.\r
RRULE:FREQ=WEEKLY;BYDAY=MO,WE\r
EXDATE;TZID=Europe/Berlin:20261014T090000\r
CATEGORIES:Work,Team\r
COLOR:tomato\r
X-MOZ-GENERATION:3\r
BEGIN:VALARM\r
ACTION:DISPLAY\r
TRIGGER;RELATED=START:-PT10M\r
DESCRIPTION:Reminder\r
END:VALARM\r
END:VEVENT\r
BEGIN:VEVENT\r
CREATED:20261001T080000Z\r
DTSTAMP:20261002T090000Z\r
UID:5f1c2a9e-5d1b-4bd6-a1b2-nextcloud\r
RECURRENCE-ID;TZID=America/New_York:20261019T030000\r
DTSTART;TZID=Europe/Berlin:20261019T100000\r
DTEND;TZID=Europe/Berlin:20261019T103000\r
SUMMARY:Standup (moved)\r
X-NEXTCLOUD-BC-FIELD-TYPE:link\r
END:VEVENT\r
END:VCALENDAR\r
";

const GOOGLE: &str = "BEGIN:VCALENDAR\r
PRODID:-//Google Inc//Google Calendar 70.9054//EN\r
VERSION:2.0\r
CALSCALE:GREGORIAN\r
BEGIN:VEVENT\r
DTSTART:20261013T130000Z\r
DTEND:20261013T140000Z\r
DTSTAMP:20261008T101010Z\r
ORGANIZER;CN=anna@example.com:mailto:anna@example.com\r
UID:7kukuqrfedlm2f9t0vb3o1ijc4@google.com\r
ATTENDEE;CUTYPE=INDIVIDUAL;ROLE=REQ-PARTICIPANT;PARTSTAT=ACCEPTED;CN=anna@ex\r
 ample.com;X-NUM-GUESTS=0:mailto:anna@example.com\r
X-GOOGLE-CONFERENCE:https://meet.google.com/abc-defg-hij\r
CREATED:20261008T101000Z\r
DESCRIPTION:-::~:~::~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~\r
 :~:~::-\\nJoin with Google Meet: https://meet.google.com/abc-defg-hij\\n\\nLea\r
 rn more about Meet at: https://support.google.com/a/users/answer/9282720\r
LAST-MODIFIED:20261008T101010Z\r
LOCATION:\r
SEQUENCE:0\r
STATUS:CONFIRMED\r
SUMMARY:Planning\r
TRANSP:OPAQUE\r
BEGIN:VALARM\r
ACTION:EMAIL\r
DESCRIPTION:This is an event reminder\r
SUMMARY:Alarm notification\r
ATTENDEE:mailto:anna@example.com\r
TRIGGER:-P0DT0H30M0S\r
END:VALARM\r
END:VEVENT\r
END:VCALENDAR\r
";

const ICLOUD: &str = "BEGIN:VCALENDAR\r
VERSION:2.0\r
PRODID:-//Apple Inc.//macOS 15.0//EN\r
CALSCALE:GREGORIAN\r
BEGIN:VTIMEZONE\r
TZID:America/New_York\r
BEGIN:DAYLIGHT\r
TZOFFSETFROM:-0500\r
RRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=2SU\r
DTSTART:20070311T020000\r
TZNAME:EDT\r
TZOFFSETTO:-0400\r
END:DAYLIGHT\r
BEGIN:STANDARD\r
TZOFFSETFROM:-0400\r
RRULE:FREQ=YEARLY;BYMONTH=11;BYDAY=1SU\r
DTSTART:20071104T020000\r
TZNAME:EST\r
TZOFFSETTO:-0500\r
END:STANDARD\r
END:VTIMEZONE\r
BEGIN:VEVENT\r
TRANSP:OPAQUE\r
DTEND;TZID=America/New_York:20261014T120000\r
X-APPLE-TRAVEL-ADVISORY-BEHAVIOR:AUTOMATIC\r
UID:9C4E1D3B-1F2A-4B7C-8D9E-0A1B2C3D4E5F\r
DTSTAMP:20261009T120000Z\r
LOCATION:Apple Park\\nOne Apple Park Way\\, Cupertino\\, CA 95014\\, United St\r
 ates\r
X-APPLE-STRUCTURED-LOCATION;VALUE=URI;X-ADDRESS=\"One Apple Park Way, Cupert\r
 ino, CA 95014, United States\";X-APPLE-RADIUS=141.1750506;X-TITLE=Apple Par\r
 k:geo:37.334922,-122.009033\r
SEQUENCE:1\r
SUMMARY:Lunch\r
LAST-MODIFIED:20261009T120000Z\r
CREATED:20261009T115900Z\r
DTSTART;TZID=America/New_York:20261014T110000\r
BEGIN:VALARM\r
X-WR-ALARMUID:5B3C9D4E-6F7A-4B8C-9D0E-1F2A3B4C5D6E\r
UID:5B3C9D4E-6F7A-4B8C-9D0E-1F2A3B4C5D6E\r
TRIGGER:-PT15M\r
ATTACH;VALUE=URI:Chord\r
ACTION:AUDIO\r
END:VALARM\r
END:VEVENT\r
END:VCALENDAR\r
";

const EXCHANGE: &str = "BEGIN:VCALENDAR\r
METHOD:PUBLISH\r
PRODID:Microsoft Exchange Server 2010\r
VERSION:2.0\r
BEGIN:VTIMEZONE\r
TZID:W. Europe Standard Time\r
BEGIN:STANDARD\r
DTSTART:16010101T030000\r
TZOFFSETFROM:+0200\r
TZOFFSETTO:+0100\r
RRULE:FREQ=YEARLY;INTERVAL=1;BYDAY=-1SU;BYMONTH=10\r
END:STANDARD\r
BEGIN:DAYLIGHT\r
DTSTART:16010101T020000\r
TZOFFSETFROM:+0100\r
TZOFFSETTO:+0200\r
RRULE:FREQ=YEARLY;INTERVAL=1;BYDAY=-1SU;BYMONTH=3\r
END:DAYLIGHT\r
END:VTIMEZONE\r
BEGIN:VEVENT\r
DESCRIPTION:\\n\r
UID:040000008200E00074C5B7101A82E00800000000A0B1C2D3E4F5A6B7C8D9E0F1A2B3C4D5\r
SUMMARY;LANGUAGE=de-DE:Projekttreffen\r
DTSTART;TZID=W. Europe Standard Time:20261015T140000\r
DTEND;TZID=W. Europe Standard Time:20261015T150000\r
CLASS:PUBLIC\r
PRIORITY:5\r
DTSTAMP:20261009T090000Z\r
TRANSP:OPAQUE\r
STATUS:CONFIRMED\r
SEQUENCE:0\r
LOCATION;LANGUAGE=de-DE:Raum 2\r
X-MICROSOFT-CDO-APPT-SEQUENCE:0\r
X-MICROSOFT-CDO-BUSYSTATUS:BUSY\r
X-MICROSOFT-CDO-INTENDEDSTATUS:BUSY\r
X-MICROSOFT-CDO-ALLDAYEVENT:FALSE\r
X-MICROSOFT-CDO-IMPORTANCE:1\r
X-MICROSOFT-CDO-INSTTYPE:0\r
X-MICROSOFT-DISALLOW-COUNTER:FALSE\r
END:VEVENT\r
END:VCALENDAR\r
";

const EVOLUTION: &str = "BEGIN:VCALENDAR\r
VERSION:2.0\r
PRODID:-//Ximian//NONSGML Evolution Calendar//EN\r
BEGIN:VEVENT\r
UID:20261001T101010Z-1234-1000-1-0@laptop\r
DTSTAMP:20261001T101010Z\r
DTSTART;VALUE=DATE:20261020\r
DTEND;VALUE=DATE:20261021\r
SEQUENCE:1\r
SUMMARY:Birthday\r
CLASS:PRIVATE\r
TRANSP:TRANSPARENT\r
RRULE:FREQ=YEARLY\r
X-EVOLUTION-CALDAV-HREF:/dav/calendars/anna/home/20261001T101010Z-1234.ics\r
END:VEVENT\r
END:VCALENDAR\r
";

/// [`NEXTCLOUD`] as Radicale 3.8.3 returned it after a PUT: rewritten by its
/// library, which orders properties anew, puts some after the alarm, and
/// adds a `VTIMEZONE`.
const RADICALE_NEXTCLOUD: &str = "BEGIN:VCALENDAR\r
VERSION:2.0\r
CALSCALE:GREGORIAN\r
PRODID:-//IDN nextcloud.com//Calendar app 4.6.0//EN\r
BEGIN:VTIMEZONE\r
TZID:Europe/Berlin\r
BEGIN:STANDARD\r
DTSTART:19701025T030000\r
RRULE:FREQ=YEARLY;BYMONTH=10;BYDAY=-1SU\r
TZNAME:CET\r
TZOFFSETFROM:+0200\r
TZOFFSETTO:+0100\r
END:STANDARD\r
BEGIN:DAYLIGHT\r
DTSTART:19700329T020000\r
RRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=-1SU\r
TZNAME:CEST\r
TZOFFSETFROM:+0100\r
TZOFFSETTO:+0200\r
END:DAYLIGHT\r
END:VTIMEZONE\r
BEGIN:VTIMEZONE\r
TZID:America/New_York\r
BEGIN:STANDARD\r
DTSTART:20000101T000000\r
RRULE:FREQ=YEARLY;BYMONTH=1;UNTIL=20060101T050000Z\r
TZNAME:EST\r
TZOFFSETFROM:-0500\r
TZOFFSETTO:-0500\r
END:STANDARD\r
BEGIN:STANDARD\r
DTSTART:20071104T030000\r
RRULE:FREQ=YEARLY;BYDAY=1SU;BYMONTH=11\r
TZNAME:EST\r
TZOFFSETFROM:-0400\r
TZOFFSETTO:-0500\r
END:STANDARD\r
BEGIN:DAYLIGHT\r
DTSTART:20070311T020000\r
RRULE:FREQ=YEARLY;BYDAY=2SU;BYMONTH=3\r
TZNAME:EDT\r
TZOFFSETFROM:-0500\r
TZOFFSETTO:-0400\r
END:DAYLIGHT\r
END:VTIMEZONE\r
BEGIN:VEVENT\r
UID:5f1c2a9e-5d1b-4bd6-a1b2-nextcloud\r
DTSTART;TZID=Europe/Berlin:20261012T090000\r
DTEND;TZID=Europe/Berlin:20261012T093000\r
CATEGORIES:Work,Team\r
COLOR:tomato\r
CREATED:20261001T080000Z\r
DESCRIPTION:Daily sync\\, short.\r
DTSTAMP:20261001T080512Z\r
EXDATE;TZID=Europe/Berlin:20261014T090000\r
LAST-MODIFIED:20261001T080512Z\r
LOCATION:Room 4\r
RRULE:FREQ=WEEKLY;BYDAY=MO,WE\r
SEQUENCE:2\r
STATUS:CONFIRMED\r
SUMMARY:Standup\r
BEGIN:VALARM\r
ACTION:DISPLAY\r
DESCRIPTION:Reminder\r
TRIGGER;RELATED=START:-PT10M\r
END:VALARM\r
X-MOZ-GENERATION:3\r
END:VEVENT\r
BEGIN:VEVENT\r
UID:5f1c2a9e-5d1b-4bd6-a1b2-nextcloud\r
RECURRENCE-ID;TZID=America/New_York:20261019T030000\r
DTSTART;TZID=Europe/Berlin:20261019T100000\r
DTEND;TZID=Europe/Berlin:20261019T103000\r
CREATED:20261001T080000Z\r
DTSTAMP:20261002T090000Z\r
SUMMARY:Standup (moved)\r
X-NEXTCLOUD-BC-FIELD-TYPE:link\r
END:VEVENT\r
END:VCALENDAR\r
";

/// [`GOOGLE`] as Radicale 3.8.3 returned it after a PUT: rewritten by its
/// library, which orders properties anew, puts some after the alarm, and
/// adds a `VTIMEZONE`.
const RADICALE_GOOGLE: &str = "BEGIN:VCALENDAR\r
VERSION:2.0\r
CALSCALE:GREGORIAN\r
PRODID:-//Google Inc//Google Calendar 70.9054//EN\r
BEGIN:VEVENT\r
UID:7kukuqrfedlm2f9t0vb3o1ijc4@google.com\r
DTSTART:20261013T130000Z\r
DTEND:20261013T140000Z\r
ATTENDEE;CN=anna@example.com;CUTYPE=INDIVIDUAL;PARTSTAT=ACCEPTED;ROLE=REQ-P\r
 ARTICIPANT;X-NUM-GUESTS=0:mailto:anna@example.com\r
CREATED:20261008T101000Z\r
DESCRIPTION:-::~:~::~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~\r
 :~:~:~::-\\nJoin with Google Meet: https://meet.google.com/abc-defg-hij\\n\\n\r
 Learn more about Meet at: https://support.google.com/a/users/answer/928272\r
 0\r
DTSTAMP:20261008T101010Z\r
LAST-MODIFIED:20261008T101010Z\r
LOCATION:\r
ORGANIZER;CN=anna@example.com:mailto:anna@example.com\r
SEQUENCE:0\r
STATUS:CONFIRMED\r
SUMMARY:Planning\r
TRANSP:OPAQUE\r
BEGIN:VALARM\r
ACTION:EMAIL\r
ATTENDEE:mailto:anna@example.com\r
DESCRIPTION:This is an event reminder\r
SUMMARY:Alarm notification\r
TRIGGER:-PT30M\r
END:VALARM\r
X-GOOGLE-CONFERENCE:https://meet.google.com/abc-defg-hij\r
END:VEVENT\r
END:VCALENDAR\r
";

const SERVER_TEXTS: [&str; 7] = [
    NEXTCLOUD,
    GOOGLE,
    ICLOUD,
    EXCHANGE,
    EVOLUTION,
    RADICALE_NEXTCLOUD,
    RADICALE_GOOGLE,
];

/// Event Rows given as they are, as the Sender loads them from the Local
/// Store.
struct Rows(Vec<UidEvent>);

impl Source for Rows {
    fn events_with_uid(&self, _: Uid) -> crate::Result<Vec<UidEvent>> {
        Ok(self.0.clone())
    }
}

/// The Event Rows that reading `text` gives, as an import or an Incoming
/// Sync stores them.
fn rows_of(text: &str) -> Vec<UidEvent> {
    let directory = tempfile::TempDir::new().unwrap();
    let store = Store::open(&directory.path().join("freecal.sqlite3")).unwrap();
    let calendar_id = store.list_calendars().unwrap()[0].id;
    let file = ImportFile::new("server.ics", text.as_bytes().to_vec());
    let preview = import::preview(&file, calendar_id, &HashSet::new(), Tz::UTC).unwrap();
    journal::record(
        &store,
        TransactionOrigin::Import,
        0,
        |journal_transaction| import::run(journal_transaction, &preview, ExistingEvents::Update),
    )
    .unwrap();
    let uid = preview.events[0].uid.clone();
    store.events_with_uid(Uid::In(calendar_id, &uid)).unwrap()
}

fn loaded(rows: Vec<UidEvent>) -> Loaded {
    let uid = rows[0].uid.clone();
    Loaded::of_uid(&Rows(rows), CalendarId(1), &uid).unwrap()
}

fn now() -> DateTime<Utc> {
    "2026-11-08T12:00:00Z".parse().unwrap()
}

fn send(rows: Vec<UidEvent>, server: Option<&str>) -> String {
    resource_to_send(&loaded(rows), server, now())
        .unwrap()
        .unwrap()
}

/// The Event Row that isn't an Exception.
fn event(rows: &mut [UidEvent]) -> &mut UidEvent {
    rows.iter_mut()
        .find(|row| row.fields.recurrence_id.is_none())
        .unwrap()
}

/// The content lines of `text`, unfolded.
fn content_lines(text: &str) -> Vec<String> {
    icalendar::parser::unfold(text)
        .split("\r\n")
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// That `text` reads back through the import's reader to `rows`: each
/// Event pairs with an Event Row by Occurrence, every Event Row with one,
/// and FreeCal reads the Event Row's fields from it.
fn assert_reads_back(text: &str, rows: Vec<UidEvent>) {
    let loaded = loaded(rows);
    let read = read_events("sent.ics", text.as_bytes(), Tz::UTC).unwrap();
    let events: Vec<ImportedEvent> = read.events.into_iter().chain(read.exceptions).collect();
    let pairing = loaded.pair_with_server(&events);
    assert!(pairing.left.is_empty(), "every Event Row is sent");
    for (event, row) in events.iter().zip(&pairing.paired) {
        let row = row.expect("every Event sent is an Event Row's");
        assert_eq!(event.uid, loaded.uid());
        let read = as_propagated(event, row);
        for group in Group::ALL {
            let recurring = pairing.recurring.as_ref();
            assert!(!group.differs(row, &read, recurring), "{group:?} of {text}");
        }
    }
}

/// That `text` is iCalendar FreeCal writes: CRLF line ends, lines of at
/// most 75 octets with no control characters, and components that nest.
fn assert_well_formed(text: &str) {
    assert!(text.ends_with("\r\n"));
    for line in text.strip_suffix("\r\n").unwrap().split("\r\n") {
        assert!(line.len() <= 75, "{line:?}");
        assert!(
            !line.chars().any(|c| c.is_ascii_control() && c != '\t'),
            "{line:?}"
        );
    }
    assert!(components(text).is_some());
}

/// `text` with each line `(old, new)` of `changes`, which it holds once,
/// changed.
fn changed(text: &str, changes: &[(&str, &str)]) -> String {
    let mut text = text.to_owned();
    for (old, new) in changes {
        assert_eq!(text.matches(old).count(), 1, "{old}");
        text = text.replace(old, new);
    }
    text
}

const STAMP: &str = "DTSTAMP:20261108T120000Z\r\n";
const MODIFIED: &str = "LAST-MODIFIED:20261108T120000Z\r\n";

/// The changes a revision made `now()` makes to the revision properties of
/// the `VEVENT` of `server` that isn't an Exception: `DTSTAMP` and
/// `LAST-MODIFIED` are `now()`, written where they were or added, and
/// `SEQUENCE` one higher.
fn revision_of(server: &str) -> Vec<(&'static str, &'static str)> {
    match server {
        NEXTCLOUD | RADICALE_NEXTCLOUD => vec![
            ("DTSTAMP:20261001T080512Z\r\n", STAMP),
            ("LAST-MODIFIED:20261001T080512Z\r\n", MODIFIED),
            ("SEQUENCE:2\r\n", "SEQUENCE:3\r\n"),
        ],
        GOOGLE | RADICALE_GOOGLE => vec![
            ("DTSTAMP:20261008T101010Z\r\n", STAMP),
            ("LAST-MODIFIED:20261008T101010Z\r\n", MODIFIED),
            ("SEQUENCE:0\r\n", "SEQUENCE:1\r\n"),
        ],
        ICLOUD => vec![
            ("DTSTAMP:20261009T120000Z\r\n", STAMP),
            ("LAST-MODIFIED:20261009T120000Z\r\n", MODIFIED),
            ("SEQUENCE:1\r\n", "SEQUENCE:2\r\n"),
        ],
        EXCHANGE => vec![
            ("DTSTAMP:20261009T090000Z\r\n", STAMP),
            ("\r\nSEQUENCE:0\r\n", "\r\nSEQUENCE:1\r\n"),
            (
                "COUNTER:FALSE\r\nEND:VEVENT",
                "COUNTER:FALSE\r\nLAST-MODIFIED:20261108T120000Z\r\nEND:VEVENT",
            ),
        ],
        EVOLUTION => vec![
            ("DTSTAMP:20261001T101010Z\r\n", STAMP),
            ("SEQUENCE:1\r\n", "SEQUENCE:2\r\n"),
            (
                "1234.ics\r\nEND:VEVENT",
                "1234.ics\r\nLAST-MODIFIED:20261108T120000Z\r\nEND:VEVENT",
            ),
        ],
        _ => unreachable!(),
    }
}

fn wall(text: &str) -> NaiveDateTime {
    text.parse().unwrap()
}

#[test]
fn no_event_rows_is_nothing_to_send() {
    let loaded = Loaded::of_uid(&Rows(Vec::new()), CalendarId(1), "gone").unwrap();
    assert_eq!(
        resource_to_send(&loaded, Some(GOOGLE), now()).unwrap(),
        None
    );
}

#[test]
fn event_rows_as_the_server_object_holds_them_send_it_byte_for_byte() {
    for server in SERVER_TEXTS {
        assert_eq!(send(rows_of(server), Some(server)), server);
    }
}

#[test]
fn an_edited_title_changes_only_its_line_and_the_revision() {
    let edits = [
        (NEXTCLOUD, "SUMMARY:Standup\r\n"),
        (GOOGLE, "SUMMARY:Planning\r\n"),
        (ICLOUD, "SUMMARY:Lunch\r\n"),
        (EXCHANGE, "SUMMARY;LANGUAGE=de-DE:Projekttreffen\r\n"),
        (EVOLUTION, "SUMMARY:Birthday\r\n"),
        (RADICALE_NEXTCLOUD, "SUMMARY:Standup\r\n"),
        (RADICALE_GOOGLE, "SUMMARY:Planning\r\n"),
    ];
    for (server, line) in edits {
        let mut rows = rows_of(server);
        event(&mut rows).fields.title = "Retro; with Bob, at 10".to_owned();

        let sent = send(rows, Some(server));

        let mut changes = revision_of(server);
        changes.push((line, "SUMMARY:Retro\\; with Bob\\, at 10\r\n"));
        assert_eq!(sent, changed(server, &changes));
    }
}

#[test]
fn edited_times_replace_dtstart_dtend_and_add_the_vtimezone_they_need() {
    let mut rows = rows_of(EXCHANGE);
    event(&mut rows).fields.when.start = "2026-10-15T16:00:00".to_owned();
    event(&mut rows).fields.when.end = "2026-10-15T17:00:00".to_owned();

    let sent = send(rows.clone(), Some(EXCHANGE));

    let lines = content_lines(&sent);
    let vtimezones: Vec<usize> = (0..lines.len())
        .filter(|&at| lines[at] == "BEGIN:VTIMEZONE")
        .collect();
    assert_eq!(vtimezones.len(), 2, "Exchange's is kept, Berlin's added");
    assert_eq!(lines[vtimezones[1] + 1], "TZID:Europe/Berlin");
    let vevent = lines
        .iter()
        .position(|line| line == "BEGIN:VEVENT")
        .unwrap();
    assert!(vtimezones[1] < vevent);
    assert!(lines.contains(&"DTSTART;TZID=Europe/Berlin:20261015T160000".to_owned()));
    assert!(lines.contains(&"DTEND;TZID=Europe/Berlin:20261015T170000".to_owned()));
    assert!(!sent.contains("TZID=W. Europe Standard Time:"));
    assert!(sent.contains("X-MICROSOFT-CDO-BUSYSTATUS:BUSY\r\n"));
    assert!(sent.contains("SUMMARY;LANGUAGE=de-DE:Projekttreffen\r\n"));
    assert_reads_back(&sent, rows);
}

#[test]
fn a_utc_time_is_written_in_utc_without_a_vtimezone() {
    let mut rows = rows_of(GOOGLE);
    event(&mut rows).fields.when.start = "2026-10-13T15:00:00".to_owned();
    event(&mut rows).fields.when.end = "2026-10-13T16:00:00".to_owned();

    let sent = send(rows.clone(), Some(GOOGLE));

    let mut changes = revision_of(GOOGLE);
    changes.push(("DTSTART:20261013T130000Z", "DTSTART:20261013T150000Z"));
    changes.push(("DTEND:20261013T140000Z", "DTEND:20261013T160000Z"));
    assert_eq!(sent, changed(GOOGLE, &changes));
    assert_reads_back(&sent, rows);
}

#[test]
fn a_duration_gives_way_to_an_end_and_new_groups_come_before_alarms() {
    let server = ICLOUD
        .replace(
            "DTEND;TZID=America/New_York:20261014T120000\r\n",
            "DURATION:PT1H\r\n",
        )
        .replace("SUMMARY:Lunch\r\n", "");
    let mut rows = rows_of(&server);
    event(&mut rows).fields.when.end = "2026-10-14T12:30:00".to_owned();
    event(&mut rows).fields.title = "Lunch".to_owned();

    let sent = send(rows.clone(), Some(&server));

    let lines = content_lines(&sent);
    assert!(!sent.contains("DURATION"));
    let at = |text: &str| lines.iter().position(|line| line == text).unwrap();
    assert!(at("DTSTART;TZID=America/New_York:20261014T110000") < at("SUMMARY:Lunch"));
    assert!(at("SUMMARY:Lunch") < at("BEGIN:VALARM"));
    assert_eq!(
        lines
            .iter()
            .filter(|line| *line == "BEGIN:VTIMEZONE")
            .count(),
        1,
        "the server's America/New_York is used"
    );
    assert_reads_back(&sent, rows);
}

#[test]
fn an_edited_exception_changes_only_its_vevent_matched_by_occurrence() {
    let mut rows = rows_of(NEXTCLOUD);
    let exception = rows
        .iter_mut()
        .find(|row| row.fields.recurrence_id.is_some())
        .unwrap();
    // The same Occurrence, obtained in another form (ADR 0012).
    exception.fields.recurrence_id = Some(RecurrenceId::obtained("2026-10-19T09:00:00"));
    exception.fields.title = "Standup (later)".to_owned();

    let sent = send(rows.clone(), Some(NEXTCLOUD));

    let changes = [
        ("SUMMARY:Standup (moved)\r\n", "SUMMARY:Standup (later)\r\n"),
        ("DTSTAMP:20261002T090000Z\r\n", STAMP),
        (
            "link\r\nEND:VEVENT",
            "link\r\nSEQUENCE:1\r\nLAST-MODIFIED:20261108T120000Z\r\nEND:VEVENT",
        ),
    ];
    assert_eq!(sent, changed(NEXTCLOUD, &changes));
    assert_reads_back(&sent, rows);
}

/// Moving a Recurring Event's time moves its Exceptions' recurrence IDs
/// by as much (ADR 0012): each still pairs with its `VEVENT`, which keeps
/// what FreeCal doesn't show and gets the new recurrence ID.
#[test]
fn an_exception_whose_recurring_event_moved_keeps_its_vevent() {
    let mut rows = rows_of(NEXTCLOUD);
    let recurring = event(&mut rows);
    recurring.fields.when.start = "2026-10-12T10:00:00".to_owned();
    recurring.fields.when.end = "2026-10-12T10:30:00".to_owned();
    recurring.fields.exdates = "2026-10-14T10:00:00".to_owned();
    let exception = rows
        .iter_mut()
        .find(|row| row.fields.recurrence_id.is_some())
        .unwrap();
    exception.fields.recurrence_id = Some(RecurrenceId::obtained("2026-10-19T10:00:00"));

    let sent = send(rows.clone(), Some(NEXTCLOUD));

    assert_eq!(sent.matches("BEGIN:VEVENT").count(), 2);
    assert!(
        sent.contains("X-NEXTCLOUD-BC-FIELD-TYPE:link\r\n"),
        "{sent}"
    );
    assert!(
        sent.contains("RECURRENCE-ID;TZID=Europe/Berlin:20261019T100000\r\n"),
        "{sent}"
    );
    assert!(!sent.contains("America/New_York:20261019T030000"), "{sent}");
    assert_reads_back(&sent, rows);
}

#[test]
fn an_exception_the_event_rows_add_gets_a_vevent_after_the_last() {
    let mut rows = rows_of(NEXTCLOUD);
    let recurring = event(&mut rows).clone();
    let mut fields = recurring.fields.clone();
    fields.rule = None;
    fields.exdates = String::new();
    fields.recurrence_id = Some(RecurrenceId::obtained("2026-10-21T09:00:00"));
    fields.when = StoredWhen {
        start: "2026-10-21T11:00:00".to_owned(),
        end: "2026-10-21T11:30:00".to_owned(),
        ..fields.when
    };
    fields.title = "Standup, Wednesday".to_owned();
    rows.push(UidEvent {
        id: EventId(99),
        fields,
        ..recurring
    });

    let sent = send(rows.clone(), Some(NEXTCLOUD));

    let added = "BEGIN:VEVENT\r\n\
        UID:5f1c2a9e-5d1b-4bd6-a1b2-nextcloud\r\n\
        DTSTAMP:20261108T120000Z\r\n\
        LAST-MODIFIED:20261108T120000Z\r\n\
        RECURRENCE-ID;TZID=Europe/Berlin:20261021T090000\r\n\
        DTSTART;TZID=Europe/Berlin:20261021T110000\r\n\
        DTEND;TZID=Europe/Berlin:20261021T113000\r\n\
        SUMMARY:Standup\\, Wednesday\r\n\
        LOCATION:Room 4\r\n\
        DESCRIPTION:Daily sync\\, short.\r\n\
        END:VEVENT\r\n";
    let expected = NEXTCLOUD.replace("END:VCALENDAR\r\n", &format!("{added}END:VCALENDAR\r\n"));
    assert_eq!(sent, expected);
    assert_reads_back(&sent, rows);
}

#[test]
fn an_exception_the_event_rows_no_longer_hold_loses_its_vevent() {
    let mut rows = rows_of(NEXTCLOUD);
    rows.retain(|row| row.fields.recurrence_id.is_none());

    let sent = send(rows.clone(), Some(NEXTCLOUD));

    let second = NEXTCLOUD.rfind("BEGIN:VEVENT").unwrap();
    let end = NEXTCLOUD.rfind("END:VCALENDAR").unwrap();
    let expected = format!("{}{}", &NEXTCLOUD[..second], &NEXTCLOUD[end..]);
    assert_eq!(sent, expected);
    assert_reads_back(&sent, rows);
}

#[test]
fn an_edited_description_goes_without_its_html() {
    let server = EVOLUTION.replace(
        "SUMMARY:Birthday\r\n",
        "SUMMARY:Birthday\r\nDESCRIPTION:Cake\r\nX-ALT-DESC;FMTTYPE=text/html:<b>Cake</b>\r\n",
    );
    let mut rows = rows_of(&server);
    event(&mut rows).fields.description = "Pie".to_owned();

    let sent = send(rows.clone(), Some(&server));

    let mut changes = revision_of(EVOLUTION);
    changes.push((
        "SUMMARY:Birthday\r\n",
        "SUMMARY:Birthday\r\nDESCRIPTION:Pie\r\n",
    ));
    assert_eq!(sent, changed(EVOLUTION, &changes));
}

#[test]
fn a_cancelled_occurrence_rewrites_the_excluded_dates() {
    let mut rows = rows_of(NEXTCLOUD);
    event(&mut rows).fields.exdates = "2026-10-14T09:00:00,2026-10-26T09:00:00".to_owned();

    let sent = send(rows.clone(), Some(NEXTCLOUD));

    let mut changes = revision_of(NEXTCLOUD);
    changes.push((
        "EXDATE;TZID=Europe/Berlin:20261014T090000\r\n",
        "EXDATE;TZID=Europe/Berlin:20261014T090000,20261026T090000\r\n",
    ));
    assert_eq!(sent, changed(NEXTCLOUD, &changes));
    assert_reads_back(&sent, rows);
}

#[test]
fn a_create_writes_a_vcalendar_with_the_vtimezones_it_needs() {
    for server in SERVER_TEXTS {
        let rows = rows_of(server);

        let sent = send(rows.clone(), None);

        assert_well_formed(&sent);
        let lines = content_lines(&sent);
        assert_eq!(
            lines[..3],
            [
                "BEGIN:VCALENDAR",
                "VERSION:2.0",
                "PRODID:-//FreeCal//FreeCal//EN"
            ]
        );
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
        assert_reads_back(&sent, rows);
    }
}

#[test]
fn a_flight_gets_the_vtimezone_of_each_end() {
    let mut rows = rows_of(EXCHANGE);
    event(&mut rows).fields.when = StoredWhen {
        all_day: false,
        start: "2026-10-15T10:00:00".to_owned(),
        end: "2026-10-15T13:00:00".to_owned(),
        start_zone: Some("Europe/Berlin".to_owned()),
        end_zone: Some("America/New_York".to_owned()),
    };

    let sent = send(rows.clone(), None);

    assert!(sent.contains("DTSTART;TZID=Europe/Berlin:20261015T100000\r\n"));
    assert!(sent.contains("DTEND;TZID=America/New_York:20261015T130000\r\n"));
    assert!(sent.contains("TZID:Europe/Berlin\r\n"));
    assert!(sent.contains("TZID:America/New_York\r\n"));
    assert_reads_back(&sent, rows);
}

#[test]
fn a_vtimezone_gives_the_offset_changes_from_the_year_of_its_first_time() {
    let mut out = Written::new();
    vtimezone(Tz::Europe__Berlin, wall("2026-10-12T09:00:00"), &mut out);
    let text = out.into_text();

    assert_well_formed(&text);
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

    let mut tokyo = Written::new();
    vtimezone(Tz::Asia__Tokyo, wall("2026-10-12T09:00:00"), &mut tokyo);
    let tokyo = content_lines(&tokyo.into_text());
    assert_eq!(
        tokyo
            .iter()
            .filter(|line| line.starts_with("BEGIN:STANDARD"))
            .count(),
        1
    );
    assert!(tokyo.contains(&"TZOFFSETTO:+0900".to_owned()));
}

#[test]
fn long_lines_are_folded_between_characters() {
    let mut rows = rows_of(EVOLUTION);
    event(&mut rows).fields.description = "Grüße aus Köln, ".repeat(20);
    event(&mut rows).fields.title = format!("{}🎂", "x".repeat(73));

    let sent = send(rows.clone(), Some(EVOLUTION));

    assert_well_formed(&sent);
    let folded = format!("SUMMARY:{}\r\n {}🎂\r\n", "x".repeat(67), "x".repeat(6));
    assert!(sent.contains(&folded), "{sent}");
    assert_reads_back(&sent, rows);
}

#[test]
fn line_ends_become_crlf_and_the_rest_is_kept() {
    let server = EVOLUTION.replace("\r\n", "\n");
    assert_eq!(send(rows_of(&server), Some(&server)), EVOLUTION);
}

#[test]
fn a_server_object_freecal_can_t_read_is_an_error() {
    let rows = || loaded(rows_of(EVOLUTION));
    for server in [
        "not iCalendar",
        "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nEND:VCALENDAR\r\n",
        "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:a\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
        "BEGIN:VCALENDAR\r\nUID:a\r\n\r\n continued\r\nEND:VCALENDAR\r\n",
        "BEGIN:VTODO\r\nEND:VTODO\r\n",
        "BEGIN:VCALENDAR\r\nX-A:a\rATTENDEE:mailto:bob@example.com\r\nEND:VCALENDAR\r\n",
        "BEGIN:VCALENDAR\r\nX-A:a\u{7}\r\nEND:VCALENDAR\r\n",
    ] {
        let sent = resource_to_send(&rows(), Some(server), now());
        assert!(
            matches!(sent, Err(CoreError::UnreadableServerObject)),
            "{server}: {sent:?}"
        );
    }
}

/// The hostile test Event (spec: "Untrusted content") in every field an
/// Event Row holds, its UID too: what is sent is valid iCalendar, holds no
/// attendee, and reads back to the text it can hold.
#[test]
fn hostile_event_rows_send_valid_icalendar() {
    let hostile: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../fixtures/hostile-event.json")).unwrap();
    let text = |key: &str| hostile[key].as_str().unwrap().to_owned();
    for server in [Some(EVOLUTION), Some(NEXTCLOUD), None] {
        let mut rows = rows_of(server.unwrap_or(EVOLUTION));
        let uid = format!("{}\r\nATTENDEE:mailto:uid@example.com", text("title"));
        for row in &mut rows {
            row.uid = uid.clone();
            row.fields.title = text("title");
            row.fields.location = text("location");
            row.fields.description = text("description");
        }
        event(&mut rows).fields.rule =
            Some("FREQ=DAILY\r\nATTENDEE:mailto:rule@example.com".to_owned());
        event(&mut rows).fields.exdates = "<b>,2026-10-21\r\nATTENDEE:x".to_owned();

        let sent = send(rows.clone(), server);

        assert_well_formed(&sent);
        assert_eq!(guests::attendees(&sent), Vec::<String>::new(), "{sent}");
        assert!(!sent.contains("FREQ=DAILY"));
        assert!(!sent.contains("EXDATE"));
        let read = read_events("sent.ics", sent.as_bytes(), Tz::UTC).unwrap();
        for event in read.events.iter().chain(&read.exceptions) {
            // An update keeps the Server Object's UID line.
            if server.is_none() {
                assert_eq!(event.uid, uid.replace("\r\n", "\n"));
            }
            assert_eq!(event.title, text("title").replace("\r\n", "\n"));
            assert_eq!(event.location, text("location"));
            assert_eq!(event.description, text("description").replace("\r\n", "\n"));
        }
    }
}
