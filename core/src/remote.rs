//! The Provider interface (spec: "Provider interface"): what the sync
//! engine asks of a server Account's server, whatever its Provider. CalDAV
//! is the first; Google, Microsoft Graph and ICS feeds implement it too, and
//! the sync engine doesn't change for them. It is internal and not a test
//! seam: tests run against real servers, or fakes of them.

use std::collections::HashMap;

use chrono_tz::Tz;

use crate::{AvailableCalendar, CalDavLogin, ImportedEvent, Provider, Result, ServerLogin, caldav};

/// The most Events a page of a Calendar's first sync holds. Each page is
/// committed on its own, so that Events appear as they load.
pub const FIRST_SYNC_PAGE: usize = 50;

/// How far a Calendar whose first sync finished is synced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SyncPoint {
    /// What its server gave to ask for the changes since, such as a CalDAV
    /// sync token (RFC 6578); none if it gives none, so that each sync
    /// lists the whole Calendar.
    pub token: Option<String>,
}

/// What changed in a Calendar on its server.
#[derive(Debug, Clone)]
pub(crate) struct Changes {
    /// The Events of each server object that is new or changed, read with
    /// the object as their original server data.
    pub changed: Vec<ImportedEvent>,
    /// The addresses of the server objects FreeCal holds that are gone, as
    /// FreeCal holds them.
    pub removed: Vec<String>,
    /// The Calendar's sync state after these changes.
    pub state: SyncPoint,
}

/// A server Account's server, logged in.
pub(crate) trait Remote {
    /// The Calendars the Account has on its server that take Events.
    fn discover(&self) -> Result<Vec<AvailableCalendar>>;

    /// Fetches every Event of the Calendar at `url` and hands them to
    /// `page`, up to [`FIRST_SYNC_PAGE`] at a time, stopping at the first
    /// error, its own or `page`'s. Floating times are pinned to the
    /// Calendar's own time zone if the server gives one, otherwise to
    /// `display` (ADR 0007). Events FreeCal can't read are left out.
    /// Returns the Calendar's sync state as of before it was listed, so that
    /// a change made meanwhile comes with the next sync.
    fn all_events(
        &self,
        url: &str,
        display: Tz,
        page: &mut dyn FnMut(Vec<ImportedEvent>) -> Result<()>,
    ) -> Result<SyncPoint>;

    /// What changed in the Calendar at `url` since `token`, or since FreeCal
    /// last held `known` without one: the server objects FreeCal holds,
    /// each one's address with its ETag. An object whose ETag FreeCal holds
    /// already isn't fetched again. Floating times are pinned as
    /// [`Remote::all_events`] pins them, and Events FreeCal can't read are
    /// left out, so that the Events of an object FreeCal can no longer read
    /// stay as they were.
    fn changes(
        &self,
        url: &str,
        token: Option<&str>,
        known: &HashMap<String, Option<String>>,
        display: Tz,
    ) -> Result<Changes>;
}

impl Remote for caldav::Client {
    fn discover(&self) -> Result<Vec<AvailableCalendar>> {
        caldav::Client::discover(self)
    }

    fn all_events(
        &self,
        url: &str,
        display: Tz,
        page: &mut dyn FnMut(Vec<ImportedEvent>) -> Result<()>,
    ) -> Result<SyncPoint> {
        caldav::Client::all_events(self, url, display, page)
    }

    fn changes(
        &self,
        url: &str,
        token: Option<&str>,
        known: &HashMap<String, Option<String>>,
        display: Tz,
    ) -> Result<Changes> {
        caldav::Client::changes(self, url, token, known, display)
    }
}

/// Logs in to the server of an Account of `provider` with `password`; none
/// for the Local Provider, which has no server.
pub(crate) fn connect(
    provider: Provider,
    server: ServerLogin,
    password: String,
) -> Result<Option<Box<dyn Remote>>> {
    match provider {
        Provider::Local => Ok(None),
        Provider::CalDav => Ok(Some(Box::new(caldav::Client::new(&CalDavLogin {
            url: server.url,
            username: server.username,
            password,
        })?))),
    }
}
