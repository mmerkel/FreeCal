use chrono::{
    DateTime, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, Offset, TimeDelta, TimeZone, Utc,
};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::{CalendarId, CoreError, Description, Result};

/// Branded in TypeScript, so that it can't be mixed up with other IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    test,
    derive(ts_rs::TS),
    ts(type = "number & { readonly __brand: \"EventId\" }")
)]
#[serde(transparent)]
pub struct EventId(pub i64);

/// When an Event takes place. Times are wall-clock times in the Display Time
/// Zone; ends are exclusive, so a one-day all-day Event ends the next day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum When {
    AllDay {
        start: NaiveDate,
        end: NaiveDate,
    },
    Timed {
        start: NaiveDateTime,
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

    /// The date it starts on, and whether it is timed, for ordering:
    /// all-day Events come first on their date.
    fn start_date(&self) -> (NaiveDate, bool) {
        match *self {
            When::AllDay { start, .. } => (start, false),
            When::Timed { start, .. } => (start.date(), true),
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
    /// Orders Occurrences, each given with its derived UTC start, by the
    /// date they start on in the Display Time Zone and then by when they
    /// happen, which differs from their wall time in the repeated hour.
    pub(crate) fn ordered(mut occurrences: Vec<(i64, Occurrence)>) -> Vec<Occurrence> {
        occurrences.sort_by_key(|(start_utc, occurrence)| {
            (
                occurrence.when.start_date(),
                *start_utc,
                occurrence.event_id.0,
            )
        });
        occurrences
            .into_iter()
            .map(|(_, occurrence)| occurrence)
            .collect()
    }
}

/// How the Local Store records when an Event takes place (ADR 0007): a timed
/// Event's start and end as wall-clock times (`YYYY-MM-DDTHH:MM:SS`), each
/// with its own Event Time Zone, and an all-day Event's as dates. The UTC
/// instants range queries use are derived from them by [`StoredWhen::instants`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredWhen {
    pub all_day: bool,
    pub start: String,
    pub end: String,
    pub start_zone: Option<String>,
    pub end_zone: Option<String>,
}

impl StoredWhen {
    /// A new Event's time: wall times as entered, in the Display Time Zone.
    pub fn new(when: When, display: Tz) -> Result<Self> {
        when.check()?;
        Ok(match when {
            When::AllDay { start, end } => Self {
                all_day: true,
                start: start.to_string(),
                end: end.to_string(),
                start_zone: None,
                end_zone: None,
            },
            When::Timed { start, end } => Self {
                all_day: false,
                start: wall_text(start),
                end: wall_text(end),
                start_zone: Some(display.name().to_owned()),
                end_zone: Some(display.name().to_owned()),
            },
        })
    }

    /// This time edited to `when`, given in the Display Time Zone. A timed
    /// Event keeps its Event Time Zones, and a start or end left as shown
    /// keeps its stored wall time exactly.
    pub fn edit(&self, when: When, display: Tz) -> Result<Self> {
        when.check()?;
        let shown = self.to_when(display).ok();
        let (
            When::Timed { start, end },
            Some(When::Timed {
                start: was_start,
                end: was_end,
            }),
        ) = (when, shown)
        else {
            return Self::new(when, display);
        };
        let wall = |time: NaiveDateTime, was: NaiveDateTime, stored: &str, zone: Tz| {
            if time == was {
                stored.to_owned()
            } else {
                wall_text(convert(time, display, zone))
            }
        };
        let (start_zone, end_zone) = self.zones()?;
        Ok(Self {
            all_day: false,
            start: wall(start, was_start, &self.start, start_zone),
            end: wall(end, was_end, &self.end, end_zone),
            start_zone: self.start_zone.clone(),
            end_zone: self.end_zone.clone(),
        })
    }

    /// Reads it back as wall-clock times in the Display Time Zone. A value
    /// FreeCal didn't write is an error, not a crash.
    pub fn to_when(&self, display: Tz) -> Result<When> {
        if self.all_day {
            let (start, end) = self.dates()?;
            Ok(When::AllDay { start, end })
        } else {
            let (start, end) = self.utc()?;
            let shown = |instant: DateTime<Utc>| instant.with_timezone(&display).naive_local();
            Ok(When::Timed {
                start: shown(start),
                end: shown(end),
            })
        }
    }

    /// The derived start and end as Unix seconds, under the time zone rules
    /// FreeCal was built with. All-day Events' dates count as midnight UTC,
    /// so that they compare with [`StoredRange`]'s dates.
    pub fn instants(&self) -> Result<(i64, i64)> {
        if self.all_day {
            let (start, end) = self.dates()?;
            Ok((date_seconds(start), date_seconds(end)))
        } else {
            let (start, end) = self.utc()?;
            Ok((start.timestamp(), end.timestamp()))
        }
    }

    /// A timed Event as ticket 03 stored it, a UTC instant in RFC 3339 and the
    /// zone it was made in, as the wall time in that zone. A zone FreeCal
    /// doesn't know becomes UTC; an unreadable instant is kept as it was.
    pub fn from_utc_instants(start: &str, end: &str, zone: Option<&str>) -> Self {
        let zone = zone.and_then(|name| name.parse().ok()).unwrap_or(Tz::UTC);
        let wall = |text: &str| {
            DateTime::parse_from_rfc3339(text).map_or_else(
                |_| text.to_owned(),
                |instant| wall_text(instant.with_timezone(&zone).naive_local()),
            )
        };
        Self {
            all_day: false,
            start: wall(start),
            end: wall(end),
            start_zone: Some(zone.name().to_owned()),
            end_zone: Some(zone.name().to_owned()),
        }
    }

    fn unreadable(&self) -> CoreError {
        CoreError::UnreadableEventTime(format!("{} – {}", self.start, self.end))
    }

    fn dates(&self) -> Result<(NaiveDate, NaiveDate)> {
        let date = |text: &str| text.parse::<NaiveDate>().map_err(|_| self.unreadable());
        Ok((date(&self.start)?, date(&self.end)?))
    }

    fn zones(&self) -> Result<(Tz, Tz)> {
        let zone = |name: &Option<String>| {
            name.as_deref()
                .and_then(|name| name.parse::<Tz>().ok())
                .ok_or_else(|| self.unreadable())
        };
        Ok((zone(&self.start_zone)?, zone(&self.end_zone)?))
    }

    fn utc(&self) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
        let (start_zone, end_zone) = self.zones()?;
        let wall = |text: &str| {
            NaiveDateTime::parse_from_str(text, WALL_TIME).map_err(|_| self.unreadable())
        };
        Ok((
            instant(wall(&self.start)?, start_zone),
            instant(wall(&self.end)?, end_zone),
        ))
    }
}

/// The bounds of the dates `from` up to `to` (exclusive) in the Display Time
/// Zone, as Unix seconds: for all-day Events the dates as in
/// [`StoredWhen::instants`], for timed Events the instants of their midnights.
pub(crate) struct StoredRange {
    pub from_day_seconds: i64,
    pub to_day_seconds: i64,
    pub from_instant: i64,
    pub to_instant: i64,
}

impl StoredRange {
    pub fn new(from: NaiveDate, to: NaiveDate, display: Tz) -> Self {
        let midnight =
            |date: NaiveDate| instant(date.and_time(NaiveTime::MIN), display).timestamp();
        Self {
            from_day_seconds: date_seconds(from),
            to_day_seconds: date_seconds(to),
            from_instant: midnight(from),
            to_instant: midnight(to),
        }
    }
}

const WALL_TIME: &str = "%Y-%m-%dT%H:%M:%S";

fn wall_text(local: NaiveDateTime) -> String {
    local.format(WALL_TIME).to_string()
}

fn date_seconds(date: NaiveDate) -> i64 {
    date.and_time(NaiveTime::MIN).and_utc().timestamp()
}

/// A wall time in `from` as the wall time of the same instant in `to`. Within
/// one zone it stays as it is, even when the clocks skip it.
fn convert(local: NaiveDateTime, from: Tz, to: Tz) -> NaiveDateTime {
    if from == to {
        local
    } else {
        instant(local, from).with_timezone(&to).naive_local()
    }
}

/// The instant a wall-clock time in `zone` names, as RFC 5545 reads it: in
/// the hour that is repeated when clocks go back, the first one; in a gap
/// when clocks go forward, the time shifted by the gap, which is the wall
/// time read with the offset from before the gap.
fn instant(local: NaiveDateTime, zone: Tz) -> DateTime<Utc> {
    match zone.from_local_datetime(&local) {
        LocalResult::Single(time) | LocalResult::Ambiguous(time, _) => time.with_timezone(&Utc),
        LocalResult::None => {
            // A day earlier is safely before the gap: no zone changes its
            // offset twice within a day.
            let before = zone
                .offset_from_utc_datetime(&(local - TimeDelta::days(1)))
                .fix();
            (local - TimeDelta::seconds(before.local_minus_utc().into())).and_utc()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BERLIN: Tz = Tz::Europe__Berlin;
    const NEW_YORK: Tz = Tz::America__New_York;

    fn timed(start: &str, end: &str) -> When {
        When::Timed {
            start: start.parse().unwrap(),
            end: end.parse().unwrap(),
        }
    }

    fn stored(start: &str, end: &str, zone: &str) -> StoredWhen {
        StoredWhen {
            all_day: false,
            start: start.to_owned(),
            end: end.to_owned(),
            start_zone: Some(zone.to_owned()),
            end_zone: Some(zone.to_owned()),
        }
    }

    fn utc(rfc3339: &str) -> i64 {
        DateTime::parse_from_rfc3339(rfc3339).unwrap().timestamp()
    }

    #[test]
    fn a_new_event_gets_the_display_time_zone_and_its_wall_times() {
        let when = timed("2026-10-05T09:00:00", "2026-10-05T10:30:00");

        let new = StoredWhen::new(when, NEW_YORK).unwrap();

        assert_eq!(
            new,
            stored(
                "2026-10-05T09:00:00",
                "2026-10-05T10:30:00",
                "America/New_York"
            )
        );
        assert_eq!(
            new.instants().unwrap(),
            (utc("2026-10-05T13:00:00Z"), utc("2026-10-05T14:30:00Z"))
        );
    }

    #[test]
    fn a_time_the_clocks_skip_is_stored_as_entered_and_kept_by_an_untouched_edit() {
        // On 29 March 2026, Berlin's clocks jump from 02:00 to 03:00.
        let entered = timed("2026-03-29T02:30:00", "2026-03-29T04:00:00");

        let new = StoredWhen::new(entered, BERLIN).unwrap();
        let shown = new.to_when(BERLIN).unwrap();
        let edited = new.edit(shown, BERLIN).unwrap();

        assert_eq!(new.start, "2026-03-29T02:30:00");
        assert_eq!(new.instants().unwrap().0, utc("2026-03-29T01:30:00Z"));
        assert_eq!(shown, timed("2026-03-29T03:30:00", "2026-03-29T04:00:00"));
        assert_eq!(edited, new);
    }

    #[test]
    fn a_time_the_clocks_repeat_is_stored_as_entered_and_is_the_first_instant() {
        // On 25 October 2026, Berlin's clocks go back from 03:00 to 02:00.
        let entered = timed("2026-10-25T02:30:00", "2026-10-25T02:45:00");

        let new = StoredWhen::new(entered, BERLIN).unwrap();

        assert_eq!(new.start, "2026-10-25T02:30:00");
        assert_eq!(new.instants().unwrap().0, utc("2026-10-25T00:30:00Z"));
        assert_eq!(new.to_when(BERLIN).unwrap(), entered);
    }

    #[test]
    fn an_edit_from_another_display_time_zone_keeps_the_event_time_zone() {
        let made_in_new_york = StoredWhen::new(
            timed("2026-10-05T09:00:00", "2026-10-05T10:00:00"),
            NEW_YORK,
        )
        .unwrap();

        // 16:00 to 17:30 in Berlin is 10:00 to 11:30 in New York.
        let edited = made_in_new_york
            .edit(timed("2026-10-05T16:00:00", "2026-10-05T17:30:00"), BERLIN)
            .unwrap();

        assert_eq!(
            edited,
            stored(
                "2026-10-05T10:00:00",
                "2026-10-05T11:30:00",
                "America/New_York"
            )
        );
    }

    #[test]
    fn an_edit_keeps_each_zone_and_the_wall_time_left_as_shown() {
        let mut flight = stored(
            "2026-03-29T02:30:00",
            "2026-03-29T05:00:00",
            "Europe/Berlin",
        );
        flight.end_zone = Some("Europe/London".to_owned());
        let When::Timed { start, .. } = flight.to_when(NEW_YORK).unwrap() else {
            unreachable!()
        };

        // Only the end moves: to 01:00 in New York, 06:00 in London.
        let edited = flight
            .edit(
                When::Timed {
                    start,
                    end: "2026-03-29T01:00:00".parse().unwrap(),
                },
                NEW_YORK,
            )
            .unwrap();

        assert_eq!(edited.start, "2026-03-29T02:30:00");
        assert_eq!(edited.end, "2026-03-29T06:00:00");
        assert_eq!(edited.start_zone.as_deref(), Some("Europe/Berlin"));
        assert_eq!(edited.end_zone.as_deref(), Some("Europe/London"));
    }

    #[test]
    fn an_all_day_event_made_timed_gets_the_display_time_zone() {
        let day = StoredWhen::new(
            When::AllDay {
                start: "2026-10-05".parse().unwrap(),
                end: "2026-10-06".parse().unwrap(),
            },
            NEW_YORK,
        )
        .unwrap();

        let edited = day
            .edit(timed("2026-10-05T09:00:00", "2026-10-05T10:00:00"), BERLIN)
            .unwrap();

        assert_eq!(
            edited,
            stored(
                "2026-10-05T09:00:00",
                "2026-10-05T10:00:00",
                "Europe/Berlin"
            )
        );
        assert_eq!(day.start_zone, None);
    }
}
