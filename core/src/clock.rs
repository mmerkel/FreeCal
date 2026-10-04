use chrono::{DateTime, FixedOffset, Local};
use chrono_tz::Tz;

/// The core's source of the current time and the system time zone. Tests
/// replace it to control both.
pub trait Clock: Send + Sync {
    /// The current time, with the offset of the system time zone.
    fn now(&self) -> DateTime<FixedOffset>;

    /// The system time zone, which is the Display Time Zone until ticket 06
    /// adds the override.
    fn time_zone(&self) -> Tz;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<FixedOffset> {
        Local::now().fixed_offset()
    }

    /// Falls back to UTC when the system names no time zone FreeCal knows.
    fn time_zone(&self) -> Tz {
        iana_time_zone::get_timezone()
            .ok()
            .and_then(|name| name.parse().ok())
            .unwrap_or(Tz::UTC)
    }
}
