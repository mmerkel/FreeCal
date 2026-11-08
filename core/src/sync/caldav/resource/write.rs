//! Writing the content lines of the CalDAV resource to send: FreeCal's
//! Properties (new term: the properties of a `VEVENT` that an Event Row
//! holds: title, times and their zones, location, description, repeat
//! rule, excluded dates and recurrence ID) of an Event Row, a new `VEVENT`
//! (which export, ticket 09, may reuse), and the `VTIMEZONE`s they need.
//!
//! Event Rows are untrusted (spec: "Untrusted content"). Every content line
//! goes through the `icalendar` library's writer, which escapes TEXT
//! values, and is folded to lines of at most 75 octets; TEXT values first
//! lose the control characters iCalendar can't hold. Values that aren't
//! TEXT are formatted here from parsed times and from rules FreeCal can
//! expand, never copied from a stored field.

use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, Offset, TimeDelta, TimeZone, Utc};
use chrono_tz::{OffsetComponents, OffsetName, Tz};
use icalendar::Property;
use icalendar::parser::unfold;

use crate::event::{EventFields, normal_form, rule};
use crate::import::{ImportedWhen, Time};
use crate::{CoreError, Result};

/// Content lines written, with the time zones their times are given in,
/// each with the earliest wall time given in it, from which its
/// `VTIMEZONE` covers.
pub(super) struct Written {
    text: String,
    zones: Vec<(Tz, NaiveDateTime)>,
}

impl Written {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            zones: Vec::new(),
        }
    }

    pub fn into_text(self) -> String {
        self.text
    }

    pub fn zones(&self) -> &[(Tz, NaiveDateTime)] {
        &self.zones
    }

    /// Adds `other`'s lines after these, and its zones.
    pub fn append(&mut self, other: &Written) {
        self.text.push_str(&other.text);
        for &(zone, wall) in &other.zones {
            self.zone(zone, wall);
        }
    }

    /// A content line as received, kept byte for byte: its physical lines,
    /// without their line ends, each ended with CRLF.
    pub fn kept(&mut self, physical: &[&str]) {
        for line in physical {
            self.text.push_str(line);
            self.text.push_str("\r\n");
        }
    }

    /// One content line, through the library's writer, folded again: the
    /// library leaves a line of 76 octets when it folds before a space.
    fn property(&mut self, property: Property) {
        let line: String = property
            .try_into()
            .expect("writing to a String doesn't fail");
        // The value is escaped, so the line holds no line break to unfold.
        let mut rest = unfold(&line);
        rest.truncate(rest.trim_end_matches("\r\n").len());
        let mut limit = 75;
        while rest.len() > limit {
            let mut at = limit;
            while !rest.is_char_boundary(at) {
                at -= 1;
            }
            self.text.push_str(&rest[..at]);
            self.text.push_str("\r\n ");
            rest.drain(..at);
            // A continued line starts with the space.
            limit = 74;
        }
        self.text.push_str(&rest);
        self.text.push_str("\r\n");
    }

    pub fn begin(&mut self, kind: &str) {
        self.property(Property::new("BEGIN", kind));
    }

    pub fn end(&mut self, kind: &str) {
        self.property(Property::new("END", kind));
    }

    /// A TEXT property, unless `value` is empty once it can be written.
    pub fn text_property(&mut self, name: &str, value: &str) {
        let value = text_value(value);
        if !value.is_empty() {
            self.property(Property::new(name, value));
        }
    }

    /// A property of the times `times`, all of the kind of the first, as
    /// `DTSTART`, `DTEND`, `RECURRENCE-ID` and `EXDATE` take them: a date as
    /// a date, a wall time in UTC as a UTC time, one in another zone with
    /// its `TZID`, whose `VTIMEZONE` it then needs, and one without a zone
    /// floating.
    fn times(&mut self, name: &str, times: &[Time]) {
        let Some(first) = times.first() else {
            return;
        };
        let (format, parameter) = match first {
            Time::Date(_) => ("%Y%m%d", Some(("VALUE", "DATE"))),
            Time::Wall(_, Some(Tz::UTC)) => ("%Y%m%dT%H%M%SZ", None),
            Time::Wall(_, Some(zone)) => ("%Y%m%dT%H%M%S", Some(("TZID", zone.name()))),
            Time::Wall(_, None) => ("%Y%m%dT%H%M%S", None),
        };
        let values: Vec<String> = times
            .iter()
            .map(|time| match time {
                Time::Date(date) => date.format(format).to_string(),
                Time::Wall(wall, _) => wall.format(format).to_string(),
            })
            .collect();
        if let Time::Wall(_, Some(zone)) = first {
            for time in times {
                if let (Time::Wall(wall, _), false) = (time, *zone == Tz::UTC) {
                    self.zone(*zone, *wall);
                }
            }
        }
        let mut property = Property::new(name, values.join(","));
        if let Some((key, value)) = parameter {
            property.add_parameter(key, value);
        }
        self.property(property);
    }

    /// Notes that the wall time `wall` is given in `zone`.
    fn zone(&mut self, zone: Tz, wall: NaiveDateTime) {
        match self.zones.iter_mut().find(|(known, _)| *known == zone) {
            Some((_, earliest)) => *earliest = (*earliest).min(wall),
            None => self.zones.push((zone, wall)),
        }
    }
}

/// `text` as an iCalendar TEXT value can hold it: line breaks as `\n`,
/// which the writer escapes, and without the other control characters,
/// which RFC 5545 doesn't allow, so that no text can end its line.
fn text_value(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .chars()
        .filter(|c| !c.is_ascii_control() || matches!(c, '\n' | '\t'))
        .collect()
}

/// A group of FreeCal's Properties that is kept or written as one: a
/// `VEVENT` keeps the content lines of a group whose field FreeCal reads
/// from them as the Event Row holds it, and gets them written anew from the
/// Event Row otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Group {
    /// The recurrence ID. An Event Row and the `VEVENT` it pairs with name
    /// the same Occurrence, but for an Exception whose Recurring Event's
    /// time moved, which moved its Occurrence too (ADR 0012).
    RecurrenceId,
    When,
    Rule,
    Cancelled,
    Title,
    Location,
    Description,
}

impl Group {
    /// In the order a new `VEVENT` has them.
    pub const ALL: [Group; 7] = [
        Group::RecurrenceId,
        Group::When,
        Group::Rule,
        Group::Cancelled,
        Group::Title,
        Group::Location,
        Group::Description,
    ];

    /// The group of the property `name`, in upper case, if it is one of
    /// FreeCal's Properties.
    pub fn of(name: &str) -> Option<Group> {
        Some(match name {
            "RECURRENCE-ID" => Group::RecurrenceId,
            "DTSTART" | "DTEND" | "DURATION" => Group::When,
            "RRULE" => Group::Rule,
            "EXDATE" => Group::Cancelled,
            "SUMMARY" => Group::Title,
            "LOCATION" => Group::Location,
            // The description as HTML, which Outlook and Thunderbird show
            // in its place: an edited description goes without it.
            "DESCRIPTION" | "X-ALT-DESC" => Group::Description,
            _ => return None,
        })
    }

    /// Whether `row`'s field of this group differs from `server`'s, the
    /// fields FreeCal reads from the `VEVENT` it pairs with. Recurrence IDs
    /// are compared by Occurrence, against the Recurring Event at
    /// `recurring` (ADR 0012).
    pub fn differs(
        self,
        row: &EventFields,
        server: &EventFields,
        recurring: Option<&ImportedWhen>,
    ) -> bool {
        match self {
            Group::RecurrenceId => normal_form(row, recurring) != normal_form(server, recurring),
            Group::When => row.when != server.when,
            Group::Rule => row.rule != server.rule,
            Group::Cancelled => row.exdates != server.exdates,
            Group::Title => row.title != server.title,
            Group::Location => row.location != server.location,
            Group::Description => row.description != server.description,
        }
    }

    /// Writes this group's properties from `row`, an Event Row of the
    /// Recurring Event at `recurring`, if it has one. An Exception's
    /// recurrence ID is written as obtained (ADR 0012); one in the Normal
    /// Form, a wall time without a zone, is given in the Event Time Zone of
    /// `recurring`, as it names the Occurrence there, and floats without
    /// one. A rule FreeCal can't expand isn't written, nor its cancelled
    /// Occurrences, as the Event shows as its first Occurrence only.
    pub fn write(
        self,
        row: &EventFields,
        recurring: Option<&ImportedWhen>,
        out: &mut Written,
    ) -> Result<()> {
        match self {
            Group::RecurrenceId => {
                if let Some(recurrence_id) = &row.recurrence_id {
                    let original = recurrence_id.original().ok_or_else(|| {
                        CoreError::UnreadableEventTime(recurrence_id.as_obtained().to_owned())
                    })?;
                    let original = match (original, recurring) {
                        (
                            &Time::Wall(wall, None),
                            Some(&ImportedWhen::Timed { start_zone, .. }),
                        ) => Time::Wall(wall, Some(start_zone)),
                        (original, _) => original.clone(),
                    };
                    out.times("RECURRENCE-ID", &[original]);
                }
            }
            Group::When => match row.when.to_imported()? {
                ImportedWhen::AllDay { start, end } => {
                    out.times("DTSTART", &[Time::Date(start)]);
                    out.times("DTEND", &[Time::Date(end)]);
                }
                ImportedWhen::Timed {
                    start,
                    start_zone,
                    end,
                    end_zone,
                } => {
                    out.times("DTSTART", &[Time::Wall(start, Some(start_zone))]);
                    out.times("DTEND", &[Time::Wall(end, Some(end_zone))]);
                }
            },
            Group::Rule => {
                if let Some(rule) = rule::written(row) {
                    out.property(Property::new("RRULE", rule));
                }
            }
            Group::Cancelled => {
                if rule::written(row).is_some() {
                    let starts = rule::cancelled_starts(row);
                    let times: Vec<Time> = match row.when.to_imported()? {
                        ImportedWhen::AllDay { .. } => starts
                            .into_iter()
                            .map(|start| Time::Date(start.date()))
                            .collect(),
                        ImportedWhen::Timed { start_zone, .. } => starts
                            .into_iter()
                            .map(|start| Time::Wall(start, Some(start_zone)))
                            .collect(),
                    };
                    out.times("EXDATE", &times);
                }
            }
            Group::Title => out.text_property("SUMMARY", &row.title),
            Group::Location => out.text_property("LOCATION", &row.location),
            Group::Description => out.text_property("DESCRIPTION", &row.description),
        }
        Ok(())
    }
}

/// The properties that say when a `VEVENT` was last revised and how often
/// (RFC 5545 §3.8.7): a `VEVENT` whose FreeCal's Properties are written
/// anew gets them written anew too, as `DTSTAMP` means `LAST-MODIFIED`
/// without a `METHOD`, and RFC 5546 §2.1.4 asks for a higher `SEQUENCE`
/// when times, the repeat rule or excluded dates change. One rule covers
/// every change: other changes may raise it too, and a higher `SEQUENCE`
/// only marks the version as the later one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Revision {
    Sequence,
    Stamp,
    Modified,
}

impl Revision {
    pub const ALL: [Revision; 3] = [Revision::Sequence, Revision::Stamp, Revision::Modified];

    /// The revision property `name`, in upper case, if it is one.
    pub fn of(name: &str) -> Option<Revision> {
        Some(match name {
            "SEQUENCE" => Revision::Sequence,
            "DTSTAMP" => Revision::Stamp,
            "LAST-MODIFIED" => Revision::Modified,
            _ => return None,
        })
    }

    /// Writes it for a revision made `now`, the `sequence`th.
    pub fn write(self, sequence: i64, now: DateTime<Utc>, out: &mut Written) {
        let now = [Time::Wall(now.naive_utc(), Some(Tz::UTC))];
        match self {
            Revision::Sequence => out.property(Property::new("SEQUENCE", sequence.to_string())),
            Revision::Stamp => out.times("DTSTAMP", &now),
            Revision::Modified => out.times("LAST-MODIFIED", &now),
        }
    }
}

/// Writes `row`, an Event Row of the UID `uid` and of the Recurring Event
/// at `recurring`, if it has one, as a new `VEVENT`, stamped and last
/// revised `now`; its `SEQUENCE` is 0, the default.
pub(super) fn new_vevent(
    uid: &str,
    row: &EventFields,
    recurring: Option<&ImportedWhen>,
    now: DateTime<Utc>,
    out: &mut Written,
) -> Result<()> {
    out.begin("VEVENT");
    out.text_property("UID", uid);
    Revision::Stamp.write(0, now, out);
    Revision::Modified.write(0, now, out);
    for group in Group::ALL {
        group.write(row, recurring, out)?;
    }
    out.end("VEVENT");
    Ok(())
}

/// The last year a `VTIMEZONE` covers: the time zone rules FreeCal is built
/// with give no changes after it.
const LAST_COVERED_YEAR: i32 = 2099;

/// Writes the `VTIMEZONE` of `zone` from the time zone rules FreeCal is
/// built with, so that clients that don't know IANA names read the times
/// as FreeCal does: each offset change from the start of the year of
/// `from` to the end of [`LAST_COVERED_YEAR`], those of one kind in one
/// observance, its first as its `DTSTART` and the others as `RDATE`s, after
/// an observance for the offset the year starts with.
pub(super) fn vtimezone(zone: Tz, from: NaiveDateTime, out: &mut Written) {
    let year_start = |year: i32| {
        NaiveDate::from_ymd_opt(year, 1, 1)
            .expect("the first of January exists")
            .and_hms_opt(0, 0, 0)
            .expect("midnight exists")
    };
    // A day early, so that the start of the year in the zone is covered.
    let start = year_start(from.year()).and_utc() - TimeDelta::days(1);
    let until = year_start(LAST_COVERED_YEAR + 1).and_utc();
    let first = ZoneOffset::of(zone, start);
    let mut observances = vec![Observance {
        from: first.utc_offset,
        kind: first.clone(),
        onsets: vec![start.naive_utc() + first.delta()],
    }];
    let mut day = start;
    let mut current = first;
    while day < until {
        let next_day = day + TimeDelta::days(1);
        let next = ZoneOffset::of(zone, next_day);
        if next != current {
            // The first second of the new offset, as no zone changes its
            // offset twice within a day.
            let (mut before, mut after) = (day, next_day);
            while after - before > TimeDelta::seconds(1) {
                let middle = before + (after - before) / 2;
                if ZoneOffset::of(zone, middle) == current {
                    before = middle;
                } else {
                    after = middle;
                }
            }
            let onset = after.naive_utc() + current.delta();
            match observances
                .iter_mut()
                .find(|observance| observance.kind == next && observance.from == current.utc_offset)
            {
                Some(observance) => observance.onsets.push(onset),
                None => observances.push(Observance {
                    kind: next.clone(),
                    from: current.utc_offset,
                    onsets: vec![onset],
                }),
            }
            current = next;
        }
        day = next_day;
    }
    out.begin("VTIMEZONE");
    out.text_property("TZID", zone.name());
    for observance in &observances {
        let kind = if observance.kind.dst {
            "DAYLIGHT"
        } else {
            "STANDARD"
        };
        out.begin(kind);
        let (first, others) = observance
            .onsets
            .split_first()
            .expect("an observance has an onset");
        out.times("DTSTART", &[Time::Wall(*first, None)]);
        let others: Vec<Time> = others
            .iter()
            .map(|onset| Time::Wall(*onset, None))
            .collect();
        out.times("RDATE", &others);
        out.property(Property::new("TZOFFSETFROM", utc_offset(observance.from)));
        out.property(Property::new(
            "TZOFFSETTO",
            utc_offset(observance.kind.utc_offset),
        ));
        if let Some(name) = &observance.kind.name {
            out.text_property("TZNAME", name);
        }
        out.end(kind);
    }
    out.end("VTIMEZONE");
}

/// The offsets of one kind that a zone changes to, from the offset
/// `from`, with the wall times they start at in the offset before.
struct Observance {
    kind: ZoneOffset,
    from: i32,
    onsets: Vec<NaiveDateTime>,
}

/// A zone's offset from UTC at one moment, as a `VTIMEZONE` tells them
/// apart.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ZoneOffset {
    utc_offset: i32,
    dst: bool,
    name: Option<String>,
}

impl ZoneOffset {
    fn of(zone: Tz, at: DateTime<Utc>) -> Self {
        let offset = zone.offset_from_utc_datetime(&at.naive_utc());
        Self {
            utc_offset: offset.fix().local_minus_utc(),
            dst: !offset.dst_offset().is_zero(),
            name: offset.abbreviation().map(str::to_owned),
        }
    }

    fn delta(&self) -> TimeDelta {
        TimeDelta::seconds(self.utc_offset.into())
    }
}

/// An offset in seconds as a `UTC-OFFSET` value, such as `+0100` or
/// `+005328`.
fn utc_offset(seconds: i32) -> String {
    let sign = if seconds < 0 { '-' } else { '+' };
    let seconds = seconds.unsigned_abs();
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if seconds == 0 {
        format!("{sign}{hours:02}{minutes:02}")
    } else {
        format!("{sign}{hours:02}{minutes:02}{seconds:02}")
    }
}
