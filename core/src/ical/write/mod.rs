//! The writer of one CalDAV resource (ADR 0017): the iCalendar text of a
//! UID's single Events, each built from its managed fields and its
//! Unimplemented Fields alone, with no text from a server. The reader
//! ([`super::read`]) reads what it writes back to the same managed fields
//! and an equal tree.
//!
//! - **Managed properties** are written from the managed fields, each with
//!   the unknown parameters the tree holds under its name. The end is
//!   always a `DTEND`, also one read from `DURATION`.
//! - **The tree's entries** go back as properties and components after the
//!   managed properties, in the tree's order; the `ical:VCALENDAR` entries
//!   of the Event the reader puts them on go outside the `VEVENT`s. An
//!   entry without the `ical:` prefix isn't written, as a Calendar of
//!   another Provider holds it; what of an `ical:` entry isn't written is
//!   returned ([`LeftOut`]).
//! - **Fresh on every write:** `DTSTAMP`, `LAST-MODIFIED`, `VERSION`,
//!   `PRODID`, and a `VTIMEZONE` for each zone a written time is given in,
//!   by a managed field or by a `TZID` of the tree that names a zone
//!   FreeCal knows, covering from the earliest time given in it. The
//!   `VTIMEZONE` of a `TZID` FreeCal doesn't know is the one the tree keeps
//!   (an `ical:VTIMEZONE` entry), written at the calendar level, once.
//!   `SEQUENCE` is an entry like any other, written back unchanged.
//! - **Order:** the Event that isn't an Exception first, then its
//!   Exceptions by recurrence ID, so that the same input gives the same
//!   text.
//!
//! Managed fields and tree alike are untrusted (spec: "Untrusted content").
//! The writer writes every content line itself: a TEXT value of a managed
//! field loses the control characters iCalendar can't hold and is escaped;
//! a value of the tree is written as it is held, escapes and all, without
//! its control characters; a parameter value loses those and its quotes,
//! and is quoted where it holds `:`, `;` or `,`; a name that isn't an
//! iCalendar name, or that would begin or end a component, isn't written.
//! Lines are folded at 75 octets, between characters, and end with CRLF.

use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, Offset, TimeDelta, TimeZone, Utc};
use chrono_tz::{OffsetComponents, OffsetName, Tz, TzOffset};

use std::collections::BTreeMap;
use std::fmt;

use super::read::{Time, iana_zone, unquoted};
use super::{is_name, tree_tzids};
use crate::Result;
use crate::event::{EventFields, rule};
use crate::import::ImportedWhen;
use crate::unimplemented::{
    Component, Entry, EntryValue, ICAL, Parameter, Property, UnimplementedFields,
};

/// The `PRODID` of every resource FreeCal writes.
const PRODID: &str = "-//FreeCal//FreeCal//EN";

/// The parameters of a time that the writer writes from the managed time,
/// so never from the tree.
const TIME_PARAMETERS: &[&str] = &["TZID", "VALUE"];

/// The iCalendar text of the CalDAV resource of the UID `uid`, holding
/// `events`, its single Events, each as managed fields and tree, stamped
/// and last modified `now`.
#[allow(dead_code, reason = "until ticket 187 sends through it")]
pub(crate) fn write_resource(
    uid: &str,
    events: &[(&EventFields, &UnimplementedFields)],
    now: DateTime<Utc>,
) -> Result<Written> {
    let mut events = events.to_vec();
    // Stable, so that revisions of one Occurrence keep their order.
    events.sort_by_cached_key(|(fields, _)| {
        let id = fields.recurrence_id.as_ref();
        id.map(|id| id.normal_form(None).into_string())
    });
    let recurring = match events.first() {
        Some((fields, _)) if fields.recurrence_id.is_none() => Some(fields.when.to_imported()?),
        _ => None,
    };
    let mut vevents = Lines::default();
    for (fields, tree) in &events {
        vevent(uid, fields, tree, recurring.as_ref(), now, &mut vevents)?;
    }
    let mut out = Lines::default();
    out.begin("VCALENDAR");
    out.line("VERSION", &[], "2.0");
    out.line("PRODID", &[], PRODID);
    // The reader puts the calendar level on the first Event of this order.
    let level = events.first().map(|(_, tree)| calendar_level(tree));
    let (properties, components) = level.unwrap_or_default();
    let occurrence = |fields: &EventFields| {
        let id = fields.recurrence_id.as_ref();
        id.map(|id| id.normal_form(None).into_string())
    };
    let first = events.first().map(|(fields, _)| occurrence(fields));
    for (fields, tree) in events.iter().skip(1) {
        let other = calendar_level(tree);
        let held = !other.0.is_empty() || !other.1.is_empty();
        let revision = Some(occurrence(fields)) == first;
        if held && (!revision || (&other.0, &other.1) != (&properties, &components)) {
            out.leave_out(LeftOut::Misplaced);
        }
    }
    for property in &properties {
        if matches!(property.name.as_str(), "VERSION" | "PRODID") {
            out.leave_out(LeftOut::SetByWriter);
        } else {
            out.tree_property(property);
        }
    }
    for (tzid, zone, from) in &vevents.zones {
        vtimezone(tzid, *zone, *from, &mut out);
    }
    for kept in kept_vtimezones(&events, &vevents.zones, &mut out) {
        out.tree_component(&kept);
    }
    for component in &components {
        if matches!(component.name.as_str(), "VEVENT" | "VTIMEZONE") {
            out.leave_out(LeftOut::Structure);
        } else {
            out.tree_component(component);
        }
    }
    out.text.push_str(&vevents.text);
    out.end("VCALENDAR");
    for left_out in vevents.left_out {
        out.leave_out(left_out);
    }
    Ok(Written {
        text: out.text,
        left_out: out.left_out,
    })
}

/// The text of a resource, and what of its fields and trees isn't in it.
#[allow(
    dead_code,
    reason = "until ticket 187 sends the text and logs the rest"
)]
pub(crate) struct Written {
    pub(crate) text: String,
    /// For the sender's Log; none unless the Local Store holds what the
    /// reader never makes.
    pub(crate) left_out: Vec<LeftOut>,
}

/// What of the fields or the tree the writer leaves out. It names no
/// untrusted content, so a Log line may hold it as it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum LeftOut {
    /// A control character other than a tab.
    ControlCharacter,
    /// A quote in a parameter value.
    Quote,
    /// A property, parameter or component whose name has characters other
    /// than letters, digits and dashes.
    NotAName,
    /// A property `BEGIN` or `END`, or a `VEVENT` or `VTIMEZONE` at the
    /// calendar level, which would change the text's structure.
    Structure,
    /// A property the writer sets itself, such as `DTSTAMP` or `DURATION`
    /// (the end is always a `DTEND`), the `TZID` or `VALUE` of a time, or a
    /// `VTIMEZONE` it writes fresh.
    SetByWriter,
    /// An entry where the reader never puts it: the parameters of a
    /// property that isn't written; a property the reader would read as a
    /// managed field (a `RECURRENCE-ID` on an Event that isn't an
    /// Exception, a rule FreeCal can expand, a readable `EXDATE` of a
    /// Recurring Event); a calendar level other than on the revisions of
    /// the first Event, or unlike the first's; a `VCALENDAR` or
    /// `VTIMEZONE` that isn't a component; or a `VTIMEZONE` of a `TZID` the
    /// tree doesn't use.
    Misplaced,
}

impl fmt::Display for LeftOut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            LeftOut::ControlCharacter => "control characters",
            LeftOut::Quote => "quotes in a parameter value",
            LeftOut::NotAName => "a name with characters other than letters, digits and dashes",
            LeftOut::Structure => "a property or component that would change the structure",
            LeftOut::SetByWriter => "a property the writer sets itself",
            LeftOut::Misplaced => "an entry where the reader never puts it",
        })
    }
}

/// The `ical:VTIMEZONE` entries of the trees of `events`, those of the
/// `TZID`s FreeCal doesn't know: one per `TZID`, the first, by `TZID`, none
/// for a zone written fresh.
fn kept_vtimezones(
    events: &[(&EventFields, &UnimplementedFields)],
    fresh: &[(String, Tz, NaiveDateTime)],
    out: &mut Lines,
) -> Vec<Component> {
    let mut kept: BTreeMap<String, Component> = BTreeMap::new();
    let is_zone = |entry: &&Entry| entry.name.strip_prefix(ICAL) == Some("VTIMEZONE");
    let zones = events.iter().flat_map(|(_, tree)| {
        let others = tree.entries().iter().filter(|entry| !is_zone(entry));
        let used = tree_tzids(&UnimplementedFields::new(others.cloned().collect()));
        let zones = tree.entries().iter().filter(is_zone);
        zones.map(move |zone| (zone, used.clone()))
    });
    for (entry, used) in zones {
        let EntryValue::Component {
            properties,
            components,
        } = &entry.value
        else {
            out.leave_out(LeftOut::Misplaced);
            continue;
        };
        let tzid = properties
            .iter()
            .find(|property| property.name.eq_ignore_ascii_case("TZID"))
            .map(|property| unquoted(&property.value).to_owned())
            .unwrap_or_default();
        if fresh.iter().any(|(written, ..)| *written == tzid) {
            out.leave_out(LeftOut::SetByWriter);
        } else if !used.iter().any(|(named, _)| *named == tzid) {
            // The reader keeps only the VTIMEZONE of a TZID the tree uses.
            out.leave_out(LeftOut::Misplaced);
        } else {
            kept.entry(tzid).or_insert_with(|| Component {
                name: "VTIMEZONE".to_owned(),
                properties: properties.clone(),
                components: components.clone(),
            });
        }
    }
    kept.into_values().collect()
}

/// The properties and components of the `ical:VCALENDAR` entries of `tree`.
fn calendar_level(tree: &UnimplementedFields) -> (Vec<Property>, Vec<Component>) {
    let mut properties = Vec::new();
    let mut components = Vec::new();
    for entry in tree.entries() {
        if let (
            Some("VCALENDAR"),
            EntryValue::Component {
                properties: p,
                components: c,
            },
        ) = (entry.name.strip_prefix(ICAL), &entry.value)
        {
            properties.extend(p.iter().cloned());
            components.extend(c.iter().cloned());
        }
    }
    (properties, components)
}

/// Writes the `VEVENT` of `fields` and `tree`, a single Event of the UID
/// `uid` and of the Recurring Event at `recurring`, if it has one.
fn vevent(
    uid: &str,
    fields: &EventFields,
    tree: &UnimplementedFields,
    recurring: Option<&ImportedWhen>,
    now: DateTime<Utc>,
    out: &mut Lines,
) -> Result<()> {
    // The managed properties written, whose parameters the tree may hold.
    let mut managed = vec![
        "UID",
        "DTSTART",
        "DTEND",
        "SUMMARY",
        "LOCATION",
        "DESCRIPTION",
    ];
    let unknown = |name: &str| unknown_parameters(tree, name);
    out.begin("VEVENT");
    out.escaped_text("UID", &unknown("UID"), uid);
    let now = [Time::Wall(now.naive_utc(), Some(Tz::UTC))];
    out.times("DTSTAMP", &[], &now);
    out.times("LAST-MODIFIED", &[], &now);
    if let Some(recurrence_id) = &fields.recurrence_id {
        // One in the Normal Form, a wall time without a zone, names the
        // Occurrence in the Event Time Zone of its Recurring Event (ADR
        // 0012).
        let original = match (recurrence_id.original(), recurring) {
            (&Time::Wall(wall, None), Some(&ImportedWhen::Timed { start_zone, .. })) => {
                Time::Wall(wall, Some(start_zone))
            }
            (original, _) => original.clone(),
        };
        out.times("RECURRENCE-ID", &unknown("RECURRENCE-ID"), &[original]);
        managed.push("RECURRENCE-ID");
    }
    let when = fields.when.to_imported()?;
    let (start, end) = match when {
        ImportedWhen::AllDay { start, end } => (Time::Date(start), Time::Date(end)),
        ImportedWhen::Timed {
            start,
            start_zone,
            end,
            end_zone,
        } => (
            Time::Wall(start, Some(start_zone)),
            Time::Wall(end, Some(end_zone)),
        ),
    };
    out.times("DTSTART", &unknown("DTSTART"), &[start]);
    out.times("DTEND", &unknown("DTEND"), &[end]);
    if let Some(rule) = rule::written(fields) {
        out.line("RRULE", &unknown("RRULE"), &rule);
        managed.push("RRULE");
        let cancelled: Vec<Time> = rule::cancelled_starts(fields)
            .into_iter()
            .map(|start| match when {
                ImportedWhen::AllDay { .. } => Time::Date(start.date()),
                ImportedWhen::Timed { start_zone, .. } => Time::Wall(start, Some(start_zone)),
            })
            .collect();
        if !cancelled.is_empty() {
            out.times("EXDATE", &unknown("EXDATE"), &cancelled);
            managed.push("EXDATE");
        }
    }
    out.escaped_text("SUMMARY", &unknown("SUMMARY"), &fields.title);
    out.escaped_text("LOCATION", &unknown("LOCATION"), &fields.location);
    out.escaped_text("DESCRIPTION", &unknown("DESCRIPTION"), &fields.description);
    let entries = tree.entries().iter();
    let entries = entries.filter_map(|entry| Some((entry.name.strip_prefix(ICAL)?, &entry.value)));
    let (entries, outside): (Vec<_>, Vec<_>) =
        entries.partition(|(name, _)| !matches!(*name, "VCALENDAR" | "VTIMEZONE"));
    for (_, value) in outside {
        if !matches!(value, EntryValue::Component { .. }) {
            out.leave_out(LeftOut::Misplaced);
        }
    }
    let exception = fields.recurrence_id.is_some();
    let ruled = managed.contains(&"RRULE");
    for (name, value) in &entries {
        if let EntryValue::Parameters(parameters) = value
            && !parameters.is_empty()
            && !managed.contains(name)
        {
            out.leave_out(LeftOut::Misplaced);
        }
    }
    for (name, value) in &entries {
        if let EntryValue::Property { parameters, value } = value {
            if matches!(*name, "DTSTAMP" | "LAST-MODIFIED" | "DURATION") {
                out.leave_out(LeftOut::SetByWriter);
                continue;
            }
            // What the reader would read as a managed field, where the
            // managed property doesn't come first.
            let read_as_managed = match *name {
                "RECURRENCE-ID" => !exception,
                "RRULE" => !exception && !ruled && rule::imported(value, &fields.when).is_some(),
                "EXDATE" => ruled && value.split(',').all(is_time),
                _ => false,
            };
            if read_as_managed {
                out.leave_out(LeftOut::Misplaced);
                continue;
            }
            out.tree_property(&Property {
                name: (*name).to_owned(),
                parameters: parameters.clone(),
                value: value.clone(),
            });
        }
    }
    for (name, value) in &entries {
        if let EntryValue::Component {
            properties,
            components,
        } = value
        {
            out.tree_component(&Component {
                name: (*name).to_owned(),
                properties: properties.clone(),
                components: components.clone(),
            });
        }
    }
    out.end("VEVENT");
    out.tree_zones(tree, &when);
    Ok(())
}

/// The parameters the entries of `tree` hold for the managed property
/// `name`.
fn unknown_parameters(tree: &UnimplementedFields, name: &str) -> Vec<Parameter> {
    let entries = tree.entries().iter();
    let held = entries.filter(|entry| entry.name.strip_prefix(ICAL) == Some(name));
    held.filter_map(|entry| match &entry.value {
        EntryValue::Parameters(parameters) => Some(parameters.iter().cloned()),
        _ => None,
    })
    .flatten()
    .collect()
}

/// Content lines written, with the zones their times are given in: each
/// `TZID` with its zone and the earliest wall time given in it, from which
/// its `VTIMEZONE` covers.
#[derive(Default)]
struct Lines {
    text: String,
    zones: Vec<(String, Tz, NaiveDateTime)>,
    left_out: Vec<LeftOut>,
}

impl Lines {
    fn begin(&mut self, kind: &str) {
        self.line("BEGIN", &[], kind);
    }

    fn end(&mut self, kind: &str) {
        self.line("END", &[], kind);
    }

    /// The content line `name`, with `parameters` and `value` as given, but
    /// for the characters a line can't hold; folded.
    fn line(&mut self, name: &str, parameters: &[Parameter], value: &str) {
        let mut line = name.to_owned();
        for parameter in parameters {
            if !is_name(&parameter.name) {
                self.leave_out(LeftOut::NotAName);
                continue;
            }
            line.push(';');
            line.push_str(&parameter.name);
            let values: Vec<String> = parameter
                .values
                .iter()
                .map(|value| {
                    if value.contains('"') {
                        self.leave_out(LeftOut::Quote);
                    }
                    self.without_controls(value).replace('"', "")
                })
                .collect();
            match values.as_slice() {
                [] => {}
                [one] if !one.contains([':', ';', ',']) => {
                    line.push('=');
                    line.push_str(one);
                }
                several => {
                    line.push('=');
                    let quoted: Vec<String> = several.iter().map(|v| format!("\"{v}\"")).collect();
                    line.push_str(&quoted.join(","));
                }
            }
        }
        line.push(':');
        line.push_str(&self.without_controls(value));
        self.folded(&line);
    }

    /// `value` without the control characters a content line can't hold,
    /// noted as left out.
    fn without_controls(&mut self, value: &str) -> String {
        let kept = without_controls(value);
        if kept.len() != value.len() {
            self.leave_out(LeftOut::ControlCharacter);
        }
        kept
    }

    /// Notes that something of the fields or the tree isn't written.
    fn leave_out(&mut self, left_out: LeftOut) {
        if !self.left_out.contains(&left_out) {
            self.left_out.push(left_out);
        }
    }

    /// `line`, folded to lines of at most 75 octets, between characters.
    fn folded(&mut self, line: &str) {
        let mut rest = line;
        let mut limit = 75;
        while rest.len() > limit {
            let mut at = limit;
            while !rest.is_char_boundary(at) {
                at -= 1;
            }
            self.text.push_str(&rest[..at]);
            self.text.push_str("\r\n ");
            rest = &rest[at..];
            // A continued line starts with the space.
            limit = 74;
        }
        self.text.push_str(rest);
        self.text.push_str("\r\n");
    }

    /// A TEXT property, escaped; none if `value` is empty
    /// and there are no parameters to keep.
    fn escaped_text(&mut self, name: &str, parameters: &[Parameter], value: &str) {
        let held = text_value(value);
        if held.chars().count() != value.replace("\r\n", "\n").chars().count() {
            self.leave_out(LeftOut::ControlCharacter);
        }
        let value = escaped(&held);
        if !value.is_empty() || !parameters.is_empty() {
            self.line(name, parameters, &value);
        }
    }

    /// A property of the times `times`, all of the kind of the first, as
    /// `DTSTART`, `DTEND`, `RECURRENCE-ID` and `EXDATE` take them: a date as
    /// a date, a wall time in UTC as a UTC time, one in another zone with
    /// its `TZID`, whose `VTIMEZONE` it then needs, and one without a zone
    /// floating. `unknown`'s parameters follow, but for those of a time.
    fn times(&mut self, name: &str, unknown: &[Parameter], times: &[Time]) {
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
        if let Time::Wall(_, Some(zone)) = first
            && *zone != Tz::UTC
        {
            for time in times {
                if let Time::Wall(wall, _) = time {
                    self.zone(zone.name(), *zone, *wall);
                }
            }
        }
        let mut parameters: Vec<Parameter> = parameter
            .map(|(name, value)| Parameter {
                name: name.to_owned(),
                values: vec![value.to_owned()],
            })
            .into_iter()
            .collect();
        for parameter in unknown {
            if TIME_PARAMETERS.contains(&parameter.name.as_str()) {
                self.leave_out(LeftOut::SetByWriter);
            } else {
                parameters.push(parameter.clone());
            }
        }
        self.line(name, &parameters, &values.join(","));
    }

    /// A property of the tree, as held; none if its name isn't one.
    fn tree_property(&mut self, property: &Property) {
        if !is_name(&property.name) {
            self.leave_out(LeftOut::NotAName);
        } else if matches!(property.name.as_str(), "BEGIN" | "END") {
            self.leave_out(LeftOut::Structure);
        } else {
            self.line(&property.name, &property.parameters, &property.value);
        }
    }

    /// A component of the tree, with its properties and components; none if
    /// its name isn't one.
    fn tree_component(&mut self, component: &Component) {
        if !is_name(&component.name) {
            self.leave_out(LeftOut::NotAName);
            return;
        }
        self.begin(&component.name);
        for property in &component.properties {
            self.tree_property(property);
        }
        for inner in &component.components {
            self.tree_component(inner);
        }
        self.end(&component.name);
    }

    /// Notes the zones the `TZID`s of `tree`'s properties name, where
    /// FreeCal knows them, as given from the start of `when` or the
    /// property's own earliest time.
    fn tree_zones(&mut self, tree: &UnimplementedFields, when: &ImportedWhen) {
        let start = match *when {
            ImportedWhen::AllDay { start, .. } => start.and_time(Default::default()),
            ImportedWhen::Timed { start, .. } => start,
        };
        for (tzid, times) in tree_tzids(tree) {
            let tzid = without_controls(&tzid).replace('"', "");
            if let Some(zone) = iana_zone(&tzid).filter(|zone| *zone != Tz::UTC) {
                let from = times.into_iter().fold(start, NaiveDateTime::min);
                self.zone(&tzid, zone, from);
            }
        }
    }

    /// Notes that the wall time `wall` is given in `zone`, named `tzid`.
    fn zone(&mut self, tzid: &str, zone: Tz, wall: NaiveDateTime) {
        match self.zones.iter_mut().find(|(known, ..)| known == tzid) {
            Some((_, _, earliest)) => *earliest = (*earliest).min(wall),
            None => self.zones.push((tzid.to_owned(), zone, wall)),
        }
    }
}

/// Whether `value` is a date or a date-time as iCalendar writes them.
fn is_time(value: &str) -> bool {
    let value = value.strip_suffix('Z').unwrap_or(value);
    NaiveDate::parse_from_str(value, "%Y%m%d").is_ok()
        || NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S").is_ok()
}

/// `value` without the control characters a content line can't hold.
fn without_controls(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_control() || *c == '\t')
        .collect()
}

/// `text` as an iCalendar TEXT value can hold it: line breaks as `\n`,
/// which [`escaped`] escapes, and without the other control characters,
/// which RFC 5545 doesn't allow, so that no text can end its line.
fn text_value(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .collect()
}

/// A TEXT value escaped (RFC 5545, 3.3.11).
fn escaped(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace(',', "\\,")
        .replace(';', "\\;")
        .replace('\n', "\\n")
}

/// The last year a `VTIMEZONE` covers: the time zone rules FreeCal is built
/// with give no changes after it.
const LAST_COVERED_YEAR: i32 = 2099;

/// The text of the `VTIMEZONE` of `zone`, named `tzid`, as
/// [`write_resource`] writes it, for the patching writer until ticket 187
/// removes it.
pub(crate) fn vtimezone_text(tzid: &str, zone: Tz, from: NaiveDateTime) -> String {
    let mut out = Lines::default();
    vtimezone(tzid, zone, from, &mut out);
    out.text
}

/// Writes the `VTIMEZONE` of `zone`, named `tzid`, from the time zone rules
/// FreeCal is built with, so that clients that don't know IANA names read
/// the times as FreeCal does: each offset change from the start of the year
/// of `from` to the end of [`LAST_COVERED_YEAR`], those of one kind in one
/// observance, its first as its `DTSTART` and the others as `RDATE`s, after
/// an observance for the offset the year starts with.
fn vtimezone(tzid: &str, zone: Tz, from: NaiveDateTime, out: &mut Lines) {
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
    // Compared without building a `ZoneOffset`, which is made of it, on
    // every day: those are tens of thousands.
    let mut current_raw = zone.offset_from_utc_datetime(&start.naive_utc());
    while day < until {
        let next_day = day + TimeDelta::days(1);
        let next_raw = zone.offset_from_utc_datetime(&next_day.naive_utc());
        if next_raw == current_raw {
            day = next_day;
            continue;
        }
        current_raw = next_raw;
        let next = ZoneOffset::from(next_raw);
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
    out.escaped_text("TZID", &[], tzid);
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
        out.times("DTSTART", &[], &[Time::Wall(*first, None)]);
        let others: Vec<Time> = others
            .iter()
            .map(|onset| Time::Wall(*onset, None))
            .collect();
        out.times("RDATE", &[], &others);
        out.line("TZOFFSETFROM", &[], &utc_offset(observance.from));
        out.line("TZOFFSETTO", &[], &utc_offset(observance.kind.utc_offset));
        if let Some(name) = &observance.kind.name {
            out.escaped_text("TZNAME", &[], name);
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
        Self::from(zone.offset_from_utc_datetime(&at.naive_utc()))
    }

    fn from(offset: TzOffset) -> Self {
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

#[cfg(test)]
mod tests;
