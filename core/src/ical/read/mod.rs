//! The reader of iCalendar text, a CalDAV resource or an `.ics` file alike:
//! each `VEVENT` read in one pass into the fields FreeCal manages and its
//! Unimplemented Fields (ADR 0017), so that no property is in both or in
//! neither.
//!
//! What a precise rule decides is consumed; what the reader can't represent
//! exactly goes into the tree whole: an unmanaged property or component, a
//! second managed property RFC 5545 allows once, a value form FreeCal
//! doesn't model (a rule it can't expand, with the excluded dates, or an
//! excluded date on an Event without a rule). The unknown parameters of a
//! managed property go into the tree under its name, without its value.
//! `SEQUENCE` is an entry like any other; the reader also looks at it to
//! tell revisions of one Occurrence apart. What the precise rules leave out
//! on purpose is listed in [`DROPPED`], and nowhere else.
//!
//! What a text holds at the calendar level, outside its Events, is one
//! `ical:VCALENDAR` entry per `VCALENDAR`, on the text's single Event or
//! Recurring Event, or, when it holds only Exceptions, on the one with the
//! earliest recurrence ID.
//!
//! The text is untrusted (spec: "Untrusted content"): it is parsed by the
//! `icalendar` library, and one it can't read is refused whole. The RFC
//! violations the reader notices come back with what it read, for the
//! caller's Log.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Utc};
use chrono_tz::Tz;
use icalendar::parser::{
    Component as Parsed, Parameter as ParsedParameter, ParseString, Property as ParsedProperty,
};
use icalendar::{CalendarDateTime, DatePerhapsTime, Property as IcalProperty};

use super::windows_zones::WINDOWS_ZONES;
use super::{is_name, tree_tzids};
use crate::event::{RecurrenceId, StoredWhen, in_normal_form, instant, new_uid, rule};
use crate::import::{ImportedEvent, ImportedWhen};
use crate::unimplemented::{
    Component, Entry, EntryValue, ICAL, Parameter, Property, UnimplementedFields,
};
use crate::{CoreError, Result};

/// What the reader's precise rules leave out on purpose, each with the
/// rule, as `COMPONENT.PROPERTY` or `COMPONENT.PROPERTY;PARAMETER`, or
/// `*` for a component outside every `VCALENDAR`. The
/// round-trip check (ticket 197) accepts these as missing.
#[allow(dead_code, reason = "until ticket 197's round-trip check reads it")]
pub(crate) const DROPPED: &[(&str, &str)] = &[
    (
        "*",
        "a component outside every VCALENDAR, which holds no Event",
    ),
    ("VCALENDAR.VERSION", "the writer sets it"),
    ("VCALENDAR.PRODID", "the writer sets it"),
    (
        "VCALENDAR.CALSCALE",
        "only GREGORIAN, the default, which the writer sets",
    ),
    (
        "VCALENDAR.VTIMEZONE",
        "time zone definitions: the writer writes those its times need; one whose \
         TZID FreeCal doesn't know is kept on each Event whose tree uses it",
    ),
    (
        "VCALENDAR",
        "the calendar-level content of a text with several UIDs, which no one Event holds",
    ),
    ("VEVENT.DTSTAMP", "the writer sets it fresh"),
    ("VEVENT.LAST-MODIFIED", "the writer sets it fresh"),
    (
        "VEVENT.DURATION",
        "read as the end, which the writer writes as DTEND; its parameters, \
         a second one, one FreeCal can't read and one beside a DTEND are left out",
    ),
    ("VEVENT.DTSTART;TZID", TIME_RULE),
    ("VEVENT.DTSTART;VALUE", TIME_RULE),
    ("VEVENT.DTEND;TZID", TIME_RULE),
    ("VEVENT.DTEND;VALUE", TIME_RULE),
    ("VEVENT.RECURRENCE-ID;TZID", TIME_RULE),
    ("VEVENT.RECURRENCE-ID;VALUE", TIME_RULE),
    ("VEVENT.EXDATE;TZID", TIME_RULE),
    ("VEVENT.EXDATE;VALUE", TIME_RULE),
];

/// Why [`DROPPED`] leaves out the parameters of a time.
#[allow(dead_code, reason = "until ticket 197's round-trip check reads it")]
const TIME_RULE: &str = "the managed time holds its zone and whether it is a date; a TZID \
     FreeCal doesn't know reads as floating and is pinned (ADR 0007)";

/// The parameters of a time that the managed time holds.
const TIME_PARAMETERS: &[&str] = &["TZID", "VALUE"];

/// What iCalendar text holds: its Events, without their `exists` yet, and
/// the Exceptions of Recurring Events, each with the original start of the
/// Occurrence it changed.
#[derive(Default)]
pub(crate) struct ReadFile {
    pub(crate) events: Vec<ImportedEvent>,
    pub(crate) exceptions: Vec<ImportedEvent>,
    /// The RFC violations noticed while reading, for the caller's Log.
    pub(crate) violations: Vec<RfcViolation>,
}

/// A violation of RFC 5545 the reader noticed in a `VEVENT`. It names no
/// untrusted content, so a Log line may hold it as it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RfcViolation {
    /// A property the RFC allows once came more than once.
    Repeated(&'static str),
    /// A required property is missing.
    Missing(&'static str),
    /// Both `DTEND` and `DURATION`.
    EndAndDuration,
    /// An end before its start, or for an all-day Event not after it.
    EndBeforeStart,
    /// A control character other than a tab, which the reader leaves out.
    ControlCharacter,
    /// A property, parameter or component whose name has characters other
    /// than letters, digits and dashes, which the reader leaves out.
    NotAName,
}

impl fmt::Display for RfcViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RfcViolation::Repeated(name) => write!(f, "a VEVENT has {name} more than once"),
            RfcViolation::Missing(name) => write!(f, "a VEVENT has no {name}"),
            RfcViolation::EndAndDuration => write!(f, "a VEVENT has both DTEND and DURATION"),
            RfcViolation::EndBeforeStart => write!(f, "a VEVENT ends before it starts"),
            RfcViolation::ControlCharacter => {
                write!(f, "the text holds control characters, left out")
            }
            RfcViolation::NotAName => write!(
                f,
                "a name holds characters other than letters, digits and dashes; \
                 left out with what it names"
            ),
        }
    }
}

/// Writes to the Log the distinct RFC violations in `violations`, noticed
/// in the iCalendar text `source` names, an untrusted file name or href.
pub(crate) fn log_violations(source: &str, violations: &[RfcViolation]) {
    let mut distinct: Vec<String> = Vec::new();
    for violation in violations {
        let violation = violation.to_string();
        if !distinct.contains(&violation) {
            distinct.push(violation);
        }
    }
    if !distinct.is_empty() {
        let source: String = source.chars().take(LOGGED_SOURCE_LENGTH).collect();
        log::info!("{source:?} breaks RFC 5545: {}", distinct.join("; "));
    }
}

/// The most characters of a file name or href a Log line holds.
const LOGGED_SOURCE_LENGTH: usize = 200;

/// The Events of the iCalendar text `contents`, named `name` in errors.
pub(crate) fn read_events(name: &str, contents: &[u8], display: Tz) -> Result<ReadFile> {
    let not_calendar = || CoreError::ImportNotCalendar {
        file: name.to_owned(),
    };
    let text = std::str::from_utf8(contents).map_err(|_| not_calendar())?;
    let mut read = read_vcalendars(text, |roots| read_roots(&roots, display))
        .flatten()
        .ok_or_else(not_calendar)?;
    if text.chars().any(stray_control) {
        read.violations.push(RfcViolation::ControlCharacter);
    }
    Ok(read)
}

/// Whether `c` is a control character no content line holds: any but a tab
/// and the line ends.
fn stray_control(c: char) -> bool {
    c.is_control() && !matches!(c, '\t' | '\r' | '\n')
}

/// Gives `read` the `VCALENDAR` components at the top of iCalendar text,
/// and returns what it makes of them; nothing if the text can't be read.
/// Every reader of iCalendar text starts here, so that they agree on what
/// FreeCal can read.
pub(crate) fn read_vcalendars<R>(text: &str, read: impl FnOnce(Vec<Parsed<'_>>) -> R) -> Option<R> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let text: String = text.chars().filter(|&c| !stray_control(c)).collect();
    let unfolded = names_in_lower_case(&icalendar::parser::unfold(&text));
    let roots = icalendar::parser::read_components(&unfolded).ok()?;
    Some(read(
        roots
            .into_iter()
            .filter(|root| is(&root.name, "VCALENDAR"))
            .collect(),
    ))
}

/// The unfolded text `unfolded` with the names of its properties and their
/// parameters in lower case, `BEGIN` and `END` lines as they are. The
/// parser unescapes the values of the text properties it knows, but only of
/// a name in upper case; given none, it unescapes no value, so that every
/// value is as written, whatever the case of its name, and the reader
/// unescapes those it manages ([`unescaped`]).
fn names_in_lower_case(unfolded: &str) -> String {
    let mut text = String::with_capacity(unfolded.len());
    for line in unfolded.split_inclusive('\n') {
        let head = line.get(..6).unwrap_or(line).to_ascii_uppercase();
        if head.starts_with("BEGIN:") || head.starts_with("END:") {
            text.push_str(line);
            continue;
        }
        let mut in_name = true;
        let mut quoted = false;
        let mut chars = line.chars();
        while let Some(c) = chars.next() {
            match c {
                '"' => quoted = !quoted,
                ':' if !quoted => {
                    text.push(c);
                    text.push_str(chars.as_str());
                    break;
                }
                ';' if !quoted => in_name = true,
                '=' if !quoted => in_name = false,
                _ => {}
            }
            text.push(if in_name { c.to_ascii_lowercase() } else { c });
        }
    }
    text
}

/// The value `value` of a text property, as written, unescaped (RFC 5545,
/// 3.3.11).
fn unescaped(value: &str) -> String {
    ParseString::from(value).unescape_text().into_owned().into()
}

/// The first property `name` of `component`, whatever the case of names.
fn first_named<'c>(component: &'c Parsed<'_>, name: &str) -> Option<&'c ParsedProperty<'c>> {
    component
        .properties
        .iter()
        .find(|property| is(&property.name, name))
}

/// The IANA time zone of a CalDAV Calendar's `calendar-timezone` (RFC 4791),
/// an iCalendar object with one `VTIMEZONE`, if FreeCal can tell.
pub(crate) fn calendar_zone(text: &str) -> Option<Tz> {
    read_vcalendars(text, |roots| {
        roots.iter().find_map(|root| {
            let zones = Zones::of(root);
            root.components
                .iter()
                .filter(|component| is(&component.name, "VTIMEZONE"))
                .find_map(|zone| zones.zone(&unescaped(first_named(zone, "TZID")?.val.as_ref())))
        })
    })
    .flatten()
}

/// What the `VCALENDAR`s `roots` hold; none if one of their Events can't be
/// read, or there are none.
fn read_roots(roots: &[Parsed<'_>], display: Tz) -> Option<ReadFile> {
    if roots.is_empty() {
        return None;
    }
    let mut read = ReadFile::default();
    if !roots.iter().all(all_named) {
        read.violations.push(RfcViolation::NotAName);
    }
    let mut calendar_level = Vec::new();
    for root in roots {
        let zones = Zones::of(root);
        let mut level = Component {
            name: "VCALENDAR".to_owned(),
            properties: Vec::new(),
            components: Vec::new(),
        };
        for property in root.properties.iter().filter(named).map(tree_property) {
            let set_by_writer = match property.name.as_str() {
                "VERSION" | "PRODID" => true,
                "CALSCALE" => property.value.eq_ignore_ascii_case("GREGORIAN"),
                _ => false,
            };
            if !set_by_writer {
                level.properties.push(property);
            }
        }
        for component in &root.components {
            if is(&component.name, "VEVENT") {
                let event = read_vevent(component, &zones, display, &mut read.violations)?;
                match event.recurrence_id {
                    None => read.events.push(event),
                    Some(_) => read.exceptions.push(event),
                }
            } else if !is(&component.name, "VTIMEZONE") && is_name(component.name.as_ref()) {
                level.components.push(tree_component(component));
            }
        }
        if !level.properties.is_empty() || !level.components.is_empty() {
            calendar_level.push(level);
        }
    }
    for holder in calendar_level_holders(&mut read) {
        for level in &calendar_level {
            holder.unimplemented.push(Entry {
                name: format!("{ICAL}VCALENDAR"),
                value: EntryValue::Component {
                    properties: level.properties.clone(),
                    components: level.components.clone(),
                },
            });
        }
    }
    let unknown_zones = unknown_zones(roots);
    for event in read.events.iter_mut().chain(&mut read.exceptions) {
        let used: HashSet<String> = tree_tzids(&event.unimplemented)
            .into_iter()
            .map(|(tzid, _)| tzid)
            .collect();
        for (_, zone) in unknown_zones
            .iter()
            .filter(|(tzid, _)| used.contains(*tzid))
        {
            event.unimplemented.push(Entry {
                name: format!("{ICAL}VTIMEZONE"),
                value: EntryValue::Component {
                    properties: zone.properties.clone(),
                    components: zone.components.clone(),
                },
            });
        }
    }
    Some(read)
}

/// The `VTIMEZONE`s of `roots` whose `TZID` FreeCal doesn't know, by
/// `TZID`; the first of each.
fn unknown_zones(roots: &[Parsed<'_>]) -> BTreeMap<String, Component> {
    let mut zones = BTreeMap::new();
    for root in roots {
        let vtimezones = root.components.iter();
        for zone in vtimezones.filter(|component| is(&component.name, "VTIMEZONE")) {
            let Some(tzid) = first_named(zone, "TZID") else {
                continue;
            };
            let tzid = unquoted(&unescaped(tzid.val.as_ref())).to_owned();
            if iana_zone(&tzid).is_none() {
                zones.entry(tzid).or_insert_with(|| tree_component(zone));
            }
        }
    }
    zones
}

/// The Events that hold the calendar-level content of `read`: every
/// revision of its single Event or Recurring Event, or of the Exception
/// with the earliest recurrence ID, so that whichever revision is kept
/// holds it (`event::latest`); none when it holds several UIDs.
fn calendar_level_holders(read: &mut ReadFile) -> Vec<&mut ImportedEvent> {
    let uids: HashSet<&str> = read
        .events
        .iter()
        .chain(&read.exceptions)
        .map(|event| event.uid.as_str())
        .collect();
    if uids.len() != 1 {
        return Vec::new();
    }
    if !read.events.is_empty() {
        return read.events.iter_mut().collect();
    }
    let occurrence = |exception: &ImportedEvent| {
        let id = exception.recurrence_id.as_ref();
        id.map(|id| id.normal_form(None).into_string())
    };
    let earliest = read.exceptions.iter().map(occurrence).min();
    read.exceptions
        .iter_mut()
        .filter(|exception| Some(occurrence(exception)) == earliest)
        .collect()
}

/// The `VEVENT` `vevent` of a `VCALENDAR` naming its zones `zones`, with
/// floating times pinned to `display`; none if it can't be read. Adds the
/// RFC violations it notices to `violations`.
fn read_vevent(
    vevent: &Parsed<'_>,
    zones: &Zones,
    display: Tz,
    violations: &mut Vec<RfcViolation>,
) -> Option<ImportedEvent> {
    let mut properties = Properties::of(vevent);
    let start = properties.read("DTSTART", TIME_PARAMETERS, DatePerhapsTime::from_property)?;
    let end = properties.read("DTEND", TIME_PARAMETERS, DatePerhapsTime::from_property);
    // `DURATION` is only another way to give the end, which the writer
    // writes as `DTEND`: none is kept, nor its parameters.
    let duration = match end {
        Some(_) => None,
        None => properties.read("DURATION", &[], |property| duration(property.value())),
    };
    properties.forget_parameters("DURATION");
    if properties.drop_all("DURATION") && end.is_some() {
        properties.violations.push(RfcViolation::EndAndDuration);
    }
    let read_when = zones.when(start, end, duration, display);
    if read_when.end_before_start {
        properties.violations.push(RfcViolation::EndBeforeStart);
    }
    let when = read_when.when;
    // A `RECURRENCE-ID` that doesn't read makes the text unreadable, as a
    // start does, rather than its Exception a second Event of the UID
    // (ticket 215).
    let recurrence_id = properties.read("RECURRENCE-ID", TIME_PARAMETERS, |property| {
        DatePerhapsTime::from_property(property)
            .map(|original| RecurrenceId::of_original(zones.time(original)))
    });
    if recurrence_id.is_none() && properties.has("RECURRENCE-ID") {
        return None;
    }
    let stored_when = StoredWhen::imported(&when);
    // A rule FreeCal can't expand stays in the tree with its excluded
    // dates, and the Event is its first Occurrence.
    let rule = match recurrence_id {
        None => properties.read("RRULE", &[], |property| {
            rule::imported(property.value(), &stored_when)
        }),
        Some(_) => None,
    };
    let exdates = match rule {
        Some(_) => properties.read_all("EXDATE", TIME_PARAMETERS, |exdate| {
            single_values(exdate)
                .iter()
                .map(|single| {
                    let exdate = DatePerhapsTime::from_property(single)?;
                    Some(in_normal_form(&zones.time(exdate), &when).into_string())
                })
                .collect::<Option<Vec<_>>>()
        }),
        None => Vec::new(),
    };
    let uid = properties.read("UID", &[], |uid| Some(unescaped(uid.value())));
    if uid.is_none() {
        properties.violations.push(RfcViolation::Missing("UID"));
    }
    let text = |properties: &mut Properties, name| {
        properties
            .read(name, &[], |text| Some(unescaped(text.value())))
            .unwrap_or_default()
    };
    let title = text(&mut properties, "SUMMARY");
    let location = text(&mut properties, "LOCATION");
    let description = text(&mut properties, "DESCRIPTION");
    if !properties.drop_all("DTSTAMP") {
        properties.violations.push(RfcViolation::Missing("DTSTAMP"));
    }
    properties.drop_all("LAST-MODIFIED");
    let sequence = properties
        .value("SEQUENCE")
        .and_then(|sequence| sequence.trim().parse().ok())
        .unwrap_or(0);
    violations.append(&mut properties.violations);
    Some(ImportedEvent {
        uid: uid.filter(|uid| !uid.is_empty()).unwrap_or_else(new_uid),
        title,
        when,
        location,
        description_html: looks_like_html(&description),
        description,
        zone_pinned: read_when.zone_pinned,
        all_floating: read_when.all_floating,
        exists: false,
        rule,
        exdates: rule::exdates_of(exdates.into_iter().flatten()),
        sequence,
        exceptions: Vec::new(),
        recurrence_id,
        unimplemented: properties.into_tree(vevent),
    })
}

/// The properties of one `VEVENT` while it is read: those not read yet, and
/// the tree's entries so far.
struct Properties {
    /// In the order of the text; a read one is taken out.
    left: Vec<Option<Property>>,
    tree: Vec<Entry>,
    violations: Vec<RfcViolation>,
}

impl Properties {
    fn of(vevent: &Parsed<'_>) -> Self {
        Self {
            left: vevent
                .properties
                .iter()
                .filter(named)
                .map(tree_property)
                .map(Some)
                .collect(),
            tree: Vec::new(),
            violations: Vec::new(),
        }
    }

    /// The first property `name` not read yet, read by `read`, a precise
    /// rule. When it reads it, the property is consumed, but for the
    /// parameters not in `known`, which go into the tree; a second one
    /// stays for the tree, an RFC violation. When it doesn't, the property
    /// stays for the tree, whole.
    fn read<T>(
        &mut self,
        name: &'static str,
        known: &[&str],
        read: impl FnOnce(&IcalProperty) -> Option<T>,
    ) -> Option<T> {
        let at = self.position(name)?;
        let value = self.consume(at, known, read)?;
        if self.position(name).is_some() {
            self.violations.push(RfcViolation::Repeated(name));
        }
        Some(value)
    }

    /// Each property `name`, read by `read` as [`Properties::read`] reads
    /// one; the RFC allows several.
    fn read_all<T>(
        &mut self,
        name: &str,
        known: &[&str],
        read: impl Fn(&IcalProperty) -> Option<T>,
    ) -> Vec<T> {
        let named: Vec<usize> = (0..self.left.len())
            .filter(|&at| self.left[at].as_ref().is_some_and(|left| left.name == name))
            .collect();
        named
            .into_iter()
            .filter_map(|at| self.consume(at, known, &read))
            .collect()
    }

    fn consume<T>(
        &mut self,
        at: usize,
        known: &[&str],
        read: impl FnOnce(&IcalProperty) -> Option<T>,
    ) -> Option<T> {
        let value = read(&ical_property(self.left[at].as_ref()?))?;
        let property = self.left[at].take()?;
        let unknown: Vec<Parameter> = property
            .parameters
            .into_iter()
            .filter(|parameter| !known.contains(&parameter.name.as_str()))
            .collect();
        if !unknown.is_empty() {
            self.tree.push(Entry {
                name: format!("{ICAL}{}", property.name),
                value: EntryValue::Parameters(unknown),
            });
        }
        Some(value)
    }

    /// Leaves out the parameters a read property `name` put into the tree.
    fn forget_parameters(&mut self, name: &str) {
        let entry = format!("{ICAL}{name}");
        self.tree
            .retain(|kept| kept.name != entry || !matches!(kept.value, EntryValue::Parameters(_)));
    }

    /// Leaves out every property `name`; whether there was one.
    fn drop_all(&mut self, name: &str) -> bool {
        let mut dropped = false;
        for left in &mut self.left {
            if left.as_ref().is_some_and(|left| left.name == name) {
                *left = None;
                dropped = true;
            }
        }
        dropped
    }

    /// The value of the first property `name` not read yet, which stays.
    fn value(&self, name: &str) -> Option<&str> {
        Some(self.left[self.position(name)?].as_ref()?.value.as_str())
    }

    fn has(&self, name: &str) -> bool {
        self.position(name).is_some()
    }

    fn position(&self, name: &str) -> Option<usize> {
        self.left
            .iter()
            .position(|left| left.as_ref().is_some_and(|left| left.name == name))
    }

    /// The tree: the entries so far, then every property not read, and
    /// every component of `vevent`, whole.
    fn into_tree(self, vevent: &Parsed<'_>) -> UnimplementedFields {
        let mut entries = self.tree;
        entries.extend(self.left.into_iter().flatten().map(|property| Entry {
            name: format!("{ICAL}{}", property.name),
            value: EntryValue::Property {
                parameters: property.parameters,
                value: property.value,
            },
        }));
        entries.extend(
            vevent
                .components
                .iter()
                .map(tree_component)
                .map(|component| Entry {
                    name: format!("{ICAL}{}", component.name),
                    value: EntryValue::Component {
                        properties: component.properties,
                        components: component.components,
                    },
                }),
        );
        UnimplementedFields::new(entries)
    }
}

/// Whether the name `name` of a component is `expected`, whatever its case.
fn is(name: &impl AsRef<str>, expected: &str) -> bool {
    name.as_ref().eq_ignore_ascii_case(expected)
}

/// The component `parsed`, but for what has a name that isn't one.
fn tree_component(parsed: &Parsed<'_>) -> Component {
    let properties = parsed.properties.iter().filter(named);
    let components = parsed.components.iter();
    let components = components.filter(|component| is_name(component.name.as_ref()));
    Component {
        name: parsed.name.as_ref().to_ascii_uppercase(),
        properties: properties.map(tree_property).collect(),
        components: components.map(tree_component).collect(),
    }
}

/// The property `parsed`, but for the parameters whose name isn't one.
fn tree_property(parsed: &ParsedProperty<'_>) -> Property {
    let parameters = parsed.params.iter();
    let parameters = parameters.filter(|parameter| is_name(parameter.key.as_ref()));
    Property {
        name: parsed.name.as_ref().to_ascii_uppercase(),
        parameters: parameters.map(tree_parameter).collect(),
        value: parsed.val.as_ref().to_owned(),
    }
}

/// Whether the property `parsed` has a name that is one.
fn named(parsed: &&ParsedProperty<'_>) -> bool {
    is_name(parsed.name.as_ref())
}

/// Whether every name in `parsed` is one: of it, its properties, their
/// parameters and its components.
fn all_named(parsed: &Parsed<'_>) -> bool {
    is_name(parsed.name.as_ref())
        && parsed.properties.iter().all(|property| {
            is_name(property.name.as_ref())
                && property
                    .params
                    .iter()
                    .all(|parameter| is_name(parameter.key.as_ref()))
        })
        && parsed.components.iter().all(all_named)
}

/// A parameter with its values: those of a quoted list, such as
/// `MEMBER="mailto:a@example.org","mailto:b@example.org"`, each unquoted.
/// The parser unquotes a single quoted value, such as `CN="Doe, Jane"`, so
/// a value without quotes is one value, commas and all: an unquoted list,
/// which RFC 5545 allows only of values without `:`, `;` or `,`, reads as
/// one value.
fn tree_parameter(parsed: &ParsedParameter<'_>) -> Parameter {
    let mut values = Vec::new();
    match parsed.val.as_ref().map(AsRef::as_ref) {
        None => {}
        Some(value) if !value.contains('"') => values.push(value.to_owned()),
        Some(list) => {
            let mut current = String::new();
            let mut quoted = false;
            for c in list.chars() {
                match c {
                    '"' => quoted = !quoted,
                    ',' if !quoted => values.push(std::mem::take(&mut current)),
                    c => current.push(c),
                }
            }
            values.push(current);
        }
    }
    Parameter {
        name: parsed.key.as_ref().to_ascii_uppercase(),
        values,
    }
}

/// A property of the tree as the `icalendar` library reads its value.
fn ical_property(property: &Property) -> IcalProperty {
    let mut ical = IcalProperty::new(&property.name, &property.value);
    for parameter in &property.parameters {
        ical.add_parameter(&parameter.name, &parameter.values.join(","));
    }
    ical
}

/// When a `VEVENT` takes place, as [`Zones::when`] reads it.
struct ReadWhen {
    when: ImportedWhen,
    /// Whether a floating time was pinned.
    zone_pinned: bool,
    /// Whether both were.
    all_floating: bool,
    /// Whether the end given was before the start, which then is the end
    /// too, or for an all-day Event not after it.
    end_before_start: bool,
}

/// A property such as `EXDATE` with one value per property, for each value
/// of its comma-separated list.
fn single_values(property: &IcalProperty) -> Vec<IcalProperty> {
    property
        .value()
        .split(',')
        .map(|value| {
            let mut single = IcalProperty::new(property.key(), value.trim());
            for parameter in property.params().values() {
                single.append_parameter(parameter.clone());
            }
            single
        })
        .collect()
}

/// A start or end as a file gives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Time {
    Date(NaiveDate),
    /// A wall time in a zone, or floating when FreeCal has no zone for it.
    Wall(NaiveDateTime, Option<Tz>),
}

/// How one `VCALENDAR` names its time zones.
struct Zones {
    /// Its `X-WR-TIMEZONE`, which Google writes for the Calendar.
    calendar: Option<Tz>,
    /// Each `VTIMEZONE`'s `TZID` with its `X-LIC-LOCATION`.
    locations: HashMap<String, String>,
}

impl Zones {
    fn of(calendar: &Parsed<'_>) -> Self {
        let locations = calendar
            .components
            .iter()
            .filter(|component| is(&component.name, "VTIMEZONE"))
            .filter_map(|zone| {
                Some((
                    unescaped(first_named(zone, "TZID")?.val.as_ref()),
                    unescaped(first_named(zone, "X-LIC-LOCATION")?.val.as_ref()),
                ))
            })
            .collect();
        let calendar = first_named(calendar, "X-WR-TIMEZONE");
        Self {
            calendar: calendar.and_then(|zone| iana_zone(&unescaped(zone.val.as_ref()))),
            locations,
        }
    }

    /// The IANA time zone `tzid` stands for, if FreeCal can tell.
    fn zone(&self, tzid: &str) -> Option<Tz> {
        let tzid = unquoted(tzid);
        iana_zone(tzid).or_else(|| {
            self.locations
                .iter()
                .find(|(id, _)| unquoted(id) == tzid)
                .and_then(|(_, name)| iana_zone(name))
        })
    }

    fn time(&self, time: DatePerhapsTime) -> Time {
        match time {
            DatePerhapsTime::Date(date) => Time::Date(date),
            DatePerhapsTime::DateTime(CalendarDateTime::Floating(wall)) => Time::Wall(wall, None),
            DatePerhapsTime::DateTime(CalendarDateTime::Utc(utc)) => self.utc(utc),
            DatePerhapsTime::DateTime(CalendarDateTime::WithTimezone { date_time, tzid }) => {
                Time::Wall(date_time, self.zone(&tzid))
            }
        }
    }

    /// A UTC time in the Calendar's zone if the file names one: the same
    /// instant, in the zone the user meant.
    fn utc(&self, utc: DateTime<Utc>) -> Time {
        let zone = self.calendar.unwrap_or(Tz::UTC);
        Time::Wall(utc.with_timezone(&zone).naive_local(), Some(zone))
    }

    /// When an Event takes place, a floating time pinned
    /// to the zone of the other end if only one end floats, otherwise to
    /// `display` (ADR 0007). A missing end follows the duration or RFC 5545's
    /// default, a day for an all-day Event and no length for a timed one.
    fn when(
        &self,
        start: DatePerhapsTime,
        end: Option<DatePerhapsTime>,
        duration: Option<TimeDelta>,
        display: Tz,
    ) -> ReadWhen {
        let given = end.is_some() || duration.is_some();
        let end = end.map(|end| self.time(end));
        match self.time(start) {
            Time::Date(start) => {
                let end = match end {
                    Some(Time::Date(end)) => end,
                    Some(Time::Wall(end, _)) => end.date(),
                    None => duration.map_or(start, |duration| start + duration),
                };
                let end_before_start = given && end <= start;
                let end = if end > start {
                    end
                } else {
                    start + TimeDelta::days(1)
                };
                ReadWhen {
                    when: ImportedWhen::AllDay { start, end },
                    zone_pinned: false,
                    all_floating: false,
                    end_before_start,
                }
            }
            Time::Wall(start, start_zone) => {
                let (end, end_zone) = match end {
                    Some(Time::Wall(end, zone)) => (end, zone),
                    Some(Time::Date(end)) => (end.and_time(NaiveTime::MIN), start_zone),
                    None => (start + duration.unwrap_or_default(), start_zone),
                };
                let zone_pinned = start_zone.is_none() || end_zone.is_none();
                let all_floating = start_zone.is_none() && end_zone.is_none();
                let start_zone = start_zone.or(end_zone).unwrap_or(display);
                let end_zone = end_zone.unwrap_or(start_zone);
                let end_before_start = instant(end, end_zone) < instant(start, start_zone);
                let (end, end_zone) = if end_before_start {
                    (start, start_zone)
                } else {
                    (end, end_zone)
                };
                let when = ImportedWhen::Timed {
                    start,
                    start_zone,
                    end,
                    end_zone,
                };
                ReadWhen {
                    when,
                    zone_pinned,
                    all_floating,
                    end_before_start,
                }
            }
        }
    }
}

/// The IANA time zone a `TZID` names: an IANA name, a path ending in one
/// (`/mozilla.org/20050126_1/Europe/Berlin`) or a Windows name as Outlook
/// writes it (`W. Europe Standard Time`).
pub(crate) fn iana_zone(name: &str) -> Option<Tz> {
    let name = unquoted(name);
    if let Ok(zone) = name.parse() {
        return Some(zone);
    }
    if let Some((_, iana)) = WINDOWS_ZONES.iter().find(|(windows, _)| *windows == name) {
        return iana.parse().ok();
    }
    name.match_indices('/')
        .find_map(|(index, _)| name[index + 1..].parse().ok())
}

/// A parameter value without the quotes RFC 5545 allows around it.
pub(crate) fn unquoted(value: &str) -> &str {
    value.trim().trim_matches('"')
}

/// An RFC 5545 duration, such as `PT1H30M` or `P2D`.
fn duration(text: &str) -> Option<TimeDelta> {
    let parsed = iso8601::duration(text.trim()).ok()?;
    TimeDelta::from_std(parsed.into()).ok()
}

/// The tags whose presence makes a description HTML rather than plain text
/// that happens to hold a `<`.
const HTML_TAGS: &[&str] = &[
    "a",
    "b",
    "i",
    "u",
    "s",
    "p",
    "br",
    "hr",
    "em",
    "strong",
    "span",
    "div",
    "font",
    "ul",
    "ol",
    "li",
    "img",
    "html",
    "body",
    "script",
    "style",
    "table",
    "tr",
    "td",
    "th",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "pre",
    "code",
    "blockquote",
    "sub",
    "sup",
    "small",
    "big",
    "center",
];

/// Whether a description is HTML: whether it has an opening or closing tag
/// of [`HTML_TAGS`], such as `<br>`, `</p>` or `<a href=…>`.
fn looks_like_html(text: &str) -> bool {
    text.match_indices('<').any(|(index, _)| {
        let rest = &text[index + 1..];
        let rest = rest.strip_prefix('/').unwrap_or(rest);
        let name_end = rest
            .find(|c: char| !c.is_ascii_alphanumeric())
            .unwrap_or(rest.len());
        let after = rest[name_end..].chars().next();
        HTML_TAGS.contains(&rest[..name_end].to_ascii_lowercase().as_str())
            && matches!(after, Some('>' | '/') | Some(' ' | '\t' | '\r' | '\n'))
    })
}

#[cfg(test)]
mod tests;
