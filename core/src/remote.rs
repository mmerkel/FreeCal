//! The Provider interface (spec: "Provider interface"): what the Sync
//! Engine asks of a server Account's server, whatever its Provider. CalDAV
//! is the first; Google, Microsoft Graph and ICS feeds implement it too, and
//! the Sync Engine doesn't change for them. It is internal and not a test
//! seam: tests run against real servers, or fakes of them.

use std::collections::HashMap;

use chrono_tz::Tz;

use crate::import::Holds;
use crate::{AvailableCalendar, CalDavLogin, Provider, Result, ServerLogin, ServerObject, caldav};

/// The most Events a page of a Calendar's First Sync holds. Each page is
/// committed on its own, so that Events appear as they load.
pub const FIRST_SYNC_PAGE: usize = 50;

/// How far a Calendar whose First Sync finished is synced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SyncPoint {
    /// What its server gave to ask for the changes since, such as a CalDAV
    /// sync token (RFC 6578); none if it gives none, so that each
    /// Incoming Sync lists the whole Calendar.
    pub token: Option<String>,
}

/// A Server Object as an Incoming Sync fetched it, with what FreeCal reads
/// in it.
#[derive(Debug, Clone)]
pub(crate) struct Fetched {
    pub object: ServerObject,
    /// What FreeCal reads in it.
    pub holds: Holds,
}

/// A Server Object FreeCal holds, or an Ignored Resource it remembers, as
/// an Incoming Sync asks for what changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Held {
    /// The ETag of the version FreeCal holds; none if the server gave none.
    pub etag: Option<String>,
    /// Whether FreeCal couldn't read the server's version, so that it holds
    /// an earlier one.
    pub unreadable: bool,
}

/// What changed in a Calendar on its server.
#[derive(Debug, Clone)]
pub(crate) struct Changes {
    /// Each Server Object that is new or changed.
    pub changed: Vec<Fetched>,
    /// The addresses of the Server Objects FreeCal holds that are gone, as
    /// FreeCal holds them.
    pub removed: Vec<String>,
    /// The Calendar's sync state after these changes.
    pub state: SyncPoint,
    /// The zone floating times are pinned to (ADR 0007), for those of the
    /// objects FreeCal holds already as well.
    pub zone: Tz,
}

/// A server Account's server, logged in.
pub(crate) trait Remote {
    /// The Calendars the Account has on its server that take Events.
    fn discover(&self) -> Result<Vec<AvailableCalendar>>;

    /// Fetches every Server Object of the Calendar at `url` and hands them
    /// to `page`, up to [`FIRST_SYNC_PAGE`] at a time, stopping at the first
    /// error, its own or `page`'s. Floating times are pinned to the
    /// Calendar's own time zone if the server gives one, otherwise to
    /// `display` (ADR 0007), the zone `page` gets with them. Returns the
    /// Calendar's sync state as of before it was listed, so that a change
    /// made meanwhile comes with the next Incoming Sync.
    fn all_events(
        &self,
        url: &str,
        display: Tz,
        page: &mut dyn FnMut(Vec<Fetched>, Tz) -> Result<()>,
    ) -> Result<SyncPoint>;

    /// What changed in the Calendar at `url` since `token`, or since FreeCal
    /// last held `known` without one: the Server Objects FreeCal holds and
    /// the Ignored Resources it remembers, by address. An object whose ETag
    /// FreeCal holds already isn't fetched again, unless FreeCal couldn't
    /// read it, so that it is fetched on each Incoming Sync until it reads.
    /// A known one that is gone is among the removed. Floating times are pinned as
    /// [`Remote::all_events`] pins them.
    fn changes(
        &self,
        url: &str,
        token: Option<&str>,
        known: &HashMap<String, Held>,
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
        page: &mut dyn FnMut(Vec<Fetched>, Tz) -> Result<()>,
    ) -> Result<SyncPoint> {
        caldav::Client::all_events(self, url, display, page)
    }

    fn changes(
        &self,
        url: &str,
        token: Option<&str>,
        known: &HashMap<String, Held>,
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
