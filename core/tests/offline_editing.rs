//! Offline editing and replay (ticket 18): changes made while an Account is
//! Offline wait as Unsent Changes and are sent once the network returns;
//! Occurrences and Events say whether they have an Unsent Change, since
//! when and why its last send failed; and quitting first tries to send
//! what waits, for a few seconds. Tested against Radicale.

mod support;

use std::time::Duration;

use chrono::{DateTime, Utc};
use freecal_core::{
    EventDraft, Occurrence, RepeatEnd, Scope, SendFailure, Signal, StartedBy, SyncStatuses,
    UnsentChange,
};
use support::radicale::{NewCalendar, Radicale, RewritingProxy};
use support::{OnServer, dentist, draft, ics, on, timed, utc, yoga};

/// When the harness clock starts, 12:00 in Berlin.
const LAUNCH: &str = "2026-10-03T10:00:00Z";

impl OnServer {
    /// A writable Calendar with no Events, whose launch syncs ran.
    fn launched_writable() -> Self {
        let on_server = Self::with(&[]).writable();
        on_server.core.run_scheduled_syncs();
        on_server.harness.signals.clear();
        on_server
    }

    /// Creates the Dentist on 5 October.
    fn create_dentist(&self) {
        self.core.create_event(dentist(self.work)).unwrap();
    }

    /// The Occurrence of the Dentist, on 5 October.
    fn dentist(&self) -> Occurrence {
        on(&self.core, "2026-10-05")
    }

    /// The Unsent Changes of every Calendar after a try to send them before
    /// quitting.
    fn unsent_after(statuses: &SyncStatuses) -> u32 {
        statuses
            .calendars
            .iter()
            .map(|status| status.report.unsent_changes)
            .sum()
    }
}

fn at(time: &str) -> DateTime<Utc> {
    utc(time)
}

#[test]
fn an_edit_while_the_network_is_away_is_made_and_waits() {
    let on_server = OnServer::launched_writable();
    on_server.core.set_network_available(false);

    on_server.create_dentist();
    on_server.core.run_scheduled_syncs();

    assert_eq!(on_server.titles(), ["Dentist"]);
    assert_eq!(on_server.unsent(on_server.work), 1);
    assert!(on_server.radicale.event_names(&on_server.url).is_empty());
}

#[test]
fn the_networks_return_sends_what_waited() {
    let on_server = OnServer::launched_writable();
    on_server.core.set_network_available(false);
    on_server.create_dentist();
    on_server
        .core
        .create_event(EventDraft {
            title: "Lunch".to_owned(),
            ..dentist(on_server.work)
        })
        .unwrap();

    on_server.core.set_network_available(true);
    on_server.core.run_scheduled_syncs();

    assert_eq!(on_server.unsent(on_server.work), 0);
    assert_eq!(on_server.radicale.event_names(&on_server.url).len(), 2);
}

#[test]
fn an_occurrence_says_since_when_it_has_unsent_changes() {
    let on_server = OnServer::launched_writable();
    on_server.core.set_network_available(false);
    on_server.create_dentist();
    on_server.harness.clock.set("2026-10-03T12:01:00+02:00");
    let dentist = on_server.dentist();
    on_server
        .core
        .reschedule_event(
            dentist.event_id,
            timed("2026-10-05T11:00:00", "2026-10-05T12:00:00"),
        )
        .unwrap();

    // Each Event counts once, since its first change.
    assert_eq!(on_server.dentist().unsent_since, Some(at(LAUNCH)));

    on_server.core.set_network_available(true);
    on_server.core.run_scheduled_syncs();
    assert_eq!(on_server.dentist().unsent_since, None);
}

/// Sending changes no Event, but it changes what the window shows of
/// them: the outline and "Not yet synced".
#[test]
fn a_sync_that_had_unsent_changes_to_send_says_the_occurrences_changed() {
    let on_server = OnServer::launched_writable();
    on_server.core.set_network_available(false);
    on_server.create_dentist();
    on_server.core.set_network_available(true);
    on_server.harness.signals.clear();

    on_server.core.run_scheduled_syncs();

    assert!(
        on_server
            .harness
            .signals
            .received()
            .iter()
            .any(|signal| matches!(
                signal,
                Signal::OccurrencesChanged { calendar_ids } if calendar_ids == &[on_server.work]
            ))
    );
}

#[test]
fn only_the_changed_occurrence_of_a_recurring_event_has_unsent_changes() {
    let on_server = OnServer::launched_writable();
    yoga(
        &on_server.core,
        on_server.work,
        RepeatEnd::Count { count: 3 },
    );
    on_server.core.run_scheduled_syncs();
    on_server.core.set_network_available(false);
    on_server.harness.clock.set("2026-10-03T12:10:00+02:00");

    let second = on(&on_server.core, "2026-10-12");
    let changed = on_server
        .core
        .edit_occurrence(
            second.event_id,
            second.recurrence_id.as_deref().unwrap(),
            Scope::This,
            false,
            EventDraft {
                title: "Yoga outside".to_owned(),
                repeat: on_server.core.event(second.event_id).unwrap().repeat,
                ..draft(on_server.work, "", second.when)
            },
        )
        .unwrap();

    let since = Some(at("2026-10-03T10:10:00Z"));
    assert_eq!(changed.value.unsent_since, since);
    assert_eq!(on(&on_server.core, "2026-10-12").unsent_since, since);
    assert_eq!(on(&on_server.core, "2026-10-05").unsent_since, None);
    assert_eq!(on(&on_server.core, "2026-10-19").unsent_since, None);
}

#[test]
fn a_changed_recurring_event_has_unsent_changes_in_every_occurrence() {
    let on_server = OnServer::launched_writable();
    on_server.core.set_network_available(false);

    yoga(
        &on_server.core,
        on_server.work,
        RepeatEnd::Count { count: 2 },
    );

    for day in ["2026-10-05", "2026-10-12"] {
        assert_eq!(on(&on_server.core, day).unsent_since, Some(at(LAUNCH)));
    }
}

#[test]
fn an_event_says_since_when_it_has_unsent_changes_and_why_the_last_send_failed() {
    let on_server = OnServer::with(&[(
        "standup",
        ics(
            "BEGIN:VEVENT\nUID:standup\nSUMMARY:Standup\nDTSTART:20261005T070000Z\n\
             DTEND:20261005T073000Z\nEND:VEVENT\n",
        ),
    )])
    .writable();
    let standup = on_server.event("Standup");
    assert_eq!(standup.unsent_change, None);
    on_server
        .core
        .edit_event(
            standup.id,
            EventDraft {
                title: "Standup moved".to_owned(),
                ..draft(on_server.work, "", standup.when)
            },
        )
        .unwrap();
    assert_eq!(
        on_server.event("Standup moved").unsent_change,
        Some(UnsentChange {
            since: at(LAUNCH),
            send_failure: None,
        })
    );
    // An update with no ETag to make it conditional on waits.
    on_server
        .harness
        .arrange_local_store("UPDATE caldav_server_object SET etag = NULL");

    on_server
        .core
        .sync_account(on_server.account, StartedBy::FreeCal)
        .unwrap();

    // The Incoming Sync after it may bring in the server's version.
    let event_id = on(&on_server.core, "2026-10-05").event_id;
    assert_eq!(
        on_server.core.event(event_id).unwrap().unsent_change,
        Some(UnsentChange {
            since: at(LAUNCH),
            send_failure: Some(SendFailure::WaitingForServerVersion),
        })
    );
}

#[test]
fn a_stored_unsent_change_that_cant_be_read_still_counts_as_waiting_since_long_ago() {
    let on_server = OnServer::launched_writable();
    on_server.core.set_network_available(false);
    on_server.create_dentist();
    on_server.harness.arrange_local_store(
        "UPDATE unsent_change SET since = 'soon', send_failure = '{\"kind\":\"failed\"}'",
    );

    assert_eq!(on_server.dentist().unsent_since, Some(DateTime::UNIX_EPOCH));
    assert_eq!(
        on_server.event("Dentist").unsent_change,
        Some(UnsentChange {
            since: DateTime::UNIX_EPOCH,
            send_failure: None,
        })
    );
}

/// Ticket 18: quitting first tries to send what waits, for a few seconds.
mod sending_before_quit {
    use super::*;

    /// A writable Calendar reached through `proxy`, whose launch syncs ran,
    /// with the Dentist created while its Account was Offline, which asks
    /// for no sync.
    fn dentist_made_offline(proxy: &RewritingProxy) -> OnServer {
        let on_server = OnServer::arranged(
            Radicale::shared().user(),
            NewCalendar::default(),
            Radicale::create_calendar,
            Some(&proxy.url),
            |_, _, _| {},
        )
        .writable();
        on_server.core.run_scheduled_syncs();
        proxy.go_away();
        assert!(
            on_server
                .core
                .sync_account(on_server.account, StartedBy::FreeCal)
                .is_err()
        );
        on_server.create_dentist();
        on_server
    }

    #[test]
    fn with_nothing_unsent_it_returns_at_once() {
        let on_server = OnServer::launched_writable();

        let statuses = on_server.core.send_before_quit().unwrap();

        assert_eq!(OnServer::unsent_after(&statuses), 0);
    }

    #[test]
    fn it_returns_once_the_syncs_it_asked_for_ran() {
        let proxy = RewritingProxy::passthrough(Radicale::shared());
        let on_server = dentist_made_offline(&proxy);
        proxy.come_back();

        let statuses = std::thread::scope(|scope| {
            let quitting = scope.spawn(|| on_server.core.send_before_quit().unwrap());
            // The shell's thread that runs the syncs.
            on_server.core.wait_for_sync_request(Duration::from_secs(5));
            on_server.core.run_scheduled_syncs();
            quitting.join().unwrap()
        });

        assert_eq!(OnServer::unsent_after(&statuses), 0);
        assert_eq!(on_server.radicale.event_names(&on_server.url).len(), 1);
    }

    #[test]
    fn it_gives_up_once_five_seconds_passed_on_the_clock() {
        let proxy = RewritingProxy::passthrough(Radicale::shared());
        let on_server = dentist_made_offline(&proxy);

        let statuses = std::thread::scope(|scope| {
            let quitting = scope.spawn(|| on_server.core.send_before_quit().unwrap());
            // Nothing runs the syncs it asks for.
            on_server.core.wait_for_sync_request(Duration::from_secs(5));
            on_server.harness.clock.set("2026-10-03T12:00:04+02:00");
            std::thread::sleep(Duration::from_millis(200));
            assert!(!quitting.is_finished());
            on_server.harness.clock.set("2026-10-03T12:00:05+02:00");
            quitting.join().unwrap()
        });

        assert_eq!(OnServer::unsent_after(&statuses), 1);
    }

    /// With no network, nothing would run the syncs: waiting would last
    /// until the clock, which stands still here, passed five seconds.
    #[test]
    fn with_no_network_it_returns_at_once() {
        let on_server = OnServer::launched_writable();
        on_server.core.set_network_available(false);
        on_server.create_dentist();

        let statuses = on_server.core.send_before_quit().unwrap();

        assert_eq!(OnServer::unsent_after(&statuses), 1);
    }
}
