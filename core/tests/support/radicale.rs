//! A real Radicale CalDAV server, started once per test run, and stand-ins
//! for servers that misbehave in ways Radicale doesn't.
//!
//! Radicale must be installed for `python3` (`pip install --user radicale`),
//! or for the Python that `FREECAL_RADICALE_PYTHON` names.

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};

use base64::Engine;
use quick_xml::Writer;
use tempfile::TempDir;

/// Runs Radicale and exits as soon as its standard input closes, which
/// happens when the test run ends, however it ends. So no server outlives
/// the run.
const RUN_RADICALE: &str = "
import os, sys, threading, runpy
threading.Thread(target=lambda: (sys.stdin.read(), os._exit(0)), daemon=True).start()
sys.argv = ['radicale'] + sys.argv[1:]
runpy.run_module('radicale', run_name='__main__')
";

pub struct Radicale {
    dir: TempDir,
    url: String,
    /// How many users there are. Held while adding one, so that tests
    /// adding users at the same time don't mix up their lines.
    users: Mutex<usize>,
    /// Kept open: closing it stops the server.
    _stdin: Mutex<ChildStdin>,
    _process: Child,
}

/// A Radicale user, with a password of its own.
pub struct User {
    pub name: String,
    pub password: String,
}

impl Radicale {
    /// The server of this test run, started by the first test that needs it.
    /// Tests share it and keep apart by each using users of their own.
    pub fn shared() -> &'static Radicale {
        static SHARED: OnceLock<Radicale> = OnceLock::new();
        SHARED.get_or_init(Radicale::start)
    }

    fn start() -> Self {
        let dir = TempDir::new().expect("create Radicale's directory");
        std::fs::write(
            dir.path().join("users"),
            format!("{ADMIN}:{ADMIN_PASSWORD}\n"),
        )
        .unwrap();
        std::fs::write(dir.path().join("rights"), RIGHTS).unwrap();
        let config = dir.path().join("config");
        std::fs::write(
            &config,
            format!(
                "[server]\nhosts = 127.0.0.1:0\n\
                 [auth]\ntype = htpasswd\nhtpasswd_filename = {users}\n\
                 htpasswd_encryption = plain\ndelay = 0\n\
                 [rights]\ntype = from_file\nfile = {rights}\n\
                 [storage]\nfilesystem_folder = {storage}\n\
                 [logging]\nlevel = info\n",
                users = dir.path().join("users").display(),
                rights = dir.path().join("rights").display(),
                storage = dir.path().join("storage").display(),
            ),
        )
        .unwrap();
        let python =
            std::env::var("FREECAL_RADICALE_PYTHON").unwrap_or_else(|_| "python3".to_owned());
        let mut process = Command::new(&python)
            .args(["-c", RUN_RADICALE, "--config"])
            .arg(&config)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| panic!("start Radicale with {python}: {error}"));
        let stdin = process.stdin.take().unwrap();
        let mut log = BufReader::new(process.stderr.take().unwrap()).lines();
        // Radicale chose a free port and names it as it starts listening.
        let address = loop {
            let line = log.next().and_then(Result::ok).unwrap_or_else(|| {
                panic!(
                    "Radicale didn't start; is it installed for {python}? \
                         (pip install --user radicale)"
                )
            });
            if let Some((_, rest)) = line.split_once("Listening on '") {
                break rest.split('\'').next().unwrap().to_owned();
            }
        };
        // Keep reading its log, so that Radicale never blocks writing it.
        std::thread::spawn(move || log.for_each(drop));
        Self {
            dir,
            url: format!("http://{address}/"),
            users: Mutex::new(0),
            _stdin: Mutex::new(stdin),
            _process: process,
        }
    }

    /// The server's root, which is also its CalDAV context path.
    pub fn url(&self) -> String {
        self.url.clone()
    }

    /// A new user, who can log in straight away.
    pub fn user(&self) -> User {
        let mut users = self.users.lock().unwrap();
        *users += 1;
        let user = User {
            name: format!("user{users}"),
            password: format!("secret-of-user{users}"),
        };
        OpenOptions::new()
            .append(true)
            .open(self.dir.path().join("users"))
            .unwrap()
            .write_all(format!("{}:{}\n", user.name, user.password).as_bytes())
            .unwrap();
        user
    }
}

/// What a Calendar created on the server is like.
pub struct NewCalendar<'a> {
    /// Its name; none leaves the server's `displayname` unset.
    pub name: Option<&'a str>,
    pub colour: Option<&'a str>,
    /// The components it takes, such as `VEVENT`.
    pub components: &'a [&'a str],
    /// Its `calendar-timezone` (RFC 4791): an iCalendar object with one
    /// `VTIMEZONE`.
    pub timezone: Option<&'a str>,
}

impl Default for NewCalendar<'_> {
    fn default() -> Self {
        Self {
            name: None,
            colour: None,
            components: &["VEVENT"],
            timezone: None,
        }
    }
}

impl Radicale {
    /// Creates a Calendar for `user` on the server, as another app would,
    /// and returns its URL.
    pub fn create_calendar(&self, user: &User, calendar: NewCalendar) -> String {
        let url = format!(
            "{}{}/{}/",
            self.url,
            user.name,
            uuid::Uuid::new_v4().simple()
        );
        let status = self.request(user, "MKCALENDAR", &url, XML, mkcalendar_body(&calendar));
        assert_eq!(status, 201, "create a Calendar on Radicale");
        url
    }

    /// Creates a Calendar for `user` on the server that they may only read,
    /// and returns its URL.
    pub fn create_read_only_calendar(&self, user: &User, calendar: NewCalendar) -> String {
        // Logging in makes the user's principal, which holds their Calendars.
        let principal = format!("{}{}/", self.url, user.name);
        let status = self.request(user, "PROPFIND", &principal, XML, Vec::new());
        assert_eq!(status, 207, "log in to Radicale");
        let url = format!("{principal}read-only-{}/", uuid::Uuid::new_v4().simple());
        let admin = User {
            name: ADMIN.to_owned(),
            password: ADMIN_PASSWORD.to_owned(),
        };
        let status = self.request(&admin, "MKCALENDAR", &url, XML, mkcalendar_body(&calendar));
        assert_eq!(status, 201, "create a read-only Calendar on Radicale");
        url
    }

    /// Puts an Event, an iCalendar object, into a Calendar of `user` on the
    /// server, as another app would, under the name `name`, replacing any
    /// it holds there.
    pub fn put_event(&self, user: &User, calendar_url: &str, name: &str, ics: Vec<u8>) {
        let url = format!("{calendar_url}{name}.ics");
        let status = self.request(user, "PUT", &url, "text/calendar; charset=utf-8", ics);
        assert!(
            matches!(status, 201 | 204),
            "put an Event on Radicale: {status}"
        );
    }

    /// Writes an Event, an iCalendar object, straight into Radicale's
    /// storage as the object `name` of the Calendar at `calendar_url`,
    /// replacing any it holds there. So it holds what a server that checks
    /// less would take and Radicale refuses, such as a second object with a
    /// UID, or an object whose UID changed.
    pub fn write_event_file(&self, calendar_url: &str, name: &str, ics: Vec<u8>) {
        let path = calendar_url
            .strip_prefix(&self.url)
            .expect("a Calendar on this server");
        let file = self
            .dir
            .path()
            .join("storage/collection-root")
            .join(path)
            .join(format!("{name}.ics"));
        std::fs::write(file, ics).expect("write into Radicale's storage");
    }

    /// The ETag and the iCalendar object of the Event `name` in a Calendar
    /// of `user`, as the server serves it.
    pub fn get_event(&self, user: &User, calendar_url: &str, name: &str) -> (String, String) {
        let url = format!("{calendar_url}{name}.ics");
        let mut response = self.send(user, "GET", &url, "text/plain", Vec::new());
        assert_eq!(response.status(), 200, "get an Event from Radicale");
        let etag = response.headers()["ETag"].to_str().unwrap().to_owned();
        (etag, response.body_mut().read_to_string().unwrap())
    }

    /// Deletes the Event `name` from a Calendar of `user` on the server, as
    /// another app would.
    pub fn delete_event(&self, user: &User, calendar_url: &str, name: &str) {
        let url = format!("{calendar_url}{name}.ics");
        let status = self.request(user, "DELETE", &url, XML, Vec::new());
        assert_eq!(status, 200, "delete an Event on Radicale");
    }

    /// Deletes a Calendar of `user` on the server.
    pub fn delete_calendar(&self, user: &User, url: &str) {
        let status = self.request(user, "DELETE", url, XML, Vec::new());
        assert_eq!(status, 200, "delete a Calendar on Radicale");
    }

    fn request(
        &self,
        user: &User,
        method: &str,
        url: &str,
        content_type: &str,
        body: Vec<u8>,
    ) -> u16 {
        self.send(user, method, url, content_type, body)
            .status()
            .as_u16()
    }

    fn send(
        &self,
        user: &User,
        method: &str,
        url: &str,
        content_type: &str,
        body: Vec<u8>,
    ) -> ureq::http::Response<ureq::Body> {
        let credentials = base64::engine::general_purpose::STANDARD
            .encode(format!("{}:{}", user.name, user.password));
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .allow_non_standard_methods(true)
            .build()
            .new_agent();
        let request = ureq::http::Request::builder()
            .method(ureq::http::Method::from_bytes(method.as_bytes()).unwrap())
            .uri(url)
            .header("Authorization", format!("Basic {credentials}"))
            .header("Content-Type", content_type)
            .body(body)
            .unwrap();
        agent.run(request).unwrap()
    }
}

/// The user who may change every Calendar, to make those another user may
/// only read.
const ADMIN: &str = "admin";
const ADMIN_PASSWORD: &str = "secret-of-admin";

/// Radicale's `owner_only` rights, except that a user may only read their
/// own Calendars whose names start with `read-only-`, and the admin may do
/// anything.
const RIGHTS: &str = "\
[admin]
user: admin
collection: .*
permissions: RWrw

[root]
user: .+
collection:
permissions: R

[principal]
user: .+
collection: {user}
permissions: RW

[read-only]
user: .+
collection: {user}/read-only-[^/]+
permissions: r

[calendars]
user: .+
collection: {user}/[^/]+
permissions: rw
";

const XML: &str = "application/xml; charset=utf-8";

/// A `MKCALENDAR` request body (RFC 4791), written with the XML writer like
/// FreeCal's own requests, so that any name stays text.
fn mkcalendar_body(calendar: &NewCalendar) -> Vec<u8> {
    let mut writer = Writer::new(Vec::new());
    writer
        .create_element("C:mkcalendar")
        .with_attribute(("xmlns:D", "DAV:"))
        .with_attribute(("xmlns:C", "urn:ietf:params:xml:ns:caldav"))
        .with_attribute(("xmlns:I", "http://apple.com/ns/ical/"))
        .write_inner_content(|writer| {
            writer
                .create_element("D:set")
                .write_inner_content(|writer| {
                    writer
                        .create_element("D:prop")
                        .write_inner_content(|writer| {
                            if let Some(name) = calendar.name {
                                writer
                                    .create_element("D:displayname")
                                    .write_text_content(quick_xml::events::BytesText::new(name))?;
                            }
                            if let Some(colour) = calendar.colour {
                                writer
                                    .create_element("I:calendar-color")
                                    .write_text_content(quick_xml::events::BytesText::new(
                                        colour,
                                    ))?;
                            }
                            if let Some(timezone) = calendar.timezone {
                                writer
                                    .create_element("C:calendar-timezone")
                                    .write_text_content(quick_xml::events::BytesText::new(
                                        timezone,
                                    ))?;
                            }
                            writer
                                .create_element("C:supported-calendar-component-set")
                                .write_inner_content(|writer| {
                                    for component in calendar.components {
                                        writer
                                            .create_element("C:comp")
                                            .with_attribute(("name", *component))
                                            .write_empty()?;
                                    }
                                    Ok(())
                                })?;
                            Ok(())
                        })?;
                    Ok(())
                })?;
            Ok(())
        })
        .unwrap();
    writer.into_inner()
}

/// A server that answers every request with `response`, a whole HTTP
/// response as it goes over the wire. Returns its URL.
pub fn http_stub(response: &str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    let response = response.to_owned();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            read_request(&mut stream);
            let _ = stream.write_all(response.as_bytes());
        }
    });
    url
}

/// A stand-in server that answers by path, from whole HTTP responses as
/// they go over the wire, and 404 for any other path. It records the
/// requests it gets.
pub struct RoutingStub {
    pub url: String,
    requests: Arc<Mutex<Vec<String>>>,
}

impl RoutingStub {
    pub fn new(routes: &[(&str, String)]) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let routes: Vec<(String, String)> = routes
            .iter()
            .map(|(path, response)| (path.to_string(), response.clone()))
            .collect();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = requests.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let request = read_request(&mut stream);
                let path = request.split(' ').nth(1).unwrap_or_default().to_owned();
                recorded.lock().unwrap().push(request);
                let response = routes
                    .iter()
                    .find(|(route, _)| *route == path)
                    .map(|(_, response)| response.as_str())
                    .unwrap_or("HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
                let _ = stream.write_all(response.as_bytes());
            }
        });
        Self { url, requests }
    }

    /// The request lines it got, such as `PROPFIND /dav/ HTTP/1.1`.
    pub fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

/// A `207 Multi-Status` answer with `body`, a WebDAV `multistatus`.
pub fn multistatus(body: &str) -> String {
    format!(
        "HTTP/1.1 207 Multi-Status\r\nContent-Type: application/xml\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

/// A redirect to `location`.
pub fn redirect(location: &str) -> String {
    format!("HTTP/1.1 301 Moved Permanently\r\nLocation: {location}\r\nContent-Length: 0\r\n\r\n")
}

/// Reads a whole request, so that closing the connection after answering
/// doesn't reset it, and returns its request line.
fn read_request(stream: &mut impl Read) -> String {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    let _ = reader.read_line(&mut request_line);
    let mut length = 0;
    let mut line = String::new();
    while reader.read_line(&mut line).is_ok_and(|read| read > 0) && line != "\r\n" {
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().unwrap_or(0);
        }
        line.clear();
    }
    let _ = reader.read_exact(&mut vec![0; length]);
    request_line.trim_end().to_owned()
}

/// The URL of a server that isn't there: nothing listens on its port.
pub fn closed_port_url() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    drop(listener);
    url
}
