//! The Provider interface (spec: "Provider interface"): what the Sync
//! Engine asks of a server Account's server, whatever its Provider. CalDAV
//! is the first; Google, Microsoft Graph and ICS feeds implement it too, and
//! the Sync Engine doesn't change for them. It is internal and not a test
//! seam: tests run against real servers, or fakes of them.

use std::collections::HashMap;

use chrono_tz::Tz;

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

/// The Known Server Resources of a Calendar as the Provider interface takes
/// them: each one's address as the server gave it, with the ETag of the
/// version FreeCal holds; none if the server gave none or FreeCal couldn't
/// read the server's version, so that it is fetched on every Incoming Sync
/// (ADR 0015).
pub(crate) type KnownVersions = HashMap<String, Option<String>>;

/// What changed in a Calendar on its server.
#[derive(Debug, Clone)]
pub(crate) struct Changes {
    /// Each Server Object fetched, as received: those new or changed, and
    /// the Known Server Resources asked for with them.
    pub changed: Vec<ServerObject>,
    /// The addresses of the Known Server Resources that are gone, as
    /// FreeCal holds them.
    pub removed: Vec<String>,
    /// The Calendar's sync state after these changes.
    pub state: SyncPoint,
    /// The zone floating times are pinned to (ADR 0007), for those of the
    /// objects FreeCal holds already as well.
    pub zone: Tz,
}

/// Given the addresses of the Known Server Resources an Incoming Sync is
/// about to fetch or report removed, the addresses of other Known Server
/// Resources to fetch with them, in the same request.
pub(crate) type FetchToo<'a> = &'a dyn Fn(&[String]) -> Vec<String>;

/// The condition a write of a Server Object holds on, so that it never
/// writes over a version FreeCal hasn't read (ADR 0015).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Precondition<'a> {
    /// Nothing is at its address yet: a create (`If-None-Match: *`).
    Absent,
    /// The server holds the version with this ETag, the one the Mirror
    /// holds (`If-Match`).
    Matches(&'a str),
}

/// How a conditional write of a Server Object went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WriteOutcome<T> {
    /// The server took it.
    Done(T),
    /// Its precondition failed: the server's version changed since FreeCal
    /// read it, or it is gone. Finding that change is an Incoming Sync's.
    ServerChanged,
}

impl<T> WriteOutcome<T> {
    /// What it was done with, made by `done`.
    pub(crate) fn map<U>(self, done: impl FnOnce(T) -> U) -> WriteOutcome<U> {
        match self {
            WriteOutcome::Done(value) => WriteOutcome::Done(done(value)),
            WriteOutcome::ServerChanged => WriteOutcome::ServerChanged,
        }
    }
}

/// A Server Object the server took from a `PUT`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Put {
    /// Its address as the server lists it.
    pub href: String,
    /// The ETag of what the server stored, if it gave one that names the
    /// text sent: none when it gave none, or a weak one, as when it changed
    /// the text (RFC 4791 §5.3.4).
    pub etag: Option<String>,
}

/// A server Account's server, logged in.
pub(crate) trait Remote {
    /// The Account's Provider, which [`connect`] logged in to; never the
    /// Local Provider, which has no server.
    fn provider(&self) -> Provider;

    /// The Calendars the Account has on its server that take Events.
    fn discover(&self) -> Result<Vec<AvailableCalendar>>;

    /// Whether the server doesn't let the user change the Events of the
    /// Calendar at `url`, as it says now.
    fn server_read_only(&self, url: &str) -> Result<bool>;

    /// Fetches every Server Object of the Calendar at `url` and hands them,
    /// as received, to `page`, up to [`FIRST_SYNC_PAGE`] at a time,
    /// stopping at the first error, its own or `page`'s. `page` gets them
    /// with the zone their floating times are pinned to (ADR 0007): the
    /// Calendar's own time zone if the server gives one, otherwise
    /// `display`. Returns the
    /// Calendar's sync state as of before it was listed, so that a change
    /// made meanwhile comes with the next Incoming Sync.
    fn all_events(
        &self,
        url: &str,
        display: Tz,
        page: &mut dyn FnMut(Vec<ServerObject>, Tz) -> Result<()>,
    ) -> Result<SyncPoint>;

    /// What changed in the Calendar at `url` since `token`, or since FreeCal
    /// last held `known_server_resources` without one: the Known Server
    /// Resources, by address with their versions. A resource whose ETag
    /// FreeCal holds already isn't fetched again; one FreeCal holds without
    /// an ETag is fetched on every Incoming Sync, listed as changed or not. A
    /// Known Server Resource that is gone is among the removed. Those that
    /// `fetch_too` asks for are fetched with the rest, changed or not. The
    /// zone in [`Changes`] is the one [`Remote::all_events`] gives `page`.
    fn changes(
        &self,
        url: &str,
        token: Option<&str>,
        known_server_resources: &KnownVersions,
        display: Tz,
        fetch_too: FetchToo,
    ) -> Result<Changes>;

    /// Writes `data` as the Server Object at `href` in the Calendar at
    /// `calendar`, an address as the server gave it, or a name for a new
    /// one, if `precondition` holds.
    fn put(
        &self,
        calendar: &str,
        href: &str,
        data: &str,
        precondition: Precondition,
    ) -> Result<WriteOutcome<Put>>;

    /// Deletes the Server Object at `href` in the Calendar at `calendar`,
    /// an address as the server gave it, if the server holds the version
    /// with the ETag `etag`.
    fn delete(&self, calendar: &str, href: &str, etag: &str) -> Result<WriteOutcome<()>>;
}

impl Remote for caldav::Client {
    fn provider(&self) -> Provider {
        Provider::CalDav
    }

    fn discover(&self) -> Result<Vec<AvailableCalendar>> {
        caldav::Client::discover(self).map(|discovered| discovered.calendars)
    }

    fn server_read_only(&self, url: &str) -> Result<bool> {
        caldav::Client::server_read_only(self, url)
    }

    fn all_events(
        &self,
        url: &str,
        display: Tz,
        page: &mut dyn FnMut(Vec<ServerObject>, Tz) -> Result<()>,
    ) -> Result<SyncPoint> {
        caldav::Client::all_events(self, url, display, page)
    }

    fn changes(
        &self,
        url: &str,
        token: Option<&str>,
        known_server_resources: &KnownVersions,
        display: Tz,
        fetch_too: FetchToo,
    ) -> Result<Changes> {
        caldav::Client::changes(self, url, token, known_server_resources, display, fetch_too)
    }

    fn put(
        &self,
        calendar: &str,
        href: &str,
        data: &str,
        precondition: Precondition,
    ) -> Result<WriteOutcome<Put>> {
        caldav::Client::put(self, calendar, href, data, precondition)
    }

    fn delete(&self, calendar: &str, href: &str, etag: &str) -> Result<WriteOutcome<()>> {
        caldav::Client::delete(self, calendar, href, etag)
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
