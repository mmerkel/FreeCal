//! Test harness for the core application interface: a real SQLite Local Store
//! in a temporary directory, a controllable clock and a recording Signal
//! subscriber.

#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use chrono::{DateTime, FixedOffset};
use freecal_core::{Clock, Core, Signal, SignalSink};
use tempfile::TempDir;

pub struct Harness {
    data_dir: TempDir,
    pub clock: Arc<TestClock>,
    pub signals: Arc<RecordingSignals>,
}

impl Harness {
    pub fn new() -> Self {
        Self {
            data_dir: TempDir::new().expect("create temporary data directory"),
            clock: Arc::new(TestClock::at("2026-10-03T12:00:00+02:00")),
            signals: Arc::new(RecordingSignals::default()),
        }
    }

    /// Opens the core on this harness's data directory, like launching FreeCal.
    /// Opening again on the same harness is a relaunch on the same Local Store.
    pub fn open(&self) -> Core {
        Core::open(
            self.data_dir.path(),
            self.clock.clone(),
            self.signals.clone(),
        )
        .expect("open core")
    }
}

pub struct TestClock {
    now: Mutex<DateTime<FixedOffset>>,
}

impl TestClock {
    pub fn at(rfc3339: &str) -> Self {
        Self {
            now: Mutex::new(parse(rfc3339)),
        }
    }

    pub fn set(&self, rfc3339: &str) {
        *self.now.lock().unwrap() = parse(rfc3339);
    }
}

impl Clock for TestClock {
    fn now(&self) -> DateTime<FixedOffset> {
        *self.now.lock().unwrap()
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
}

impl SignalSink for RecordingSignals {
    fn send(&self, signal: Signal) {
        self.received.lock().unwrap().push(signal);
    }
}
