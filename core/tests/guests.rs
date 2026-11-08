//! Events with guests (spec: "Attendees", US-46): an Event whose
//! Corresponding Server Object has an attendee other than the user is
//! read-only, as an Event of a Read-only Calendar is, whatever the
//! Transaction Origin but an Incoming Sync.

mod support;

use freecal_core::{Colour, CoreError, ExistingEvents, ImportFile, Scope, StartedBy};
use support::radicale::{NewCalendar, Radicale, User};
use support::{OnServer, date, draft, ics, on, put_events, timed};

/// An attendee who isn't the user.
const GUEST: &str = "ATTENDEE;CN=Bob;PARTSTAT=ACCEPTED:mailto:bob@example.com\n";

/// A one-hour Event on 12 October 2026 whose title is its UID, with the
/// property lines `more`.
fn meeting(uid: &str, more: &str) -> Vec<u8> {
    ics(&format!(
        "BEGIN:VEVENT\nUID:{uid}\nSUMMARY:{uid}\n\
         DTSTART:20261012T090000Z\nDTEND:20261012T100000Z\n{more}END:VEVENT\n"
    ))
}

/// A weekly Event from 5 October 2026 whose title is its UID, with an
/// Exception on 12 October that has the property lines `exception`.
fn weekly_with_exception(uid: &str, exception: &str) -> Vec<u8> {
    ics(&format!(
        "BEGIN:VEVENT\nUID:{uid}\nSUMMARY:{uid}\n\
         DTSTART:20261005T090000Z\nDTEND:20261005T100000Z\nRRULE:FREQ=WEEKLY;COUNT=4\n\
         END:VEVENT\n\
         BEGIN:VEVENT\nUID:{uid}\nSUMMARY:{uid} moved\nRECURRENCE-ID:20261012T090000Z\n\
         DTSTART:20261012T110000Z\nDTEND:20261012T120000Z\n{exception}END:VEVENT\n"
    ))
}

impl OnServer {
    /// `user`'s writable Calendar with `events` (name, iCalendar object)
    /// on the server first.
    fn of(user: User, events: &[(&str, Vec<u8>)]) -> Self {
        Self::arranged(
            user,
            NewCalendar::default(),
            Radicale::create_calendar,
            None,
            put_events(events),
        )
        .writable()
    }

    fn writable_with(events: &[(&str, Vec<u8>)]) -> Self {
        Self::of(Radicale::shared().user(), events)
    }

    fn sync(&self) {
        self.core
            .sync_account(self.account, StartedBy::FreeCal)
            .unwrap();
    }

    fn has_guests(&self, title: &str) -> bool {
        self.event(title).has_guests
    }
}

fn refused_for_guests<T: std::fmt::Debug>(result: Result<T, CoreError>) {
    assert!(
        matches!(result, Err(CoreError::EventHasGuests)),
        "refused as it has guests: {result:?}"
    );
}

#[test]
fn an_event_with_an_attendee_other_than_the_user_has_guests() {
    let server = OnServer::writable_with(&[
        ("meeting", meeting("meeting", GUEST)),
        ("alone", meeting("alone", "")),
    ]);
    assert!(server.has_guests("meeting"));
    assert!(!server.has_guests("alone"));
    let occurrences = server
        .core
        .occurrences(date("2026-10-12"), date("2026-10-13"))
        .unwrap();
    let mut guests: Vec<(&str, bool)> = occurrences
        .iter()
        .map(|occurrence| (occurrence.title.as_str(), occurrence.has_guests))
        .collect();
    guests.sort();
    assert_eq!(guests, [("alone", false), ("meeting", true)]);
}

#[test]
fn the_core_refuses_edits_moves_and_deletes_of_an_event_with_guests() {
    let server = OnServer::writable_with(&[("meeting", meeting("meeting", GUEST))]);
    let core = &server.core;
    let id = server.event("meeting").id;
    let when = timed("2026-10-13T09:00:00", "2026-10-13T10:00:00");
    refused_for_guests(core.edit_event(id, draft(server.work, "renamed", when)));
    refused_for_guests(core.reschedule_event(id, when));
    refused_for_guests(core.delete_event(id));
    // Moving it to another Calendar is an edit too.
    let local = core.list_accounts().unwrap()[0].id;
    let home = core
        .create_calendar(local, "Home", Colour::parse("#ad1457").unwrap())
        .unwrap()
        .id;
    refused_for_guests(core.edit_event(id, draft(home, "meeting", when)));
    assert_eq!(on(core, "2026-10-12").title, "meeting");
    assert!(core.event(id).unwrap().has_guests);
}

#[test]
fn an_import_doesn_t_update_an_event_with_guests() {
    let server = OnServer::writable_with(&[("meeting", meeting("meeting", GUEST))]);
    let imported = ics("BEGIN:VEVENT\nUID:meeting\nSUMMARY:imported\n\
         DTSTART:20261012T090000Z\nDTEND:20261012T100000Z\nEND:VEVENT\n");
    let file = ImportFile::new("meeting.ics", imported);
    let preview = server.core.preview_import(&file, server.work).unwrap();
    refused_for_guests(server.core.run_import(&preview, ExistingEvents::Update));
    assert_eq!(on(&server.core, "2026-10-12").title, "meeting");
}

#[test]
fn guests_of_one_exception_make_its_whole_recurring_event_read_only() {
    let server = OnServer::writable_with(&[("weekly", weekly_with_exception("weekly", GUEST))]);
    let core = &server.core;
    assert!(server.has_guests("weekly"));
    assert!(server.has_guests("weekly moved"));
    let occurrences = core
        .occurrences(date("2026-10-01"), date("2026-11-01"))
        .unwrap();
    assert_eq!(occurrences.len(), 4);
    assert!(occurrences.iter().all(|occurrence| occurrence.has_guests));
    // An Occurrence without guests of its own is still part of it.
    let first = on(core, "2026-10-05");
    let recurrence_id = first.recurrence_id.as_deref().unwrap();
    let when = timed("2026-10-05T10:00:00", "2026-10-05T11:00:00");
    refused_for_guests(core.reschedule_occurrence(
        first.event_id,
        recurrence_id,
        Scope::This,
        false,
        when,
    ));
    refused_for_guests(core.delete_occurrence(first.event_id, recurrence_id));
}

/// Asserts that the Event "mine" has no guests and can be changed.
fn mine_is_writable(server: &OnServer) {
    assert!(!server.has_guests("mine"));
    let id = server.event("mine").id;
    let when = timed("2026-10-12T09:00:00", "2026-10-12T10:00:00");
    server
        .core
        .edit_event(id, draft(server.work, "renamed", when))
        .unwrap();
}

#[test]
fn an_attendee_with_the_username_as_address_is_no_guest() {
    let user = Radicale::shared().user_with_address();
    // Addresses compare whatever their case and that of `mailto:`.
    let user_attendees = format!(
        "ORGANIZER:mailto:{name}\nATTENDEE:MAILTO:{upper}\n",
        name = user.name,
        upper = user.name.to_uppercase(),
    );
    mine_is_writable(&OnServer::of(
        user,
        &[("mine", meeting("mine", &user_attendees))],
    ));
}

#[test]
fn an_attendee_with_an_address_the_server_gives_for_the_user_is_no_guest() {
    let user = Radicale::shared().user();
    // Radicale gives the user's principal as their address.
    let user_attendees = format!("ATTENDEE:/{}/\n", user.name);
    mine_is_writable(&OnServer::of(
        user,
        &[("mine", meeting("mine", &user_attendees))],
    ));
}

#[test]
fn an_incoming_sync_still_changes_an_event_with_guests() {
    let server = OnServer::writable_with(&[("meeting", meeting("meeting", GUEST))]);
    let renamed = String::from_utf8(meeting("meeting", GUEST))
        .unwrap()
        .replace("SUMMARY:meeting", "SUMMARY:renamed")
        .into_bytes();
    server
        .radicale
        .put_event(&server.user, &server.url, "meeting", renamed);
    server.sync();
    assert!(server.has_guests("renamed"));
    // Once the server drops the guest, the Event is the user's to change.
    server
        .radicale
        .put_event(&server.user, &server.url, "meeting", meeting("meeting", ""));
    server.sync();
    assert!(!server.has_guests("meeting"));
    let id = server.event("meeting").id;
    server.core.delete_event(id).unwrap();
}

/// The hostile test Event: its title holds `\r\nATTENDEE:…`, which is text
/// of its `SUMMARY`, escaped, and no attendee.
#[test]
fn the_hostile_event_s_title_names_no_guest() {
    use icalendar::{Component, EventLike};
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/hostile-event.json")).unwrap();
    let field = |name: &str| fixture[name].as_str().unwrap().to_owned();
    let hostile = icalendar::Calendar::new()
        .push(
            icalendar::Event::new()
                .uid("hostile")
                .summary(&field("title"))
                .location(&field("location"))
                .description(&field("description"))
                .starts(support::time("2026-10-12T09:00:00"))
                .ends(support::time("2026-10-12T10:00:00"))
                .done(),
        )
        .done()
        .to_string()
        .into_bytes();
    let server = OnServer::writable_with(&[("hostile", hostile)]);
    let occurrence = on(&server.core, "2026-10-12");
    assert!(!occurrence.has_guests);
    assert!(!server.core.event(occurrence.event_id).unwrap().has_guests);
}
