//! Reading `.ics` files, or `.zip` files of them such as a Google Takeout
//! export, into an import preview, and running a previewed import in one
//! Change Journal unit.
//!
//! The files are untrusted (spec: "Untrusted content"): they are parsed by
//! the `icalendar` and `zip` libraries, a `.zip` is read in memory and never
//! extracted, and what may be unpacked is limited by [`MAX_IMPORT_SIZE`] and
//! [`MAX_IMPORT_ENTRIES`].
//!
//! "Floating" here always means a time in the file that has no time zone.
//! FreeCal itself has no floating times: they are pinned on reading (ADR 0007).

mod windows_zones;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{Cursor, Read};
use std::path::Path;

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Utc};
use chrono_tz::Tz;
use icalendar::{
    CalendarComponent, CalendarDateTime, Component, DatePerhapsTime, EventLike, Property,
};
use serde::{Deserialize, Serialize};

use crate::event::{StoredWhen, instant, wall_text};
use crate::journal::{Unit, UnitId};
use crate::recurrence;
use crate::store::{EventFields, OriginalServerData, StoredEvent};
use crate::{CalendarId, CoreError, Result, StructuredText};
use windows_zones::WINDOWS_ZONES;

/// The most a file to import may hold, and the most the `.ics` files in a
/// `.zip` may unpack to together: 128 MiB. A heavy user's Google Takeout
/// export, about 15,000 Events, unpacks to some 15–45 MB.
pub const MAX_IMPORT_SIZE: usize = 128 * 1024 * 1024;

/// The most entries a `.zip` to import may have, counting folders and files
/// that aren't `.ics`. A Google Takeout export of Calendar has one `.ics`
/// per Calendar and a few other files.
pub const MAX_IMPORT_ENTRIES: usize = 10_000;

/// A file to import, read into memory: an `.ics` file or a `.zip` of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportFile {
    /// The file's name, which errors give.
    pub name: String,
    pub contents: Vec<u8>,
}

impl ImportFile {
    pub fn new(name: impl Into<String>, contents: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            contents,
        }
    }

    /// Reads the file at `path`, up to [`MAX_IMPORT_SIZE`].
    pub fn read(path: &Path) -> Result<Self> {
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        let mut contents = Vec::new();
        let read = std::fs::File::open(path).and_then(|file| {
            file.take(MAX_IMPORT_SIZE as u64 + 1)
                .read_to_end(&mut contents)
        });
        if read.is_err() {
            return Err(CoreError::ImportUnreadable { file: name });
        }
        if contents.len() > MAX_IMPORT_SIZE {
            return Err(CoreError::ImportTooLarge { file: name });
        }
        Ok(Self { name, contents })
    }
}

/// What importing a file into a Calendar would do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPreview {
    pub calendar_id: CalendarId,
    /// How many Events have a UID the Calendar doesn't have.
    pub new_events: usize,
    /// How many Events have a UID the Calendar already has.
    pub existing_events: usize,
    /// The zones floating times were pinned to (ADR 0007), by zone name.
    pub pinned: Vec<PinnedZone>,
    /// The Events, in file order, each UID once.
    pub events: Vec<ImportedEvent>,
}

impl ImportPreview {
    /// What the window shows of the preview, without the Events.
    pub fn summary(&self) -> ImportSummary {
        ImportSummary {
            calendar_id: self.calendar_id,
            new_events: self.new_events,
            existing_events: self.existing_events,
            pinned: self.pinned.clone(),
        }
    }
}

/// An import preview as the window shows it: the counts, not the Events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub calendar_id: CalendarId,
    pub new_events: usize,
    pub existing_events: usize,
    pub pinned: Vec<PinnedZone>,
}

/// How many Events of an import had floating times pinned to `zone`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct PinnedZone {
    pub zone: String,
    pub events: usize,
}

/// An Event as read from a file to import. Its fields are kept as the file
/// has them, after iCalendar's unescaping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedEvent {
    pub uid: String,
    pub title: String,
    pub when: ImportedWhen,
    pub location: String,
    pub description: String,
    /// Whether `description` is HTML, as Google writes it, rather than plain
    /// text.
    pub description_html: bool,
    /// Whether a start or end had no time zone and was given one (ADR 0007).
    pub zone_pinned: bool,
    /// Whether both start and end had no time zone, so that neither gave the
    /// other its zone.
    pub all_floating: bool,
    /// Whether the Calendar already has an Event with this UID.
    pub exists: bool,
    /// A Recurring Event's repeat rule, as FreeCal stores it.
    pub rule: Option<String>,
    /// A Recurring Event's cancelled Occurrences: their recurrence IDs,
    /// separated by commas.
    pub exdates: String,
    /// For an Exception, the recurrence ID of the Occurrence it changed.
    pub recurrence_id: Option<String>,
    /// A Recurring Event's Exceptions, which share its UID.
    pub exceptions: Vec<ImportedEvent>,
    /// For an Event from a server, the object the server holds it in; an
    /// Exception has none, as its Recurring Event's holds it.
    pub server_data: Option<OriginalServerData>,
}

impl ImportedEvent {
    pub fn description(&self) -> StructuredText {
        StructuredText::from_description(&self.description, self.description_html)
    }

    pub fn location(&self) -> StructuredText {
        StructuredText::from_plain_text(&self.location)
    }

    /// Adds it with its Exceptions to the Calendar `calendar_id`, under the
    /// UID `uid`.
    pub(crate) fn create_in(
        &self,
        unit: &mut Unit,
        calendar_id: CalendarId,
        uid: &str,
    ) -> Result<()> {
        let new = |fields| StoredEvent {
            calendar_id,
            fields,
        };
        unit.create(uid, &new(self.fields()))?;
        for exception in &self.exceptions {
            unit.create(uid, &new(exception.fields()))?;
        }
        Ok(())
    }

    fn fields(&self) -> EventFields {
        EventFields {
            title: self.title.clone(),
            when: StoredWhen::imported(&self.when),
            location: self.location.clone(),
            description: self.description.clone(),
            description_html: self.description_html,
            zone_pinned: self.zone_pinned,
            rule: self.rule.clone(),
            exdates: self.exdates.clone(),
            recurrence_id: self.recurrence_id.clone(),
            server_data: self.server_data.clone(),
        }
    }

    /// Its fields as the new version of an Event that has `previous`: one
    /// floating end takes the zone of the other (ADR 0007, rule 3), so only
    /// a version floating at both keeps the zone pinned before (rule 2).
    fn fields_over(&self, previous: &EventFields) -> EventFields {
        let fields = self.fields();
        if self.all_floating && previous.zone_pinned {
            EventFields {
                when: fields.when.in_zones_of(&previous.when),
                ..fields
            }
        } else {
            fields
        }
    }
}

/// When an imported Event takes place: a timed Event's wall times each with
/// its Event Time Zone, an all-day Event's dates. Ends are exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportedWhen {
    AllDay {
        start: NaiveDate,
        end: NaiveDate,
    },
    Timed {
        start: NaiveDateTime,
        start_zone: Tz,
        end: NaiveDateTime,
        end_zone: Tz,
    },
}

/// What an import does with the Events its Calendar already has (same UID),
/// chosen once per import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum ExistingEvents {
    /// The Event takes the imported version.
    Update,
    /// The Event stays as it is and the imported version is left out.
    Skip,
    /// The imported version is added as a new Event with a new UID.
    Copy,
}

/// What running an import did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
    /// What undoes it; none when it changed nothing.
    pub unit: Option<UnitId>,
    /// How many Events it added, copies included.
    pub imported: usize,
    pub updated: usize,
    pub skipped: usize,
}

/// Runs the import `preview` in `unit`, which records each Event's previous
/// version. Whether an Event exists is looked up again rather than taken from
/// the preview.
pub(crate) fn run(
    unit: &mut Unit,
    preview: &ImportPreview,
    existing: ExistingEvents,
) -> Result<ImportOutcome> {
    let calendar_id = preview.calendar_id;
    unit.writable(calendar_id)?;
    let mut outcome = ImportOutcome {
        unit: None,
        imported: 0,
        updated: 0,
        skipped: 0,
    };
    for event in &preview.events {
        let new = |fields| StoredEvent {
            calendar_id,
            fields,
        };
        match (unit.find_event_by_uid(calendar_id, &event.uid)?, existing) {
            (Some(_), ExistingEvents::Skip) => outcome.skipped += 1,
            (Some((event_id, previous)), ExistingEvents::Update) => {
                outcome.updated += 1;
                unit.replace(event_id, &new(event.fields_over(&previous)))?;
                // Exceptions are matched by the Occurrence they changed.
                let mut left = unit.exceptions(calendar_id, &event.uid)?;
                for exception in &event.exceptions {
                    let same = left
                        .iter()
                        .position(|(_, fields)| fields.recurrence_id == exception.recurrence_id);
                    match same.map(|index| left.remove(index)) {
                        Some((id, previous)) => {
                            unit.replace(id, &new(exception.fields_over(&previous)))?
                        }
                        None => _ = unit.create(&event.uid, &new(exception.fields()))?,
                    }
                }
                for (id, _) in left {
                    unit.delete(id)?;
                }
            }
            // A new Event, or a copy of one the Calendar has.
            (found, _) => {
                outcome.imported += 1;
                let uid = match found {
                    None => event.uid.clone(),
                    Some(_) => uuid::Uuid::new_v4().to_string(),
                };
                event.create_in(unit, calendar_id, &uid)?;
            }
        }
    }
    Ok(outcome)
}

/// Previews importing `file` into the Calendar `calendar_id`, which has
/// Events with the UIDs `existing`. Floating times are pinned to `display`,
/// the Display Time Zone, because a Local Calendar has no zone of its own.
pub(crate) fn preview(
    file: &ImportFile,
    calendar_id: CalendarId,
    existing: &HashSet<String>,
    display: Tz,
) -> Result<ImportPreview> {
    let read = calendar_files(file)?
        .into_iter()
        .map(|(name, contents)| read_events(&name, &contents, display))
        .collect::<Result<Vec<_>>>()?;
    let mut events = assemble(read);
    for event in &mut events {
        event.exists = existing.contains(&event.uid);
    }
    let existing_events = events.iter().filter(|event| event.exists).count();
    let mut pinned = BTreeMap::<String, usize>::new();
    for event in &events {
        if let Some(zone) = pinned_zone(event) {
            *pinned.entry(zone.name().to_owned()).or_default() += 1;
        }
    }
    Ok(ImportPreview {
        calendar_id,
        new_events: events.len() - existing_events,
        existing_events,
        pinned: pinned
            .into_iter()
            .map(|(zone, events)| PinnedZone { zone, events })
            .collect(),
        events,
    })
}

/// The Events of iCalendar objects as a server holds them, read as
/// [`preview`] reads files, each with the object as its original server
/// data. Floating times are pinned to `zone`. An object FreeCal can't read
/// is left out.
pub(crate) fn read_server_events(objects: &[OriginalServerData], zone: Tz) -> Vec<ImportedEvent> {
    assemble(objects.iter().filter_map(|object| {
        let mut read = read_events(&object.href, object.data.as_bytes(), zone).ok()?;
        for event in &mut read.events {
            event.server_data = Some(object.clone());
        }
        Some(read)
    }))
}

/// The Events of `.ics` files read together, in order and each UID once.
/// An Exception joins its Recurring Event, once per Occurrence; one without
/// it in the files is left out.
fn assemble(files: impl IntoIterator<Item = ReadFile>) -> Vec<ImportedEvent> {
    let mut seen = HashMap::new();
    let mut events: Vec<ImportedEvent> = Vec::new();
    let mut exceptions = Vec::new();
    for read in files {
        for event in read.events {
            if !seen.contains_key(&event.uid) {
                seen.insert(event.uid.clone(), events.len());
                events.push(event);
            }
        }
        exceptions.extend(read.exceptions);
    }
    for (original, mut exception) in exceptions {
        let Some(&index) = seen.get(&exception.uid) else {
            continue;
        };
        let recurring = &mut events[index];
        if recurring.rule.is_none() {
            continue;
        }
        let id = recurrence_id_of(&original, &recurring.when);
        if recurring
            .exceptions
            .iter()
            .all(|known| known.recurrence_id.as_ref() != Some(&id))
        {
            exception.recurrence_id = Some(id);
            recurring.exceptions.push(exception);
        }
    }
    events
}

/// The `.ics` files in `file`, each with the name its errors give: the file
/// itself, or each `.ics` in a `.zip` as `takeout.zip/Calendar/Work.ics`.
fn calendar_files(file: &ImportFile) -> Result<Vec<(String, Vec<u8>)>> {
    if file.contents.len() > MAX_IMPORT_SIZE {
        return Err(CoreError::ImportTooLarge {
            file: file.name.clone(),
        });
    }
    if !file.contents.starts_with(b"PK") {
        return Ok(vec![(file.name.clone(), file.contents.clone())]);
    }
    let unreadable = || CoreError::ImportUnreadable {
        file: file.name.clone(),
    };
    let too_large = || CoreError::ImportTooLarge {
        file: file.name.clone(),
    };
    let mut archive =
        zip::ZipArchive::new(Cursor::new(&file.contents)).map_err(|_| unreadable())?;
    if archive.len() > MAX_IMPORT_ENTRIES {
        return Err(too_large());
    }
    let mut left = MAX_IMPORT_SIZE;
    let mut files = Vec::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|_| unreadable())?;
        let Ok(entry_name) = entry.name().map(|name| name.into_owned()) else {
            return Err(unreadable());
        };
        if !entry.is_file() || !entry_name.to_ascii_lowercase().ends_with(".ics") {
            continue;
        }
        let name = format!("{}/{entry_name}", file.name);
        // The sizes an entry declares are untrusted: only what is unpacked
        // counts, and unpacking stops at the limit.
        let mut contents = Vec::new();
        entry
            .take(left as u64 + 1)
            .read_to_end(&mut contents)
            .map_err(|_| CoreError::ImportUnreadable { file: name.clone() })?;
        left = left.checked_sub(contents.len()).ok_or_else(too_large)?;
        files.push((name, contents));
    }
    if files.is_empty() {
        return Err(CoreError::ImportZipWithoutCalendars {
            file: file.name.clone(),
        });
    }
    Ok(files)
}

/// What an `.ics` file has: its Events, without their `exists` yet, and the
/// Exceptions of Recurring Events, each with the original start of the
/// Occurrence it changed.
struct ReadFile {
    events: Vec<ImportedEvent>,
    exceptions: Vec<(Time, ImportedEvent)>,
}

/// The Events of the `.ics` file `name`.
fn read_events(name: &str, contents: &[u8], display: Tz) -> Result<ReadFile> {
    let not_calendar = || CoreError::ImportNotCalendar {
        file: name.to_owned(),
    };
    let text = std::str::from_utf8(contents).map_err(|_| not_calendar())?;
    let calendars = calendars(text).ok_or_else(not_calendar)?;
    let mut read = ReadFile {
        events: Vec::new(),
        exceptions: Vec::new(),
    };
    for calendar in &calendars {
        let zones = Zones::of(calendar);
        for event in calendar.events() {
            let start = event.get_start().ok_or_else(not_calendar)?;
            let end = event.get_end();
            let duration = event.property_value("DURATION").and_then(duration);
            let (when, zone_pinned, all_floating) = zones.when(start, end, duration, display);
            let description = event.get_description().unwrap_or_default();
            let original = event.get_recurrence_id().map(|time| zones.time(time));
            // A rule FreeCal can't expand leaves the Event as its first
            // Occurrence.
            let rule = event
                .property_value("RRULE")
                .filter(|_| original.is_none())
                .and_then(|rule| recurrence::imported_rule(rule, &StoredWhen::imported(&when)));
            let exdates = match rule {
                None => String::new(),
                Some(_) => event
                    .multi_properties()
                    .get("EXDATE")
                    .into_iter()
                    .flatten()
                    .flat_map(single_values)
                    .filter_map(|exdate| DatePerhapsTime::from_property(&exdate))
                    .map(|exdate| recurrence_id_of(&zones.time(exdate), &when))
                    .fold(String::new(), |exdates, id| {
                        recurrence::with_exdate(&exdates, &id)
                    }),
            };
            let imported = ImportedEvent {
                uid: event
                    .get_uid()
                    .filter(|uid| !uid.is_empty())
                    .map_or_else(|| uuid::Uuid::new_v4().to_string(), str::to_owned),
                title: event.get_summary().unwrap_or_default().to_owned(),
                when,
                location: event.get_location().unwrap_or_default().to_owned(),
                description: description.to_owned(),
                description_html: looks_like_html(description),
                zone_pinned,
                all_floating,
                exists: false,
                rule,
                exdates,
                recurrence_id: None,
                exceptions: Vec::new(),
                server_data: None,
            };
            match original {
                None => read.events.push(imported),
                Some(original) => read.exceptions.push((original, imported)),
            }
        }
    }
    Ok(read)
}

/// The `VCALENDAR`s of iCalendar text, none if it has none or can't be read.
fn calendars(text: &str) -> Option<Vec<icalendar::Calendar>> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let unfolded = icalendar::parser::unfold(text);
    let calendars: Vec<_> = icalendar::parser::read_components(&unfolded)
        .ok()?
        .into_iter()
        .filter(|root| root.name.as_str().eq_ignore_ascii_case("VCALENDAR"))
        .map(|root| {
            icalendar::Calendar::from(icalendar::parser::Calendar {
                properties: root.properties,
                components: root.components,
            })
        })
        .collect();
    (!calendars.is_empty()).then_some(calendars)
}

/// The IANA time zone of a CalDAV Calendar's `calendar-timezone` (RFC 4791),
/// an iCalendar object with one `VTIMEZONE`, if FreeCal can tell.
pub(crate) fn calendar_zone(text: &str) -> Option<Tz> {
    calendars(text)?.iter().find_map(|calendar| {
        let zones = Zones::of(calendar);
        calendar
            .components
            .iter()
            .find_map(|component| match component {
                CalendarComponent::Other(other)
                    if other.component_kind().eq_ignore_ascii_case("VTIMEZONE") =>
                {
                    zones.zone(other.property_value("TZID")?)
                }
                _ => None,
            })
    })
}

/// A property such as `EXDATE` with one value per property, for each value
/// of its comma-separated list.
fn single_values(property: &Property) -> Vec<Property> {
    property
        .value()
        .split(',')
        .map(|value| {
            let mut single = Property::new(property.key(), value.trim());
            for parameter in property.params().values() {
                single.append_parameter(parameter.clone());
            }
            single
        })
        .collect()
}

/// The recurrence ID of the Occurrence of a Recurring Event at `recurring`
/// that starts at `original`, as the Local Store writes it: a wall time in
/// the Event Time Zone of its start, or a date for an all-day Event. A time
/// without a zone is taken as given there; a date for a timed Event means
/// the Occurrence on that date.
fn recurrence_id_of(original: &Time, recurring: &ImportedWhen) -> String {
    match (*recurring, original) {
        (ImportedWhen::AllDay { .. }, Time::Date(date)) => date.to_string(),
        (ImportedWhen::AllDay { .. }, Time::Wall(wall, _)) => wall.date().to_string(),
        (ImportedWhen::Timed { start, .. }, Time::Date(date)) => {
            wall_text(date.and_time(start.time()))
        }
        (ImportedWhen::Timed { start_zone, .. }, &Time::Wall(wall, Some(zone)))
            if zone != start_zone =>
        {
            wall_text(instant(wall, zone).with_timezone(&start_zone).naive_local())
        }
        (ImportedWhen::Timed { .. }, Time::Wall(wall, _)) => wall_text(*wall),
    }
}

/// The zone a pinned Event's floating time was pinned to.
fn pinned_zone(event: &ImportedEvent) -> Option<Tz> {
    match event.when {
        ImportedWhen::Timed { start_zone, .. } if event.zone_pinned => Some(start_zone),
        _ => None,
    }
}

/// A start or end as a file gives it.
enum Time {
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
    fn of(calendar: &icalendar::Calendar) -> Self {
        let locations = calendar
            .components
            .iter()
            .filter_map(|component| match component {
                CalendarComponent::Other(other)
                    if other.component_kind().eq_ignore_ascii_case("VTIMEZONE") =>
                {
                    Some((
                        other.property_value("TZID")?.to_owned(),
                        other.property_value("X-LIC-LOCATION")?.to_owned(),
                    ))
                }
                _ => None,
            })
            .collect();
        Self {
            calendar: calendar.property_value("X-WR-TIMEZONE").and_then(iana_zone),
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

    /// When an Event takes place, whether a floating time was pinned, and
    /// whether both were:
    /// to the zone of the other end if only one end floats, otherwise to
    /// `display` (ADR 0007). A missing end follows the duration or RFC 5545's
    /// default, a day for an all-day Event and no length for a timed one.
    fn when(
        &self,
        start: DatePerhapsTime,
        end: Option<DatePerhapsTime>,
        duration: Option<TimeDelta>,
        display: Tz,
    ) -> (ImportedWhen, bool, bool) {
        let end = end.map(|end| self.time(end));
        match self.time(start) {
            Time::Date(start) => {
                let end = match end {
                    Some(Time::Date(end)) => end,
                    Some(Time::Wall(end, _)) => end.date(),
                    None => duration.map_or(start, |duration| start + duration),
                };
                let end = if end > start {
                    end
                } else {
                    start + TimeDelta::days(1)
                };
                (ImportedWhen::AllDay { start, end }, false, false)
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
                let (end, end_zone) = if instant(end, end_zone) < instant(start, start_zone) {
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
                (when, zone_pinned, all_floating)
            }
        }
    }
}

/// The IANA time zone a `TZID` names: an IANA name, a path ending in one
/// (`/mozilla.org/20050126_1/Europe/Berlin`) or a Windows name as Outlook
/// writes it (`W. Europe Standard Time`).
fn iana_zone(name: &str) -> Option<Tz> {
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
fn unquoted(value: &str) -> &str {
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
mod tests {
    use super::*;
    use crate::journal::{self, Origin};
    use crate::store::Store;

    /// UIDs aren't part of the core interface, so this looks at the Local
    /// Store itself.
    #[test]
    fn a_copy_gets_a_new_uid() {
        let directory = tempfile::TempDir::new().unwrap();
        let store = Store::open(&directory.path().join("freecal.sqlite3")).unwrap();
        let calendar_id = store.list_calendars().unwrap()[0].id;
        let file = ImportFile::new(
            "events.ics",
            b"BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:kept\r\nDTSTART:20261005T070000Z\r\n\
              END:VEVENT\r\nEND:VCALENDAR\r\n"
                .to_vec(),
        );
        let preview = preview(&file, calendar_id, &HashSet::new(), Tz::UTC).unwrap();

        for _ in 0..2 {
            journal::record(&store, Origin::Import, 0, |unit| {
                run(unit, &preview, ExistingEvents::Copy)
            })
            .unwrap();
        }

        let uids = store.event_uids(calendar_id).unwrap();
        assert_eq!(uids.len(), 2);
        assert!(uids.contains("kept"));
    }

    #[test]
    fn every_windows_name_maps_to_a_zone_freecal_knows() {
        for (windows, iana) in WINDOWS_ZONES {
            assert!(iana.parse::<Tz>().is_ok(), "{windows} → {iana}");
        }
    }

    #[test]
    fn html_is_told_apart_from_plain_text_with_angle_brackets() {
        for html in [
            "Agenda<br>Notes",
            "<p>Agenda</p>",
            "see <a href=\"https://example.org\">here</a>",
            "<B>bold</B>",
            "line<br/>",
        ] {
            assert!(looks_like_html(html), "{html:?}");
        }
        for plain in [
            "if a < b and c > d",
            "Mail <someone@example.org>",
            "<placeholder>",
            "x <3 y",
            "trailing <",
        ] {
            assert!(!looks_like_html(plain), "{plain:?}");
        }
    }
}
