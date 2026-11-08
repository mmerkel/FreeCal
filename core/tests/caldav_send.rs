//! The CalDAV Sender (ticket 127): every sync of an Account sends the
//! Unsent Changes of its writable CalDAV Calendars, with conditional
//! requests, before their Incoming Syncs. A change is sent right after it
//! is made, and a server refusing writes is a Sync Error of the Calendar
//! (ticket 124). Tested against Radicale.

mod support;

use freecal_core::{
    AccountId, CalDavLogin, CalendarId, Core, ErrorCode, Event, EventDraft, JournalTransactionId,
    ServerObject, Signal, StartedBy, SyncStatus,
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

/// Puts `events` (name, iCalendar object) on the server, for
/// [`OnServer::arranged`].
fn put(events: &[(&str, Vec<u8>)]) -> impl FnOnce(&Radicale, &User, &str) {
    move |radicale, user, url| {
        for (name, ics) in events {
            radicale.put_event(user, url, name, ics.clone());
        }
    }
}

impl OnServer {
    /// With `events` (name, iCalendar object) on the server first.
    fn with(events: &[(&str, Vec<u8>)]) -> Self {
        Self::arranged(None, put(events))
    }

    /// Like [`OnServer::with`], writing `files` straight into the server's
    /// storage, as Radicale refuses a second resource holding one UID.
    fn with_files(files: &[(&str, Vec<u8>)]) -> Self {
        Self::arranged(None, |radicale, _, url| {
            for (name, ics) in files {
                radicale.write_event_file(url, name, ics.clone());
            }
        })
    }

    /// Like [`OnServer::with`], reaching the server through `proxy`.
    fn through(proxy: &RewritingProxy, events: &[(&str, Vec<u8>)]) -> Self {
        Self::arranged(Some(&proxy.url), put(events))
    }

    /// The Account and its Calendar once `arrange` put what the Calendar
    /// holds on the server, which FreeCal reaches through the proxy at
    /// `through` if given.
    fn arranged(through: Option<&str>, arrange: impl FnOnce(&Radicale, &User, &str)) -> Self {
        Self::set_up(through, Radicale::create_calendar, arrange)
    }

    /// Like [`OnServer::through`] with no Events, but the server lets the
    /// user only read the Calendar; FreeCal takes it as writable while
    /// `proxy` says so.
    fn read_only_through(proxy: &RewritingProxy) -> Self {
        Self::set_up(
            Some(&proxy.url),
            Radicale::create_read_only_calendar,
            |_, _, _| {},
        )
    }

    /// Like [`OnServer::arranged`], with the Calendar made by `create`.
    fn set_up(
        through: Option<&str>,
        create: fn(&Radicale, &User, NewCalendar) -> String,
        arrange: impl FnOnce(&Radicale, &User, &str),
    ) -> Self {
        let radicale = Radicale::shared();
        let user = radicale.user();
        let url = create(
            radicale,
            &user,
            NewCalendar {
                name: Some("Work"),
                ..NewCalendar::default()
            },
        );
        arrange(radicale, &user, &url);
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

    /// The Sync Status of the Calendar `id`.
    fn status(&self, id: CalendarId) -> SyncStatus {
        calendar_sync_status(&self.core, id).report.status
    }

    /// Runs the syncs FreeCal launches with, and forgets their Signals.
    fn launched(&self) {
        self.core.run_scheduled_syncs();
        self.harness.signals.clear();
    }

    /// Runs the syncs the Sync Scheduler finds due, and returns the
    /// Calendars that synced, in order.
    fn run_scheduled_syncs(&self) -> Vec<CalendarId> {
        self.harness.signals.clear();
        self.core.run_scheduled_syncs();
        self.harness
            .signals
            .received()
            .into_iter()
            .filter_map(|signal| match signal {
                Signal::CalendarSyncStatusChanged(status) => match status.report.status {
                    SyncStatus::Syncing { .. } => Some(status.calendar_id),
                    _ => None,
                },
                _ => None,
            })
            .collect()
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

/// The titles of the Occurrences in October 2026, in order.
fn titles(on_server: &OnServer) -> Vec<String> {
    on_server
        .core
        .occurrences(date("2026-10-01"), date("2026-11-01"))
        .unwrap()
        .into_iter()
        .map(|occurrence| occurrence.title)
        .collect()
}

/// Ticket 128 (ADR 0015): a sent DELETE frees its UID for the Ignored
/// Resources holding it, and the first by the fixed order takes over in
/// the same sync, as after an Incoming Sync, so the Event comes back. The
/// Sync Engine writes it, so it is no Unsent Change, and the Sender
/// deletes none of the Ignored Resources.
#[test]
fn a_delete_hands_its_uid_on_to_the_first_ignored_resource_holding_it() {
    let on_server = OnServer::with_files(&[
        ("a-standup", standup("Standup A")),
        ("c-standup", standup("Standup C")),
        ("b-standup", standup("Standup B")),
    ]);
    assert_eq!(titles(&on_server), ["Standup A"]);
    on_server
        .core
        .delete_event(on_server.event("Standup A").id)
        .unwrap();

    on_server.sync();

    let status = on_server
        .radicale
        .status_of(&on_server.user, &format!("{}a-standup.ics", on_server.url));
    assert_eq!(status, 404);
    assert_eq!(titles(&on_server), ["Standup B"]);
    assert!(
        on_server
            .held("Standup B")
            .unwrap()
            .href
            .ends_with("/b-standup.ics")
    );
    assert!(
        on_server
            .on_server("b-standup")
            .contains("SUMMARY:Standup B")
    );
    assert!(
        on_server
            .on_server("c-standup")
            .contains("SUMMARY:Standup C")
    );
    assert_eq!(on_server.unsent(on_server.work), 0);
    assert_eq!(on_server.sync(), []);
    assert_eq!(titles(&on_server), ["Standup B"]);
}

/// [`standup`] with a start FreeCal can't read: a UTC time with a zone,
/// which Radicale takes.
fn unreadable_standup(title: &str) -> Vec<u8> {
    ics(&format!(
        "BEGIN:VEVENT\nUID:standup\nSUMMARY:{title}\n\
         DTSTART;TZID=Europe/Berlin:20261005T070000Z\n\
         DTEND;TZID=Europe/Berlin:20261005T073000Z\nEND:VEVENT\n"
    ))
}

/// With no Ignored Resource holding the UID readable, the UID stays
/// without Event Rows; once one reads, it takes over.
#[test]
fn a_delete_with_no_readable_ignored_resource_holding_its_uid_leaves_no_event_rows() {
    let on_server = OnServer::with_files(&[
        ("a-standup", standup("Standup A")),
        ("b-standup", unreadable_standup("Standup B")),
    ]);
    assert_eq!(titles(&on_server), ["Standup A"]);
    on_server
        .core
        .delete_event(on_server.event("Standup A").id)
        .unwrap();

    on_server.sync();

    assert_eq!(titles(&on_server), Vec::<String>::new());
    assert_eq!(on_server.unsent(on_server.work), 0);
    on_server
        .radicale
        .write_event_file(&on_server.url, "b-standup", standup("Standup B"));
    on_server.sync();
    assert_eq!(titles(&on_server), ["Standup B"]);
    assert_eq!(on_server.unsent(on_server.work), 0);
}

/// Ticket 124: a change is sent right after it is made, by a sync of its
/// Calendar alone, both ways.
mod sent_as_made {
    use super::*;

    #[test]
    fn a_change_is_sent_right_after_it_is_made() {
        let on_server = OnServer::with(&[]);
        on_server.launched();
        on_server
            .core
            .create_event(dentist(on_server.work))
            .unwrap();

        assert_eq!(on_server.run_scheduled_syncs(), [on_server.work]);

        let held = on_server.held("Dentist").expect("sent");
        assert!(
            on_server
                .served(&on_server.url, &held.href)
                .1
                .contains("SUMMARY:Dentist")
        );
        assert_eq!(on_server.unsent(on_server.work), 0);
    }

    #[test]
    fn a_change_syncs_only_the_calendars_it_changed() {
        let on_server = OnServer::with(&[("standup", standup("Standup"))]);
        let (home, home_url) = on_server.add_home();
        // A third Calendar the change leaves alone.
        let url = on_server.radicale.create_calendar(
            &on_server.user,
            NewCalendar {
                name: Some("Third"),
                ..NewCalendar::default()
            },
        );
        on_server
            .core
            .add_calendars(on_server.account, &[url])
            .unwrap();
        on_server.launched();

        // Moving an Event changes the Calendar it leaves and the one it
        // joins.
        on_server.edit("Standup", "Standup", home);

        assert_eq!(on_server.run_scheduled_syncs(), [on_server.work, home]);
        let held = on_server.held("Standup").unwrap();
        assert!(
            on_server
                .served(&home_url, &held.href)
                .1
                .contains("UID:standup")
        );
        assert_eq!(on_server.unsent(on_server.work), 0);
        assert_eq!(on_server.unsent(home), 0);
    }

    /// Offline replay is ticket 18's: the Poll, or the network's return,
    /// sends what waits. The harness clock starts at 12:00, when the
    /// launch's Poll runs.
    #[test]
    fn a_change_while_its_account_is_offline_waits_for_the_next_sync() {
        let proxy = RewritingProxy::passthrough(Radicale::shared());
        let on_server = OnServer::through(&proxy, &[]);
        on_server.launched();
        proxy.go_away();
        assert!(
            on_server
                .core
                .sync_account(on_server.account, StartedBy::FreeCal)
                .is_err()
        );
        proxy.come_back();

        on_server
            .core
            .create_event(dentist(on_server.work))
            .unwrap();

        assert_eq!(on_server.run_scheduled_syncs(), []);
        assert_eq!(on_server.unsent(on_server.work), 1);
        on_server.harness.clock.set("2026-10-03T12:05:00+02:00");
        assert_eq!(on_server.run_scheduled_syncs(), [on_server.work]);
        assert_eq!(on_server.unsent(on_server.work), 0);
    }

    /// The sync right after a delete runs the Calendar's Incoming Sync
    /// too, so an Ignored Resource holding the UID takes it over at once
    /// (ticket 128).
    #[test]
    fn a_deleted_uid_is_handed_on_right_after_the_delete_is_sent() {
        let on_server = OnServer::with_files(&[
            ("a-standup", standup("Standup A")),
            ("b-standup", standup("Standup B")),
        ]);
        on_server.launched();
        on_server
            .core
            .delete_event(on_server.event("Standup A").id)
            .unwrap();

        on_server.run_scheduled_syncs();

        assert_eq!(titles(&on_server), ["Standup B"]);
    }
}

/// Ticket 124: a server refusing writes to a Calendar is a Sync Error of
/// that Calendar, and its Unsent Changes stay.
mod writes_refused {
    use super::*;

    /// Radicale lets the user only read the Calendar, and answers a write
    /// with 403; the proxy says they may change it.
    #[test]
    fn a_write_the_server_refuses_is_a_sync_error_of_the_calendar() {
        let proxy = RewritingProxy::new(Radicale::shared(), "<read />", "<all />");
        let on_server = OnServer::read_only_through(&proxy);
        on_server
            .core
            .create_event(dentist(on_server.work))
            .unwrap();
        on_server
            .core
            .create_event(EventDraft {
                title: "Lunch".to_owned(),
                ..dentist(on_server.work)
            })
            .unwrap();

        let synced = on_server
            .core
            .sync_account(on_server.account, StartedBy::FreeCal);

        assert_eq!(synced.unwrap_err().code(), ErrorCode::WritesRefused,);
        assert_eq!(
            on_server.status(on_server.work),
            SyncStatus::SyncError {
                error: ErrorCode::WritesRefused
            }
        );
        assert_eq!(on_server.unsent(on_server.work), 2);
        // Once refused, the Calendar's other changes aren't tried.
        assert_eq!(proxy.requests_holding("BEGIN:VCALENDAR"), 1);
    }

    /// Privileges lost while Unsent Changes wait: the server says so
    /// before any write.
    #[test]
    fn unsent_changes_of_a_calendar_the_server_made_read_only_are_a_sync_error() {
        let proxy = RewritingProxy::new(Radicale::shared(), "<read />", "<all />");
        let on_server = OnServer::read_only_through(&proxy);
        on_server
            .core
            .create_event(dentist(on_server.work))
            .unwrap();
        proxy.stop_rewriting();

        let synced = on_server
            .core
            .sync_account(on_server.account, StartedBy::FreeCal);

        assert_eq!(synced.unwrap_err().code(), ErrorCode::WritesRefused);
        assert_eq!(
            on_server.status(on_server.work),
            SyncStatus::SyncError {
                error: ErrorCode::WritesRefused
            }
        );
        assert_eq!(on_server.unsent(on_server.work), 1);
        assert_eq!(proxy.requests_holding("BEGIN:VCALENDAR"), 0);
    }

    /// The user's own Read-only mark is their choice, not a refusal.
    #[test]
    fn unsent_changes_of_a_calendar_the_user_made_read_only_are_no_sync_error() {
        let on_server = OnServer::with(&[]);
        on_server
            .core
            .create_event(dentist(on_server.work))
            .unwrap();
        on_server
            .core
            .set_calendar_read_only(on_server.work, true)
            .unwrap();

        on_server.sync();

        assert_eq!(on_server.status(on_server.work), SyncStatus::Synced);
        assert_eq!(on_server.unsent(on_server.work), 1);
    }
}
