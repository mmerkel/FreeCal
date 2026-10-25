use chrono::{
    DateTime, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, Offset, TimeDelta, TimeZone, Utc,
};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::{CoreError, ImportedWhen, Result, StructuredText};

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
    pub(super) fn start_date(&self) -> (NaiveDate, bool) {
        match *self {
            When::AllDay { start, .. } => (start, false),
            When::Timed { start, .. } => (start.date(), true),
        }
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

    /// An imported Event's time, as the file gives it.
    pub fn imported(when: &ImportedWhen) -> Self {
        match *when {
            ImportedWhen::AllDay { start, end } => Self {
                all_day: true,
                start: start.to_string(),
                end: end.to_string(),
                start_zone: None,
                end_zone: None,
            },
            ImportedWhen::Timed {
                start,
                start_zone,
                end,
                end_zone,
            } => Self {
                all_day: false,
                start: wall_text(start),
                end: wall_text(end),
                start_zone: Some(start_zone.name().to_owned()),
                end_zone: Some(end_zone.name().to_owned()),
            },
        }
    }

    /// This time's wall times in the Event Time Zones of `pinned`, the time
    /// of an Event whose floating time was pinned before: a new version that
    /// is still floating keeps the zone it was pinned to (ADR 0007, rule 2).
    /// Stays as it is when either is all-day or `pinned`'s zones can't be read.
    pub fn in_zones_of(self, pinned: &StoredWhen) -> Self {
        if self.all_day || pinned.all_day || pinned.zones().is_err() {
            return self;
        }
        Self {
            start_zone: pinned.start_zone.clone(),
            end_zone: pinned.end_zone.clone(),
            ..self
        }
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

/// An Event's fields as the Local Store holds them.
#[derive(Debug, Clone)]
pub(crate) struct EventFields {
    pub title: String,
    pub when: StoredWhen,
    pub location: String,
    /// The description exactly as it was received or written.
    pub description: String,
    /// Whether `description` is HTML (from Google or an import) rather than
    /// plain text.
    pub description_html: bool,
    /// Whether the Event arrived with a floating time that was pinned to its
    /// Event Time Zone (ADR 0007).
    pub zone_pinned: bool,
    /// A Recurring Event's repeat rule, an RFC 5545 `RRULE` value.
    pub rule: Option<String>,
    /// A Recurring Event's cancelled Occurrences: their recurrence IDs,
    /// separated by commas.
    pub exdates: String,
    /// For an Exception, the recurrence ID of the Occurrence it changed.
    pub recurrence_id: Option<String>,
    /// The server object the row came from or will be written to; none for
    /// a row not on a server yet.
    pub object_id: Option<ObjectId>,
}

impl EventFields {
    pub fn description(&self) -> StructuredText {
        StructuredText::from_description(&self.description, self.description_html)
    }
}

/// A server object's row in the Local Store, which names it for good: IDs
/// are never given out again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectId(pub i64);

/// A server object (ADR 0010): the unit a server stores and FreeCal writes
/// back, one CalDAV resource at its href, or one Google event resource. Its
/// row mirrors the server's last known state of it (address, version tag,
/// data as received), and only a sync or a successful write to the server
/// changes it. Event rows refer to the object they came from or will be
/// written to. A read-only feed has no server objects, because FreeCal never
/// writes to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerObject {
    /// The object's address on the server, as the server gave it.
    pub href: String,
    /// The object's ETag, which names its version; none if the server gave
    /// none.
    pub etag: Option<String>,
    /// The object as received, such as a CalDAV iCalendar object.
    pub data: String,
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

    /// Whether an Event with the derived `instants` overlaps the range, as
    /// the Local Store's range query has it: an Event of no length counts
    /// when it starts inside the range.
    pub fn overlaps(&self, all_day: bool, (start, end): (i64, i64)) -> bool {
        if all_day {
            start < self.to_day_seconds && end > self.from_day_seconds
        } else {
            start < self.to_instant
                && (end > self.from_instant || (end == start && start >= self.from_instant))
        }
    }
}

pub(crate) const WALL_TIME: &str = "%Y-%m-%dT%H:%M:%S";

pub(crate) fn wall_text(local: NaiveDateTime) -> String {
    local.format(WALL_TIME).to_string()
}

pub(crate) fn date_seconds(date: NaiveDate) -> i64 {
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
pub(crate) fn instant(local: NaiveDateTime, zone: Tz) -> DateTime<Utc> {
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
