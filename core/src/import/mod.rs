//! Reading `.ics` files, or `.zip` files of them such as a Google Takeout
//! export, into an import preview, and running a previewed import in one
//! Journal Transaction.
//!
//! The files are untrusted (spec: "Untrusted content"): they are parsed by
//! the `icalendar` and `zip` libraries, a `.zip` is read in memory and never
//! extracted, and what may be unpacked is limited by [`MAX_IMPORT_SIZE`] and
//! [`MAX_IMPORT_ENTRIES`].
//!
//! "Floating" here always means a time in the file that has no time zone.
//! FreeCal itself has no floating times: they are pinned on reading (ADR 0007).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{Cursor, Read};
use std::path::Path;

use chrono::{NaiveDate, NaiveDateTime};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::event::{Loaded, RecurrenceId, latest, new_uid};
use crate::ical::read::{ReadFile, log_violations, read_events};
use crate::journal::{JournalTransaction, JournalTransactionId};
use crate::unimplemented::UnimplementedFields;
use crate::{CalendarId, CoreError, Result, StructuredText};

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
#[derive(Debug, Clone)]
pub struct ImportPreview {
    pub calendar_id: CalendarId,
    /// How many Events have a UID the Calendar doesn't have.
    pub new_events: usize,
    /// How many Events have a UID the Calendar already has.
    pub existing_events: usize,
    /// The zones floating times were pinned to (ADR 0007), by zone name.
    pub pinned: Vec<PinnedZone>,
    /// The Events, in file order, each UID once, then the Exceptions without
    /// their Recurring Event, each an Event of its own.
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
#[derive(Debug, Clone)]
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
    /// Its `SEQUENCE`, which tells revisions of one Event apart: of several
    /// for one Occurrence, the highest is the latest.
    pub sequence: i64,
    /// A Recurring Event's Exceptions, which share its UID.
    pub exceptions: Vec<ImportedEvent>,
    /// For an Exception, the recurrence ID of the Occurrence it changed, as
    /// obtained (ADR 0012); none for every other Event.
    pub(crate) recurrence_id: Option<RecurrenceId>,
    #[allow(dead_code, reason = "until ticket 186 stores it")]
    pub(crate) unimplemented: UnimplementedFields,
}

impl ImportedEvent {
    pub fn description(&self) -> StructuredText {
        StructuredText::from_description(&self.description, self.description_html)
    }

    pub fn location(&self) -> StructuredText {
        StructuredText::from_plain_text(&self.location)
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
    pub journal_transaction: Option<JournalTransactionId>,
    /// How many Events it added, copies included.
    pub imported: usize,
    pub updated: usize,
    pub skipped: usize,
}

/// Runs the import `preview` in `journal_transaction`, which records each
/// Event's previous Event Version. Whether an Event exists is looked up again
/// rather than taken from the preview, once per UID, as an Exception without
/// its Recurring Event is an Event of its own with its Recurring Event's UID.
pub(crate) fn run(
    journal_transaction: &mut JournalTransaction,
    preview: &ImportPreview,
    existing: ExistingEvents,
) -> Result<ImportOutcome> {
    let calendar_id = preview.calendar_id;
    journal_transaction.writable(calendar_id)?;
    let mut outcome = ImportOutcome {
        journal_transaction: None,
        imported: 0,
        updated: 0,
        skipped: 0,
    };
    // Each UID's: whether the Calendar has it, and the UID its Events are
    // written under, none when they are skipped.
    let mut decided: HashMap<&str, (bool, Option<String>)> = HashMap::new();
    for event in &preview.events {
        let (exists, into) = match decided.get(event.uid.as_str()) {
            Some(decision) => decision.clone(),
            None => {
                let exists = Loaded::of_uid(journal_transaction, calendar_id, &event.uid)?.exists();
                let into = match (exists, existing) {
                    (true, ExistingEvents::Skip) => None,
                    // A copy is added under a UID of its own.
                    (true, ExistingEvents::Copy) => Some(new_uid()),
                    _ => Some(event.uid.clone()),
                };
                decided.insert(&event.uid, (exists, into.clone()));
                (exists, into)
            }
        };
        let Some(uid) = into else {
            outcome.skipped += 1;
            continue;
        };
        if exists && existing == ExistingEvents::Update {
            outcome.updated += 1;
        } else {
            outcome.imported += 1;
        }
        let loaded = Loaded::of_uid(journal_transaction, calendar_id, &uid)?;
        match event.recurrence_id {
            None => loaded.take(journal_transaction, event)?,
            Some(_) => loaded.take_exception(journal_transaction, event)?,
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
        .map(|(name, contents)| {
            let read = read_events(&name, &contents, display)?;
            log_violations(&name, &read.violations);
            Ok(read)
        })
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

/// The Events of `.ics` files read together, in order and each UID once.
/// An Exception joins its Recurring Event; several for one Occurrence all
/// join, for [`Loaded`] to keep the latest. Those without it in the files
/// follow, each an Event of its own (ticket 88), the latest of several for
/// one Occurrence, their recurrence IDs counting as obtained. Recurrence IDs
/// stay as obtained (ADR 0012).
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
    let mut without_recurring = Vec::new();
    for exception in exceptions {
        let recurring = seen
            .get(&exception.uid)
            .map(|&index| &mut events[index])
            .filter(|recurring| recurring.rule.is_some());
        match recurring {
            Some(recurring) => recurring.exceptions.push(exception),
            None => without_recurring.push(exception),
        }
    }
    let alone = latest(without_recurring.iter().map(|exception| {
        let obtained = exception.recurrence_id.as_ref();
        let rid = obtained.map(|obtained| obtained.normal_form(None));
        ((exception.uid.as_str(), rid), exception)
    }));
    events.extend(alone.into_iter().map(|(_, exception)| exception.clone()));
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

/// The zone a pinned Event's floating time was pinned to.
fn pinned_zone(event: &ImportedEvent) -> Option<Tz> {
    match event.when {
        ImportedWhen::Timed { start_zone, .. } if event.zone_pinned => Some(start_zone),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::{self, TransactionOrigin};
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
            journal::record(
                &store,
                TransactionOrigin::Import,
                0,
                |journal_transaction| run(journal_transaction, &preview, ExistingEvents::Copy),
            )
            .unwrap();
        }

        let uids = store.event_uids(calendar_id).unwrap();
        assert_eq!(uids.len(), 2);
        assert!(uids.contains("kept"));
    }
}
