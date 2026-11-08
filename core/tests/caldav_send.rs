//! The CalDAV Sender (ticket 127): every sync of an Account sends the
//! Unsent Changes of its writable CalDAV Calendars, with conditional
//! requests, before their Incoming Syncs. Tested against Radicale.

mod support;

use freecal_core::{
    AccountId, CalDavLogin, CalendarId, Core, Event, EventDraft, JournalTransactionId,
    ServerObject, StartedBy,
};
use support::radicale::{NewCalendar, Radicale, RewritingProxy, User};
use support::{Harness, calendar_sync_status, date, draft, ics, timed, user_login};

/// A user on Radicale with a writable Calendar "Work", FreeCal's Account
/// for it and the Calendar's First Sync done.
struct OnServer {
    radicale: &'static Radicale,
    user: User,
    /// The Calendar's URL on the server.
    url: String,
    harness: Harness,
    core: Core,
    account: AccountId,
    work: CalendarId,
}

impl OnServer {
    /// With `events` (name, iCalendar object) on the server first.
    fn with(events: &[(&str, Vec<u8>)]) -> Self {
        Self::arranged(None, events)
    }

    /// Like [`OnServer::with`], reaching the server through `proxy`.
    fn through(proxy: &RewritingProxy, events: &[(&str, Vec<u8>)]) -> Self {
        Self::arranged(Some(&proxy.url), events)
    }

    fn arranged(through: Option<&str>, events: &[(&str, Vec<u8>)]) -> Self {
        let radicale = Radicale::shared();
        let user = radicale.user();
        let url = radicale.create_calendar(
            &user,
            NewCalendar {
                name: Some("Work"),
                ..NewCalendar::default()
            },
        );
        for (name, ics) in events {
            radicale.put_event(&user, &url, name, ics.clone());
        }
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let server = through.map_or_else(|| radicale.url(), str::to_owned);
        let account = core
            .add_caldav_account(&CalDavLogin {
                url: server.clone(),
                ..user_login(radicale, &user)
            })
            .unwrap()
            .account
            .id;
        let work = core
            .add_calendars(account, &[url.replace(&radicale.url(), &server)])
            .unwrap()[0]
            .id;
        core.set_calendar_read_only(work, false).unwrap();
        Self {
            radicale,
            user,
            url,
            harness,
            core,
            account,
            work,
        }
    }

    /// A second writable Calendar "Home" of the same Account, synced.
    fn add_home(&self) -> (CalendarId, String) {
        let url = self.radicale.create_calendar(
            &self.user,
            NewCalendar {
                name: Some("Home"),
                ..NewCalendar::default()
            },
        );
        let home = self
            .core
            .add_calendars(self.account, std::slice::from_ref(&url))
            .unwrap()[0]
            .id;
        self.core.set_calendar_read_only(home, false).unwrap();
        (home, url)
    }

    /// Syncs the Account, and returns the Journal Transactions its
    /// Calendars' syncs recorded.
    fn sync(&self) -> Vec<JournalTransactionId> {
        self.core
            .sync_account(self.account, StartedBy::FreeCal)
            .unwrap()
            .into_iter()
            .filter_map(|changed| changed.journal_transaction)
            .collect()
    }

    /// The Calendar's Event titled `title`, in October 2026.
    fn event(&self, title: &str) -> Event {
        let occurrence = self
            .core
            .occurrences(date("2026-10-01"), date("2026-11-01"))
            .unwrap()
            .into_iter()
            .find(|occurrence| occurrence.title == title)
            .unwrap_or_else(|| panic!("an Event titled {title:?}"));
        self.core.event(occurrence.event_id).unwrap()
    }

    /// Renames the Event titled `title` to `new_title`, moving it to the
    /// Calendar `calendar_id`.
    fn edit(&self, title: &str, new_title: &str, calendar_id: CalendarId) {
        let event = self.event(title);
        self.core
            .edit_event(
                event.id,
                EventDraft {
                    title: new_title.to_owned(),
                    ..draft(calendar_id, "", event.when)
                },
            )
            .unwrap();
    }

    /// The Server Object the Event titled `title` is in, as FreeCal holds it.
    fn held(&self, title: &str) -> Option<ServerObject> {
        self.core
            .original_server_data(self.event(title).id)
            .unwrap()
    }

    /// The ETag and text the server serves the resource at `href` with, a
    /// resource of the Calendar at `calendar`.
    fn served(&self, calendar: &str, href: &str) -> (String, String) {
        let name = href
            .rsplit('/')
            .next()
            .and_then(|name| name.strip_suffix(".ics"))
            .expect("an href ending in .ics");
        self.radicale.get_event(&self.user, calendar, name)
    }

    /// The text of the resource `name` of the Calendar "Work" on the server.
    fn on_server(&self, name: &str) -> String {
        self.radicale.get_event(&self.user, &self.url, name).1
    }

    /// The Unsent Changes of the Calendar `id`.
    fn unsent(&self, id: CalendarId) -> u32 {
        calendar_sync_status(&self.core, id).report.unsent_changes
    }
}

/// "Dentist" on 5 October 2026, 9:00 to 10:00 in Berlin.
fn dentist(calendar_id: CalendarId) -> EventDraft {
    draft(
        calendar_id,
        "Dentist",
        timed("2026-10-05T09:00:00", "2026-10-05T10:00:00"),
    )
}

/// A server Event with properties FreeCal doesn't show.
fn standup(title: &str) -> Vec<u8> {
    ics(&format!(
        "BEGIN:VEVENT\nUID:standup\nSUMMARY:{title}\nDTSTART:20261005T070000Z\n\
         DTEND:20261005T073000Z\nX-CUSTOM;X-PARAM=1:kept as it is\nEND:VEVENT\n"
    ))
}

#[test]
fn a_created_event_is_created_on_the_server_and_its_unsent_change_goes() {
    let on_server = OnServer::with(&[]);
    on_server
        .core
        .create_event(dentist(on_server.work))
        .unwrap();
    assert_eq!(on_server.unsent(on_server.work), 1);

    on_server.sync();

    let held = on_server.held("Dentist").expect("a Server Object");
    let (etag, text) = on_server.served(&on_server.url, &held.href);
    assert!(text.contains("SUMMARY:Dentist"), "{text}");
    assert_eq!(held.etag, Some(etag));
    assert_eq!(on_server.unsent(on_server.work), 0);
}

#[test]
fn an_edit_is_written_over_the_servers_version_keeping_what_freecal_doesnt_show() {
    let on_server = OnServer::with(&[("standup", standup("Standup"))]);
    on_server.edit("Standup", "Standup moved", on_server.work);

    on_server.sync();

    let text = on_server.on_server("standup");
    assert!(text.contains("SUMMARY:Standup moved"), "{text}");
    assert!(text.contains("X-CUSTOM;X-PARAM=1:kept as it is"), "{text}");
    let held = on_server.held("Standup moved").unwrap();
    assert!(held.href.ends_with("/standup.ics"));
    assert_eq!(
        held.etag,
        Some(on_server.served(&on_server.url, &held.href).0)
    );
    assert_eq!(on_server.unsent(on_server.work), 0);
}

#[test]
fn a_deleted_event_is_deleted_on_the_server() {
    let on_server = OnServer::with(&[("standup", standup("Standup"))]);
    on_server
        .core
        .delete_event(on_server.event("Standup").id)
        .unwrap();

    on_server.sync();

    let status = on_server
        .radicale
        .status_of(&on_server.user, &format!("{}standup.ics", on_server.url));
    assert_eq!(status, 404);
    assert_eq!(on_server.unsent(on_server.work), 0);
}

#[test]
fn an_event_moved_to_another_calendar_moves_on_the_server() {
    let on_server = OnServer::with(&[("standup", standup("Standup"))]);
    let (home, home_url) = on_server.add_home();
    on_server.edit("Standup", "Standup", home);

    on_server.sync();

    let status = on_server
        .radicale
        .status_of(&on_server.user, &format!("{}standup.ics", on_server.url));
    assert_eq!(status, 404);
    let held = on_server.held("Standup").unwrap();
    assert!(
        held.href
            .starts_with(&home_url.replace(&on_server.radicale.url(), "/"))
    );
    let text = on_server.served(&home_url, &held.href).1;
    assert!(text.contains("UID:standup"), "{text}");
    assert_eq!(on_server.unsent(on_server.work), 0);
    assert_eq!(on_server.unsent(home), 0);
}

/// ADR 0014: a server change met while sending is the Conflict code's
/// (tickets 20 and 21), so the Sender writes nothing over it.
#[test]
fn an_edit_is_not_written_over_a_newer_server_version() {
    let on_server = OnServer::with(&[("standup", standup("Standup"))]);
    on_server.radicale.put_event(
        &on_server.user,
        &on_server.url,
        "standup",
        standup("Standup on the server"),
    );
    on_server.edit("Standup", "Standup in FreeCal", on_server.work);

    on_server.sync();

    assert!(
        on_server
            .on_server("standup")
            .contains("SUMMARY:Standup on the server")
    );
}

/// ADR 0015's amendment: with no stored ETag, an update can't be
/// conditional, so it waits.
#[test]
fn an_update_without_a_stored_etag_waits() {
    let on_server = OnServer::with(&[("standup", standup("Standup"))]);
    on_server.edit("Standup", "Standup moved", on_server.work);
    on_server
        .harness
        .arrange_local_store("UPDATE caldav_server_object SET etag = NULL");

    on_server.sync();

    assert!(
        on_server
            .on_server("standup")
            .contains("SUMMARY:Standup\r\n")
    );
}

#[test]
fn a_delete_without_a_stored_etag_waits() {
    let on_server = OnServer::with(&[("standup", standup("Standup"))]);
    on_server
        .core
        .delete_event(on_server.event("Standup").id)
        .unwrap();
    on_server
        .harness
        .arrange_local_store("UPDATE caldav_server_object SET etag = NULL");

    on_server.sync();

    assert!(on_server.on_server("standup").contains("SUMMARY:Standup"));
}

/// RFC 4791 §5.3.4 lets a server that changed the text it stores answer
/// without an ETag; then the Incoming Sync fetches what it stored, in the
/// same sync, as the Outgoing Sync runs first.
#[test]
fn a_write_answered_without_an_etag_leaves_the_incoming_sync_to_fetch_the_servers_text() {
    let proxy = RewritingProxy::passthrough(Radicale::shared());
    proxy.drop_etags_of_writes();
    let on_server = OnServer::through(&proxy, &[]);
    on_server
        .core
        .create_event(dentist(on_server.work))
        .unwrap();

    on_server.sync();

    let held = on_server.held("Dentist").unwrap();
    let (etag, text) = on_server.served(&on_server.url, &held.href);
    assert_eq!(held.etag, Some(etag));
    assert_eq!(held.data, text, "the text as the server stored it");
    assert_eq!(on_server.unsent(on_server.work), 0);
}

/// An Unsent Change about an Event created and deleted before a sync
/// matches neither Event Rows nor a Server Object, so it is dropped.
#[test]
fn an_unsent_change_matching_nothing_is_dropped() {
    let on_server = OnServer::with(&[]);
    let created = on_server
        .core
        .create_event(dentist(on_server.work))
        .unwrap();
    on_server.core.delete_event(created.value.id).unwrap();
    assert_eq!(on_server.unsent(on_server.work), 1);

    on_server.sync();

    assert_eq!(on_server.unsent(on_server.work), 0);
}

#[test]
fn unsent_changes_of_a_read_only_calendar_wait() {
    let on_server = OnServer::with(&[("standup", standup("Standup"))]);
    on_server
        .core
        .delete_event(on_server.event("Standup").id)
        .unwrap();
    on_server
        .core
        .set_calendar_read_only(on_server.work, true)
        .unwrap();

    on_server.sync();

    assert!(on_server.on_server("standup").contains("SUMMARY:Standup"));
    assert_eq!(on_server.unsent(on_server.work), 1);
}

/// Ticket 69: every change is in the Change Journal as it is made, before
/// it is sent; sending it records nothing more.
#[test]
fn a_sent_change_is_in_the_change_journal_and_sending_records_nothing() {
    let on_server = OnServer::with(&[]);
    let created = on_server
        .core
        .create_event(dentist(on_server.work))
        .unwrap();
    let creation = created
        .journal_transaction
        .expect("recorded as it was made");

    assert_eq!(on_server.sync(), []);

    let href = on_server.held("Dentist").expect("sent").href;
    // The Change Journal holds the Event's previous Event Version, none,
    // so undoing the creation deletes it, and that is sent too.
    on_server.core.undo_journal_transaction(creation).unwrap();
    assert_eq!(on_server.sync(), []);
    let status = on_server.radicale.status_of(
        &on_server.user,
        &format!(
            "{}{}",
            on_server.radicale.url(),
            href.trim_start_matches('/')
        ),
    );
    assert_eq!(status, 404);
    assert_eq!(on_server.unsent(on_server.work), 0);
}

/// Ticket 69, for an edit and a delete: the Change Journal holds each
/// Event's previous Event Version, so undoing a sent change gives it back,
/// and the undo is sent like any change.
#[test]
fn a_sent_edit_and_delete_are_in_the_change_journal() {
    let on_server = OnServer::with(&[("standup", standup("Standup"))]);
    let edited = on_server
        .core
        .edit_event(
            on_server.event("Standup").id,
            EventDraft {
                title: "Standup moved".to_owned(),
                ..draft(on_server.work, "", on_server.event("Standup").when)
            },
        )
        .unwrap()
        .journal_transaction
        .unwrap();
    assert_eq!(on_server.sync(), []);
    assert!(
        on_server
            .on_server("standup")
            .contains("SUMMARY:Standup moved")
    );

    on_server.core.undo_journal_transaction(edited).unwrap();
    assert_eq!(on_server.sync(), []);
    assert!(
        on_server
            .on_server("standup")
            .contains("SUMMARY:Standup\r\n")
    );

    let deleted = on_server
        .core
        .delete_event(on_server.event("Standup").id)
        .unwrap()
        .journal_transaction
        .unwrap();
    assert_eq!(on_server.sync(), []);
    on_server.core.undo_journal_transaction(deleted).unwrap();
    assert_eq!(on_server.sync(), []);

    let held = on_server.held("Standup").expect("created again");
    let text = on_server.served(&on_server.url, &held.href).1;
    assert!(text.contains("SUMMARY:Standup\r\n"), "{text}");
    assert_eq!(on_server.unsent(on_server.work), 0);
}
