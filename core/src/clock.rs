use chrono::{DateTime, FixedOffset, Local};

/// The core's source of the current time. Tests replace it to control time.
pub trait Clock: Send + Sync {
    /// The current time, with the offset of the system time zone.
    fn now(&self) -> DateTime<FixedOffset>;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<FixedOffset> {
        Local::now().fixed_offset()
    }
}
