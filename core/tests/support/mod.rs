//! Test harness for the core application interface: a real SQLite Local Store
//! in a temporary directory, a controllable clock, a recording Signal
//! subscriber, a recording keyring and a real Radicale server.

#![allow(dead_code)]

pub mod radicale;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, Utc};
use chrono_tz::Tz;
use freecal_core::{
    Calendar, CalendarId, Clock, Colour, Core, CoreError, EventDraft, Keyring, KeyringError,
    Signal, SignalSink, When,
};
use tempfile::TempDir;

pub struct Harness {
    data_dir: TempDir,
    pub clock: Arc<TestClock>,
    pub signals: Arc<RecordingSignals>,
    pub keyring: Arc<RecordingKeyring>,
}

impl Harness {
    pub fn new() -> Self {
        Self {
            data_dir: TempDir::new().expect("create temporary data directory"),
            clock: Arc::new(TestClock::at("2026-10-03T12:00:00+02:00")),
            signals: Arc::new(RecordingSignals::default()),
            keyring: Arc::new(RecordingKeyring::default()),
        }
    }

    /// Opens the core on this harness's data directory, like launching FreeCal.
    /// Opening again on the same harness is a relaunch on the same Local Store.
    pub fn open(&self) -> Core {
        self.try_open().expect("open core")
    }

    /// Like [`Harness::open`], for tests where launching is expected to fail.
    pub fn try_open(&self) -> Result<Core, CoreError> {
        Core::open(
            self.data_dir.path(),
            self.clock.clone(),
            self.signals.clone(),
            self.keyring.clone(),
        )
    }

    /// Like [`Harness::open`], on a Local Store without the Calendars a new
    /// one starts with, and with no Signals recorded yet.
    pub fn open_without_starting_calendars(&self) -> Core {
        let core = self.open();
        for calendar in core.list_calendars().expect("list calendars") {
            core.delete_calendar(calendar.id).expect("delete calendar");
        }
        self.signals.clear();
        core
    }

    /// Arranges the Local Store's state before a launch by running `sql`
    /// directly on its file. Only for putting the file into a state the
    /// interface can't produce; tests verify through the interface, never here.
    pub fn arrange_local_store(&self, sql: &str) {
        self.try_arrange_local_store(sql)
            .expect("arrange the Local Store");
    }

    /// Like [`Harness::arrange_local_store`], for SQL the Local Store is
    /// expected to refuse.
    pub fn try_arrange_local_store(&self, sql: &str) -> rusqlite::Result<()> {
        let path = self.data_dir.path().join("freecal.sqlite3");
        rusqlite::Connection::open(path).and_then(|conn| conn.execute_batch(sql))
    }
}

impl Harness {
    /// Whether any file in the data directory holds `bytes`, as written to
    /// disk so far.
    pub fn data_dir_contains(&self, bytes: &[u8]) -> bool {
        std::fs::read_dir(self.data_dir.path())
            .unwrap()
            .map(|entry| std::fs::read(entry.unwrap().path()).unwrap())
            .any(|file| file.windows(bytes.len()).any(|window| window == bytes))
    }
}

/// A clock that stands still where a test puts it, in a system time zone the
/// test chooses (Europe/Berlin unless it chooses one).
pub struct TestClock {
    now: Mutex<DateTime<Utc>>,
    zone: Mutex<Tz>,
}

impl TestClock {
    pub fn at(rfc3339: &str) -> Self {
        Self {
            now: Mutex::new(parse(rfc3339).with_timezone(&Utc)),
            zone: Mutex::new(Tz::Europe__Berlin),
        }
    }

    pub fn set(&self, rfc3339: &str) {
        *self.now.lock().unwrap() = parse(rfc3339).with_timezone(&Utc);
    }

    /// Changes the system time zone, as when travelling.
    pub fn set_time_zone(&self, name: &str) {
        *self.zone.lock().unwrap() = name.parse().expect("a known time zone");
    }
}

impl Clock for TestClock {
    fn now(&self) -> DateTime<FixedOffset> {
        let zone = *self.zone.lock().unwrap();
        self.now.lock().unwrap().with_timezone(&zone).fixed_offset()
    }

    fn time_zone(&self) -> Tz {
        *self.zone.lock().unwrap()
    }
}

fn parse(rfc3339: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(rfc3339).expect("valid RFC 3339 time")
}

#[derive(Default)]
pub struct RecordingSignals {
    received: Mutex<Vec<Signal>>,
}

impl RecordingSignals {
    pub fn received(&self) -> Vec<Signal> {
        self.received.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.received.lock().unwrap().clear();
    }
}

impl SignalSink for RecordingSignals {
    fn send(&self, signal: Signal) {
        self.received.lock().unwrap().push(signal);
    }
}

/// A keyring that keeps secrets in memory, or fails like a system without a
/// working keyring once broken.
#[derive(Default)]
pub struct RecordingKeyring {
    secrets: Mutex<BTreeMap<String, String>>,
    broken: AtomicBool,
}

impl RecordingKeyring {
    /// The secrets it holds, in the order of their keys.
    pub fn secrets(&self) -> Vec<String> {
        self.secrets.lock().unwrap().values().cloned().collect()
    }

    /// Deletes every secret, as a user can in an app that shows the keyring.
    pub fn forget_all(&self) {
        self.secrets.lock().unwrap().clear();
    }

    /// Makes every further request fail, as without a keyring.
    pub fn break_it(&self) {
        self.broken.store(true, Ordering::Relaxed);
    }

    fn working(&self) -> Result<(), KeyringError> {
        match self.broken.load(Ordering::Relaxed) {
            true => Err(KeyringError("no keyring in this test".to_owned())),
            false => Ok(()),
        }
    }
}

impl Keyring for RecordingKeyring {
    fn store(&self, key: &str, _label: &str, secret: &str) -> Result<(), KeyringError> {
        self.working()?;
        self.secrets
            .lock()
            .unwrap()
            .insert(key.to_owned(), secret.to_owned());
        Ok(())
    }

    fn secret(&self, key: &str) -> Result<Option<String>, KeyringError> {
        self.working()?;
        Ok(self.secrets.lock().unwrap().get(key).cloned())
    }

    fn delete(&self, key: &str) -> Result<(), KeyringError> {
        self.working()?;
        self.secrets.lock().unwrap().remove(key);
        Ok(())
    }
}

/// A new Local Calendar.
pub fn calendar(core: &Core, name: &str) -> Calendar {
    let local = core.list_accounts().unwrap()[0].id;
    core.create_calendar(local, name, Colour::parse("#3366cc").unwrap())
        .unwrap()
}

// Events and their times, written as text.

pub fn date(text: &str) -> NaiveDate {
    text.parse().unwrap()
}

pub fn time(text: &str) -> NaiveDateTime {
    text.parse().unwrap()
}

pub fn timed(start: &str, end: &str) -> When {
    When::Timed {
        start: time(start),
        end: time(end),
    }
}

pub fn all_day(start: &str, end: &str) -> When {
    When::AllDay {
        start: date(start),
        end: date(end),
    }
}

pub fn draft(calendar_id: CalendarId, title: &str, when: When) -> EventDraft {
    EventDraft {
        calendar_id,
        title: title.to_owned(),
        when,
        location: String::new(),
        description: String::new(),
        repeat: None,
    }
}

// Files to import, written as text.

/// An `.ics` file with `body` between `BEGIN:VCALENDAR` and `END:VCALENDAR`,
/// one property per line.
pub fn ics(body: &str) -> Vec<u8> {
    format!("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Test//EN\r\n{body}END:VCALENDAR\r\n")
        .replace("\r\n", "\n")
        .replace('\n', "\r\n")
        .into_bytes()
}

/// A `VEVENT` whose title is its UID, with `start` and `end` as whole lines.
pub fn event(uid: &str, start: &str, end: &str) -> String {
    format!("BEGIN:VEVENT\nUID:{uid}\nSUMMARY:{uid}\n{start}\n{end}\nEND:VEVENT\n")
}
