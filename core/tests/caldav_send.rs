//! The CalDAV Sender (ticket 127): every sync of an Account sends the
//! Unsent Changes of its writable CalDAV Calendars, with conditional
//! requests, before their Incoming Syncs. A change is sent right after it
//! is made, and a server refusing writes is a Sync Error of the Calendar
//! (ticket 124). Tested against Radicale.

mod support;

use freecal_core::{
    CalendarId, Changed, Core, ErrorCode, EventDraft, EventId, JournalTransactionId, Occurrence,
    RepeatEnd, Scope, Signal, StartedBy, SyncStatus, When,
};
use support::radicale::{NewCalendar, Radicale, RewritingProxy};
use support::{
    Harness, OnServer, add_account, date, dentist, draft, ics, on, put_events, timed, yoga,
};

impl OnServer {
    /// A writable Calendar with `events` (name, iCalendar object) on the
    /// server first.
    fn writable_with(events: &[(&str, Vec<u8>)]) -> Self {
        Self::with(events).writable()
    }

    /// Like [`OnServer::writable_with`], with `files` written straight into
    /// the server's storage, as Radicale refuses a second resource holding
    /// one UID.
    fn writable_with_files(files: &[(&str, Vec<u8>)]) -> Self {
        Self::with_files(files).writable()
    }

    /// Like [`OnServer::writable_with`], reaching the server through
    /// `proxy`.
    fn through(proxy: &RewritingProxy, events: &[(&str, Vec<u8>)]) -> Self {
        Self::arranged(
            Radicale::shared().user(),
            NewCalendar::default(),
            Radicale::create_calendar,
            Some(&proxy.url),
            put_events(events),
        )
        .writable()
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

    /// What the Calendar shows, as [`shows`].
    fn shows(&self, calendar_id: CalendarId) -> Vec<(String, When)> {
        shows(&self.core, calendar_id)
    }

    /// What a second client shows of the Calendar at `url` after its First
    /// Sync, a whole listing of what the server holds.
    fn another_client_shows(&self, url: &str) -> Vec<(String, When)> {
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let account = add_account(&core, self.radicale, &self.user);
        let calendar = core.add_calendars(account, &[url.to_owned()]).unwrap()[0].id;
        shows(&core, calendar)
    }

    /// Asserts that a second client reads back from the server what the
    /// Calendar `calendar_id` at `url` shows, and that nothing waits.
    fn assert_read_back(&self, calendar_id: CalendarId, url: &str) {
        assert_eq!(self.unsent(calendar_id), 0);
        assert_eq!(self.status(calendar_id), SyncStatus::Synced);
        assert_eq!(self.another_client_shows(url), self.shows(calendar_id));
    }

    /// The names of the resources the Calendar at `url` holds on the server.
    fn names(&self, url: &str) -> Vec<String> {
        self.radicale.event_names(url)
    }

    /// The one Occurrence the Calendar shows on `date`.
    fn on(&self, date: &str) -> Occurrence {
        on(&self.core, date)
    }

    /// Yoga as [`yoga`] with `count` Occurrences, created in "Work" and sent.
    fn with_yoga(count: u32) -> (Self, EventId) {
        let on_server = Self::writable_with(&[]);
        let id = yoga(&on_server.core, on_server.work, RepeatEnd::Count { count });
        on_server.sync();
        assert_eq!(on_server.names(&on_server.url).len(), 1);
        (on_server, id)
    }

    /// Renames the Occurrence on `date` to `title`, in `scope`.
    fn rename(&self, date: &str, scope: Scope, title: &str) -> Changed<Occurrence> {
        let occurrence = self.on(date);
        let event = self.core.event(occurrence.event_id).unwrap();
        self.core
            .edit_occurrence(
                occurrence.event_id,
                rid(&occurrence),
                scope,
                false,
                EventDraft {
                    title: title.to_owned(),
                    repeat: event.repeat,
                    ..draft(event.calendar_id, "", occurrence.when)
                },
            )
            .unwrap()
    }

    /// Moves the Occurrence on `date` to `when`, in `scope`.
    fn reschedule(&self, date: &str, scope: Scope, when: When) -> Changed<Occurrence> {
        let occurrence = self.on(date);
        self.core
            .reschedule_occurrence(occurrence.event_id, rid(&occurrence), scope, false, when)
            .unwrap()
    }

    /// Undoes the Journal Transaction `id`, and returns the undo's.
    fn undo(&self, id: JournalTransactionId) -> JournalTransactionId {
        self.core
            .undo_journal_transaction(id)
            .unwrap()
            .journal_transaction
            .expect("an undo is a Journal Transaction")
    }
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
    let on_server = OnServer::writable_with(&[]);
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
    let on_server = OnServer::writable_with(&[("standup", standup("Standup"))]);
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
    let on_server = OnServer::writable_with(&[("standup", standup("Standup"))]);
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
    let on_server = OnServer::writable_with(&[("standup", standup("Standup"))]);
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
    let on_server = OnServer::writable_with(&[("standup", standup("Standup"))]);
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
    let on_server = OnServer::writable_with(&[("standup", standup("Standup"))]);
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
    let on_server = OnServer::writable_with(&[("standup", standup("Standup"))]);
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
    let on_server = OnServer::writable_with(&[]);
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
fn unsent_changes_made_before_the_user_marked_the_calendar_read_only_are_sent() {
    let on_server = OnServer::writable_with(&[("standup", standup("Standup"))]);
    on_server
        .core
        .delete_event(on_server.event("Standup").id)
        .unwrap();
    on_server
        .core
        .set_calendar_read_only(on_server.work, true)
        .unwrap();

    on_server.sync();

    assert_eq!(on_server.status(on_server.work), SyncStatus::Synced);
    assert_eq!(on_server.unsent(on_server.work), 0);
    assert_eq!(
        on_server
            .radicale
            .status_of(&on_server.user, &format!("{}standup.ics", on_server.url)),
        404
    );
}

/// Ticket 69: every change is in the Change Journal as it is made, before
/// it is sent; sending it records nothing more.
#[test]
fn a_sent_change_is_in_the_change_journal_and_sending_records_nothing() {
    let on_server = OnServer::writable_with(&[]);
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
    let on_server = OnServer::writable_with(&[("standup", standup("Standup"))]);
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

/// Ticket 128 (ADR 0015): a sent DELETE frees its UID for the Ignored
/// Resources holding it, and the first by the fixed order takes over in
/// the same sync, as after an Incoming Sync, so the Event comes back. The
/// Sync Engine writes it, so it is no Unsent Change, and the Sender
/// deletes none of the Ignored Resources.
#[test]
fn a_delete_hands_its_uid_on_to_the_first_ignored_resource_holding_it() {
    let on_server = OnServer::writable_with_files(&[
        ("a-standup", standup("Standup A")),
        ("c-standup", standup("Standup C")),
        ("b-standup", standup("Standup B")),
    ]);
    assert_eq!(on_server.titles(), ["Standup A"]);
    on_server
        .core
        .delete_event(on_server.event("Standup A").id)
        .unwrap();

    on_server.sync();

    let status = on_server
        .radicale
        .status_of(&on_server.user, &format!("{}a-standup.ics", on_server.url));
    assert_eq!(status, 404);
    assert_eq!(on_server.titles(), ["Standup B"]);
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
    assert_eq!(on_server.titles(), ["Standup B"]);
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
    let on_server = OnServer::writable_with_files(&[
        ("a-standup", standup("Standup A")),
        ("b-standup", unreadable_standup("Standup B")),
    ]);
    assert_eq!(on_server.titles(), ["Standup A"]);
    on_server
        .core
        .delete_event(on_server.event("Standup A").id)
        .unwrap();

    on_server.sync();

    assert_eq!(on_server.titles(), Vec::<String>::new());
    assert_eq!(on_server.unsent(on_server.work), 0);
    on_server
        .radicale
        .write_event_file(&on_server.url, "b-standup", standup("Standup B"));
    on_server.sync();
    assert_eq!(on_server.titles(), ["Standup B"]);
    assert_eq!(on_server.unsent(on_server.work), 0);
}

/// Ticket 124: a change is sent right after it is made, by a sync of its
/// Calendar alone, both ways.
mod sent_as_made {
    use super::*;

    #[test]
    fn a_change_is_sent_right_after_it_is_made() {
        let on_server = OnServer::writable_with(&[]);
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
        let on_server = OnServer::writable_with(&[("standup", standup("Standup"))]);
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
        let on_server = OnServer::writable_with_files(&[
            ("a-standup", standup("Standup A")),
            ("b-standup", standup("Standup B")),
        ]);
        on_server.launched();
        on_server
            .core
            .delete_event(on_server.event("Standup A").id)
            .unwrap();

        on_server.run_scheduled_syncs();

        assert_eq!(on_server.titles(), ["Standup B"]);
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

    /// Ticket 130: once a refusal made the Calendar Server read-only, its
    /// Unsent Changes fail without a request.
    #[test]
    fn unsent_changes_of_a_server_read_only_calendar_are_a_sync_error_without_a_request() {
        let proxy = RewritingProxy::new(Radicale::shared(), "<read />", "<all />");
        let on_server = OnServer::read_only_through(&proxy);
        on_server
            .core
            .create_event(dentist(on_server.work))
            .unwrap();
        proxy.stop_rewriting();
        let refused = on_server
            .core
            .sync_account(on_server.account, StartedBy::FreeCal);
        assert_eq!(refused.unwrap_err().code(), ErrorCode::WritesRefused);

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
        assert_eq!(proxy.requests_holding("BEGIN:VCALENDAR"), 1);
    }
}

/// What a Calendar shows in October and November 2026: each Occurrence's
/// title and time, in order.
fn shows(core: &Core, calendar_id: CalendarId) -> Vec<(String, When)> {
    core.occurrences(date("2026-10-01"), date("2026-12-01"))
        .unwrap()
        .into_iter()
        .filter(|occurrence| occurrence.calendar_id == calendar_id)
        .map(|occurrence| (occurrence.title, occurrence.when))
        .collect()
}

fn rid(occurrence: &Occurrence) -> &str {
    occurrence
        .recurrence_id
        .as_deref()
        .expect("an Occurrence of a Recurring Event")
}

/// Ticket 125: recurrence scope edits, moves and deletes, and new, changed
/// and deleted Exceptions, reach the server as one resource per UID, and a
/// second client reads the same Events back.
mod recurrence {
    use super::*;

    #[test]
    fn a_new_exception_joins_its_recurring_events_resource() {
        let (on_server, _) = OnServer::with_yoga(3);
        let name = on_server.names(&on_server.url);

        on_server.rename("2026-10-12", Scope::This, "Yoga outside");
        on_server.sync();

        assert_eq!(on_server.names(&on_server.url), name);
        let text = on_server.on_server(&name[0]);
        assert!(text.contains("RECURRENCE-ID"), "{text}");
        assert!(text.contains("SUMMARY:Yoga outside"), "{text}");
        on_server.assert_read_back(on_server.work, &on_server.url);
    }

    #[test]
    fn a_changed_exception_and_a_cancelled_occurrence_reach_the_server() {
        let (on_server, _) = OnServer::with_yoga(4);
        on_server.rename("2026-10-12", Scope::This, "Yoga outside");
        on_server.sync();

        on_server.reschedule(
            "2026-10-12",
            Scope::This,
            timed("2026-10-13T08:00:00", "2026-10-13T09:00:00"),
        );
        let third = on_server.on("2026-10-19");
        on_server
            .core
            .delete_occurrence(third.event_id, rid(&third))
            .unwrap();
        on_server.sync();

        assert_eq!(on_server.shows(on_server.work).len(), 3);
        assert_eq!(on_server.names(&on_server.url).len(), 1);
        on_server.assert_read_back(on_server.work, &on_server.url);
    }

    #[test]
    fn a_deleted_exception_reaches_the_server() {
        let (on_server, _) = OnServer::with_yoga(3);
        on_server.rename("2026-10-12", Scope::This, "Yoga outside");
        on_server.sync();

        let exception = on_server.on("2026-10-12");
        on_server
            .core
            .delete_occurrence(exception.event_id, rid(&exception))
            .unwrap();
        on_server.sync();

        assert_eq!(on_server.shows(on_server.work).len(), 2);
        let text = on_server.on_server(&on_server.names(&on_server.url)[0]);
        assert!(!text.contains("Yoga outside"), "{text}");
        on_server.assert_read_back(on_server.work, &on_server.url);
    }

    #[test]
    fn this_and_following_events_reach_the_server_as_a_second_resource() {
        let (on_server, _) = OnServer::with_yoga(4);
        on_server.rename("2026-10-12", Scope::This, "Yoga outside");
        on_server.sync();

        on_server.reschedule(
            "2026-10-12",
            Scope::ThisAndFollowing,
            timed("2026-10-13T17:00:00", "2026-10-13T18:00:00"),
        );
        on_server.sync();

        assert_eq!(on_server.names(&on_server.url).len(), 2);
        on_server.assert_read_back(on_server.work, &on_server.url);
    }

    #[test]
    fn all_events_reach_the_server_with_their_exceptions() {
        let (on_server, _) = OnServer::with_yoga(3);
        on_server.rename("2026-10-12", Scope::This, "Yoga outside");
        on_server.sync();

        on_server.reschedule(
            "2026-10-19",
            Scope::All,
            timed("2026-10-20T07:00:00", "2026-10-20T08:00:00"),
        );
        on_server.sync();

        assert_eq!(on_server.names(&on_server.url).len(), 1);
        on_server.assert_read_back(on_server.work, &on_server.url);
    }

    #[test]
    fn all_events_moved_to_another_calendar_move_on_the_server_with_their_exceptions() {
        let (on_server, _) = OnServer::with_yoga(3);
        let (home, home_url) = on_server.add_home();
        on_server.rename("2026-10-12", Scope::This, "Yoga outside");
        on_server.sync();

        let first = on_server.on("2026-10-05");
        let event = on_server.core.event(first.event_id).unwrap();
        on_server
            .core
            .edit_occurrence(
                first.event_id,
                rid(&first),
                Scope::All,
                false,
                EventDraft {
                    repeat: event.repeat,
                    ..draft(home, "Yoga", first.when)
                },
            )
            .unwrap();
        on_server.sync();

        assert_eq!(on_server.names(&on_server.url), Vec::<String>::new());
        assert_eq!(on_server.names(&home_url).len(), 1);
        assert_eq!(on_server.shows(home).len(), 3);
        on_server.assert_read_back(home, &home_url);
        on_server.assert_read_back(on_server.work, &on_server.url);
    }

    #[test]
    fn a_deleted_recurring_event_is_deleted_on_the_server_with_its_exceptions() {
        let (on_server, id) = OnServer::with_yoga(3);
        on_server.rename("2026-10-12", Scope::This, "Yoga outside");
        on_server.sync();

        on_server.core.delete_event(id).unwrap();
        on_server.sync();

        assert_eq!(on_server.names(&on_server.url), Vec::<String>::new());
        on_server.assert_read_back(on_server.work, &on_server.url);
    }

    /// A server Recurring Event and its Exception with properties and
    /// parameters FreeCal doesn't show.
    fn server_yoga() -> Vec<u8> {
        ics(
            "BEGIN:VEVENT\nUID:yoga\nSUMMARY:Yoga\nDTSTART:20261005T160000Z\n\
             DTEND:20261005T170000Z\nRRULE:FREQ=WEEKLY;COUNT=4\n\
             X-CUSTOM;X-PARAM=1:on the Recurring Event\n\
             ATTACH;FMTTYPE=text/plain:https://example.com/mat.txt\nEND:VEVENT\n\
             BEGIN:VEVENT\nUID:yoga\nRECURRENCE-ID:20261012T160000Z\nSUMMARY:Yoga outside\n\
             DTSTART:20261012T160000Z\nDTEND:20261012T170000Z\n\
             X-CUSTOM;X-PARAM=2:on the Exception\nEND:VEVENT\n",
        )
    }

    /// Ticket 122 tests the text; this, that it survives the round trip.
    #[test]
    fn unknown_fields_survive_scope_edits_on_the_server() {
        let on_server = OnServer::writable_with(&[("yoga", server_yoga())]);

        on_server.rename("2026-10-12", Scope::This, "Yoga in the park");
        on_server.rename("2026-10-19", Scope::This, "Yoga late");
        let moved = on_server
            .reschedule(
                "2026-10-05",
                Scope::All,
                timed("2026-10-05T19:00:00", "2026-10-05T20:00:00"),
            )
            .journal_transaction
            .unwrap();
        on_server.sync();

        let kept = [
            "X-CUSTOM;X-PARAM=1:on the Recurring Event",
            "ATTACH;FMTTYPE=text/plain:https://example.com/mat.txt",
            "X-CUSTOM;X-PARAM=2:on the Exception",
            "SUMMARY:Yoga in the park",
            "SUMMARY:Yoga late",
        ];
        let text = on_server.on_server("yoga");
        for kept in kept {
            assert!(text.contains(kept), "{kept} in {text}");
        }
        assert!(text.contains("RECURRENCE-ID:20261012T170000Z"), "{text}");
        assert_eq!(on_server.names(&on_server.url), ["yoga"]);
        on_server.assert_read_back(on_server.work, &on_server.url);

        // Undone, the Occurrences move back, and so do the Exceptions'
        // recurrence IDs.
        on_server.undo(moved);
        on_server.sync();
        let text = on_server.on_server("yoga");
        for kept in kept {
            assert!(text.contains(kept), "{kept} in {text}");
        }
        assert!(text.contains("RECURRENCE-ID:20261012T160000Z"), "{text}");
        on_server.assert_read_back(on_server.work, &on_server.url);

        // Cancelling an Exception and another Occurrence leaves the other
        // VEVENTs as they were.
        let late = on_server.on("2026-10-19");
        on_server
            .core
            .delete_occurrence(late.event_id, rid(&late))
            .unwrap();
        let last = on_server.on("2026-10-26");
        on_server
            .core
            .delete_occurrence(last.event_id, rid(&last))
            .unwrap();
        on_server.sync();
        let text = on_server.on_server("yoga");
        for kept in &kept[..4] {
            assert!(text.contains(kept), "{kept} in {text}");
        }
        assert!(!text.contains("Yoga late"), "{text}");
        assert!(text.contains("EXDATE"), "{text}");
        on_server.assert_read_back(on_server.work, &on_server.url);
    }
    /// A move by a whole week: each Exception's moved recurrence ID is the
    /// one the next Exception had on the server, and each keeps its own
    /// `VEVENT` all the same.
    #[test]
    fn exceptions_keep_their_own_vevents_through_a_move_by_a_whole_period() {
        let yoga = ics(
            "BEGIN:VEVENT\nUID:yoga\nSUMMARY:Yoga\nDTSTART:20261005T160000Z\n\
             DTEND:20261005T170000Z\nRRULE:FREQ=WEEKLY;COUNT=4\nEND:VEVENT\n\
             BEGIN:VEVENT\nUID:yoga\nRECURRENCE-ID:20261012T160000Z\nSUMMARY:Yoga outside\n\
             DTSTART:20261012T160000Z\nDTEND:20261012T170000Z\nX-CUSTOM:second\nEND:VEVENT\n\
             BEGIN:VEVENT\nUID:yoga\nRECURRENCE-ID:20261019T160000Z\nSUMMARY:Yoga late\n\
             DTSTART:20261019T160000Z\nDTEND:20261019T170000Z\nX-CUSTOM:third\nEND:VEVENT\n",
        );
        let on_server = OnServer::writable_with(&[("yoga", yoga)]);

        on_server.reschedule(
            "2026-10-05",
            Scope::All,
            timed("2026-10-12T18:00:00", "2026-10-12T19:00:00"),
        );
        on_server.sync();

        let text = on_server.on_server("yoga");
        let vevent_holding = |line: &str| {
            text.split("BEGIN:VEVENT")
                .find(|vevent| vevent.contains(line))
                .unwrap_or_else(|| panic!("{line} in {text}"))
                .to_owned()
        };
        let second = vevent_holding("X-CUSTOM:second");
        assert!(second.contains("RECURRENCE-ID:20261019T160000Z"), "{text}");
        assert!(second.contains("SUMMARY:Yoga outside"), "{text}");
        let third = vevent_holding("X-CUSTOM:third");
        assert!(third.contains("RECURRENCE-ID:20261026T160000Z"), "{text}");
        assert!(third.contains("SUMMARY:Yoga late"), "{text}");
        assert_eq!(text.matches("BEGIN:VEVENT").count(), 3, "{text}");
        on_server.assert_read_back(on_server.work, &on_server.url);
    }
}

/// Ticket 125: an undo or redo in a server Calendar is sent like any
/// other change, under the original identity, and an undone sync stays
/// undone.
mod undo {
    use super::*;

    #[test]
    fn undoing_and_redoing_this_and_following_events_is_sent_under_the_original_identity() {
        let (on_server, _) = OnServer::with_yoga(4);
        let original = on_server.names(&on_server.url);
        let before = on_server.shows(on_server.work);
        let split = on_server
            .rename("2026-10-19", Scope::ThisAndFollowing, "Yoga later")
            .journal_transaction
            .unwrap();
        on_server.sync();
        let split_names = on_server.names(&on_server.url);
        let after = on_server.shows(on_server.work);
        assert_eq!(split_names.len(), 2);

        let undo = on_server.undo(split);
        on_server.sync();
        assert_eq!(on_server.names(&on_server.url), original);
        assert_eq!(on_server.shows(on_server.work), before);
        on_server.assert_read_back(on_server.work, &on_server.url);

        on_server.undo(undo);
        on_server.sync();
        assert_eq!(on_server.names(&on_server.url), split_names);
        assert_eq!(on_server.shows(on_server.work), after);
        on_server.assert_read_back(on_server.work, &on_server.url);
    }

    #[test]
    fn undoing_a_cancelled_occurrence_is_sent() {
        let (on_server, _) = OnServer::with_yoga(3);
        let second = on_server.on("2026-10-12");
        let cancelled = on_server
            .core
            .delete_occurrence(second.event_id, rid(&second))
            .unwrap()
            .journal_transaction
            .unwrap();
        on_server.sync();
        assert_eq!(on_server.another_client_shows(&on_server.url).len(), 2);

        let undo = on_server.undo(cancelled);
        on_server.sync();
        assert_eq!(on_server.shows(on_server.work).len(), 3);
        on_server.assert_read_back(on_server.work, &on_server.url);

        on_server.undo(undo);
        on_server.sync();
        assert_eq!(on_server.shows(on_server.work).len(), 2);
        on_server.assert_read_back(on_server.work, &on_server.url);
    }

    /// A server Event at a name that isn't its UID's. Once the DELETE is
    /// sent, the Mirror holds neither its href nor its text, so the undo
    /// creates it anew, at the name FreeCal gives the UID, from its Event
    /// Rows alone; keeping its href and what FreeCal doesn't show is
    /// ticket 129.
    #[test]
    fn undoing_a_sent_delete_puts_the_event_back_under_its_uid() {
        let on_server = OnServer::writable_with(&[("from-elsewhere", standup("Standup"))]);
        let deleted = on_server
            .core
            .delete_event(on_server.event("Standup").id)
            .unwrap()
            .journal_transaction
            .unwrap();
        on_server.sync();
        assert_eq!(on_server.names(&on_server.url), Vec::<String>::new());

        on_server.undo(deleted);
        on_server.sync();

        assert_eq!(on_server.names(&on_server.url), ["standup"]);
        let text = on_server.on_server("standup");
        assert!(text.contains("UID:standup"), "{text}");
        assert!(text.contains("SUMMARY:Standup\r\n"), "{text}");
        on_server.assert_read_back(on_server.work, &on_server.url);
    }

    /// The undo leaves the Calendar's sync token alone, so only sending it
    /// keeps the server's version from coming back.
    #[test]
    fn an_undone_sync_is_sent_and_stays_undone() {
        let on_server = OnServer::writable_with(&[("standup", standup("Standup"))]);
        on_server.radicale.put_event(
            &on_server.user,
            &on_server.url,
            "standup",
            standup("Standup on the server"),
        );
        let synced = on_server.sync();
        assert_eq!(on_server.titles(), ["Standup on the server"]);

        on_server.undo(synced[0]);
        on_server.sync();

        assert_eq!(on_server.titles(), ["Standup"]);
        let text = on_server.on_server("standup");
        assert!(text.contains("SUMMARY:Standup\r\n"), "{text}");
        assert!(text.contains("X-CUSTOM;X-PARAM=1:kept as it is"), "{text}");
        on_server.assert_read_back(on_server.work, &on_server.url);
        on_server.sync();
        assert_eq!(on_server.titles(), ["Standup"]);
    }

    /// ADR 0015's takeover: another client deletes the resource keeping
    /// the UID, so an Ignored Resource holding it takes over. Undoing that
    /// sync brings back Events the resource that took over doesn't hold;
    /// sending the undo writes them into it, so that no later sync drops
    /// them.
    #[test]
    fn an_undone_sync_with_a_takeover_writes_the_events_back_into_the_resource_that_took_over() {
        let on_server = OnServer::writable_with_files(&[
            ("a-standup", standup("Standup A")),
            ("b-standup", standup("Standup B")),
        ]);
        on_server
            .radicale
            .delete_event(&on_server.user, &on_server.url, "a-standup");
        let synced = on_server.sync();
        assert_eq!(on_server.titles(), ["Standup B"]);

        on_server.undo(synced[0]);
        on_server.sync();

        assert_eq!(on_server.titles(), ["Standup A"]);
        assert_eq!(on_server.names(&on_server.url), ["b-standup"]);
        assert!(
            on_server
                .on_server("b-standup")
                .contains("SUMMARY:Standup A")
        );
        on_server.assert_read_back(on_server.work, &on_server.url);
        on_server.sync();
        assert_eq!(on_server.titles(), ["Standup A"]);
    }
}
