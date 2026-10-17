mod support;

use std::io::Write;

use chrono_tz::Tz;
use freecal_core::{
    Calendar, Colour, Core, CoreError, ErrorCode, ImportFile, ImportPreview, ImportedWhen,
    MAX_IMPORT_ENTRIES, MAX_IMPORT_SIZE, PinnedZone, link_allowed,
};
use icalendar::{Component, EventLike};
use support::{Harness, date, time};

fn calendar(core: &Core, name: &str) -> Calendar {
    let local = core.list_accounts().unwrap()[0].id;
    core.create_calendar(local, name, Colour::parse("#3366cc").unwrap())
        .unwrap()
}

/// An `.ics` file with `body` between `BEGIN:VCALENDAR` and `END:VCALENDAR`,
/// one property per line.
fn ics(body: &str) -> Vec<u8> {
    format!("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Test//EN\r\n{body}END:VCALENDAR\r\n")
        .replace("\r\n", "\n")
        .replace('\n', "\r\n")
        .into_bytes()
}

fn event(uid: &str, start: &str, end: &str) -> String {
    format!("BEGIN:VEVENT\nUID:{uid}\nSUMMARY:{uid}\n{start}\n{end}\nEND:VEVENT\n")
}

fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, contents) in entries {
        writer
            .start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(contents).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn preview(core: &Core, name: &str, contents: Vec<u8>) -> Result<ImportPreview, CoreError> {
    let home = core.list_calendars().unwrap()[0].id;
    core.preview_import(&ImportFile::new(name, contents), home)
}

fn uids(preview: &ImportPreview) -> Vec<&str> {
    preview
        .events
        .iter()
        .map(|event| event.uid.as_str())
        .collect()
}

fn timed(start: &str, start_zone: Tz, end: &str, end_zone: Tz) -> ImportedWhen {
    ImportedWhen::Timed {
        start: time(start),
        start_zone,
        end: time(end),
        end_zone,
    }
}

const BERLIN: Tz = Tz::Europe__Berlin;
const NEW_YORK: Tz = Tz::America__New_York;

#[test]
fn an_ics_file_is_previewed_with_its_events_and_nothing_is_stored() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let contents = ics(&format!(
        "{}{}",
        "BEGIN:VEVENT\nUID:dentist\nSUMMARY:Dentist\nLOCATION:Main Street 1\n\
         DESCRIPTION:Bring the card\\, please\nDTSTART;TZID=America/New_York:20261005T090000\n\
         DTEND;TZID=America/New_York:20261005T103000\nEND:VEVENT\n",
        event(
            "holiday",
            "DTSTART;VALUE=DATE:20261005",
            "DTEND;VALUE=DATE:20261007"
        ),
    ));
    harness.signals.clear();

    let preview = core
        .preview_import(&ImportFile::new("events.ics", contents), home.id)
        .unwrap();

    assert_eq!(preview.calendar_id, home.id);
    assert_eq!((preview.new_events, preview.existing_events), (2, 0));
    assert_eq!(preview.pinned, []);
    let dentist = &preview.events[0];
    assert_eq!(dentist.uid, "dentist");
    assert_eq!(dentist.title, "Dentist");
    assert_eq!(dentist.location().text, "Main Street 1");
    assert_eq!(dentist.description().text, "Bring the card, please");
    assert_eq!(
        dentist.when,
        timed(
            "2026-10-05T09:00:00",
            NEW_YORK,
            "2026-10-05T10:30:00",
            NEW_YORK
        )
    );
    assert!(!dentist.zone_pinned);
    assert_eq!(
        preview.events[1].when,
        ImportedWhen::AllDay {
            start: date("2026-10-05"),
            end: date("2026-10-07"),
        }
    );
    assert_eq!(
        core.occurrences(date("2026-10-01"), date("2026-10-31"))
            .unwrap(),
        []
    );
    assert_eq!(harness.signals.received(), []);
}

#[test]
fn a_zip_of_ics_files_is_previewed_like_a_google_takeout_export() {
    let harness = Harness::new();
    let core = harness.open();
    let work = ics(&event(
        "standup",
        "DTSTART:20261005T070000Z",
        "DTEND:20261005T071500Z",
    ));
    let family = ics(&event(
        "birthday",
        "DTSTART;VALUE=DATE:20261010",
        "DTEND;VALUE=DATE:20261011",
    ));
    let contents = zip(&[
        ("Takeout/archive_browser.html", b"<html></html>"),
        ("Takeout/Calendar/", b""),
        ("Takeout/Calendar/Work.ics", &work),
        ("Takeout/Calendar/Family.ICS", &family),
    ]);

    let preview = preview(&core, "takeout.zip", contents).unwrap();

    assert_eq!(uids(&preview), ["standup", "birthday"]);
    assert_eq!(preview.new_events, 2);
}

#[test]
fn events_whose_uid_the_calendar_has_count_as_existing() {
    let harness = Harness::new();
    let core = harness.open();
    let home = calendar(&core, "Home");
    let other = calendar(&core, "Other");
    for (calendar, uid) in [(home.id, "kept"), (other.id, "elsewhere")] {
        let created = core
            .create_event(support::draft(
                calendar,
                uid,
                support::timed("2026-10-05T09:00:00", "2026-10-05T10:00:00"),
            ))
            .unwrap();
        harness.arrange_local_store(&format!(
            "UPDATE event SET uid = '{uid}' WHERE id = {}",
            created.id.0
        ));
    }
    let times = ("DTSTART:20261005T070000Z", "DTEND:20261005T080000Z");
    let contents = ics(&[
        event("kept", times.0, times.1),
        event("elsewhere", times.0, times.1),
        event("new", times.0, times.1),
        // The same Event twice, as in two files of one export, counts once.
        event("new", times.0, times.1),
    ]
    .concat());

    let preview = core
        .preview_import(&ImportFile::new("events.ics", contents), home.id)
        .unwrap();

    assert_eq!((preview.new_events, preview.existing_events), (2, 1));
    assert_eq!(uids(&preview), ["kept", "elsewhere", "new"]);
    let exists: Vec<_> = preview.events.iter().map(|event| event.exists).collect();
    assert_eq!(exists, [true, false, false]);
}

#[test]
fn an_event_without_a_uid_is_new_and_gets_one() {
    let harness = Harness::new();
    let core = harness.open();
    let contents = ics(
        "BEGIN:VEVENT\nSUMMARY:No UID\nDTSTART:20261005T070000Z\nEND:VEVENT\n\
         BEGIN:VEVENT\nSUMMARY:No UID either\nDTSTART:20261005T070000Z\nEND:VEVENT\n",
    );

    let preview = preview(&core, "events.ics", contents).unwrap();

    assert_eq!(preview.new_events, 2);
    assert!(!preview.events[0].uid.is_empty());
    assert_ne!(preview.events[0].uid, preview.events[1].uid);
}

#[test]
fn a_missing_end_follows_the_duration_or_rfc_5545s_default() {
    let harness = Harness::new();
    let core = harness.open();
    let contents = ics(&[
        event(
            "duration",
            "DTSTART;TZID=Europe/Berlin:20261005T090000",
            "DURATION:PT1H30M",
        ),
        event(
            "moment",
            "DTSTART;TZID=Europe/Berlin:20261005T090000",
            "X-NOTHING:1",
        ),
        event("day", "DTSTART;VALUE=DATE:20261005", "X-NOTHING:1"),
        event("days", "DTSTART;VALUE=DATE:20261005", "DURATION:P2D"),
    ]
    .concat());

    let preview = preview(&core, "events.ics", contents).unwrap();

    let whens: Vec<_> = preview.events.iter().map(|event| event.when).collect();
    assert_eq!(
        whens,
        [
            timed("2026-10-05T09:00:00", BERLIN, "2026-10-05T10:30:00", BERLIN),
            timed("2026-10-05T09:00:00", BERLIN, "2026-10-05T09:00:00", BERLIN),
            ImportedWhen::AllDay {
                start: date("2026-10-05"),
                end: date("2026-10-06"),
            },
            ImportedWhen::AllDay {
                start: date("2026-10-05"),
                end: date("2026-10-07"),
            },
        ]
    );
}

#[test]
fn utc_times_get_the_files_calendar_time_zone_or_stay_utc() {
    let harness = Harness::new();
    let core = harness.open();
    let times = ("DTSTART:20261005T070000Z", "DTEND:20261005T080000Z");
    let google = ics(&format!(
        "X-WR-TIMEZONE:Europe/Berlin\n{}",
        event("google", times.0, times.1)
    ));
    let plain = ics(&event("plain", times.0, times.1));

    let google = preview(&core, "google.ics", google).unwrap();
    let plain = preview(&core, "plain.ics", plain).unwrap();

    assert_eq!(
        google.events[0].when,
        timed("2026-10-05T09:00:00", BERLIN, "2026-10-05T10:00:00", BERLIN)
    );
    assert_eq!(
        plain.events[0].when,
        timed(
            "2026-10-05T07:00:00",
            Tz::UTC,
            "2026-10-05T08:00:00",
            Tz::UTC
        )
    );
    assert_eq!(google.pinned, []);
}

mod floating_times {
    use super::*;

    #[test]
    fn are_pinned_to_the_display_time_zone_and_marked() {
        let harness = Harness::new();
        harness.clock.set_time_zone("America/New_York");
        let core = harness.open();
        let contents = ics(&[
            event(
                "floating",
                "DTSTART:20261005T090000",
                "DTEND:20261005T100000",
            ),
            event("also", "DTSTART:20261006T090000", "DTEND:20261006T100000"),
            event(
                "zoned",
                "DTSTART;TZID=Europe/Berlin:20261005T090000",
                "DTEND;TZID=Europe/Berlin:20261005T100000",
            ),
        ]
        .concat());

        let preview = preview(&core, "events.ics", contents).unwrap();

        assert_eq!(
            preview.events[0].when,
            timed(
                "2026-10-05T09:00:00",
                NEW_YORK,
                "2026-10-05T10:00:00",
                NEW_YORK
            )
        );
        let pinned: Vec<_> = preview.events.iter().map(|e| e.zone_pinned).collect();
        assert_eq!(pinned, [true, true, false]);
        assert_eq!(
            preview.pinned,
            [PinnedZone {
                zone: "America/New_York".to_owned(),
                events: 2,
            }]
        );
    }

    #[test]
    fn take_the_zone_of_the_other_end_when_only_one_end_floats() {
        let harness = Harness::new();
        let core = harness.open();
        let contents = ics(&event(
            "half",
            "DTSTART;TZID=America/New_York:20261005T090000",
            "DTEND:20261005T100000",
        ));

        let preview = preview(&core, "events.ics", contents).unwrap();

        assert_eq!(
            preview.events[0].when,
            timed(
                "2026-10-05T09:00:00",
                NEW_YORK,
                "2026-10-05T10:00:00",
                NEW_YORK
            )
        );
        assert!(preview.events[0].zone_pinned);
        assert_eq!(
            preview.pinned,
            [PinnedZone {
                zone: "America/New_York".to_owned(),
                events: 1,
            }]
        );
    }
}

mod time_zone_names {
    use super::*;

    fn zone_of(core: &Core, body: &str) -> (Tz, bool) {
        let preview = preview(core, "events.ics", ics(body)).unwrap();
        let ImportedWhen::Timed { start_zone, .. } = preview.events[0].when else {
            panic!("a timed Event");
        };
        (start_zone, preview.events[0].zone_pinned)
    }

    #[test]
    fn outlooks_windows_names_are_mapped_to_iana_names() {
        let harness = Harness::new();
        let core = harness.open();

        let zone = zone_of(
            &core,
            &event(
                "outlook",
                "DTSTART;TZID=W. Europe Standard Time:20261005T090000",
                "DTEND;TZID=W. Europe Standard Time:20261005T100000",
            ),
        );

        assert_eq!(zone, (BERLIN, false));
    }

    #[test]
    fn a_path_ending_in_an_iana_name_is_that_name() {
        let harness = Harness::new();
        let core = harness.open();

        let zone = zone_of(
            &core,
            &event(
                "mozilla",
                "DTSTART;TZID=/mozilla.org/20050126_1/America/New_York:20261005T090000",
                "DTEND;TZID=/mozilla.org/20050126_1/America/New_York:20261005T100000",
            ),
        );

        assert_eq!(zone, (NEW_YORK, false));
    }

    #[test]
    fn a_custom_vtimezone_is_mapped_through_its_location() {
        let harness = Harness::new();
        let core = harness.open();

        let zone = zone_of(
            &core,
            &format!(
                "BEGIN:VTIMEZONE\nTZID:Custom Berlin\nX-LIC-LOCATION:Europe/Berlin\n\
                 BEGIN:STANDARD\nDTSTART:19701025T030000\nTZOFFSETFROM:+0200\n\
                 TZOFFSETTO:+0100\nEND:STANDARD\nEND:VTIMEZONE\n{}",
                event(
                    "custom",
                    "DTSTART;TZID=Custom Berlin:20261005T090000",
                    "DTEND;TZID=Custom Berlin:20261005T100000",
                )
            ),
        );

        assert_eq!(zone, (BERLIN, false));
    }

    #[test]
    fn a_name_without_a_mapping_is_pinned_like_a_floating_time() {
        let harness = Harness::new();
        harness.clock.set_time_zone("America/New_York");
        let core = harness.open();

        let zone = zone_of(
            &core,
            &event(
                "unknown",
                "DTSTART;TZID=\"(UTC+01:00) Somewhere\":20261005T090000",
                "DTEND;TZID=\"(UTC+01:00) Somewhere\":20261005T100000",
            ),
        );

        assert_eq!(zone, (NEW_YORK, true));
    }
}

mod failures {
    use super::*;

    fn failure(name: &str, contents: Vec<u8>) -> ErrorCode {
        let harness = Harness::new();
        let core = harness.open();
        preview(&core, name, contents).unwrap_err().code()
    }

    #[test]
    fn a_file_that_cant_be_read_is_named() {
        let directory = tempfile::TempDir::new().unwrap();

        let error = ImportFile::read(&directory.path().join("gone.ics")).unwrap_err();

        assert_eq!(
            error.code(),
            ErrorCode::ImportUnreadable {
                file: "gone.ics".to_owned()
            }
        );
    }

    #[test]
    fn a_file_that_can_be_read_is_previewed_under_its_name() {
        let directory = tempfile::TempDir::new().unwrap();
        let path = directory.path().join("events.ics");
        std::fs::write(&path, ics("")).unwrap();

        let file = ImportFile::read(&path).unwrap();

        assert_eq!(file.name, "events.ics");
        assert_eq!(file.contents, ics(""));
    }

    #[test]
    fn a_csv_file_is_not_a_calendar() {
        let csv = b"Subject,Start Date,Start Time\nDentist,10/05/2026,09:00\n".to_vec();

        assert_eq!(
            failure("events.csv", csv),
            ErrorCode::ImportNotCalendar {
                file: "events.csv".to_owned()
            }
        );
    }

    #[test]
    fn a_broken_ics_file_is_not_a_calendar() {
        for broken in [
            b"BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\n".to_vec(),
            b"\xff\xfe\x00binary".to_vec(),
            ics("BEGIN:VEVENT\nUID:timeless\nSUMMARY:No start\nEND:VEVENT\n"),
        ] {
            assert_eq!(
                failure("broken.ics", broken),
                ErrorCode::ImportNotCalendar {
                    file: "broken.ics".to_owned()
                }
            );
        }
    }

    #[test]
    fn a_broken_ics_file_in_a_zip_is_named_inside_the_zip() {
        let contents = zip(&[
            ("Calendar/Good.ics", &ics("")),
            ("Calendar/Bad.ics", b"BEGIN:VCALENDAR\r\n"),
        ]);

        assert_eq!(
            failure("takeout.zip", contents),
            ErrorCode::ImportNotCalendar {
                file: "takeout.zip/Calendar/Bad.ics".to_owned()
            }
        );
    }

    #[test]
    fn an_ics_file_in_a_zip_that_cant_be_unpacked_is_named_inside_the_zip() {
        let stored = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        writer.start_file("Calendar/Work.ics", stored).unwrap();
        writer.write_all(&ics("")).unwrap();
        let mut contents = writer.finish().unwrap().into_inner();
        // Damages the entry's data, so that its checksum no longer matches.
        let at = contents
            .windows(7)
            .position(|window| window == b"VERSION")
            .unwrap();
        contents[at] = b'X';

        assert_eq!(
            failure("takeout.zip", contents),
            ErrorCode::ImportUnreadable {
                file: "takeout.zip/Calendar/Work.ics".to_owned()
            }
        );
    }

    #[test]
    fn a_file_too_large_is_refused_when_previewed() {
        let huge = vec![b' '; MAX_IMPORT_SIZE + 1];

        assert_eq!(
            failure("huge.ics", huge),
            ErrorCode::ImportTooLarge {
                file: "huge.ics".to_owned()
            }
        );
    }

    #[test]
    fn a_zip_without_ics_files_is_named() {
        let contents = zip(&[("Takeout/archive_browser.html", b"<html></html>")]);

        assert_eq!(
            failure("takeout.zip", contents),
            ErrorCode::ImportZipWithoutCalendars {
                file: "takeout.zip".to_owned()
            }
        );
    }

    #[test]
    fn a_damaged_zip_is_unreadable() {
        let mut contents = zip(&[("Calendar/Work.ics", &ics(""))]);
        contents.truncate(contents.len() / 2);

        assert_eq!(
            failure("takeout.zip", contents),
            ErrorCode::ImportUnreadable {
                file: "takeout.zip".to_owned()
            }
        );
    }

    #[test]
    fn a_zip_with_too_many_entries_is_refused() {
        let names: Vec<String> = (0..=MAX_IMPORT_ENTRIES)
            .map(|n| format!("{n}.txt"))
            .collect();
        let entries: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), &b""[..])).collect();

        assert_eq!(
            failure("many.zip", zip(&entries)),
            ErrorCode::ImportTooLarge {
                file: "many.zip".to_owned()
            }
        );
    }

    #[test]
    fn a_zip_that_unpacks_too_large_is_refused_whatever_its_entries_claim() {
        // Compresses to almost nothing, like a zip bomb.
        let huge = vec![b' '; MAX_IMPORT_SIZE / 2 + 1];
        let contents = zip(&[("a.ics", &huge), ("b.ics", &huge)]);

        assert_eq!(
            failure("bomb.zip", contents),
            ErrorCode::ImportTooLarge {
                file: "bomb.zip".to_owned()
            }
        );
    }

    #[test]
    fn a_file_too_large_is_refused_when_read() {
        let directory = tempfile::TempDir::new().unwrap();
        let path = directory.path().join("huge.ics");
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(MAX_IMPORT_SIZE as u64 + 1).unwrap();

        let error = ImportFile::read(&path).unwrap_err();

        assert_eq!(
            error.code(),
            ErrorCode::ImportTooLarge {
                file: "huge.ics".to_owned()
            }
        );
    }

    #[test]
    fn a_calendar_that_doesnt_exist_is_refused() {
        let harness = Harness::new();
        let core = harness.open();
        let gone = calendar(&core, "Gone");
        core.delete_calendar(gone.id).unwrap();

        let error = core
            .preview_import(&ImportFile::new("events.ics", ics("")), gone.id)
            .unwrap_err();

        assert!(matches!(error, CoreError::CalendarNotFound(id) if id == gone.id));
    }
}

mod hostile {
    use super::*;

    fn fixture() -> serde_json::Value {
        serde_json::from_str(include_str!("../../fixtures/hostile-event.json")).unwrap()
    }

    fn field(name: &str) -> String {
        fixture()[name].as_str().unwrap().to_owned()
    }

    /// The hostile test Event as an `.ics` file, written by the library.
    fn hostile_ics() -> Vec<u8> {
        let event = icalendar::Event::new()
            .uid("hostile")
            .summary(&field("title"))
            .location(&field("location"))
            .description(&field("description"))
            .starts(time("2026-10-05T09:00:00"))
            .ends(time("2026-10-05T10:00:00"))
            .done();
        icalendar::Calendar::new()
            .push(event)
            .done()
            .to_string()
            .into_bytes()
    }

    #[test]
    fn previewing_it_runs_nothing_and_its_html_description_becomes_text_and_links() {
        let harness = Harness::new();
        let core = harness.open();

        let preview = preview(&core, &field("location"), hostile_ics()).unwrap();

        let event = &preview.events[0];
        assert_eq!(event.title, field("title"));
        assert_eq!(event.location, field("location"));
        assert_eq!(event.description, field("description"));
        assert_eq!(
            serde_json::to_value(event.description()).unwrap(),
            fixture()["expected"]["html"]
        );
        assert_eq!(
            serde_json::to_value(event.location()).unwrap(),
            fixture()["expected"]["location"]
        );
        for piece in event
            .description()
            .paragraphs
            .iter()
            .chain(&event.location().paragraphs)
            .flatten()
        {
            if let freecal_core::Piece::Link { url, .. } = piece {
                assert!(link_allowed(url), "{url:?} must not be a link");
            }
        }
        // Its `\r\nATTENDEE:` stayed inside the text: no attendee was read.
        assert_eq!(preview.new_events, 1);
    }

    #[test]
    fn its_file_name_is_given_back_unchanged_in_an_error() {
        let harness = Harness::new();
        let core = harness.open();

        let error = preview(&core, &field("fileName"), b"not a calendar".to_vec()).unwrap_err();

        assert_eq!(
            error.code(),
            ErrorCode::ImportNotCalendar {
                file: field("fileName")
            }
        );
    }

    #[test]
    fn a_plain_text_description_stays_plain_text() {
        let harness = Harness::new();
        let core = harness.open();
        let contents = ics(
            "BEGIN:VEVENT\nUID:plain\nDESCRIPTION:if a < b and c > d\\nthen https://example.org\n\
             DTSTART:20261005T070000Z\nEND:VEVENT\n",
        );

        let preview = preview(&core, "events.ics", contents).unwrap();

        let description = preview.events[0].description();
        assert_eq!(
            description.text,
            "if a < b and c > d\nthen https://example.org"
        );
    }
}
