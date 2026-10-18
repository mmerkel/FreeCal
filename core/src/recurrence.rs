//! Recurring Events: their repeat rules and the Occurrences those give.
//!
//! A Recurring Event keeps its rule as an RFC 5545 `RRULE` value, with
//! `UNTIL` as RFC 5545 asks: a date for an all-day Event, a UTC time for a
//! timed one. Rules are expanded in wall time in the Event Time Zone of the
//! start (ADR 0007), so a weekly meeting at 9:00 stays at 9:00 when the clocks
//! change. The `rrule` library expands them; it is given wall times dressed
//! as UTC, so that its own time zone handling never comes into it.
//!
//! A rule is stored text, untrusted like every stored field: one FreeCal
//! can't read gives no further Occurrences, so its Event shows as its first
//! Occurrence only.

use std::str::FromStr;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, TimeZone};
use chrono_tz::Tz;
use rrule::{RRule, RRuleSet, Unvalidated};
use serde::{Deserialize, Serialize};

use crate::event::{StoredRange, StoredWhen, WALL_TIME, date_seconds, instant, wall_text};
use crate::store::EventFields;
use crate::{CoreError, Result};

/// A repeat rule as the editor sets it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Repeat {
    pub frequency: Frequency,
    /// Every how many days, weeks, months or years; at least 1.
    pub interval: u16,
    pub end: RepeatEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum Frequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl Frequency {
    fn rrule(self) -> rrule::Frequency {
        match self {
            Frequency::Daily => rrule::Frequency::Daily,
            Frequency::Weekly => rrule::Frequency::Weekly,
            Frequency::Monthly => rrule::Frequency::Monthly,
            Frequency::Yearly => rrule::Frequency::Yearly,
        }
    }

    fn of(frequency: rrule::Frequency) -> Option<Self> {
        match frequency {
            rrule::Frequency::Daily => Some(Frequency::Daily),
            rrule::Frequency::Weekly => Some(Frequency::Weekly),
            rrule::Frequency::Monthly => Some(Frequency::Monthly),
            rrule::Frequency::Yearly => Some(Frequency::Yearly),
            _ => None,
        }
    }
}

/// When a Recurring Event stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RepeatEnd {
    Never,
    /// Its last Occurrence starts on this date or before, in its Event Time
    /// Zone.
    Until {
        date: NaiveDate,
    },
    /// After this many Occurrences, cancelled ones included; at least 1.
    Count {
        count: u32,
    },
}

/// The rule parts the editor sets. Any other part, such as `BYDAY`, makes a
/// rule custom.
const EDITOR_PARTS: &[&str] = &["FREQ", "INTERVAL", "COUNT", "UNTIL"];

/// How a stored rule reads in the editor: the [`Repeat`] it shows, none for a
/// rule it can't show at all, such as an hourly one, and whether the rule
/// has more than the editor shows.
pub(crate) fn repeat_of(fields: &EventFields) -> (Option<Repeat>, bool) {
    let Some(rule) = fields.rule.as_deref().and_then(Rule::parse) else {
        return (None, false);
    };
    let custom = rule
        .parts
        .iter()
        .any(|(key, _)| !EDITOR_PARTS.contains(&key.as_str()) && key != "WKST");
    let Some(frequency) = Frequency::of(rule.rrule.get_freq()) else {
        return (None, true);
    };
    let end = match (rule.rrule.get_count(), rule.until_wall(&fields.when)) {
        (Some(count), _) => RepeatEnd::Count { count },
        (None, Some(until)) => RepeatEnd::Until { date: until.date() },
        (None, None) => RepeatEnd::Never,
    };
    let repeat = Repeat {
        frequency,
        interval: rule.rrule.get_interval(),
        end,
    };
    (Some(repeat), custom)
}

/// The rule text for `repeat` on an Event at `when`. The parts of `kept`,
/// the rule the Event had, that the editor doesn't set are kept while the
/// frequency stays, so that "every 2 weeks on Sunday and Tuesday" made
/// weekly stays on Sunday and Tuesday. Rules keep their own `WKST`; new
/// rules get RFC 5545's default, Monday.
pub(crate) fn rule_text(repeat: Repeat, kept: Option<&str>, when: &StoredWhen) -> Result<String> {
    let start = first_start(when)?;
    let valid = repeat.interval >= 1
        && match repeat.end {
            RepeatEnd::Never => true,
            RepeatEnd::Count { count } => count >= 1,
            RepeatEnd::Until { date } => date >= start.date(),
        };
    if !valid {
        return Err(CoreError::InvalidRepeat);
    }
    let frequency = repeat.frequency.rrule();
    let kept = kept
        .and_then(Rule::parse)
        .filter(|rule| rule.rrule.get_freq() == frequency)
        .map(|rule| rule.without(&["INTERVAL", "COUNT"]))
        .and_then(|text| RRule::<Unvalidated>::from_str(&text).ok());
    let mut rrule = kept
        .unwrap_or_else(|| RRule::new(frequency))
        .interval(repeat.interval);
    if let RepeatEnd::Count { count } = repeat.end {
        rrule = rrule.count(count);
    }
    let mut text = rrule.to_string();
    if let RepeatEnd::Until { date } = repeat.end {
        // RFC 5545: a date for an all-day Event, otherwise the UTC instant,
        // here the end of that day in the Event Time Zone.
        let until = if when.all_day {
            date.format("%Y%m%d").to_string()
        } else {
            let zone = start_zone(when)?;
            let last = date.and_time(NaiveTime::MIN) + TimeDelta::days(1) - TimeDelta::seconds(1);
            instant(last, zone).format("%Y%m%dT%H%M%SZ").to_string()
        };
        text.push_str(&format!(";UNTIL={until}"));
    }
    Ok(text)
}

/// A rule from a file, as FreeCal stores it: as given, but with an `UNTIL`
/// in the form RFC 5545 asks for the Event at `when`; one without a zone is
/// read in the Event Time Zone. None when FreeCal can't expand it.
pub(crate) fn imported_rule(text: &str, when: &StoredWhen) -> Option<String> {
    let rule = Rule::parse(text)?;
    let mut stored = rule.without(&[]);
    if let Some(until) = rule.until_wall(when) {
        let until = if when.all_day {
            until.format("%Y%m%d").to_string()
        } else {
            instant(until, start_zone(when).ok()?)
                .format("%Y%m%dT%H%M%SZ")
                .to_string()
        };
        stored.push_str(&format!(";UNTIL={until}"));
    }
    OccurrenceStarts::new(Rule::parse(&stored)?, when)?;
    Some(stored)
}

/// The derived start and end of an Event as Unix seconds (see
/// [`StoredWhen::instants`]): for a Recurring Event, the start of its first
/// Occurrence and the end of its last, `i64::MAX` when it never ends.
pub(crate) fn instants(fields: &EventFields) -> Result<(i64, i64)> {
    let first = fields.when.instants()?;
    let Some(starts) = OccurrenceStarts::of(fields) else {
        return Ok(first);
    };
    if !starts.ends {
        return Ok((first.0, i64::MAX));
    }
    let mut end = first.1;
    for (index, start) in starts.into_iter().enumerate() {
        if index >= MAX_COUNTED {
            return Ok((first.0, i64::MAX));
        }
        end = end.max(occurrence(&fields.when, start)?.instants()?.1);
    }
    Ok((first.0, end))
}

/// The most Occurrences [`instants`] counts to find the last one, and
/// [`occurrences_in`] walks to find those in a range; a rule with more
/// counts as never ending.
const MAX_COUNTED: usize = 50_000;

/// The Occurrences of a Recurring Event in `range`, each with its
/// recurrence ID, the text of its original start as the Local Store writes
/// starts. Without those cancelled in its `exdates` and those in
/// `exceptions`, the recurrence IDs of its changed Occurrences, which are
/// Events of their own. None for an Event that doesn't repeat.
pub(crate) fn occurrences_in(
    fields: &EventFields,
    exceptions: &[String],
    range: &StoredRange,
) -> Result<Option<Vec<(String, StoredWhen)>>> {
    let Some(starts) = OccurrenceStarts::of(fields) else {
        return Ok(None);
    };
    let skipped: Vec<NaiveDateTime> = split_ids(&fields.exdates)
        .chain(exceptions.iter().map(String::as_str))
        .filter_map(start_of_id)
        .collect();
    let last_day = range.to_day_seconds + TimeDelta::days(2).num_seconds();
    let mut found = Vec::new();
    // An hourly rule from long ago would otherwise walk for ever; one with
    // more starts than are counted has none further in the range.
    for start in starts.into_iter().take(MAX_COUNTED) {
        if date_seconds(start.date()) > last_day {
            break;
        }
        if skipped.contains(&start) {
            continue;
        }
        let when = occurrence(&fields.when, start)?;
        if range.overlaps(when.all_day, when.instants()?) {
            found.push((when.start.clone(), when));
        }
    }
    Ok(Some(found))
}

/// A recurrence ID as the Local Store writes it, from the text the window
/// gives back; none when it isn't one.
pub(crate) fn recurrence_id(text: &str) -> Option<String> {
    let start = start_of_id(text)?;
    Some(if text.contains('T') {
        wall_text(start)
    } else {
        start.date().to_string()
    })
}

/// `exdates` with `id` added, sorted and each once.
pub(crate) fn with_exdate(exdates: &str, id: &str) -> String {
    let mut ids: Vec<&str> = split_ids(exdates).chain([id]).collect();
    ids.sort_unstable();
    ids.dedup();
    ids.join(",")
}

fn split_ids(text: &str) -> impl Iterator<Item = &str> {
    text.split(',').map(str::trim).filter(|id| !id.is_empty())
}

/// The start a recurrence ID names, midnight for a date.
fn start_of_id(id: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(id, WALL_TIME)
        .ok()
        .or_else(|| Some(id.parse::<NaiveDate>().ok()?.and_time(NaiveTime::MIN)))
}

/// The Occurrence of an Event at `when` that starts at `start`, a wall time
/// in its start's zone (midnight for an all-day Event). It lasts as long as
/// the Event in wall time, so that a meeting from 9:00 to 10:00 keeps those
/// times when the clocks change; one that ends in another zone, such as a
/// flight, keeps its exact length.
fn occurrence(when: &StoredWhen, start: NaiveDateTime) -> Result<StoredWhen> {
    let first = first_start(when)?;
    if when.all_day {
        let length = when
            .end
            .parse::<NaiveDate>()
            .map_err(|_| unreadable(when))?
            - first.date();
        return Ok(StoredWhen {
            start: start.date().to_string(),
            end: (start.date() + length).to_string(),
            ..when.clone()
        });
    }
    let end = NaiveDateTime::parse_from_str(&when.end, WALL_TIME).map_err(|_| unreadable(when))?;
    let end = if when.start_zone == when.end_zone {
        start + (end - first)
    } else {
        let zone = start_zone(when)?;
        let end_zone: Tz = when
            .end_zone
            .as_deref()
            .and_then(|name| name.parse().ok())
            .ok_or_else(|| unreadable(when))?;
        let length = instant(end, end_zone) - instant(first, zone);
        (instant(start, zone) + length)
            .with_timezone(&end_zone)
            .naive_local()
    };
    Ok(StoredWhen {
        start: wall_text(start),
        end: wall_text(end),
        ..when.clone()
    })
}

/// The start of an Event's first Occurrence, midnight for an all-day one.
fn first_start(when: &StoredWhen) -> Result<NaiveDateTime> {
    if when.all_day {
        let date = when
            .start
            .parse::<NaiveDate>()
            .map_err(|_| unreadable(when))?;
        Ok(date.and_time(NaiveTime::MIN))
    } else {
        NaiveDateTime::parse_from_str(&when.start, WALL_TIME).map_err(|_| unreadable(when))
    }
}

fn start_zone(when: &StoredWhen) -> Result<Tz> {
    when.start_zone
        .as_deref()
        .and_then(|name| name.parse().ok())
        .ok_or_else(|| unreadable(when))
}

fn unreadable(when: &StoredWhen) -> CoreError {
    CoreError::UnreadableEventTime(format!("{} – {}", when.start, when.end))
}

/// The starts a Recurring Event's rule gives, in order, as wall times in its
/// start's zone. Cancelled ones are among them.
struct OccurrenceStarts {
    set: RRuleSet,
    /// Whether the rule has a `COUNT` or an `UNTIL`.
    ends: bool,
}

impl OccurrenceStarts {
    /// None for an Event without a rule, or with one FreeCal can't read.
    fn of(fields: &EventFields) -> Option<Self> {
        Self::new(Rule::parse(fields.rule.as_deref()?)?, &fields.when)
    }

    fn new(rule: Rule, when: &StoredWhen) -> Option<Self> {
        let ends = rule.rrule.get_count().is_some() || rule.until.is_some();
        let start = first_start(when).ok()?;
        let until = rule.until_wall(when);
        let mut rrule = rule.rrule;
        if let Some(until) = until {
            rrule = rrule.until(dressed(until));
        }
        let set = rrule.build(dressed(start)).ok()?.limit();
        Some(Self { set, ends })
    }
}

impl IntoIterator for OccurrenceStarts {
    type Item = NaiveDateTime;
    type IntoIter = Box<dyn Iterator<Item = NaiveDateTime>>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.set.into_iter().map(|start| start.naive_utc()))
    }
}

/// A wall time dressed as UTC for the `rrule` library.
fn dressed(wall: NaiveDateTime) -> chrono::DateTime<rrule::Tz> {
    rrule::Tz::UTC.from_utc_datetime(&wall)
}

/// A stored rule, read: its parts in order with upper-case names, the rule
/// without `UNTIL` for the `rrule` library, and the `UNTIL` FreeCal reads
/// itself, because the library reads one without a zone in the system's.
struct Rule {
    parts: Vec<(String, String)>,
    rrule: RRule<Unvalidated>,
    until: Option<Until>,
}

enum Until {
    /// The instant, as a UTC wall time.
    Utc(NaiveDateTime),
    Date(NaiveDate),
    Floating(NaiveDateTime),
}

impl Rule {
    fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let text = text.strip_prefix("RRULE:").unwrap_or(text);
        let mut parts = Vec::new();
        for part in text.split(';').filter(|part| !part.trim().is_empty()) {
            let (key, value) = part.split_once('=')?;
            parts.push((key.trim().to_ascii_uppercase(), value.trim().to_owned()));
        }
        let until = match parts.iter().find(|(key, _)| key == "UNTIL") {
            None => None,
            Some((_, value)) => Some(Until::parse(value)?),
        };
        let rule = Self {
            parts,
            rrule: RRule::new(rrule::Frequency::Daily),
            until,
        };
        let rrule = RRule::<Unvalidated>::from_str(&rule.without(&[])).ok()?;
        Some(Self { rrule, ..rule })
    }

    /// The rule's text without `UNTIL` and the parts `left_out`.
    fn without(&self, left_out: &[&str]) -> String {
        self.parts
            .iter()
            .filter(|(key, _)| key != "UNTIL" && !left_out.contains(&key.as_str()))
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(";")
    }

    /// The last start `UNTIL` allows, as a wall time in the start's zone of
    /// the Event at `when`; a date allows its whole day.
    fn until_wall(&self, when: &StoredWhen) -> Option<NaiveDateTime> {
        Some(match self.until.as_ref()? {
            Until::Utc(utc) if when.all_day => *utc,
            Until::Utc(utc) => {
                let zone = start_zone(when).ok()?;
                zone.from_utc_datetime(utc).naive_local()
            }
            Until::Date(date) => date.and_hms_opt(23, 59, 59)?,
            Until::Floating(wall) => *wall,
        })
    }
}

impl Until {
    fn parse(value: &str) -> Option<Self> {
        if let Some(utc) = value.strip_suffix('Z') {
            return NaiveDateTime::parse_from_str(utc, "%Y%m%dT%H%M%S")
                .ok()
                .map(Until::Utc);
        }
        if value.contains('T') {
            return NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S")
                .ok()
                .map(Until::Floating);
        }
        NaiveDate::parse_from_str(value, "%Y%m%d")
            .ok()
            .map(Until::Date)
    }
}
