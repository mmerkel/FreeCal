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
    AccountId, AccountSyncStatus, CalDavLogin, Calendar, CalendarId, CalendarSyncStatus, Clock,
    Colour, Core, CoreError, Event, EventDraft, EventId, Frequency, Keyring, KeyringError,
    Occurrence, Repeat, RepeatEnd, ServerObject, Signal, SignalSink, SyncStatus, When,
};
use tempfile::TempDir;

use radicale::{NewCalendar, Radicale, RewritingProxy, User};

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

/// What a test does with each Signal as it is sent.
type Reaction = Arc<dyn Fn(&Signal) + Send + Sync>;

#[derive(Default)]
pub struct RecordingSignals {
    received: Mutex<Vec<Signal>>,
    on_receive: Mutex<Option<Reaction>>,
}

impl RecordingSignals {
    /// Calls `react` with each Signal from now on, as a subscriber that
    /// reads the core on a Signal would.
    pub fn on_receive(&self, react: impl Fn(&Signal) + Send + Sync + 'static) {
        *self.on_receive.lock().unwrap() = Some(Arc::new(react));
    }

    pub fn received(&self) -> Vec<Signal> {
        self.received.lock().unwrap().clone()
    }

    /// The Signals received but "Sync status changed", for tests about the
    /// others.
    pub fn received_but_sync_status(&self) -> Vec<Signal> {
        self.received()
            .into_iter()
            .filter(|signal| {
                !matches!(
                    signal,
                    Signal::AccountSyncStatusChanged(_) | Signal::CalendarSyncStatusChanged(_)
                )
            })
            .collect()
    }

    pub fn clear(&self) {
        self.received.lock().unwrap().clear();
    }
}

impl SignalSink for RecordingSignals {
    fn send(&self, signal: Signal) {
        self.received.lock().unwrap().push(signal.clone());
        // Unlocked while it reacts, which may send Signals of its own.
        let react = self.on_receive.lock().unwrap().clone();
        if let Some(react) = react {
            react(&signal);
        }
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

/// An instant, written in RFC 3339.
pub fn utc(text: &str) -> DateTime<Utc> {
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

/// "Dentist" on 5 October 2026, 9:00 to 10:00 in Berlin.
pub fn dentist(calendar_id: CalendarId) -> EventDraft {
    draft(
        calendar_id,
        "Dentist",
        timed("2026-10-05T09:00:00", "2026-10-05T10:00:00"),
    )
}

pub fn weekly(end: RepeatEnd) -> Repeat {
    Repeat {
        frequency: Frequency::Weekly,
        interval: 1,
        end,
    }
}

/// Yoga on Mondays from 5 October 2026, 18:00 to 19:00 in Berlin.
pub fn yoga(core: &Core, calendar_id: CalendarId, end: RepeatEnd) -> EventId {
    let draft = EventDraft {
        repeat: Some(weekly(end)),
        ..draft(
            calendar_id,
            "Yoga",
            timed("2026-10-05T18:00:00", "2026-10-05T19:00:00"),
        )
    };
    core.create_event(draft).unwrap().value.id
}

/// The one Occurrence of a shown Calendar on `date`.
pub fn on(core: &Core, date: &str) -> Occurrence {
    let day = self::date(date);
    let mut found = core.occurrences(day, day.succ_opt().unwrap()).unwrap();
    assert_eq!(found.len(), 1, "one Occurrence on {date}");
    found.remove(0)
}

/// The titles of a Calendar's Occurrences in October 2026, in order.
pub fn titles(core: &Core, calendar_id: CalendarId) -> Vec<String> {
    core.occurrences(date("2026-10-01"), date("2026-11-01"))
        .unwrap()
        .into_iter()
        .filter(|occurrence| occurrence.calendar_id == calendar_id)
        .map(|occurrence| occurrence.title)
        .collect()
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

/// The CalDAV login at `url`.
pub fn login(url: &str, username: &str, password: &str) -> CalDavLogin {
    CalDavLogin {
        url: url.to_owned(),
        username: username.to_owned(),
        password: password.to_owned(),
    }
}

/// The login of `user` on `radicale`.
pub fn user_login(radicale: &Radicale, user: &User) -> CalDavLogin {
    login(&radicale.url(), &user.name, &user.password)
}

/// Adds a CalDAV Account for a new Radicale user, with one Calendar.
pub fn add_account_with_calendar(core: &Core, radicale: &Radicale) -> (AccountId, CalendarId) {
    let user = radicale.user();
    let url = radicale.create_calendar(&user, NewCalendar::default());
    let account = add_account(core, radicale, &user);
    let calendar = core.add_calendars(account, &[url]).unwrap()[0].id;
    (account, calendar)
}

/// Adds the CalDAV Account of `user` on `radicale`.
pub fn add_account(core: &Core, radicale: &Radicale, user: &User) -> AccountId {
    core.add_caldav_account(&user_login(radicale, user))
        .unwrap()
        .account
        .id
}

/// A Radicale user with a Calendar "Work", and a new Core with the user's
/// CalDAV Account added and that Calendar's First Sync done. The Signals
/// of setting it up are forgotten.
pub struct OnServer {
    pub radicale: &'static Radicale,
    pub user: User,
    /// The Calendar's URL on the server.
    pub url: String,
    pub harness: Harness,
    pub core: Core,
    pub account: AccountId,
    pub work: CalendarId,
}

/// Puts `events` (name, iCalendar object) on the server, for
/// [`OnServer::arranged`].
pub fn put_events(events: &[(&str, Vec<u8>)]) -> impl FnOnce(&Radicale, &User, &str) {
    move |radicale, user, url| {
        for (name, ics) in events {
            radicale.put_event(user, url, name, ics.clone());
        }
    }
}

/// Writes `files` (name, iCalendar object) straight into the server's
/// storage (see [`Radicale::write_event_file`]), for [`OnServer::arranged`].
pub fn write_files(files: &[(&str, Vec<u8>)]) -> impl FnOnce(&Radicale, &User, &str) {
    move |radicale, _, url| {
        for (name, ics) in files {
            radicale.write_event_file(url, name, ics.clone());
        }
    }
}

impl OnServer {
    /// A new user's Calendar with `events` (name, iCalendar object) on the
    /// server first. FreeCal adds it Read-only.
    pub fn with(events: &[(&str, Vec<u8>)]) -> Self {
        Self::arranged(
            Radicale::shared().user(),
            NewCalendar::default(),
            Radicale::create_calendar,
            None,
            put_events(events),
        )
    }

    /// Like [`OnServer::with`], with `files` written straight into the
    /// server's storage, as Radicale refuses a second resource holding
    /// one UID.
    pub fn with_files(files: &[(&str, Vec<u8>)]) -> Self {
        Self::arranged(
            Radicale::shared().user(),
            NewCalendar::default(),
            Radicale::create_calendar,
            None,
            write_files(files),
        )
    }

    /// `user`'s Calendar `calendar`, named "Work" and made by `create`,
    /// once `arrange` put what it holds on the server, which FreeCal
    /// reaches through the proxy at `through` if given. FreeCal adds it
    /// Read-only, by the address the server gives, which may differ from
    /// `url` in escapes, as in a username with `@`.
    pub fn arranged(
        user: User,
        calendar: NewCalendar,
        create: fn(&Radicale, &User, NewCalendar) -> String,
        through: Option<&str>,
        arrange: impl FnOnce(&Radicale, &User, &str),
    ) -> Self {
        let radicale = Radicale::shared();
        let url = create(
            radicale,
            &user,
            NewCalendar {
                name: Some("Work"),
                ..calendar
            },
        );
        arrange(radicale, &user, &url);
        let harness = Harness::new();
        let core = harness.open_without_starting_calendars();
        let server = through.map_or_else(|| radicale.url(), str::to_owned);
        let account = core
            .add_caldav_account(&CalDavLogin {
                url: server,
                ..user_login(radicale, &user)
            })
            .unwrap()
            .account
            .id;
        let offered = core.available_calendars(account).unwrap()[0].url.clone();
        let work = core.add_calendars(account, &[offered]).unwrap()[0].id;
        harness.signals.clear();
        Self {
            radicale,
            user,
            url,
            harness,
            core,
            account,
            work,
        }
    }

    /// A new user's Calendar with no Events that the server lets the user
    /// only read, reached through `proxy`, and marked writable; FreeCal
    /// takes it as writable while `proxy` says so.
    pub fn read_only_through(proxy: &RewritingProxy) -> Self {
        Self::arranged(
            Radicale::shared().user(),
            NewCalendar::default(),
            Radicale::create_read_only_calendar,
            Some(&proxy.url),
            |_, _, _| {},
        )
        .writable()
    }

    /// It with the Calendar marked writable.
    pub fn writable(self) -> Self {
        self.core.set_calendar_read_only(self.work, false).unwrap();
        self.harness.signals.clear();
        self
    }

    /// The Event titled `title`, in October 2026.
    pub fn event(&self, title: &str) -> Event {
        let occurrence = self
            .core
            .occurrences(date("2026-10-01"), date("2026-11-01"))
            .unwrap()
            .into_iter()
            .find(|occurrence| occurrence.title == title)
            .unwrap_or_else(|| panic!("an Event titled {title:?}"));
        self.core.event(occurrence.event_id).unwrap()
    }

    /// The Server Object the Event titled `title` is in, as FreeCal holds it.
    pub fn held(&self, title: &str) -> Option<ServerObject> {
        self.core
            .original_server_data(self.event(title).id)
            .unwrap()
    }

    /// The titles of the Calendar's Occurrences in October 2026, in order.
    pub fn titles(&self) -> Vec<String> {
        titles(&self.core, self.work)
    }

    /// The Unsent Changes of the Calendar `id`.
    pub fn unsent(&self, id: CalendarId) -> u32 {
        calendar_sync_status(&self.core, id).report.unsent_changes
    }

    /// The Sync Status of the Calendar `id`.
    pub fn status(&self, id: CalendarId) -> SyncStatus {
        calendar_sync_status(&self.core, id).report.status
    }
}

/// The Sync Status of the server Account `id`.
pub fn account_sync_status(core: &Core, id: AccountId) -> AccountSyncStatus {
    core.sync_statuses()
        .unwrap()
        .accounts
        .into_iter()
        .find(|status| status.account_id == id)
        .unwrap()
}

/// The Sync Status of the server Calendar `id`.
pub fn calendar_sync_status(core: &Core, id: CalendarId) -> CalendarSyncStatus {
    core.sync_statuses()
        .unwrap()
        .calendars
        .into_iter()
        .find(|status| status.calendar_id == id)
        .unwrap()
}
