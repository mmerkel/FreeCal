//! The CalDAV client: WebDAV requests (RFC 4918) to a CalDAV server
//! (RFC 4791). Request bodies are written with an XML writer and answers
//! read with a namespace-aware XML reader, never built or searched as
//! strings. Everything a server sends is untrusted.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use base64::Engine;
use chrono_tz::Tz;
use percent_encoding::percent_decode_str;
use quick_xml::events::{BytesStart, BytesText, Event as XmlEvent};
use quick_xml::name::{Namespace, ResolveResult};
use quick_xml::{NsReader, Writer};
use url::Url;

use crate::remote::{Changes, FIRST_SYNC_PAGE, SyncPoint};
use crate::{
    AvailableCalendar, CalDavLogin, Colour, CoreError, ImportedEvent, OriginalServerData, Result,
    import,
};

const DAV: &str = "DAV:";
const CALDAV: &str = "urn:ietf:params:xml:ns:caldav";
/// Apple's namespace, in which servers keep a Calendar's colour.
const APPLE_ICAL: &str = "http://apple.com/ns/ical/";

/// How deep a `PROPFIND` reaches: the resource alone, or with its members.
#[derive(Debug, Clone, Copy)]
enum Depth {
    Zero,
    One,
}

impl Depth {
    fn header(self) -> &'static str {
        match self {
            Depth::Zero => "0",
            Depth::One => "1",
        }
    }
}

/// How many redirects within the server one request follows.
const MAX_REDIRECTS: usize = 5;

/// The longest server text passed on in an error, in characters.
const MAX_MESSAGE: usize = 200;

/// A WebDAV property: its namespace and name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Property {
    namespace: &'static str,
    name: &'static str,
}

/// The URL of the principal of the user who is logged in (RFC 5397).
const CURRENT_USER_PRINCIPAL: Property = Property {
    namespace: DAV,
    name: "current-user-principal",
};

/// Where the user's Calendars live (RFC 4791).
const CALENDAR_HOME_SET: Property = Property {
    namespace: CALDAV,
    name: "calendar-home-set",
};

const RESOURCETYPE: Property = Property {
    namespace: DAV,
    name: "resourcetype",
};

const DISPLAYNAME: Property = Property {
    namespace: DAV,
    name: "displayname",
};

/// The kinds of entry a Calendar takes, such as Events or tasks.
const SUPPORTED_CALENDAR_COMPONENT_SET: Property = Property {
    namespace: CALDAV,
    name: "supported-calendar-component-set",
};

const CALENDAR_COLOR: Property = Property {
    namespace: APPLE_ICAL,
    name: "calendar-color",
};

/// A Calendar's own time zone, for its floating times (RFC 4791).
const CALENDAR_TIMEZONE: Property = Property {
    namespace: CALDAV,
    name: "calendar-timezone",
};

/// The ETag of a member, which names its version.
const GETETAG: Property = Property {
    namespace: DAV,
    name: "getetag",
};

/// A collection's sync token (RFC 6578), which names its state.
const SYNC_TOKEN: Property = Property {
    namespace: DAV,
    name: "sync-token",
};

/// An Event's iCalendar object, as the server holds it (RFC 4791).
const CALENDAR_DATA: Property = Property {
    namespace: CALDAV,
    name: "calendar-data",
};

/// A connection to one CalDAV server, logged in as one user.
pub(crate) struct Client {
    agent: ureq::Agent,
    url: Url,
    authorization: String,
}

impl Client {
    pub fn new(login: &CalDavLogin) -> Result<Self> {
        let url = Url::parse(login.url.trim()).map_err(|_| CoreError::InvalidServerUrl)?;
        if !matches!(url.scheme(), "http" | "https") || !url.has_host() {
            return Err(CoreError::InvalidServerUrl);
        }
        // Basic authentication splits at the first colon, so a username
        // with one would log in as someone else, or no one.
        if login.username.contains(':') {
            return Err(CoreError::WrongCredentials);
        }
        let credentials = format!("{}:{}", login.username, login.password);
        let agent = ureq::Agent::config_builder()
            // Answers with an error status are read, not failed.
            .http_status_as_error(false)
            // WebDAV's methods, such as PROPFIND, aren't HTTP's own.
            .allow_non_standard_methods(true)
            // Redirects aren't followed, so that the password only ever goes
            // to the server the user named.
            .max_redirects(0)
            // A new connection for each request: ureq would reuse one that
            // an HTTP/1.0 server, such as Radicale on its own, closes after
            // answering.
            .max_idle_connections(0)
            .timeout_connect(Some(Duration::from_secs(15)))
            .timeout_global(Some(Duration::from_secs(60)))
            .tls_config(
                ureq::tls::TlsConfig::builder()
                    .provider(ureq::tls::TlsProvider::NativeTls)
                    .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                    .build(),
            )
            .build()
            .new_agent();
        Ok(Self {
            agent,
            url,
            authorization: format!(
                "Basic {}",
                base64::engine::general_purpose::STANDARD.encode(credentials)
            ),
        })
    }

    /// The server's URL, as given.
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// The Calendars the user has on the server that take Events, by name.
    /// Finding them checks the username and password first.
    pub fn discover(&self) -> Result<Vec<AvailableCalendar>> {
        let principal = self.current_user_principal()?;
        let (answer, principal) = self.propfind(&principal, Depth::Zero, &[CALENDAR_HOME_SET])?;
        let homes: Vec<Url> = answer
            .property(CALENDAR_HOME_SET)
            .map(|set| {
                set.children(DAV, "href")
                    .filter_map(|href| follow_href(&principal, &href.text))
                    .collect()
            })
            .unwrap_or_default();
        if homes.is_empty() {
            return Err(CoreError::NotCalDavServer);
        }
        let mut calendars = Vec::new();
        for home in homes {
            let (answer, home) = self.propfind(
                &home,
                Depth::One,
                &[
                    RESOURCETYPE,
                    DISPLAYNAME,
                    CALENDAR_COLOR,
                    SUPPORTED_CALENDAR_COMPONENT_SET,
                ],
            )?;
            calendars.extend(
                answer
                    .children(DAV, "response")
                    .filter_map(|response| available_calendar(&home, response)),
            );
        }
        calendars
            .sort_by(|a, b| (a.name.to_lowercase(), &a.url).cmp(&(b.name.to_lowercase(), &b.url)));
        // Home sets may overlap.
        calendars.dedup_by(|a, b| a.url == b.url);
        Ok(calendars)
    }

    /// Every Event of the Calendar at `calendar`, handed to `page`
    /// [`FIRST_SYNC_PAGE`] at a time: the Calendar's members are listed,
    /// then fetched with `calendar-multiget` (RFC 4791). Floating times are
    /// pinned to the Calendar's `calendar-timezone`, otherwise to `display`.
    /// Returns the Calendar's sync token as the listing gave it.
    pub fn all_events(
        &self,
        calendar: &str,
        display: Tz,
        page: &mut dyn FnMut(Vec<ImportedEvent>) -> Result<()>,
    ) -> Result<SyncPoint> {
        let listing = self.listing(&stored_url(&self.url, calendar)?, display)?;
        let members: Vec<String> = listing.members.into_iter().map(|(path, _)| path).collect();
        for hrefs in members.chunks(FIRST_SYNC_PAGE) {
            let zone = listing.zone.unwrap_or(display);
            page(self.multiget(&listing.calendar, hrefs, zone)?)?;
        }
        Ok(SyncPoint {
            token: listing.token,
        })
    }

    /// What changed in the Calendar at `calendar` since `token`, asked with
    /// `sync-collection` (RFC 6578), or else found by listing the whole
    /// Calendar with ETags and comparing with `known`: the objects FreeCal
    /// holds, by their address as the server gave it, with their ETags.
    /// Objects whose ETag FreeCal holds are left out; the others are
    /// fetched with `calendar-multiget`.
    pub fn changes(
        &self,
        calendar: &str,
        token: Option<&str>,
        known: &HashMap<String, Option<String>>,
        display: Tz,
    ) -> Result<Changes> {
        let calendar = stored_url(&self.url, calendar)?;
        let since = match token {
            // A server that refuses the token, as when it expired, or the
            // report, is asked for the whole Calendar instead.
            Some(token) => match self.sync_collection(&calendar, token) {
                Err(CoreError::ServerRefused { .. }) => None,
                listed => Some(listed?),
            },
            None => None,
        };
        let (listing, removed) = match since {
            Some((listing, removed)) => (listing, Some(removed)),
            None => (self.listing(&calendar, display)?, None),
        };
        // What FreeCal holds, by the path `member` gives each object.
        let held: HashMap<String, (&String, &Option<String>)> = known
            .iter()
            .filter_map(|(href, etag)| {
                let path = follow_href(&listing.calendar, href)?.path().to_owned();
                Some((path, (href, etag)))
            })
            .collect();
        let removed: Vec<String> = match removed {
            Some(removed) => removed
                .iter()
                .filter_map(|path| held.get(path).map(|(href, _)| (*href).clone()))
                .collect(),
            // In a whole listing, an object it doesn't list is gone.
            None => {
                let listed: HashSet<&String> =
                    listing.members.iter().map(|(path, _)| path).collect();
                held.iter()
                    .filter(|(path, _)| !listed.contains(path))
                    .map(|(_, (href, _))| (*href).clone())
                    .collect()
            }
        };
        let fetch: Vec<String> = listing
            .members
            .into_iter()
            .filter(|(path, etag)| {
                etag.is_none() || held.get(path).is_none_or(|(_, held)| *held != etag)
            })
            .map(|(path, _)| path)
            .collect();
        let zone = match listing.zone {
            _ if fetch.is_empty() => display,
            Some(zone) => zone,
            None => {
                let (answer, _) =
                    self.propfind(&listing.calendar, Depth::Zero, &[CALENDAR_TIMEZONE])?;
                calendar_zone(&answer, display)
            }
        };
        let mut changed = Vec::new();
        for hrefs in fetch.chunks(FIRST_SYNC_PAGE) {
            changed.extend(self.multiget(&listing.calendar, hrefs, zone)?);
        }
        Ok(Changes {
            changed,
            removed,
            state: SyncPoint {
                token: listing.token,
            },
        })
    }

    /// Lists the whole Calendar at `calendar`: its members with their
    /// ETags, its sync token and its time zone, `display` if it has none.
    fn listing(&self, calendar: &Url, display: Tz) -> Result<Listing> {
        let (answer, calendar) = self.propfind(
            calendar,
            Depth::One,
            &[RESOURCETYPE, GETETAG, CALENDAR_TIMEZONE, SYNC_TOKEN],
        )?;
        let members = answer
            .children(DAV, "response")
            .filter_map(|response| {
                let etag = etag(response);
                Some((member(&calendar, response)?, etag))
            })
            .collect();
        Ok(Listing {
            members,
            token: answer.property(SYNC_TOKEN).map(token_text),
            zone: Some(calendar_zone(&answer, display)),
            calendar,
        })
    }

    /// Asks the Calendar at `calendar` what changed since `token` with
    /// `sync-collection` (RFC 6578): a listing of the members new or
    /// changed, without the Calendar's time zone, and the paths of those
    /// removed.
    fn sync_collection(&self, calendar: &Url, token: &str) -> Result<(Listing, Vec<String>)> {
        let (answer, calendar) = self.send(
            b"REPORT",
            calendar,
            Depth::Zero,
            sync_collection_body(token),
        )?;
        let mut members = Vec::new();
        let mut removed = Vec::new();
        for response in answer.children(DAV, "response") {
            let Some(path) = member(&calendar, response) else {
                continue;
            };
            match response.child(DAV, "status") {
                Some(status) if status_code(status) == Some("404") => removed.push(path),
                _ => members.push((path, etag(response))),
            }
        }
        let listing = Listing {
            members,
            token: answer.child(DAV, "sync-token").map(token_text),
            zone: None,
            calendar,
        };
        Ok((listing, removed))
    }

    /// The Events of the objects at `hrefs` in the Calendar at `calendar`,
    /// fetched with `calendar-multiget` (RFC 4791), each with its object as
    /// its original server data. Floating times are pinned to `zone`.
    fn multiget(&self, calendar: &Url, hrefs: &[String], zone: Tz) -> Result<Vec<ImportedEvent>> {
        let (answer, _) = self.send(
            b"REPORT",
            calendar,
            Depth::One,
            calendar_multiget_body(hrefs),
        )?;
        let objects: Vec<OriginalServerData> = answer
            .children(DAV, "response")
            .filter_map(|response| {
                Some(OriginalServerData {
                    href: response.child(DAV, "href")?.text.clone(),
                    etag: etag(response),
                    data: response.found(CALENDAR_DATA)?.text.clone(),
                })
            })
            .collect();
        Ok(import::read_server_events(&objects, zone))
    }

    /// The URL of the principal of the user who is logged in. Asking for it
    /// checks the username and password, and that the server speaks WebDAV.
    /// Where the server's URL doesn't lead to it, as when it is a web page,
    /// asks at `/.well-known/caldav` on the same server (RFC 6764).
    fn current_user_principal(&self) -> Result<Url> {
        let principal_at = |url: &Url| {
            let (answer, url) = self.propfind(url, Depth::Zero, &[CURRENT_USER_PRINCIPAL])?;
            answer
                .property(CURRENT_USER_PRINCIPAL)
                .and_then(|principal| principal.child(DAV, "href"))
                .and_then(|href| follow_href(&url, &href.text))
                .ok_or(CoreError::NotCalDavServer)
        };
        // A failure that may only mean that the principal is found elsewhere.
        let may_be_elsewhere = |error: &CoreError| {
            matches!(
                error,
                CoreError::NotCalDavServer
                    | CoreError::RedirectedElsewhere(_)
                    | CoreError::ServerRefused {
                        status: 404 | 405 | 501,
                        ..
                    }
            )
        };
        match principal_at(&self.url) {
            Err(error) if may_be_elsewhere(&error) => {
                let well_known = self
                    .url
                    .join("/.well-known/caldav")
                    .expect("an absolute path joins any http(s) URL");
                match principal_at(&well_known) {
                    // The first answer says more about the URL the user gave.
                    Err(other) if may_be_elsewhere(&other) => Err(error),
                    result => result,
                }
            }
            result => result,
        }
    }

    /// Asks for `properties` of the resource at `url` and, at
    /// [`Depth::One`], of its members. Follows redirects within the server. Returns the
    /// `multistatus` answer and the URL that gave it.
    fn propfind(&self, url: &Url, depth: Depth, properties: &[Property]) -> Result<(Element, Url)> {
        self.send(b"PROPFIND", url, depth, propfind_body(properties))
    }

    /// Sends a WebDAV request whose answer is a `multistatus`, such as a
    /// `PROPFIND` or a `REPORT`, with the XML `body`. Follows redirects
    /// within the server. Returns the answer and the URL that gave it.
    fn send(
        &self,
        method: &[u8],
        url: &Url,
        depth: Depth,
        body: Vec<u8>,
    ) -> Result<(Element, Url)> {
        let method = ureq::http::Method::from_bytes(method).expect("a WebDAV method is a token");
        let mut url = url.clone();
        let mut redirects = 0;
        loop {
            let request = ureq::http::Request::builder()
                .method(method.clone())
                .uri(url.as_str())
                .header("Authorization", &self.authorization)
                .header("Depth", depth.header())
                .header("Content-Type", "application/xml; charset=utf-8")
                .body(body.clone())
                .map_err(|_| CoreError::InvalidServerUrl)?;
            let response = self.agent.run(request).map_err(transport_error)?;
            let status = response.status().as_u16();
            let location = response
                .headers()
                .get("Location")
                .and_then(|value| value.to_str().ok())
                .and_then(|location| url.join(location.trim()).ok());
            match location {
                Some(to) if matches!(status, 301 | 302 | 303 | 307 | 308) => {
                    if !same_server(&url, &to) {
                        return Err(CoreError::RedirectedElsewhere(to));
                    }
                    if redirects < MAX_REDIRECTS {
                        redirects += 1;
                        url = to;
                        continue;
                    }
                }
                _ => {}
            }
            return read_multistatus(response).map(|answer| (answer, url));
        }
    }
}

/// A Calendar's members as a server listed them.
struct Listing {
    /// The Calendar's URL, as it answered.
    calendar: Url,
    /// Each member's path, as [`member`] gives it, with its ETag.
    members: Vec<(String, Option<String>)>,
    token: Option<String>,
    /// The time zone for its floating times; none when not asked for.
    zone: Option<Tz>,
}

/// A Calendar's address as FreeCal stores it, which is untrusted too.
fn stored_url(server: &Url, calendar: &str) -> Result<Url> {
    follow_href(server, calendar).ok_or(CoreError::InvalidServerUrl)
}

/// The time zone of a Calendar whose `calendar-timezone` a `multistatus`
/// may give, otherwise `display`.
fn calendar_zone(answer: &Element, display: Tz) -> Tz {
    answer
        .property(CALENDAR_TIMEZONE)
        .and_then(|zone| import::calendar_zone(&zone.text))
        .unwrap_or(display)
}

/// The ETag a `response` gives, if any.
fn etag(response: &Element) -> Option<String> {
    response.found(GETETAG).map(|etag| etag.text.clone())
}

/// A `sync-token` element's token.
fn token_text(token: &Element) -> String {
    token.text.trim().to_owned()
}

/// The code of a WebDAV `status`, such as `404` in `HTTP/1.1 404 Not Found`.
fn status_code(status: &Element) -> Option<&str> {
    status.text.split_whitespace().nth(1)
}

/// Reads an answer to a `PROPFIND`: its `multistatus`, or why there is none.
fn read_multistatus(response: ureq::http::Response<ureq::Body>) -> Result<Element> {
    let status = response.status().as_u16();
    let plain_text = response
        .headers()
        .get("Content-Type")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.trim_start().starts_with("text/plain"));
    let body = response.into_body().read_to_string().unwrap_or_default();
    match status {
        207 => parse(&body)
            .filter(|root| root.is(DAV, "multistatus"))
            .ok_or(CoreError::NotCalDavServer),
        401 => Err(CoreError::WrongCredentials),
        200..=299 => Err(CoreError::NotCalDavServer),
        _ => Err(CoreError::ServerRefused {
            status,
            message: if plain_text {
                plain_message(&body)
            } else {
                String::new()
            },
        }),
    }
}

/// Whether a redirect from `from` to `to` stays with the server the user
/// named, so that the password may go along: the same host and port, or the
/// same host moving from `http` to `https` on their usual ports.
fn same_server(from: &Url, to: &Url) -> bool {
    from.host_str() == to.host_str()
        && ((from.scheme() == to.scheme() && from.port() == to.port())
            || (from.scheme() == "http"
                && to.scheme() == "https"
                && from.port().is_none()
                && to.port().is_none()))
}

/// The URL that `href`, from an answer from `base`, names: one FreeCal may
/// send the password to. That rules out other schemes, URLs carrying a login,
/// and moving from `https` to `http`. Unlike a redirect, it may name another
/// host, as iCloud's calendar homes do: the server it comes from has the
/// password already.
fn follow_href(base: &Url, href: &str) -> Option<Url> {
    base.join(href.trim()).ok().filter(|url| {
        matches!(url.scheme(), "http" | "https")
            && !(base.scheme() == "https" && url.scheme() == "http")
            && url.username().is_empty()
            && url.password().is_none()
    })
}

/// The Calendar a `response` of a calendar home describes, if it is one that
/// takes Events.
fn available_calendar(home: &Url, response: &Element) -> Option<AvailableCalendar> {
    let url = follow_href(home, &response.child(DAV, "href")?.text)?;
    response.found(RESOURCETYPE)?.child(CALDAV, "calendar")?;
    // Without the property, a Calendar takes every kind of entry.
    if let Some(components) = response.found(SUPPORTED_CALENDAR_COMPONENT_SET)
        && !components.children(CALDAV, "comp").any(|component| {
            component
                .attribute("name")
                .is_some_and(|name| name.eq_ignore_ascii_case("VEVENT"))
        })
    {
        return None;
    }
    let name = response
        .found(DISPLAYNAME)
        .map(|name| name.text.trim())
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| last_segment(&url));
    let colour = response
        .found(CALENDAR_COLOR)
        .and_then(|colour| calendar_colour(&colour.text));
    Some(AvailableCalendar {
        url: url.to_string(),
        name,
        colour,
    })
}

/// The path of the member of the Calendar at `calendar` that a `response`
/// of a `PROPFIND` of it describes, if it is one that may hold an Event: not
/// the Calendar itself or another collection, and on the same server.
fn member(calendar: &Url, response: &Element) -> Option<String> {
    let url = follow_href(calendar, &response.child(DAV, "href")?.text)?;
    let collection = response
        .found(RESOURCETYPE)
        .is_some_and(|kind| kind.child(DAV, "collection").is_some());
    (url != *calendar && !collection && url.origin() == calendar.origin())
        .then(|| url.path().to_owned())
}

/// The last part of a URL's path, decoded, to name a Calendar without a name.
fn last_segment(url: &Url) -> String {
    url.path_segments()
        .and_then(|mut segments| segments.rfind(|segment| !segment.is_empty()))
        .map(|segment| {
            percent_decode_str(segment)
                .decode_utf8_lossy()
                .trim()
                .to_owned()
        })
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| url.to_string())
}

/// A Calendar's colour as servers keep it, `#rrggbb` or `#rrggbbaa`; the
/// alpha is left out.
fn calendar_colour(text: &str) -> Option<Colour> {
    let text = text.trim();
    let alpha = text.get(7..).unwrap_or_default();
    if !matches!(alpha.len(), 0 | 2) || !alpha.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Colour::parse(text.get(..7)?).ok()
}

/// A failure before the server answered: being offline, or a secure
/// connection that couldn't be made.
fn transport_error(error: ureq::Error) -> CoreError {
    match error {
        ureq::Error::Tls(_) | ureq::Error::NativeTls(_) | ureq::Error::Der(_) => {
            CoreError::SecureConnectionFailed(error.to_string())
        }
        ureq::Error::Protocol(_) | ureq::Error::Http(_) => CoreError::NotCalDavServer,
        error => CoreError::ServerUnreachable(error.to_string()),
    }
}

/// A server's own text, made safe to pass on: control and bidi formatting
/// characters become spaces, whitespace runs one space, and it is cut to
/// [`MAX_MESSAGE`] characters. It stays untrusted text.
fn plain_message(text: &str) -> String {
    let spaced: String = text
        .chars()
        .map(|c| match c {
            '\u{200e}'
            | '\u{200f}'
            | '\u{061c}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}' => ' ',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    spaced
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_MESSAGE)
        .collect()
}

/// A `PROPFIND` request body asking for `properties`.
fn propfind_body(properties: &[Property]) -> Vec<u8> {
    let mut writer = Writer::new(Vec::new());
    writer
        .create_element("propfind")
        .with_attribute(("xmlns", DAV))
        .write_inner_content(|writer| {
            writer
                .create_element("prop")
                .write_inner_content(|writer| {
                    for (index, property) in properties.iter().enumerate() {
                        // Each property declares its own namespace on a
                        // prefix of its own.
                        let prefix = format!("p{index}");
                        writer
                            .create_element(format!("{prefix}:{}", property.name))
                            .with_attribute((
                                format!("xmlns:{prefix}").as_str(),
                                property.namespace,
                            ))
                            .write_empty()?;
                    }
                    Ok(())
                })?;
            Ok(())
        })
        .expect("writing to memory doesn't fail");
    let mut body = br#"<?xml version="1.0" encoding="utf-8"?>"#.to_vec();
    body.extend(writer.into_inner());
    body
}

/// A `calendar-multiget` request body (RFC 4791) asking for the iCalendar
/// objects at `hrefs`.
fn calendar_multiget_body(hrefs: &[String]) -> Vec<u8> {
    let mut writer = Writer::new(Vec::new());
    writer
        .create_element("C:calendar-multiget")
        .with_attribute(("xmlns:D", DAV))
        .with_attribute(("xmlns:C", CALDAV))
        .write_inner_content(|writer| {
            writer
                .create_element("D:prop")
                .write_inner_content(|writer| {
                    writer.create_element("D:getetag").write_empty()?;
                    writer.create_element("C:calendar-data").write_empty()?;
                    Ok(())
                })?;
            for href in hrefs {
                writer
                    .create_element("D:href")
                    .write_text_content(BytesText::new(href))?;
            }
            Ok(())
        })
        .expect("writing to memory doesn't fail");
    let mut body = br#"<?xml version="1.0" encoding="utf-8"?>"#.to_vec();
    body.extend(writer.into_inner());
    body
}

/// A `sync-collection` request body (RFC 6578) asking for the ETags of the
/// members changed since `token`.
fn sync_collection_body(token: &str) -> Vec<u8> {
    let mut writer = Writer::new(Vec::new());
    writer
        .create_element("D:sync-collection")
        .with_attribute(("xmlns:D", DAV))
        .write_inner_content(|writer| {
            writer
                .create_element("D:sync-token")
                .write_text_content(BytesText::new(token))?;
            writer
                .create_element("D:sync-level")
                .write_text_content(BytesText::new("1"))?;
            writer
                .create_element("D:prop")
                .write_inner_content(|writer| {
                    writer.create_element("D:getetag").write_empty()?;
                    Ok(())
                })?;
            Ok(())
        })
        .expect("writing to memory doesn't fail");
    let mut body = br#"<?xml version="1.0" encoding="utf-8"?>"#.to_vec();
    body.extend(writer.into_inner());
    body
}

/// An XML element as read from an answer, with its namespace resolved.
#[derive(Debug, Default, PartialEq)]
struct Element {
    namespace: String,
    name: String,
    /// Its text, without that of its children.
    text: String,
    /// Its attributes without a namespace prefix, by name.
    attributes: Vec<(String, String)>,
    children: Vec<Element>,
}

impl Element {
    fn is(&self, namespace: &str, name: &str) -> bool {
        self.namespace == namespace && self.name == name
    }

    fn child(&self, namespace: &str, name: &str) -> Option<&Element> {
        self.children.iter().find(|child| child.is(namespace, name))
    }

    fn children<'a>(
        &'a self,
        namespace: &'a str,
        name: &'a str,
    ) -> impl Iterator<Item = &'a Element> {
        self.children
            .iter()
            .filter(move |child| child.is(namespace, name))
    }

    fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// In a `multistatus`, the first value of `property` that a response
    /// found.
    fn property(&self, property: Property) -> Option<&Element> {
        self.children(DAV, "response")
            .find_map(|response| response.found(property))
    }

    /// In a `response`, the value of `property` if the server found it
    /// (status 200).
    fn found(&self, property: Property) -> Option<&Element> {
        self.children(DAV, "propstat")
            .filter(|propstat| {
                propstat
                    .child(DAV, "status")
                    .is_some_and(|status| status_code(status) == Some("200"))
            })
            .filter_map(|propstat| propstat.child(DAV, "prop"))
            .find_map(|prop| prop.child(property.namespace, property.name))
    }
}

/// Reads an answer into its root element, or nothing if it isn't
/// well-formed XML.
fn parse(xml: &str) -> Option<Element> {
    let mut reader = NsReader::from_str(xml);
    let mut open: Vec<Element> = Vec::new();
    let element = |namespace: ResolveResult, start: &BytesStart| {
        let mut attributes = Vec::new();
        for attribute in start.attributes() {
            let attribute = attribute.ok()?;
            if attribute.key.prefix().is_none() && attribute.key.as_ref() != b"xmlns" {
                attributes.push((
                    String::from_utf8_lossy(attribute.key.local_name().as_ref()).into_owned(),
                    attribute.unescape_value().ok()?.into_owned(),
                ));
            }
        }
        Some(Element {
            namespace: match namespace {
                ResolveResult::Bound(Namespace(namespace)) => {
                    String::from_utf8_lossy(namespace).into_owned()
                }
                _ => String::new(),
            },
            name: String::from_utf8_lossy(start.local_name().as_ref()).into_owned(),
            attributes,
            ..Element::default()
        })
    };
    loop {
        let (namespace, event) = reader.read_resolved_event().ok()?;
        match event {
            XmlEvent::Start(start) => open.push(element(namespace, &start)?),
            XmlEvent::Empty(empty) => {
                let child = element(namespace, &empty)?;
                match open.last_mut() {
                    Some(parent) => parent.children.push(child),
                    None => return Some(child),
                }
            }
            XmlEvent::End(_) => {
                let closed = open.pop()?;
                match open.last_mut() {
                    Some(parent) => parent.children.push(closed),
                    None => return Some(closed),
                }
            }
            // Text outside the root element, such as the line break after
            // the XML declaration, is left out.
            XmlEvent::Text(text) => {
                if let Some(parent) = open.last_mut() {
                    parent.text.push_str(&text.decode().ok()?);
                }
            }
            XmlEvent::CData(data) => {
                if let Some(parent) = open.last_mut() {
                    parent.text.push_str(&data.decode().ok()?);
                }
            }
            // Only character references and XML's five entities: an answer
            // can't declare entities of its own.
            XmlEvent::GeneralRef(reference) => {
                let parent = open.last_mut()?;
                match reference.resolve_char_ref().ok()? {
                    Some(character) => parent.text.push(character),
                    None => parent.text.push_str(quick_xml::escape::resolve_xml_entity(
                        &reference.decode().ok()?,
                    )?),
                }
            }
            XmlEvent::Eof => return None,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_propfind_body_names_each_property_in_its_namespace() {
        let body = String::from_utf8(propfind_body(&[CURRENT_USER_PRINCIPAL])).unwrap();

        let root = parse(&body).unwrap();

        assert!(root.is(DAV, "propfind"));
        let prop = root.child(DAV, "prop").unwrap();
        assert_eq!(prop.children.len(), 1);
        assert!(prop.children[0].is(DAV, "current-user-principal"));
    }

    #[test]
    fn answers_are_read_by_namespace_whatever_their_prefixes() {
        for answer in [
            r#"<d:multistatus xmlns:d="DAV:"><d:response><d:href>/</d:href><d:propstat>
               <d:prop><d:current-user-principal><d:href>/alice/</d:href></d:current-user-principal></d:prop>
               <d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response></d:multistatus>"#,
            r#"<multistatus xmlns="DAV:"><response><href>/</href><propstat>
               <prop><current-user-principal><href>/alice/</href></current-user-principal></prop>
               <status>HTTP/1.1 200 OK</status></propstat></response></multistatus>"#,
        ] {
            let root = parse(answer).unwrap();

            let principal = root.property(CURRENT_USER_PRINCIPAL).unwrap();

            assert_eq!(principal.child(DAV, "href").unwrap().text, "/alice/");
        }
    }

    #[test]
    fn an_answer_may_start_with_an_xml_declaration() {
        let root = parse("<?xml version='1.0' encoding='utf-8'?>\n<multistatus xmlns=\"DAV:\"/>\n")
            .unwrap();

        assert!(root.is(DAV, "multistatus"));
    }

    #[test]
    fn a_property_the_server_didnt_find_isnt_read() {
        let root = parse(
            r#"<multistatus xmlns="DAV:"><response><href>/</href><propstat>
               <prop><current-user-principal><href>/alice/</href></current-user-principal></prop>
               <status>HTTP/1.1 404 Not Found</status></propstat></response></multistatus>"#,
        )
        .unwrap();

        assert_eq!(root.property(CURRENT_USER_PRINCIPAL), None);
    }

    #[test]
    fn the_same_name_in_another_namespace_isnt_the_property() {
        let root = parse(
            r#"<multistatus xmlns="DAV:" xmlns:x="urn:other"><response><href>/</href><propstat>
               <prop><x:current-user-principal><href>/alice/</href></x:current-user-principal></prop>
               <status>HTTP/1.1 200 OK</status></propstat></response></multistatus>"#,
        )
        .unwrap();

        assert_eq!(root.property(CURRENT_USER_PRINCIPAL), None);
    }

    #[test]
    fn text_is_unescaped() {
        let root = parse("<a xmlns='DAV:'>x &amp; y &#x3c;z&gt;</a>").unwrap();

        assert_eq!(root.text, "x & y <z>");
    }

    #[test]
    fn a_propfind_body_declares_each_propertys_own_namespace() {
        let body = String::from_utf8(propfind_body(&[DISPLAYNAME, CALENDAR_COLOR])).unwrap();

        let root = parse(&body).unwrap();

        let prop = root.child(DAV, "prop").unwrap();
        assert!(prop.children[0].is(DAV, "displayname"));
        assert!(prop.children[1].is(APPLE_ICAL, "calendar-color"));
    }

    #[test]
    fn attributes_are_read_unescaped_without_namespace_declarations() {
        let root = parse(r#"<comp xmlns="urn:x" xmlns:y="urn:y" y:other="1" name="V&amp;EVENT"/>"#)
            .unwrap();

        assert_eq!(
            root.attributes,
            vec![("name".to_owned(), "V&EVENT".to_owned())]
        );
    }

    #[test]
    fn a_redirect_stays_on_the_same_host_and_port_or_moves_to_https() {
        let url = |text| Url::parse(text).unwrap();
        let from = url("http://dav.example.org/");

        assert!(same_server(&from, &url("http://dav.example.org/dav/")));
        assert!(same_server(&from, &url("https://dav.example.org/dav/")));
        assert!(!same_server(&from, &url("http://evil.example.org/dav/")));
        assert!(!same_server(&from, &url("http://dav.example.org:8080/")));
        assert!(!same_server(
            &url("https://dav.example.org/"),
            &url("http://dav.example.org/")
        ));
    }

    #[test]
    fn an_href_never_leads_to_another_scheme_a_login_or_from_https_to_http() {
        let base = Url::parse("https://dav.example.org/dav/").unwrap();

        assert_eq!(
            follow_href(&base, " calendars/alice/ ").map(String::from),
            Some("https://dav.example.org/dav/calendars/alice/".to_owned())
        );
        assert!(follow_href(&base, "https://p01-caldav.example.com/home/").is_some());
        assert!(follow_href(&base, "http://dav.example.org/").is_none());
        assert!(follow_href(&base, "file:///etc/passwd").is_none());
        assert!(follow_href(&base, "javascript:alert(1)").is_none());
        assert!(follow_href(&base, "https://alice:pw@dav.example.org/").is_none());
    }

    #[test]
    fn calendar_colours_may_have_alpha_but_must_be_colours() {
        let colour = |text: &str| calendar_colour(text).map(|colour| colour.as_str().to_owned());

        assert_eq!(colour("#3366CC"), Some("#3366cc".to_owned()));
        assert_eq!(colour(" #3366ccff\n"), Some("#3366cc".to_owned()));
        assert_eq!(colour("#3366ccf"), None);
        assert_eq!(colour("#3366ccgg"), None);
        assert_eq!(colour("red"), None);
        assert_eq!(colour("#33é6cc"), None);
        assert_eq!(colour(""), None);
    }

    #[test]
    fn a_calendar_without_a_name_takes_the_last_part_of_its_address() {
        let url = |text| Url::parse(text).unwrap();

        assert_eq!(
            last_segment(&url("https://x.org/cal/my%20work/")),
            "my work"
        );
        assert_eq!(last_segment(&url("https://x.org/cal/home")), "home");
        assert_eq!(last_segment(&url("https://x.org/")), "https://x.org/");
    }

    /// An Event's original server data is kept as received (ADR 0004),
    /// line breaks included.
    #[test]
    fn calendar_data_keeps_its_line_breaks_and_references() {
        let answer = parse(
            "<multistatus xmlns=\"DAV:\" xmlns:C=\"urn:ietf:params:xml:ns:caldav\">\
             <response><href>/a.ics</href><propstat><prop>\
             <C:calendar-data>BEGIN:VCALENDAR\r\nX-A:&lt;b&gt;&#13;\nEND:VCALENDAR\r\n\
             </C:calendar-data></prop><status>HTTP/1.1 200 OK</status></propstat>\
             </response></multistatus>",
        )
        .unwrap();

        assert_eq!(
            answer.property(CALENDAR_DATA).unwrap().text,
            "BEGIN:VCALENDAR\r\nX-A:<b>\r\nEND:VCALENDAR\r\n"
        );
    }

    #[test]
    fn a_calendar_multiget_body_names_each_href_as_text() {
        let hrefs = ["/alice/work/a.ics".to_owned(), "/x/<b>&amp;.ics".to_owned()];

        let body = String::from_utf8(calendar_multiget_body(&hrefs)).unwrap();

        let root = parse(&body).unwrap();
        assert!(root.is(CALDAV, "calendar-multiget"));
        let prop = root.child(DAV, "prop").unwrap();
        assert!(prop.children[0].is(DAV, "getetag"));
        assert!(prop.children[1].is(CALDAV, "calendar-data"));
        let named: Vec<_> = root
            .children(DAV, "href")
            .map(|href| href.text.as_str())
            .collect();
        assert_eq!(named, hrefs);
    }

    #[test]
    fn a_sync_collection_body_names_the_token_as_text() {
        let token = "http://x.org/sync/<1>&2";

        let body = String::from_utf8(sync_collection_body(token)).unwrap();

        let root = parse(&body).unwrap();
        assert!(root.is(DAV, "sync-collection"));
        assert_eq!(root.child(DAV, "sync-token").unwrap().text, token);
        assert_eq!(root.child(DAV, "sync-level").unwrap().text, "1");
        let prop = root.child(DAV, "prop").unwrap();
        assert_eq!(prop.children.len(), 1);
        assert!(prop.children[0].is(DAV, "getetag"));
    }

    #[test]
    fn a_calendars_members_leave_out_itself_collections_and_other_servers() {
        let calendar = Url::parse("https://dav.example.org/alice/work/").unwrap();
        let answer = parse(
            r#"<multistatus xmlns="DAV:">
               <response><href>/alice/work/</href><propstat><prop><resourcetype>
                 <collection/></resourcetype></prop><status>HTTP/1.1 200 OK</status></propstat></response>
               <response><href>/alice/work/inner/</href><propstat><prop><resourcetype>
                 <collection/></resourcetype></prop><status>HTTP/1.1 200 OK</status></propstat></response>
               <response><href>a%20b.ics</href><propstat><prop><resourcetype/></prop>
                 <status>HTTP/1.1 200 OK</status></propstat></response>
               <response><href>https://evil.example.org/x.ics</href></response>
               <response><href>http://dav.example.org/alice/work/c.ics</href></response>
               <response><href>/alice/work/d.ics</href></response>
               </multistatus>"#,
        )
        .unwrap();

        let members: Vec<_> = answer
            .children(DAV, "response")
            .filter_map(|response| member(&calendar, response))
            .collect();

        assert_eq!(members, ["/alice/work/a%20b.ics", "/alice/work/d.ics"]);
    }

    #[test]
    fn malformed_xml_is_nothing() {
        assert_eq!(parse("<a><b></a>"), None);
        assert_eq!(parse("<html>Welcome"), None);
    }
}
