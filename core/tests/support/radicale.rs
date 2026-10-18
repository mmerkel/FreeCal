//! A real Radicale CalDAV server, started once per test run, and stand-ins
//! for servers that misbehave in ways Radicale doesn't.
//!
//! Radicale must be installed for `python3` (`pip install --user radicale`),
//! or for the Python that `FREECAL_RADICALE_PYTHON` names.

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Mutex, OnceLock};

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
        std::fs::write(dir.path().join("users"), "").unwrap();
        let config = dir.path().join("config");
        std::fs::write(
            &config,
            format!(
                "[server]\nhosts = 127.0.0.1:0\n\
                 [auth]\ntype = htpasswd\nhtpasswd_filename = {users}\n\
                 htpasswd_encryption = plain\ndelay = 0\n\
                 [rights]\ntype = owner_only\n\
                 [storage]\nfilesystem_folder = {storage}\n\
                 [logging]\nlevel = info\n",
                users = dir.path().join("users").display(),
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

/// Reads a whole request, so that closing the connection after answering
/// doesn't reset it.
fn read_request(stream: &mut impl Read) {
    let mut reader = BufReader::new(stream);
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
}

/// The URL of a server that isn't there: nothing listens on its port.
pub fn closed_port_url() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    drop(listener);
    url
}
