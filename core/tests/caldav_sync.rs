mod support;

use freecal_core::{CalendarId, Changed, ErrorCode, Signal, StartedBy, SyncStatus};
use support::radicale::{NewCalendar, Radicale, RewritingProxy};
use support::{Harness, OnServer, date, event, ics, put_events, write_files};

impl OnServer {
    /// Like [`OnServer::with`], with the Calendar marked writable, so that
    /// its changes can be undone.
    fn writable_with(events: &[(&str, Vec<u8>)]) -> Self {
        Self::with(events).writable()
    }

    /// Like [`OnServer::with`], through `proxy`.
    fn through(proxy: &RewritingProxy, events: &[(&str, Vec<u8>)]) -> Self {
        Self::arranged(
            Radicale::shared().user(),
            NewCalendar::default(),
            Radicale::create_calendar,
            Some(&proxy.url),
            put_events(events),
        )
    }

    /// Like [`OnServer::with_files`], through `proxy`.
    fn files_through(proxy: &RewritingProxy, files: &[(&str, Vec<u8>)]) -> Self {
        Self::arranged(
            Radicale::shared().user(),
            NewCalendar::default(),
            Radicale::create_calendar,
            Some(&proxy.url),
            write_files(files),
        )
    }

    /// Makes the Calendar's next sync the rest of a First Sync that stopped
    /// after the pages it had, as if what the server holds now came on a
    /// later page.
    fn first_sync_unfinished(&self) {
        self.harness.arrange_local_store(&format!(
            "UPDATE calendar SET synced = 0, sync_token = NULL WHERE id = {}",
            self.work.0
        ));
    }

    /// The Log line for the resource `name` ignored as it holds `uid`,
    /// which the resource `kept_by` keeps.
    fn kept_line(&self, name: &str, uid: &str, kept_by: &str) -> String {
        let calendar = self.url.strip_prefix(&self.radicale.url()).unwrap();
        let href = |name: &str| format!("/{calendar}{name}.ics");
        format!(
            "Ignored the CalDAV resource {:?} of Calendar {}: it holds the UID {uid:?} \
             that the CalDAV resource {:?} holds, which RFC 4791 §4.1 forbids",
            href(name),
            self.work.0,
            href(kept_by),
        )
    }

    fn put(&self, name: &str, ics: Vec<u8>) {
        self.radicale.put_event(&self.user, &self.url, name, ics);
    }

    fn delete(&self, name: &str) {
        self.radicale.delete_event(&self.user, &self.url, name);
    }

    /// Writes the object `name` straight into the server's storage.
    fn write(&self, name: &str, ics: Vec<u8>) {
        self.radicale.write_event_file(&self.url, name, ics);
    }

    /// Makes the Calendar sync as on a server without sync tokens, listing
    /// the whole Calendar each time.
    fn without_sync_tokens(&self) {
        self.harness.arrange_local_store(&format!(
            "UPDATE calendar SET sync_token = NULL WHERE id = {}",
            self.work.0
        ));
    }

    /// The address of the object the Event titled `title` is in.
    fn href(&self, title: &str) -> String {
        self.held(title).expect("an object").href
    }

    /// Syncs the Account, which has only this Calendar, and returns the
    /// Calendar's Journal Transaction.
    fn sync(&self) -> Option<freecal_core::JournalTransactionId> {
        let synced = self
            .core
            .sync_account(self.account, StartedBy::FreeCal)
            .unwrap();
        assert_eq!(
            synced
                .iter()
                .map(|changed| changed.value)
                .collect::<Vec<_>>(),
            [self.work]
        );
        synced[0].journal_transaction
    }

    /// The ETag the server serves the object `name` with.
    fn etag(&self, name: &str) -> String {
        self.radicale.get_event(&self.user, &self.url, name).0
    }

    fn held_etag(&self, title: &str) -> Option<String> {
        self.held(title).and_then(|server| server.etag)
    }

    fn changed_signals(&self) -> Vec<Signal> {
        vec![Signal::OccurrencesChanged {
            calendar_ids: vec![self.work],
        }]
    }
}

/// The Log lines this thread wrote about Ignored Resources and
/// resources FreeCal can't read.
mod captured_log {
    use std::cell::RefCell;
    use std::sync::Once;

    thread_local! {
        static LINES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    struct Capture;

    impl log::Log for Capture {
        fn enabled(&self, _: &log::Metadata) -> bool {
            true
        }

        fn log(&self, record: &log::Record) {
            let line = record.args().to_string();
            if line.starts_with("Ignored the CalDAV resource")
                || line.starts_with("Couldn't read the CalDAV resource")
            {
                LINES.with(|lines| lines.borrow_mut().push(line));
            }
        }

        fn flush(&self) {}
    }

    /// Starts capturing this thread's Log lines, forgetting earlier ones.
    pub(crate) fn start() {
        static INSTALL: Once = Once::new();
        INSTALL.call_once(|| {
            log::set_logger(&Capture).unwrap();
            log::set_max_level(log::LevelFilter::Warn);
        });
        LINES.with(|lines| lines.borrow_mut().clear());
    }

    /// The Log lines about Ignored Resources and resources FreeCal can't
    /// read this thread wrote since [`start`].
    pub(crate) fn lines() -> Vec<String> {
        LINES.with(|lines| lines.borrow().clone())
    }

    /// How many of [`lines`] there are.
    pub(crate) fn count() -> usize {
        lines().len()
    }
}

/// The object of an Event on `day` (`YYYYMMDD`) from 07:00 to 08:00 UTC.
fn titled(uid: &str, title: &str, day: &str) -> Vec<u8> {
    ics(&format!(
        "BEGIN:VEVENT\nUID:{uid}\nSUMMARY:{title}\nDTSTART:{day}T070000Z\n\
         DTEND:{day}T080000Z\nEND:VEVENT\n"
    ))
}

/// "Standup A" at `a-standup` and "Standup B" at `b-standup`, both with the
/// UID `standup`.
fn two_standups() -> [(&'static str, Vec<u8>); 2] {
    [
        ("a-standup", standup("Standup A")),
        ("b-standup", standup("Standup B")),
    ]
}

fn standup(title: &str) -> Vec<u8> {
    ics(&format!(
        "BEGIN:VEVENT\nUID:standup\nSUMMARY:{title}\nDTSTART:20261005T070000Z\n\
         DTEND:20261005T073000Z\nEND:VEVENT\n"
    ))
}

#[test]
fn a_sync_pulls_an_event_created_on_the_server() {
    let synced = OnServer::with(&[("standup", standup("Standup"))]);
    synced.put(
        "review",
        ics(&event(
            "review",
            "DTSTART:20261006T070000Z",
            "DTEND:20261006T080000Z",
        )),
    );

    synced
        .core
        .sync_account(synced.account, StartedBy::FreeCal)
        .unwrap();

    assert_eq!(synced.titles(), ["Standup", "review"]);
    assert_eq!(
        synced.harness.signals.received_but_sync_status(),
        [Signal::OccurrencesChanged {
            calendar_ids: vec![synced.work]
        }]
    );
}

#[test]
fn a_sync_pulls_an_event_changed_on_the_server_with_its_new_server_data() {
    let synced = OnServer::with(&[("standup", standup("Standup"))]);
    synced.put("standup", standup("Standup moved"));

    let journal_transaction = synced.sync();

    assert!(journal_transaction.is_some());
    assert_eq!(synced.titles(), ["Standup moved"]);
    assert_eq!(
        synced.held_etag("Standup moved"),
        Some(synced.etag("standup"))
    );
    assert_eq!(
        synced.harness.signals.received_but_sync_status(),
        synced.changed_signals()
    );
}

#[test]
fn a_sync_deletes_an_event_deleted_on_the_server_with_its_exceptions() {
    let synced = OnServer::with(&[
        ("standup", standup("Standup")),
        (
            "weekly",
            ics(
                "BEGIN:VEVENT\nUID:weekly\nSUMMARY:Weekly\nDTSTART:20261005T080000Z\n\
                 DTEND:20261005T090000Z\nRRULE:FREQ=WEEKLY;COUNT=3\nEND:VEVENT\n\
                 BEGIN:VEVENT\nUID:weekly\nSUMMARY:Moved\nRECURRENCE-ID:20261012T080000Z\n\
                 DTSTART:20261012T090000Z\nDTEND:20261012T100000Z\nEND:VEVENT\n",
            ),
        ),
    ]);
    assert_eq!(synced.titles(), ["Standup", "Weekly", "Moved", "Weekly"]);
    synced.delete("weekly");

    synced.sync();

    assert_eq!(synced.titles(), ["Standup"]);
}

#[test]
fn a_sync_replaces_a_changed_occurrence_whose_recurrence_id_is_written_differently() {
    let weekly = |moved: &str| {
        ics(&format!(
            "BEGIN:VEVENT\nUID:weekly\nSUMMARY:Weekly\nDTSTART:20261005T080000Z\n\
             DTEND:20261005T090000Z\nRRULE:FREQ=WEEKLY;COUNT=3\nEND:VEVENT\n\
             BEGIN:VEVENT\nUID:weekly\nSUMMARY:{moved}\nRECURRENCE-ID:20261012T080000Z\n\
             DTSTART:20261012T090000Z\nDTEND:20261012T100000Z\nEND:VEVENT\n"
        ))
    };
    let synced = OnServer::with(&[("weekly", weekly("Moved"))]);
    let id = synced.event("Moved").id;
    // Another form of the same time, as a stored field may have it.
    synced.harness.arrange_local_store(&format!(
        "UPDATE event SET recurrence_id = '2026-10-12T8:0:0' WHERE id = {}",
        id.0
    ));
    synced.put("weekly", weekly("Moved again"));

    synced.sync();

    assert_eq!(synced.titles(), ["Weekly", "Moved again", "Weekly"]);
    assert_eq!(synced.event("Moved again").id, id);
}

#[test]
fn a_sync_that_changes_nothing_records_no_journal_transaction_and_sends_nothing() {
    let synced = OnServer::with(&[("standup", standup("Standup"))]);

    let journal_transaction = synced.sync();

    assert_eq!(journal_transaction, None);
    assert_eq!(synced.harness.signals.received_but_sync_status(), []);
}

#[test]
fn many_changes_send_one_occurrences_changed() {
    let synced = OnServer::with(&[("standup", standup("Standup"))]);
    synced.put("standup", standup("Standup moved"));
    for (uid, day) in [("a", "20261006"), ("b", "20261007"), ("c", "20261008")] {
        synced.put(uid, titled(uid, uid, day));
    }

    synced.sync();

    assert_eq!(synced.titles(), ["Standup moved", "a", "b", "c"]);
    assert_eq!(
        synced.harness.signals.received_but_sync_status(),
        synced.changed_signals()
    );
}

#[test]
fn each_calendar_syncs_as_a_journal_transaction_of_its_own_with_its_own_signal() {
    let synced = OnServer::with(&[("standup", standup("Standup"))]);
    let home_url = synced.radicale.create_calendar(
        &synced.user,
        NewCalendar {
            name: Some("Home"),
            ..NewCalendar::default()
        },
    );
    let home = synced
        .core
        .add_calendars(synced.account, std::slice::from_ref(&home_url))
        .unwrap()[0]
        .id;
    synced.put("standup", standup("Standup moved"));
    synced
        .radicale
        .put_event(&synced.user, &home_url, "standup", standup("Dentist"));
    synced.harness.signals.clear();

    let changed = synced
        .core
        .sync_account(synced.account, StartedBy::FreeCal)
        .unwrap();

    assert_eq!(
        changed
            .iter()
            .map(|changed| changed.value)
            .collect::<Vec<_>>(),
        [synced.work, home]
    );
    assert!(
        changed
            .iter()
            .all(|changed| changed.journal_transaction.is_some())
    );
    assert_ne!(
        changed[0].journal_transaction,
        changed[1].journal_transaction
    );
    assert_eq!(
        synced.harness.signals.received_but_sync_status(),
        [
            Signal::OccurrencesChanged {
                calendar_ids: vec![synced.work]
            },
            Signal::OccurrencesChanged {
                calendar_ids: vec![home]
            },
        ]
    );
}

#[test]
fn a_sync_writes_into_a_read_only_calendar_and_records_it() {
    let synced = OnServer::with(&[("standup", standup("Standup"))]);
    // As every newly added server Calendar is, until 1.0.
    assert!(synced.core.list_calendars().unwrap()[0].read_only);
    synced.put("standup", standup("Standup moved"));

    let journal_transaction = synced.sync();

    assert!(journal_transaction.is_some());
    assert_eq!(synced.titles(), ["Standup moved"]);
}

/// Ticket 130: FreeCal reads whether the server lets the user change a
/// Calendar when it is added, and again only when it has a reason to think
/// that changed: a write the server refuses (a revocation), and a sync the
/// user starts or the one at launch, for Server read-only Calendars only (a
/// grant). Either change sends "Calendars changed" (spec: "Calendars
/// changed"). Radicale lets the user only read the Calendar; the proxy says
/// they may change it while it rewrites.
mod privileges {
    use super::*;
    use support::dentist;

    const PRIVILEGES: &str = "current-user-privilege-set";

    /// [`OnServer::read_only_through`] `proxy`, once it stopped rewriting:
    /// added Server read-only.
    fn server_read_only_through(proxy: &RewritingProxy) -> OnServer {
        proxy.stop_rewriting();
        OnServer::read_only_through(proxy)
    }

    /// The user's Read-only mark and Server read-only, as the Calendar
    /// shows them: `read_only` holds either.
    fn flags(on_server: &OnServer) -> (bool, bool) {
        let calendar = on_server.core.list_calendars().unwrap().pop().unwrap();
        (calendar.read_only, calendar.server_read_only)
    }

    /// Syncs the Account as started by `by`, and returns how that ended,
    /// with the Signals but "Sync status changed".
    fn try_sync(on_server: &OnServer, by: StartedBy) -> (Option<ErrorCode>, Vec<Signal>) {
        on_server.harness.signals.clear();
        let synced = on_server.core.sync_account(on_server.account, by);
        (
            synced.err().map(|error| error.code()),
            on_server.harness.signals.received_but_sync_status(),
        )
    }

    /// [`try_sync`] of a sync that succeeds.
    fn sync(on_server: &OnServer, by: StartedBy) -> Vec<Signal> {
        let (failed, signals) = try_sync(on_server, by);
        assert_eq!(failed, None);
        signals
    }

    #[test]
    fn a_poll_sends_no_privilege_request_nor_a_sync_the_user_starts_of_a_writable_calendar() {
        let proxy = RewritingProxy::new(Radicale::shared(), "<read />", "<all />");
        let on_server = OnServer::read_only_through(&proxy);
        let added = proxy.requests_holding(PRIVILEGES);

        sync(&on_server, StartedBy::FreeCal);
        sync(&on_server, StartedBy::User);

        assert_eq!(proxy.requests_holding(PRIVILEGES), added);
    }

    /// Adding a Calendar reads its privileges; the sync that follows, in
    /// its First Sync, which sends nothing, doesn't read them again.
    #[test]
    fn adding_a_server_read_only_calendar_reads_its_privileges_once() {
        let writable = RewritingProxy::new(Radicale::shared(), "<read />", "<all />");
        OnServer::read_only_through(&writable);
        let server_read_only = RewritingProxy::new(Radicale::shared(), "<read />", "<all />");
        server_read_only_through(&server_read_only);

        assert_eq!(
            server_read_only.requests_holding(PRIVILEGES),
            writable.requests_holding(PRIVILEGES)
        );
    }

    #[test]
    fn a_write_the_server_refuses_makes_the_calendar_server_read_only() {
        let proxy = RewritingProxy::new(Radicale::shared(), "<read />", "<all />");
        let on_server = OnServer::read_only_through(&proxy);
        assert_eq!(flags(&on_server), (false, false));
        on_server
            .core
            .create_event(dentist(on_server.work))
            .unwrap();
        proxy.stop_rewriting();

        assert_eq!(
            try_sync(&on_server, StartedBy::FreeCal),
            (
                Some(ErrorCode::WritesRefused),
                vec![Signal::CalendarsChanged]
            )
        );

        assert_eq!(flags(&on_server), (true, true));
        // The refused write stays the Calendar's Sync Error (ticket 124).
        assert_eq!(
            on_server.status(on_server.work),
            SyncStatus::SyncError {
                error: ErrorCode::WritesRefused
            }
        );
    }

    #[test]
    fn a_permission_granted_again_is_noticed_by_a_sync_the_user_starts_not_by_a_poll() {
        let proxy = RewritingProxy::new(Radicale::shared(), "<read />", "<all />");
        let on_server = server_read_only_through(&proxy);
        assert_eq!(flags(&on_server), (true, true));
        proxy.start_rewriting();
        let added = proxy.requests_holding(PRIVILEGES);

        assert_eq!(sync(&on_server, StartedBy::FreeCal), []);
        assert_eq!(flags(&on_server), (true, true));
        assert_eq!(proxy.requests_holding(PRIVILEGES), added);

        assert_eq!(
            sync(&on_server, StartedBy::User),
            [Signal::CalendarsChanged]
        );
        // The user's mark stayed as they left it.
        assert_eq!(flags(&on_server), (false, false));
    }

    /// Through the Sync Scheduler: the first Poll, due at launch, and sync
    /// now look; a later Poll doesn't.
    #[test]
    fn a_permission_granted_again_is_noticed_at_launch_and_by_sync_now() {
        let proxy = RewritingProxy::new(Radicale::shared(), "<read />", "<all />");
        let on_server = server_read_only_through(&proxy);
        let run_scheduled_syncs = || {
            on_server.harness.signals.clear();
            on_server.core.run_scheduled_syncs();
            on_server.harness.signals.received_but_sync_status()
        };
        proxy.start_rewriting();

        assert_eq!(run_scheduled_syncs(), [Signal::CalendarsChanged]);
        assert_eq!(flags(&on_server), (false, false));

        // Revoked again meanwhile, as a refused write finds.
        on_server.harness.arrange_local_store(&format!(
            "UPDATE calendar SET server_read_only = 1 WHERE id = {}",
            on_server.work.0
        ));
        on_server.harness.clock.set("2026-10-03T12:05:00+02:00");
        assert_eq!(run_scheduled_syncs(), []);
        assert_eq!(flags(&on_server), (true, true));

        on_server.core.sync_now(Some(on_server.account));
        assert_eq!(run_scheduled_syncs(), [Signal::CalendarsChanged]);
        assert_eq!(flags(&on_server), (false, false));
    }
}

mod undo {
    use super::*;

    /// ADR 0015: the next write is conditional on what the server has now.
    #[test]
    fn restores_the_previous_event_version_and_leaves_the_object_as_the_server_has_it() {
        let synced = OnServer::writable_with(&[("standup", standup("Standup"))]);
        synced.put("standup", standup("Standup moved"));
        let journal_transaction = synced.sync().unwrap();
        let served = synced
            .radicale
            .get_event(&synced.user, &synced.url, "standup");

        let undone = synced
            .core
            .undo_journal_transaction(journal_transaction)
            .unwrap();

        assert!(undone.journal_transaction.is_some());
        assert_eq!(synced.titles(), ["Standup"]);
        let held = synced.held("Standup").unwrap();
        assert_eq!((held.etag, held.data), (Some(served.0), served.1));
    }

    #[test]
    fn of_a_journal_transaction_is_not_blocked_by_a_later_sync_that_changed_only_the_object() {
        let synced = OnServer::writable_with(&[("standup", standup("Standup"))]);
        synced.put("standup", standup("Standup moved"));
        let journal_transaction = synced.sync().unwrap();
        // A property FreeCal doesn't read: a new ETag and data, the same Event.
        synced.put(
            "standup",
            ics(
                "BEGIN:VEVENT\nUID:standup\nSUMMARY:Standup moved\nDTSTART:20261005T070000Z\n\
                 DTEND:20261005T073000Z\nX-UNKNOWN:new\nEND:VEVENT\n",
            ),
        );

        assert_eq!(synced.sync(), None);
        assert_eq!(
            synced.held_etag("Standup moved"),
            Some(synced.etag("standup"))
        );
        synced
            .core
            .undo_journal_transaction(journal_transaction)
            .unwrap();

        assert_eq!(synced.titles(), ["Standup"]);
    }

    /// As a delete already sent would be: the Event counts as not yet on
    /// the server.
    #[test]
    fn of_a_delete_brings_back_an_event_without_the_object_the_server_removed_since() {
        let synced = OnServer::writable_with(&[("standup", standup("Standup"))]);
        let deleted = synced
            .core
            .delete_event(synced.event("Standup").id)
            .unwrap();
        synced.delete("standup");
        synced.sync();

        synced
            .core
            .undo_journal_transaction(deleted.journal_transaction.unwrap())
            .unwrap();

        let id = synced.event("Standup").id;
        assert_eq!(synced.core.original_server_data(id).unwrap(), None);
    }

    #[test]
    fn brings_back_what_the_server_deleted_and_removes_what_it_added() {
        let synced = OnServer::writable_with(&[("standup", standup("Standup"))]);
        synced.delete("standup");
        synced.put(
            "review",
            ics(&event(
                "review",
                "DTSTART:20261006T070000Z",
                "DTEND:20261006T080000Z",
            )),
        );
        let journal_transaction = synced.sync().unwrap();
        assert_eq!(synced.titles(), ["review"]);

        synced
            .core
            .undo_journal_transaction(journal_transaction)
            .unwrap();

        assert_eq!(synced.titles(), ["Standup"]);
    }
}

mod floating_times {
    use super::*;

    fn floating_standup(title: &str) -> Vec<u8> {
        ics(&format!(
            "BEGIN:VEVENT\nUID:standup\nSUMMARY:{title}\nDTSTART:20261005T090000\n\
             DTEND:20261005T093000\nEND:VEVENT\n"
        ))
    }

    /// ADR 0007, rule 2.
    #[test]
    fn a_new_version_floating_at_both_ends_keeps_the_zone_pinned_before() {
        let synced = OnServer::with(&[("standup", floating_standup("Standup"))]);
        assert_eq!(
            synced.event("Standup").pinned_zone.as_deref(),
            Some("Europe/Berlin")
        );
        synced.harness.clock.set_time_zone("Asia/Tokyo");
        synced.put("standup", floating_standup("Standup renamed"));

        synced.sync();

        let standup = synced.event("Standup renamed");
        assert_eq!(standup.pinned_zone.as_deref(), Some("Europe/Berlin"));
        // 09:00 in Berlin, shown in Tokyo.
        assert_eq!(
            standup.when,
            support::timed("2026-10-05T16:00:00", "2026-10-05T16:30:00")
        );
    }

    /// ADR 0007, rule 3.
    #[test]
    fn one_floating_end_of_a_new_version_gets_the_zone_of_the_other() {
        let synced = OnServer::with(&[("standup", floating_standup("Standup"))]);
        synced.put(
            "standup",
            ics("BEGIN:VEVENT\nUID:standup\nSUMMARY:Standup\n\
                 DTSTART;TZID=America/New_York:20261005T090000\n\
                 DTEND:20261005T093000\nEND:VEVENT\n"),
        );

        synced.sync();

        let standup = synced.event("Standup");
        assert_eq!(standup.pinned_zone.as_deref(), Some("America/New_York"));
        assert_eq!(
            standup.when,
            support::timed("2026-10-05T15:00:00", "2026-10-05T15:30:00")
        );
    }
}

#[test]
fn a_calendar_whose_first_sync_didnt_finish_gets_the_rest_of_it_unrecorded() {
    let synced = OnServer::with(&[
        ("standup", standup("Standup")),
        (
            "review",
            ics(&event(
                "review",
                "DTSTART:20261006T070000Z",
                "DTEND:20261006T080000Z",
            )),
        ),
    ]);
    // As a first sync that stopped before the review.
    synced.harness.arrange_local_store(&format!(
        "UPDATE calendar SET synced = 0, sync_token = NULL WHERE id = {0};
         DELETE FROM event WHERE calendar_id = {0} AND uid = 'review';
         DELETE FROM caldav_server_object WHERE calendar_id = {0} AND href LIKE '%/review.ics';",
        synced.work.0
    ));
    assert_eq!(synced.titles(), ["Standup"]);
    synced.put(
        "late",
        ics(&event(
            "late",
            "DTSTART:20261007T070000Z",
            "DTEND:20261007T080000Z",
        )),
    );
    synced.delete("standup");

    let journal_transaction = synced.sync();

    assert_eq!(journal_transaction, None, "the First Sync isn't recorded");
    assert_eq!(synced.titles(), ["review", "late"]);
    assert_eq!(
        synced.harness.signals.received_but_sync_status(),
        synced.changed_signals()
    );
    assert!(
        synced
            .harness
            .signals
            .received()
            .iter()
            .any(|signal| matches!(
                signal,
                Signal::CalendarSyncStatusChanged(status) if status.first_sync_unfinished
                    && matches!(status.report.status, SyncStatus::Syncing { .. })
            )),
        "its Sync Status says it is a First Sync"
    );
    // It is finished now: the next sync is recorded.
    synced.put("late", titled("late", "late moved", "20261007"));
    assert!(synced.sync().is_some());
}

#[test]
fn a_sync_token_the_server_refuses_lists_the_whole_calendar_instead() {
    let synced = OnServer::with(&[
        ("standup", standup("Standup")),
        (
            "review",
            ics(&event(
                "review",
                "DTSTART:20261006T070000Z",
                "DTEND:20261006T080000Z",
            )),
        ),
    ]);
    synced.harness.arrange_local_store(&format!(
        "UPDATE calendar SET sync_token = 'http://radicale.org/ns/sync/unknown' WHERE id = {}",
        synced.work.0
    ));
    synced.delete("standup");
    synced.put("review", titled("review", "review moved", "20261006"));

    let journal_transaction = synced.sync();

    assert!(journal_transaction.is_some());
    assert_eq!(synced.titles(), ["review moved"]);
}

#[test]
fn the_local_account_has_nothing_to_sync() {
    let harness = Harness::new();
    let core = harness.open();
    let local = core.list_accounts().unwrap()[0].id;

    assert_eq!(
        core.sync_account(local, StartedBy::FreeCal).unwrap(),
        Vec::<Changed<CalendarId>>::new()
    );
}

#[test]
fn an_event_whose_object_moved_to_another_address_stays_the_same_event() {
    let synced = OnServer::with(&[("standup", standup("Standup"))]);
    let id = synced.event("Standup").id;
    synced.delete("standup");
    synced.put("standup-moved", standup("Standup moved"));

    synced.sync();

    let moved = synced.event("Standup moved");
    assert_eq!(moved.id, id);
    let server = synced.core.original_server_data(id).unwrap().unwrap();
    assert!(
        server.href.ends_with("/standup-moved.ics"),
        "{}",
        server.href
    );
}

/// ADR 0015: sync matches Server Objects by address, and each one changes
/// only its own Event Rows.
mod server_objects {
    use super::*;

    fn weekly(title: &str) -> Vec<u8> {
        ics(&recurring(title))
    }

    /// A weekly Recurring Event with three Occurrences, from 5 October.
    fn recurring(title: &str) -> String {
        format!(
            "BEGIN:VEVENT\nUID:weekly\nSUMMARY:{title}\nDTSTART:20261005T080000Z\n\
             DTEND:20261005T090000Z\nRRULE:FREQ=WEEKLY;COUNT=3\nEND:VEVENT\n"
        )
    }

    /// An Exception of `weekly`'s second Occurrence.
    fn moved(title: &str) -> String {
        format!(
            "BEGIN:VEVENT\nUID:weekly\nSUMMARY:{title}\nRECURRENCE-ID:20261012T080000Z\n\
             DTSTART:20261012T090000Z\nDTEND:20261012T100000Z\nEND:VEVENT\n"
        )
    }

    /// As Google keeps a Recurring Event: each Exception is a Server Object
    /// of its own.
    fn google_like() -> OnServer {
        OnServer::with_files(&[
            ("weekly", weekly("Weekly")),
            ("weekly-moved", ics(&moved("Moved"))),
        ])
    }

    /// ADR 0015: of two resources alike holding one UID, the First Sync
    /// keeps the lower href; the other is an Ignored Resource, whose
    /// changes change no Event Row.
    #[test]
    fn of_two_objects_with_one_uid_a_first_sync_keeps_the_lower_href() {
        for tokens in [true, false] {
            let synced = OnServer::with_files(&[
                ("b-standup", standup("Standup B")),
                ("a-standup", standup("Standup A")),
            ]);
            if !tokens {
                synced.without_sync_tokens();
            }
            assert_eq!(synced.titles(), ["Standup A"]);
            assert!(synced.href("Standup A").ends_with("/a-standup.ics"));
            assert_eq!(synced.sync(), None);

            synced.write("b-standup", standup("Standup elsewhere"));
            assert_eq!(synced.sync(), None, "tokens: {tokens}");
            assert_eq!(synced.titles(), ["Standup A"]);

            synced.write("a-standup", standup("Standup moved"));
            assert!(synced.sync().is_some());
            assert_eq!(synced.titles(), ["Standup moved"]);
        }
    }

    /// The one holding the Recurring Event comes first in one Incoming
    /// Sync; one arriving later never pushes out the one keeping the UID.
    #[test]
    fn the_object_holding_the_recurring_event_keeps_the_uid_unless_it_came_later() {
        let together = google_like();
        let recurring_first = OnServer::with_files(&[("weekly", weekly("Weekly"))]);
        recurring_first.write("weekly-moved", ics(&moved("Moved")));
        assert_eq!(recurring_first.sync(), None);
        for synced in [together, recurring_first] {
            assert_eq!(synced.titles(), ["Weekly", "Weekly", "Weekly"]);
            assert!(synced.href("Weekly").ends_with("/weekly.ics"));
        }

        let exception_first = OnServer::with_files(&[("weekly-moved", ics(&moved("Moved")))]);
        exception_first.write("weekly", weekly("Weekly"));
        assert_eq!(exception_first.sync(), None);
        assert_eq!(exception_first.titles(), ["Moved"]);
    }

    #[test]
    fn an_ignored_object_that_changes_changes_no_event_row() {
        let synced = google_like();
        synced.write("weekly-moved", ics(&moved("Moved again")));

        assert_eq!(synced.sync(), None);

        assert_eq!(synced.titles(), ["Weekly", "Weekly", "Weekly"]);
    }

    /// ADR 0015: the Ignored Resource holding the UID of a removed Server
    /// Object is fetched again in the same Incoming Sync and takes over,
    /// with or without a sync token; the Event stays shown, with its
    /// content.
    #[test]
    fn an_ignored_object_takes_over_when_the_kept_one_goes() {
        for tokens in [true, false] {
            let synced = OnServer::writable_with(&[]);
            synced.write("weekly", weekly("Weekly"));
            synced.write("weekly-moved", ics(&moved("Moved")));
            synced.sync();
            if !tokens {
                synced.without_sync_tokens();
            }
            synced.delete("weekly");

            let journal_transaction = synced.sync().unwrap();

            assert_eq!(synced.titles(), ["Moved"], "tokens: {tokens}");
            assert!(synced.href("Moved").ends_with("/weekly-moved.ics"));
            assert_eq!(synced.sync(), None, "tokens: {tokens}");
            synced
                .core
                .undo_journal_transaction(journal_transaction)
                .unwrap();
            assert_eq!(synced.titles(), ["Weekly", "Weekly", "Weekly"]);
        }
    }

    /// The same for the kept one's UID changing, with or without a sync
    /// token: the Event of the old UID stays, in the other object.
    #[test]
    fn an_ignored_object_takes_over_when_the_kept_ones_uid_changes() {
        for tokens in [true, false] {
            let synced = OnServer::with_files(&two_standups());
            if !tokens {
                synced.without_sync_tokens();
            }
            let id = synced.event("Standup A").id;
            synced.write("a-standup", titled("review", "Review", "20261006"));

            synced.sync();

            assert_eq!(synced.titles(), ["Standup B", "Review"], "tokens: {tokens}");
            assert_eq!(synced.event("Standup B").id, id);
            assert!(synced.href("Standup B").ends_with("/b-standup.ics"));
        }
    }

    /// Of two resources arriving on different pages of a First Sync, the
    /// first by the fixed order keeps the UID, whichever page it came on:
    /// the one holding the Recurring Event, else the lower href. The other
    /// is ignored, and written to the Log once.
    #[test]
    fn a_first_sync_keeps_the_first_by_the_fixed_order_whichever_page_it_came_on() {
        let a = || ("a-standup", standup("Standup A"));
        let b = || ("b-standup", standup("Standup B"));
        let recurring = || ("z-weekly", weekly("Weekly"));
        let exception = || ("a-weekly-moved", ics(&moved("Moved")));
        let weekly = ["Weekly", "Weekly", "Weekly"].as_slice();
        // The resource on the earlier page and the one on the later page,
        // the one keeping the UID, the one ignored, the UID, and the titles
        // shown.
        let cases = [
            (
                a(),
                b(),
                "a-standup",
                "b-standup",
                "standup",
                ["Standup A"].as_slice(),
            ),
            (
                b(),
                a(),
                "a-standup",
                "b-standup",
                "standup",
                &["Standup A"],
            ),
            (
                recurring(),
                exception(),
                "z-weekly",
                "a-weekly-moved",
                "weekly",
                weekly,
            ),
            (
                exception(),
                recurring(),
                "z-weekly",
                "a-weekly-moved",
                "weekly",
                weekly,
            ),
        ];
        for (earlier, later, kept, ignored, uid, titles) in cases {
            let synced = OnServer::with_files(&[earlier]);
            synced.first_sync_unfinished();
            synced.write(later.0, later.1);
            captured_log::start();

            assert_eq!(synced.sync(), None, "a First Sync records nothing");

            assert_eq!(synced.titles(), titles, "{kept} kept");
            assert!(synced.href(titles[0]).ends_with(&format!("/{kept}.ics")));
            assert_eq!(
                captured_log::lines(),
                [synced.kept_line(ignored, uid, kept)]
            );
            assert_eq!(synced.sync(), None);
        }
    }

    /// The kept one and an Ignored Resource holding its UID both change in
    /// one Incoming Sync: the kept one keeps the UID.
    #[test]
    fn the_kept_resource_changed_beside_an_ignored_resource_keeps_the_uid() {
        let synced = OnServer::with_files(&two_standups());
        let id = synced.event("Standup A").id;
        synced.write("a-standup", standup("Standup A changed"));
        synced.write("b-standup", standup("Standup B changed"));

        synced.sync();

        assert_eq!(synced.titles(), ["Standup A changed"]);
        assert_eq!(synced.event("Standup A changed").id, id);
    }

    /// ADR 0015's amendment for ticket 108: a CalDAV Server Object is its
    /// Calendar and UID, so the kept resource moved to another href is a
    /// takeover, and competes with the Ignored Resources holding its UID by
    /// the fixed order, with or without a sync token. The Event stays.
    #[test]
    fn a_kept_resource_moved_to_another_href_competes_with_the_ignored_resources_by_the_fixed_order()
     {
        // The href it moves to, and the resource then keeping the UID with
        // its title.
        let cases = [
            ("z-standup", ("b-standup", "Standup B")),
            ("0-standup", ("0-standup", "Standup moved")),
        ];
        for (tokens, (to, (kept, title))) in [true, false]
            .into_iter()
            .flat_map(|tokens| cases.map(|case| (tokens, case)))
        {
            let synced = OnServer::with_files(&two_standups());
            if !tokens {
                synced.without_sync_tokens();
            }
            let id = synced.event("Standup A").id;
            synced.delete("a-standup");
            synced.write(to, standup("Standup moved"));

            synced.sync();

            assert_eq!(synced.titles(), [title], "tokens: {tokens}, to {to}");
            assert_eq!(synced.event(title).id, id);
            assert!(synced.href(title).ends_with(&format!("/{kept}.ics")));
            assert_eq!(synced.sync(), None, "tokens: {tokens}, to {to}");
        }
    }

    /// A takeover costs no second request: the Ignored Resources it may need
    /// are fetched in the same request as the rest, with or without a sync
    /// token. The kept one comes to hold another UID, or nothing FreeCal
    /// keeps.
    #[test]
    fn a_takeover_fetches_everything_in_one_request() {
        let todo = ics("BEGIN:VTODO\nUID:todo\nSUMMARY:Todo\nEND:VTODO\n");
        let cases = [
            (
                titled("review", "Review", "20261006"),
                ["Standup B", "Review"].as_slice(),
            ),
            (todo, &["Standup B"]),
        ];
        for tokens in [true, false] {
            for (holds, titles) in &cases {
                let proxy = RewritingProxy::passthrough(Radicale::shared());
                let synced = OnServer::files_through(&proxy, &two_standups());
                if !tokens {
                    synced.without_sync_tokens();
                }
                let id = synced.event("Standup A").id;
                synced.write("a-standup", holds.clone());
                let multigets = proxy.requests_holding("calendar-multiget");

                synced.sync();

                assert_eq!(synced.titles(), *titles, "tokens: {tokens}");
                assert_eq!(synced.event("Standup B").id, id);
                assert_eq!(
                    proxy.requests_holding("calendar-multiget") - multigets,
                    1,
                    "tokens: {tokens}"
                );
            }
        }
    }

    /// The one holding the Recurring Event first, else the lowest href.
    #[test]
    fn of_several_ignored_resources_taking_over_the_fixed_order_decides() {
        let synced = OnServer::with_files(&[
            ("a-standup", standup("Standup A")),
            ("c-standup", standup("Standup C")),
        ]);
        synced.write("b-standup", standup("Standup B"));
        assert_eq!(synced.sync(), None);
        synced.delete("a-standup");

        synced.sync();

        assert_eq!(synced.titles(), ["Standup B"]);

        let synced = OnServer::with_files(&[("m-weekly-moved", ics(&moved("Moved")))]);
        synced.write("z-weekly", weekly("Weekly"));
        synced.write("a-weekly-moved", ics(&moved("Moved again")));
        assert_eq!(synced.sync(), None);
        synced.delete("m-weekly-moved");

        synced.sync();

        assert_eq!(synced.titles(), ["Weekly", "Weekly", "Weekly"]);
        assert!(synced.href("Weekly").ends_with("/z-weekly.ics"));
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

    /// With no Ignored Resource readable when the kept one goes, the UID's
    /// Event Rows go; once one reads, it keeps the UID.
    #[test]
    fn with_no_ignored_resource_readable_on_takeover_the_event_goes() {
        let synced = OnServer::with_files(&two_standups());
        synced.write("b-standup", unreadable_standup("Standup B"));
        synced.sync();
        synced.delete("a-standup");

        synced.sync();

        assert_eq!(synced.titles(), Vec::<String>::new());
        synced.write("b-standup", standup("Standup B"));
        synced.sync();
        assert_eq!(synced.titles(), ["Standup B"]);
    }

    /// A kept one that becomes unreadable keeps the UID and its Event Rows,
    /// though an Ignored Resource holding the UID changes in the same
    /// Incoming Sync.
    #[test]
    fn a_kept_resource_that_becomes_unreadable_keeps_the_uid_beside_an_ignored_resource() {
        let synced = OnServer::with_files(&two_standups());
        let id = synced.event("Standup A").id;
        synced.write("a-standup", unreadable_standup("Standup A moved"));
        synced.write("b-standup", standup("Standup B changed"));

        assert_eq!(synced.sync(), None);

        assert_eq!(synced.titles(), ["Standup A"]);
        assert_eq!(synced.event("Standup A").id, id);
        assert!(synced.href("Standup A").ends_with("/a-standup.ics"));
        assert_eq!(synced.held_etag("Standup A"), None, "fetched again");
    }

    #[test]
    fn a_removed_ignored_object_changes_no_event_row() {
        let synced = google_like();
        synced.delete("weekly-moved");

        assert_eq!(synced.sync(), None);

        assert_eq!(synced.titles(), ["Weekly", "Weekly", "Weekly"]);
    }

    #[test]
    fn of_two_exceptions_for_one_occurrence_in_an_object_the_highest_sequence_is_kept() {
        let with_sequence = |sequence: u32, title: &str| {
            moved(title).replace("END:VEVENT", &format!("SEQUENCE:{sequence}\nEND:VEVENT"))
        };
        let synced = OnServer::with_files(&[(
            "weekly",
            ics(&format!(
                "{}{}{}",
                recurring("Weekly"),
                with_sequence(1, "Older"),
                with_sequence(2, "Latest"),
            )),
        )]);

        assert_eq!(synced.titles(), ["Weekly", "Latest", "Weekly"]);
    }

    /// When the Server Object keeping the UID goes, the Ignored Resource's
    /// Exception takes over, an Event of its own as if it had come alone
    /// (ticket 88).
    #[test]
    fn of_two_objects_with_an_exception_for_one_occurrence_the_recurring_events_shows() {
        let synced = OnServer::with_files(&[
            (
                "weekly",
                ics(&format!("{}{}", recurring("Weekly"), moved("Moved"))),
            ),
            ("weekly-moved", ics(&moved("Moved elsewhere"))),
        ]);
        assert_eq!(synced.titles(), ["Weekly", "Moved", "Weekly"]);

        synced.delete("weekly");
        synced.sync();

        assert_eq!(synced.titles(), ["Moved elsewhere"]);
    }

    /// ADR 0015: an Event Row the server doesn't have yet belongs to the
    /// Server Object holding its UID.
    #[test]
    fn a_new_exception_not_yet_sent_belongs_to_the_object_of_its_recurring_event() {
        let synced = OnServer::writable_with(&[("weekly", weekly("Weekly"))]);
        let second = support::on(&synced.core, "2026-10-12");

        let moved = synced
            .core
            .reschedule_occurrence(
                second.event_id,
                second.recurrence_id.as_deref().unwrap(),
                freecal_core::Scope::This,
                false,
                support::timed("2026-10-12T12:00:00", "2026-10-12T13:00:00"),
            )
            .unwrap()
            .value;

        assert_ne!(moved.event_id, second.event_id, "an Exception");
        let held = synced.core.original_server_data(moved.event_id).unwrap();
        assert!(held.is_some());
        assert_eq!(held, synced.held("Weekly"));
    }

    #[test]
    fn an_object_freecal_can_no_longer_read_keeps_its_last_readable_version() {
        let synced = OnServer::with(&[("standup", standup("Standup"))]);
        let held = synced.held("Standup");
        // Radicale takes a UTC time with a zone; FreeCal can't read it.
        synced.put(
            "standup",
            ics("BEGIN:VEVENT\nUID:standup\nSUMMARY:Standup moved\n\
                 DTSTART;TZID=Europe/Berlin:20261005T070000Z\n\
                 DTEND;TZID=Europe/Berlin:20261005T073000Z\nEND:VEVENT\n"),
        );

        assert_eq!(synced.sync(), None);

        assert_eq!(synced.titles(), ["Standup"]);
        let id = synced.event("Standup").id;
        let kept = synced.core.original_server_data(id).unwrap().unwrap();
        let held = held.unwrap();
        assert_eq!((kept.href, kept.data), (held.href, held.data));
        assert_eq!(kept.etag, None, "fetched again");
        synced.put("standup", standup("Standup moved"));
        synced.sync();
        assert_eq!(synced.titles(), ["Standup moved"]);
        assert_eq!(synced.event("Standup moved").id, id);
    }

    /// Exceptions without their Recurring Event, such as an invitation to
    /// one Occurrence, are Events of their own until it arrives (ticket 88).
    mod exceptions_alone {
        use super::*;

        /// `moved`, its recurrence ID given in New York's time: 04:00 there
        /// is 08:00 UTC, the Occurrence's start.
        fn moved_in_new_york(title: &str) -> String {
            moved(title).replace(
                "RECURRENCE-ID:20261012T080000Z",
                "RECURRENCE-ID;TZID=America/New_York:20261012T040000",
            )
        }

        /// The Calendar's Occurrences in October 2026, each with its title
        /// and whether it is one of a Recurring Event.
        fn shown(synced: &OnServer) -> Vec<(String, bool)> {
            synced
                .core
                .occurrences(date("2026-10-01"), date("2026-11-01"))
                .unwrap()
                .into_iter()
                .map(|occurrence| (occurrence.title, occurrence.recurrence_id.is_some()))
                .collect()
        }

        fn alone(title: &str) -> Vec<(String, bool)> {
            vec![(title.to_owned(), false)]
        }

        fn weekly_with(title: &str) -> Vec<(String, bool)> {
            ["Weekly", title, "Weekly"]
                .into_iter()
                .map(|title| (title.to_owned(), true))
                .collect()
        }

        #[test]
        fn shows_at_its_own_time_in_its_object() {
            let synced =
                OnServer::with_files(&[("weekly-moved", ics(&moved_in_new_york("Moved")))]);

            assert_eq!(shown(&synced), alone("Moved"));
            let moved = synced.event("Moved");
            assert_eq!(
                moved.when,
                support::timed("2026-10-12T11:00:00", "2026-10-12T12:00:00")
            );
            // With no Recurring Event to take the zone from, it keeps its own.
            assert_eq!(
                moved.recurrence_id.as_deref(),
                Some("2026-10-12T04:00:00[America/New_York]")
            );
            assert!(synced.href("Moved").ends_with("/weekly-moved.ics"));
        }

        /// ADR 0015: in one Incoming Sync, the object holding the Recurring
        /// Event keeps the UID and the other is ignored; once that goes, the
        /// other takes over, its recurrence IDs as obtained.
        #[test]
        fn is_ignored_beside_its_recurring_event_in_another_object_until_that_goes() {
            let synced = OnServer::with_files(&[
                ("weekly-moved", ics(&moved_in_new_york("Moved"))),
                ("weekly", weekly("Weekly")),
            ]);
            assert_eq!(shown(&synced), weekly_with("Weekly"));

            synced.delete("weekly");
            synced.sync();

            assert_eq!(shown(&synced), alone("Moved"));
            assert!(synced.href("Moved").ends_with("/weekly-moved.ics"));
            // As if it had come alone, so that its next version pairs with it.
            let moved = synced.event("Moved");
            assert_eq!(
                moved.recurrence_id.as_deref(),
                Some("2026-10-12T04:00:00[America/New_York]")
            );
            synced.write("weekly-moved", ics(&moved_in_new_york("Moved again")));
            synced.sync();
            assert_eq!(synced.event("Moved again").id, moved.id);
        }

        /// ADR 0012: with its Recurring Event, it changes the Occurrence it
        /// names, in another zone or in the Recurring Event's.
        #[test]
        fn joins_its_recurring_event_when_its_object_comes_to_hold_it() {
            for again in [moved_in_new_york("Moved again"), moved("Moved again")] {
                let synced = OnServer::with_files(&[("weekly", ics(&moved_in_new_york("Moved")))]);
                let id = synced.event("Moved").id;
                synced.write("weekly", ics(&format!("{}{again}", recurring("Weekly"))));

                synced.sync();

                assert_eq!(shown(&synced), weekly_with("Moved again"), "{again}");
                let joined = synced.event("Moved again");
                assert_eq!(joined.id, id);
                assert_eq!(joined.recurrence_id.as_deref(), Some("2026-10-12T08:00:00"));
            }
        }

        /// Undoing the Incoming Sync that brought its Recurring Event leaves
        /// it an Event of its own again.
        #[test]
        fn undoing_the_sync_that_brought_its_recurring_event_leaves_it_alone() {
            let synced = OnServer::writable_with(&[("weekly", ics(&moved_in_new_york("Moved")))]);
            let id = synced.event("Moved").id;
            synced.write(
                "weekly",
                ics(&format!(
                    "{}{}",
                    recurring("Weekly"),
                    moved_in_new_york("Moved")
                )),
            );
            let journal_transaction = synced.sync().unwrap();

            synced
                .core
                .undo_journal_transaction(journal_transaction)
                .unwrap();

            assert_eq!(shown(&synced), alone("Moved"));
            assert_eq!(synced.event("Moved").id, id);
        }

        /// Imports the `.ics` file holding `body` into the Calendar,
        /// updating the Events it has.
        fn import(synced: &OnServer, body: &str) {
            let file = freecal_core::ImportFile::new("weekly.ics", ics(body));
            let preview = synced.core.preview_import(&file, synced.work).unwrap();
            synced
                .core
                .run_import(&preview, freecal_core::ExistingEvents::Update)
                .unwrap();
        }

        /// ADR 0012: importing its Recurring Event pairs it by Occurrence,
        /// with or without an Exception of its own for that Occurrence.
        #[test]
        fn importing_its_recurring_event_pairs_it_by_occurrence() {
            for body in [
                recurring("Weekly"),
                format!("{}{}", recurring("Weekly"), moved("Moved")),
            ] {
                let synced =
                    OnServer::writable_with(&[("weekly", ics(&moved_in_new_york("Moved")))]);
                let id = synced.event("Moved").id;

                import(&synced, &body);

                assert_eq!(shown(&synced), weekly_with("Moved"), "{body}");
                assert_eq!(synced.event("Moved").id, id);
            }
        }

        /// ADR 0012: changing its Recurring Event, imported beside it,
        /// keeps it on its Occurrence, moving the Recurring Event too.
        #[test]
        fn changing_its_recurring_event_keeps_it_on_its_occurrence() {
            for (hours, recurrence_id) in [(0, "2026-10-12T08:00:00"), (1, "2026-10-12T09:00:00")] {
                let synced =
                    OnServer::writable_with(&[("weekly", ics(&moved_in_new_york("Moved")))]);
                let id = synced.event("Moved").id;
                import(&synced, &recurring("Weekly"));
                let first = support::on(&synced.core, "2026-10-05");
                let event = synced.core.event(first.event_id).unwrap();
                let freecal_core::When::Timed { start, end } = event.when else {
                    panic!("a timed Event");
                };
                let shift = chrono::TimeDelta::hours(hours);
                let draft = freecal_core::EventDraft {
                    repeat: event.repeat,
                    when: freecal_core::When::Timed {
                        start: start + shift,
                        end: end + shift,
                    },
                    ..support::draft(synced.work, "Renamed", event.when)
                };

                synced
                    .core
                    .edit_occurrence(
                        first.event_id,
                        first.recurrence_id.as_deref().unwrap(),
                        freecal_core::Scope::All,
                        false,
                        draft,
                    )
                    .unwrap();

                assert_eq!(
                    synced.titles(),
                    ["Renamed", "Moved", "Renamed"],
                    "{hours} hours"
                );
                let moved = synced.event("Moved");
                assert_eq!(moved.id, id);
                assert_eq!(moved.recurrence_id.as_deref(), Some(recurrence_id));
            }
        }

        #[test]
        fn a_known_object_that_comes_to_hold_only_it_keeps_it() {
            let synced = OnServer::with_files(&[(
                "weekly",
                ics(&format!("{}{}", recurring("Weekly"), moved("Moved"))),
            )]);
            let id = synced.event("Moved").id;
            synced.write("weekly", ics(&moved("Moved again")));

            synced.sync();

            assert_eq!(shown(&synced), alone("Moved again"));
            assert_eq!(synced.event("Moved again").id, id);
        }

        #[test]
        fn one_that_matches_no_occurrence_stays_after_its_recurring_event_arrives() {
            let odd = moved("Odd").replace("20261012T080000Z", "20261013T080000Z");
            let synced = OnServer::with_files(&[("weekly", ics(&odd))]);
            synced.write("weekly", ics(&format!("{}{odd}", recurring("Weekly"))));

            synced.sync();

            let shown = shown(&synced);
            assert_eq!(shown[2], ("Odd".to_owned(), false));
            assert_eq!(shown.len(), 4);
        }
    }

    /// With a sync token, which lists only what changed since.
    #[test]
    fn an_object_freecal_couldnt_read_is_fetched_again_on_each_sync_until_it_reads() {
        let synced = OnServer::with(&[("standup", standup("Standup"))]);
        // As if FreeCal couldn't read the version the server has, and held
        // an earlier one.
        synced.harness.arrange_local_store(&format!(
            "UPDATE caldav_server_object SET etag = NULL WHERE calendar_id = {0};
             UPDATE event SET title = 'Earlier' WHERE calendar_id = {0};",
            synced.work.0
        ));

        assert!(synced.sync().is_some());

        assert_eq!(synced.titles(), ["Standup"]);
        assert_eq!(synced.sync(), None, "it read, so it isn't fetched again");
    }
}

/// ADR 0015: a CalDAV resource holding several UIDs, which RFC 4791 §4.1
/// forbids, is an Ignored Resource, fetched and written to the Log again
/// only when it changes. Each fetch of it writes one Log line, which is how
/// these tests count them. A resource FreeCal can't read is fetched on
/// every Incoming Sync until it reads, with one Log line when it becomes
/// unreadable.
mod ignored_resources {
    use super::*;

    /// The line that [`SECOND_UID`] replaces in what the server answers.
    const MARKER: &str = "X-TEST-SECOND-UID:b";

    /// What the proxy puts in place of [`MARKER`]: the end of its Event and
    /// the start of another, with the UID `b`.
    const SECOND_UID: &str = "X-TEST-SPLIT:1\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:b\r\n\
        SUMMARY:B\r\nDTSTART:20261007T070000Z\r\nDTEND:20261007T080000Z";

    /// The Event `a` titled `title`, which the proxy shows as a resource
    /// holding the UIDs `a` and `b`.
    fn two_uids(title: &str) -> Vec<u8> {
        ics(&format!(
            "BEGIN:VEVENT\nUID:a\nSUMMARY:{title}\nDTSTART:20261006T070000Z\n\
             DTEND:20261006T080000Z\n{MARKER}\nEND:VEVENT\n"
        ))
    }

    /// A Calendar holding the Event "Standup", synced through a proxy that
    /// shows [`two_uids`] as holding two UIDs.
    fn through_proxy() -> (OnServer, RewritingProxy) {
        let proxy = RewritingProxy::new(Radicale::shared(), MARKER, SECOND_UID);
        let synced = OnServer::through(&proxy, &[("standup", standup("Standup"))]);
        (synced, proxy)
    }

    /// Syncs, without a sync token unless `tokens`, and returns how many
    /// times it fetched the resource `name` and how many Log lines
    /// [`captured_log`] counts it wrote.
    fn sync_and_count(
        synced: &OnServer,
        proxy: &RewritingProxy,
        name: &str,
        tokens: bool,
    ) -> (usize, usize) {
        if !tokens {
            synced.without_sync_tokens();
        }
        let path = format!("{name}.ics");
        let fetched = proxy.requests_holding(&path);
        captured_log::start();
        synced.sync();
        (
            proxy.requests_holding(&path) - fetched,
            captured_log::count(),
        )
    }

    #[test]
    fn is_fetched_and_logged_once_until_it_changes_and_mirrored_once_it_holds_one_uid() {
        for tokens in [false, true] {
            let (synced, proxy) = through_proxy();
            let sync = || sync_and_count(&synced, &proxy, "two", tokens);
            synced.put("two", two_uids("A"));

            assert_eq!(sync(), (1, 1), "tokens: {tokens}");
            assert_eq!(sync(), (0, 0), "tokens: {tokens}");
            assert_eq!(synced.titles(), ["Standup"]);

            synced.put("two", two_uids("A changed"));
            assert_eq!(sync(), (1, 1), "tokens: {tokens}");
            assert_eq!(sync(), (0, 0), "tokens: {tokens}");

            synced.put("two", titled("a", "A alone", "20261006"));
            assert_eq!(sync(), (1, 0), "tokens: {tokens}");
            assert_eq!(synced.titles(), ["Standup", "A alone"]);
            assert!(synced.href("A alone").ends_with("/two.ics"));
            assert_eq!(sync(), (0, 0), "tokens: {tokens}");

            synced.put("two", two_uids("A again"));
            assert_eq!(sync(), (1, 1), "tokens: {tokens}");
            assert_eq!(synced.titles(), ["Standup"], "it leaves the Mirror");
        }
    }

    /// A kept resource coming to hold several UIDs: the Ignored Resource
    /// holding its UID takes over.
    #[test]
    fn an_ignored_resource_takes_over_once_the_kept_one_holds_several_uids() {
        for tokens in [false, true] {
            let proxy = RewritingProxy::new(Radicale::shared(), MARKER, SECOND_UID);
            let synced = OnServer::files_through(&proxy, &two_standups());
            if !tokens {
                synced.without_sync_tokens();
            }
            let id = synced.event("Standup A").id;
            let several = ics(&format!(
                "BEGIN:VEVENT\nUID:standup\nSUMMARY:Standup A\nDTSTART:20261005T070000Z\n\
                 DTEND:20261005T073000Z\n{MARKER}\nEND:VEVENT\n"
            ));
            synced.write("a-standup", several);

            synced.sync();

            assert_eq!(synced.titles(), ["Standup B"], "tokens: {tokens}");
            assert_eq!(synced.event("Standup B").id, id);
        }
    }

    /// In a First Sync too, beside the resource keeping one of its UIDs,
    /// whichever comes first.
    #[test]
    fn beside_the_resource_keeping_one_of_its_uids_it_is_ignored_in_a_first_sync() {
        for (first, second) in [("a-alone", "two"), ("two", "a-alone")] {
            let proxy = RewritingProxy::new(Radicale::shared(), MARKER, SECOND_UID);
            let file = |name| match name {
                "two" => (name, two_uids("A")),
                _ => (name, titled("a", "A alone", "20261006")),
            };
            let files = [file(first), file(second)];
            captured_log::start();

            let synced = OnServer::files_through(&proxy, &files);

            assert_eq!(synced.titles(), ["A alone"]);
            assert!(synced.href("A alone").ends_with("/a-alone.ics"));
            assert_eq!(captured_log::count(), 1);
        }
    }

    /// An Ignored Resource fetched again with an unchanged ETag, as for a
    /// takeover when the Server Object keeping its UID changes but keeps
    /// it, writes no Log line.
    #[test]
    fn fetched_again_unchanged_it_writes_no_log_line() {
        for tokens in [false, true] {
            let proxy = RewritingProxy::passthrough(Radicale::shared());
            let synced = OnServer::files_through(&proxy, &two_standups());
            synced.write("a-standup", standup("Standup A changed"));

            assert_eq!(
                sync_and_count(&synced, &proxy, "b-standup", tokens),
                (1, 0),
                "tokens: {tokens}"
            );
            assert_eq!(synced.titles(), ["Standup A changed"]);
        }
    }

    /// Once removed from the server, it is forgotten: put back as it was,
    /// it is fetched and logged again.
    #[test]
    fn is_forgotten_once_removed_from_the_server() {
        for tokens in [false, true] {
            let (synced, proxy) = through_proxy();
            let sync = || sync_and_count(&synced, &proxy, "two", tokens);
            synced.put("two", two_uids("A"));
            assert_eq!(sync(), (1, 1), "tokens: {tokens}");

            synced.delete("two");
            assert_eq!(sync(), (0, 0), "tokens: {tokens}");
            assert_eq!(synced.titles(), ["Standup"]);

            synced.put("two", two_uids("A"));
            assert_eq!(sync(), (1, 1), "tokens: {tokens}");
        }
    }

    /// ADR 0015: a resource FreeCal can't read has no stored ETag, so that
    /// it is fetched on every Incoming Sync until it reads, though a sync
    /// token doesn't list it again. A proxy shows [`late`] with a start
    /// FreeCal can't read until it stops rewriting, as if an update of
    /// FreeCal could read it.
    mod unreadable {
        use super::*;

        /// The start of [`late`] as Radicale keeps it.
        const READABLE_START: &str = "DTSTART;TZID=Europe/Berlin:20261007T090000";

        /// What the proxy shows instead: a UTC time with a zone, which
        /// FreeCal can't read.
        const UNREADABLE_START: &str = "DTSTART;TZID=Europe/Berlin:20261007T090000Z";

        /// An Event with the UID `uid` titled `title` on 7 October, from
        /// 07:00 UTC, which the proxy shows unreadable.
        fn late(uid: &str, title: &str) -> Vec<u8> {
            ics(&format!(
                "BEGIN:VEVENT\nUID:{uid}\nSUMMARY:{title}\n{READABLE_START}\n\
                 DTEND;TZID=Europe/Berlin:20261007T100000\nEND:VEVENT\n"
            ))
        }

        /// A Calendar holding the Event "Standup", synced through a proxy
        /// that shows [`late`] unreadable.
        fn through_proxy() -> (OnServer, RewritingProxy) {
            let proxy = RewritingProxy::new(Radicale::shared(), READABLE_START, UNREADABLE_START);
            let synced = OnServer::through(&proxy, &[("standup", standup("Standup"))]);
            (synced, proxy)
        }

        #[test]
        fn a_new_one_is_fetched_on_each_sync_until_it_reads_and_its_event_then_shows() {
            for tokens in [false, true] {
                let (synced, proxy) = through_proxy();
                let sync = || sync_and_count(&synced, &proxy, "late", tokens);
                synced.put("late", late("late", "Late"));

                assert_eq!(sync(), (1, 1), "tokens: {tokens}");
                assert_eq!(sync(), (1, 0), "tokens: {tokens}");
                assert_eq!(synced.titles(), ["Standup"]);

                proxy.stop_rewriting();
                assert_eq!(sync(), (1, 0), "tokens: {tokens}");
                assert_eq!(synced.titles(), ["Standup", "Late"]);
                assert_eq!(sync(), (0, 0), "tokens: {tokens}");
            }
        }

        /// It keeps its UID, and takes over once it reads and the Server
        /// Object keeping the UID goes.
        #[test]
        fn an_ignored_resource_is_fetched_on_each_sync_until_it_reads() {
            for tokens in [false, true] {
                let (synced, proxy) = through_proxy();
                let sync = || sync_and_count(&synced, &proxy, "copy", tokens);
                // Radicale refuses a second resource holding a UID.
                synced.write("copy", titled("standup", "Copy", "20261008"));
                assert_eq!(sync(), (1, 1), "tokens: {tokens}");
                assert_eq!(sync(), (0, 0), "tokens: {tokens}");

                synced.write("copy", late("standup", "Copy"));
                assert_eq!(sync(), (1, 1), "tokens: {tokens}");
                assert_eq!(sync(), (1, 0), "tokens: {tokens}");
                assert_eq!(synced.titles(), ["Standup"]);

                proxy.stop_rewriting();
                assert_eq!(sync(), (1, 1), "ignored as it reads, tokens: {tokens}");
                assert_eq!(sync(), (0, 0), "tokens: {tokens}");
                synced.delete("standup");
                synced.sync();
                assert_eq!(synced.titles(), ["Copy"]);
            }
        }

        #[test]
        fn a_server_object_keeps_its_text_uid_and_event_rows_and_is_fetched_until_it_reads() {
            for tokens in [false, true] {
                let (synced, proxy) = through_proxy();
                let sync = || sync_and_count(&synced, &proxy, "late", tokens);
                synced.put("late", titled("late", "Late", "20261007"));
                assert_eq!(sync(), (1, 0), "tokens: {tokens}");
                let id = synced.event("Late").id;
                let text = synced.held("Late").unwrap().data;

                synced.put("late", late("late", "Late moved"));
                assert_eq!(sync(), (1, 1), "tokens: {tokens}");
                assert_eq!(sync(), (1, 0), "tokens: {tokens}");
                assert_eq!(synced.titles(), ["Standup", "Late"]);
                assert_eq!(synced.event("Late").id, id);
                assert_eq!(synced.held("Late").unwrap().data, text);

                proxy.stop_rewriting();
                assert_eq!(sync(), (1, 0), "tokens: {tokens}");
                assert_eq!(synced.titles(), ["Standup", "Late moved"]);
                assert_eq!(synced.event("Late moved").id, id);
                assert_eq!(sync(), (0, 0), "tokens: {tokens}");
            }
        }
    }
}
