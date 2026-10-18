//! The Provider interface (spec: "Provider interface"): what the sync
//! engine asks of a server Account's server, whatever its Provider. CalDAV
//! is the first; Google, Microsoft Graph and ICS feeds implement it too, and
//! the sync engine doesn't change for them. It is internal and not a test
//! seam: tests run against real servers, or fakes of them.

use chrono_tz::Tz;

use crate::{AvailableCalendar, CalDavLogin, ImportedEvent, Provider, Result, ServerLogin, caldav};

/// The most Events a page of a Calendar's first sync holds. Each page is
/// committed on its own, so that Events appear as they load.
pub const FIRST_SYNC_PAGE: usize = 50;

/// A server Account's server, logged in.
pub(crate) trait Remote {
    /// The Calendars the Account has on its server that take Events.
    fn discover(&self) -> Result<Vec<AvailableCalendar>>;

    /// Fetches every Event of the Calendar at `url` and hands them to
    /// `page`, up to [`FIRST_SYNC_PAGE`] at a time, stopping at the first
    /// error, its own or `page`'s. Floating times are pinned to the
    /// Calendar's own time zone if the server gives one, otherwise to
    /// `display` (ADR 0007). Events FreeCal can't read are left out.
    fn all_events(
        &self,
        url: &str,
        display: Tz,
        page: &mut dyn FnMut(Vec<ImportedEvent>) -> Result<()>,
    ) -> Result<()>;
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
    ) -> Result<()> {
        caldav::Client::all_events(self, url, display, page)
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
