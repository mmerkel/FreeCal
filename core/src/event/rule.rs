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
//!
//! A leaf of the Event module: outside it, only [`span`], [`imported`] and
//! the text of recurrence IDs are open.

use std::str::FromStr;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, TimeZone};
use chrono_tz::Tz;
use rrule::{RRule, RRuleSet, Unvalidated};
use serde::{Deserialize, Serialize};

use super::when::{
    EventFields, StoredRange, StoredWhen, WALL_TIME, date_seconds, instant, wall_text,
};
use crate::import::Time;
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
pub(super) fn repeat_of(fields: &EventFields) -> (Option<Repeat>, bool) {
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
pub(super) fn rule_text(repeat: Repeat, kept: Option<&str>, when: &StoredWhen) -> Result<String> {
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

/// A rule from a file with the recurrence IDs of its `EXDATE`s, as FreeCal
/// stores them: the rule as given, but with an `UNTIL` in the form RFC 5545
/// asks for the Event at `when`, where one without a zone is read in the
/// Event Time Zone; the recurrence IDs sorted, each once. No rule and no
/// recurrence IDs when there is no rule or FreeCal can't expand it.
pub(crate) fn imported(
    text: Option<&str>,
    exdates: impl IntoIterator<Item = String>,
    when: &StoredWhen,
) -> (Option<String>, String) {
    match text.and_then(|text| imported_rule(text, when)) {
        Some(rule) => (Some(rule), exdates_of(exdates)),
        None => (None, String::new()),
    }
}

/// The rule of [`imported`].
fn imported_rule(text: &str, when: &StoredWhen) -> Option<String> {
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

/// The span of an Event's Occurrences: its derived start and end as Unix
/// seconds (see [`StoredWhen::instants`]), for a Recurring Event the start of
/// its first Occurrence and the end of its last, `i64::MAX` when it never
/// ends.
pub(crate) fn span(fields: &EventFields) -> Result<(i64, i64)> {
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

/// The most Occurrences [`span`] counts to find the last one, and
/// [`occurrences_in`] walks to find those in a range; a rule with more
/// counts as never ending.
const MAX_COUNTED: usize = 50_000;

/// The Occurrences of a Recurring Event in `range`, each with its
/// recurrence ID, the text of its original start as the Local Store writes
/// starts. Without those cancelled in its `exdates` and those in
/// `exceptions`, the recurrence IDs of its changed Occurrences, which are
/// Events of their own. None for an Event that doesn't repeat.
pub(super) fn occurrences_in(
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

/// The time of the Occurrence of a Recurring Event with the recurrence ID
/// `id`, as its rule gives it; none when the rule gives no such Occurrence
/// or it is cancelled.
pub(super) fn occurrence_at(fields: &EventFields, id: &str) -> Result<Option<StoredWhen>> {
    let (Some(starts), Some(wanted)) = (OccurrenceStarts::of(fields), start_of_id(id)) else {
        return Ok(None);
    };
    if split_ids(&fields.exdates).any(|cancelled| start_of_id(cancelled) == Some(wanted)) {
        return Ok(None);
    }
    for start in starts.into_iter().take(MAX_COUNTED) {
        if start == wanted {
            return occurrence(&fields.when, start).map(Some);
        }
        if start > wanted {
            break;
        }
    }
    Ok(None)
}

/// Whether FreeCal can read a Recurring Event's rule and expand it.
pub(super) fn readable(fields: &EventFields) -> bool {
    OccurrenceStarts::of(fields).is_some()
}

/// A rule of an Event whose first Occurrence moved from `from` to `to`, with
/// the days it names moved by as many days: "Mondays and Wednesdays" a day
/// later is "Tuesdays and Thursdays", "the 5th" the 6th. A day of the month
/// moved outside 1 to 31 can't be kept, and the rule repeats from its start
/// instead. A rule FreeCal can't read stays as it is.
pub(super) fn rule_moved(text: &str, from: &StoredWhen, to: &StoredWhen) -> String {
    let (Some(rule), Ok(from), Ok(to)) = (Rule::parse(text), first_start(from), first_start(to))
    else {
        return text.to_owned();
    };
    let days = (to.date() - from.date()).num_days();
    if days == 0 {
        return text.to_owned();
    }
    let parts: Vec<String> = rule
        .parts
        .iter()
        .filter_map(|(key, value)| {
            let value = match key.as_str() {
                "BYDAY" => value
                    .split(',')
                    .map(|day| weekday_moved(day, days))
                    .collect::<Option<Vec<_>>>()?
                    .join(","),
                "BYMONTHDAY" => value
                    .split(',')
                    .map(|day| month_day_moved(day, days))
                    .collect::<Option<Vec<_>>>()?
                    .join(","),
                _ => value.clone(),
            };
            Some(format!("{key}={value}"))
        })
        .collect();
    parts.join(";")
}

const WEEKDAYS: [&str; 7] = ["MO", "TU", "WE", "TH", "FR", "SA", "SU"];

/// A `BYDAY` value, such as `MO` or `-1FR`, `days` later; its ordinal stays.
fn weekday_moved(value: &str, days: i64) -> Option<String> {
    let value = value.trim();
    let (ordinal, day) = value.split_at(value.len().checked_sub(2)?);
    let index = WEEKDAYS
        .iter()
        .position(|known| known.eq_ignore_ascii_case(day))?;
    let moved = (index as i64 + days).rem_euclid(7) as usize;
    Some(format!("{ordinal}{}", WEEKDAYS[moved]))
}

/// A `BYMONTHDAY` value `days` later; none when it leaves 1 to 31. Days
/// counted from the end of the month stay.
fn month_day_moved(value: &str, days: i64) -> Option<String> {
    let day: i64 = value.trim().parse().ok()?;
    if day < 0 {
        return Some(day.to_string());
    }
    (1..=31)
        .contains(&(day + days))
        .then(|| (day + days).to_string())
}

/// Whether a Recurring Event's rule, which FreeCal can read, gives no
/// Occurrence at all.
pub(super) fn gives_none(fields: &EventFields) -> bool {
    OccurrenceStarts::of(fields).is_some_and(|starts| starts.into_iter().next().is_none())
}

/// `when` changed as the change from `from` to `to` changed another time:
/// its start and its end moved by as much as theirs, in wall time. A change
/// between all-day and timed puts `to` on `when`'s date instead.
pub(super) fn shifted(when: &StoredWhen, from: &StoredWhen, to: &StoredWhen) -> Result<StoredWhen> {
    if when.all_day != from.all_day || from.all_day != to.all_day {
        let days = first_start(when)?.date() - first_start(from)?.date();
        return occurrence(to, first_start(to)? + days);
    }
    let start = first_start(when)? + (first_start(to)? - first_start(from)?);
    let end = last_end(when)? + (last_end(to)? - last_end(from)?);
    let valid = if when.all_day {
        end > start
    } else {
        end >= start
    };
    if !valid {
        return Err(CoreError::EndBeforeStart);
    }
    let text = |time: NaiveDateTime| {
        if when.all_day {
            time.date().to_string()
        } else {
            wall_text(time)
        }
    };
    Ok(StoredWhen {
        start: text(start),
        end: text(end),
        ..when.clone()
    })
}

/// `when` changed as [`shifted`] changes it, but where that would end it
/// before it starts, its start moved by as much and its length kept.
pub(super) fn shifted_or_moved(
    when: &StoredWhen,
    from: &StoredWhen,
    to: &StoredWhen,
) -> Result<StoredWhen> {
    match shifted(when, from, to) {
        Err(CoreError::EndBeforeStart) => occurrence(
            when,
            first_start(when)? + (first_start(to)? - first_start(from)?),
        ),
        shifted => shifted,
    }
}

/// The recurrence ID `id` of a Recurring Event whose first Occurrence moved
/// from `from` to `to`, moved by as much; none when `id` isn't one.
pub(super) fn moved_id(id: &str, from: &StoredWhen, to: &StoredWhen) -> Option<String> {
    let start = start_of_id(id)? + (first_start(to).ok()? - first_start(from).ok()?);
    Some(if to.all_day {
        start.date().to_string()
    } else {
        wall_text(start)
    })
}

/// Whether the recurrence ID `id` names the same Occurrence as `split` or a
/// later one. One that isn't a recurrence ID is never later.
pub(super) fn is_from(id: &str, split: &str) -> bool {
    match (start_of_id(id), start_of_id(split)) {
        (Some(id), Some(split)) => id >= split,
        _ => false,
    }
}

/// The recurrence IDs in `exdates` that `keep` keeps, changed by `map`.
pub(super) fn exdates_where(
    exdates: &str,
    keep: impl Fn(&str) -> bool,
    map: impl Fn(&str) -> Option<String>,
) -> String {
    exdates_of(split_ids(exdates).filter(|id| keep(id)).filter_map(map))
}

/// The recurrence IDs `ids` as a Recurring Event's cancelled Occurrences
/// are stored: sorted, each once.
fn exdates_of(ids: impl IntoIterator<Item = String>) -> String {
    let mut ids: Vec<String> = ids.into_iter().collect();
    ids.sort_unstable();
    ids.dedup();
    ids.join(",")
}

/// A Recurring Event's rule for its Occurrences from the one with the
/// recurrence ID `id` on: a `COUNT` less the Occurrences before it.
pub(super) fn rule_from(fields: &EventFields, id: &str) -> Option<String> {
    let text = fields.rule.as_deref()?;
    let (Some(rule), Some(start)) = (Rule::parse(text), start_of_id(id)) else {
        return Some(text.to_owned());
    };
    let Some(count) = rule.rrule.get_count() else {
        return Some(text.to_owned());
    };
    let before = OccurrenceStarts::of(fields)?
        .into_iter()
        .take(MAX_COUNTED)
        .take_while(|earlier| *earlier < start)
        .count();
    let left = count.saturating_sub(u32::try_from(before).unwrap_or(u32::MAX));
    Some(format!(
        "{};COUNT={}",
        rule.without(&["COUNT"]),
        left.max(1)
    ))
}

/// A Recurring Event's rule ended with the last Occurrence before the one
/// with the recurrence ID `id`; none when FreeCal can't read it or there is
/// none before.
pub(super) fn rule_until(fields: &EventFields, id: &str) -> Option<String> {
    let rule = Rule::parse(fields.rule.as_deref()?)?;
    let start = start_of_id(id)?;
    let last = OccurrenceStarts::of(fields)?
        .into_iter()
        .take(MAX_COUNTED)
        .take_while(|earlier| *earlier < start)
        .last()?;
    // RFC 5545: a date for an all-day Event, otherwise a UTC instant.
    let until = if fields.when.all_day {
        last.format("%Y%m%d").to_string()
    } else {
        instant(last, start_zone(&fields.when).ok()?)
            .format("%Y%m%dT%H%M%SZ")
            .to_string()
    };
    Some(format!("{};UNTIL={until}", rule.without(&["COUNT"])))
}

/// A recurrence ID as the Local Store writes it, from the text the window
/// gives back or a stored field; none when it isn't one.
pub(crate) fn recurrence_id(text: &str) -> Option<String> {
    original_of_id(text).map(|original| id_of_original(&original))
}

/// The recurrence ID of an Exception without its Recurring Event, which
/// names the original start of the Occurrence it changed as a file gave it:
/// a date, a wall time with no zone, or one in the zone it was given in,
/// written after it as `2026-10-12T03:00:00[America/New_York]`. Paired with
/// its Recurring Event, the zone goes (see [`crate::import`]).
pub(crate) fn id_of_original(original: &Time) -> String {
    match original {
        Time::Date(date) => date.to_string(),
        Time::Wall(wall, None) => wall_text(*wall),
        Time::Wall(wall, Some(zone)) => format!("{}[{}]", wall_text(*wall), zone.name()),
    }
}

/// The original start a recurrence ID as [`id_of_original`] writes it
/// names; none when it isn't one, such as for a zone FreeCal doesn't know.
pub(crate) fn original_of_id(text: &str) -> Option<Time> {
    let (wall, zone) = match text.strip_suffix(']').and_then(|rest| rest.split_once('[')) {
        Some((wall, zone)) => (wall, Some(zone.parse::<Tz>().ok()?)),
        None => (text, None),
    };
    let start = start_of_id(wall)?;
    match (wall.contains('T'), zone) {
        (true, zone) => Some(Time::Wall(start, zone)),
        (false, None) => Some(Time::Date(start.date())),
        (false, Some(_)) => None,
    }
}

/// `exdates` with `id` added, sorted and each once.
pub(super) fn with_exdate(exdates: &str, id: &str) -> String {
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

/// The end of an Event, midnight for an all-day one.
fn last_end(when: &StoredWhen) -> Result<NaiveDateTime> {
    if when.all_day {
        let date = when
            .end
            .parse::<NaiveDate>()
            .map_err(|_| unreadable(when))?;
        Ok(date.and_time(NaiveTime::MIN))
    } else {
        NaiveDateTime::parse_from_str(&when.end, WALL_TIME).map_err(|_| unreadable(when))
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
