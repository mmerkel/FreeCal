use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::{CalendarId, CoreError, Description, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(type = "number"))]
#[serde(transparent)]
pub struct EventId(pub i64);

/// When an Event takes place. Times are wall-clock times in the Display Time
/// Zone; ends are exclusive, so a one-day all-day Event ends the next day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum When {
    AllDay {
        #[cfg_attr(test, ts(type = "string"))]
        start: NaiveDate,
        #[cfg_attr(test, ts(type = "string"))]
        end: NaiveDate,
    },
    Timed {
        #[cfg_attr(test, ts(type = "string"))]
        start: NaiveDateTime,
        #[cfg_attr(test, ts(type = "string"))]
        end: NaiveDateTime,
    },
}

impl When {
    /// An Event can't end before it starts, and an all-day Event lasts at
    /// least a day.
    fn check(&self) -> Result<()> {
        let valid = match self {
            When::AllDay { start, end } => end > start,
            When::Timed { start, end } => end >= start,
        };
        if valid {
            Ok(())
        } else {
            Err(CoreError::EndBeforeStart)
        }
    }

    /// The date and time it starts, for ordering: midnight for all-day Events.
    fn start_key(&self) -> (NaiveDate, bool, NaiveDateTime) {
        match *self {
            When::AllDay { start, .. } => (start, false, start.and_time(NaiveTime::MIN)),
            When::Timed { start, .. } => (start.date(), true, start),
        }
    }
}

/// What the user gives an Event when creating or editing it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct EventDraft {
    pub calendar_id: CalendarId,
    pub title: String,
    pub when: When,
    pub location: String,
    /// The plain-text description. When it equals the plain-text form of an
    /// Event's description, the original is kept byte for byte.
    pub description: String,
}

/// An Event with everything its details and editor show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: EventId,
    pub calendar_id: CalendarId,
    pub title: String,
    pub when: When,
    pub location: String,
    pub description: Description,
}

/// One dated appearance of an Event in the grid. A single Event has exactly
/// one; Recurring Events get one per date (ticket 07).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Occurrence {
    pub event_id: EventId,
    pub calendar_id: CalendarId,
    pub title: String,
    pub when: When,
}

impl Occurrence {
    pub(crate) fn order(occurrences: &mut [Occurrence]) {
        occurrences.sort_by_key(|occurrence| (occurrence.when.start_key(), occurrence.event_id.0));
    }
}

/// How the Local Store records when an Event takes place: timed Events as
/// UTC instants in RFC 3339 with the zone they were made in, all-day Events
/// as dates. Both sort as text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredWhen {
    pub all_day: bool,
    pub start: String,
    pub end: String,
    pub time_zone: Option<String>,
}

impl StoredWhen {
    pub fn from_when(when: When, zone: Tz) -> Result<Self> {
        when.check()?;
        Ok(match when {
            When::AllDay { start, end } => Self {
                all_day: true,
                start: start.to_string(),
                end: end.to_string(),
                time_zone: None,
            },
            When::Timed { start, end } => Self {
                all_day: false,
                start: utc_text(instant(start, zone)),
                end: utc_text(instant(end, zone)),
                time_zone: Some(zone.name().to_owned()),
            },
        })
    }

    /// Reads it back as wall-clock times in `zone`. A value FreeCal didn't
    /// write is an error, not a crash.
    pub fn to_when(&self, zone: Tz) -> Result<When> {
        let unreadable = || CoreError::UnreadableEventTime(self.start.clone());
        if self.all_day {
            let date = |text: &str| text.parse::<NaiveDate>().map_err(|_| unreadable());
            Ok(When::AllDay {
                start: date(&self.start)?,
                end: date(&self.end)?,
            })
        } else {
            let time = |text: &str| {
                DateTime::parse_from_rfc3339(text)
                    .map(|instant| instant.with_timezone(&zone).naive_local())
                    .map_err(|_| unreadable())
            };
            Ok(When::Timed {
                start: time(&self.start)?,
                end: time(&self.end)?,
            })
        }
    }
}

/// The bounds of the dates `from` up to `to` (exclusive) in `zone`, as the
/// stored text of all-day and of timed Events.
pub(crate) struct StoredRange {
    pub from_date: String,
    pub to_date: String,
    pub from_instant: String,
    pub to_instant: String,
}

impl StoredRange {
    pub fn new(from: NaiveDate, to: NaiveDate, zone: Tz) -> Self {
        let midnight = |date: NaiveDate| utc_text(instant(date.and_time(NaiveTime::MIN), zone));
        Self {
            from_date: from.to_string(),
            to_date: to.to_string(),
            from_instant: midnight(from),
            to_instant: midnight(to),
        }
    }
}

/// The instant a wall-clock time in `zone` names. In the hour that is
/// repeated when clocks go back, the first one; in the hour skipped when
/// clocks go forward, the time an hour later, as a clock that was set an
/// hour ahead would show.
fn instant(local: NaiveDateTime, zone: Tz) -> DateTime<Utc> {
    zone.from_local_datetime(&local)
        .earliest()
        .or_else(|| {
            zone.from_local_datetime(&(local + TimeDelta::hours(1)))
                .earliest()
        })
        .map_or_else(|| local.and_utc(), |time| time.with_timezone(&Utc))
}

fn utc_text(instant: DateTime<Utc>) -> String {
    instant.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}
