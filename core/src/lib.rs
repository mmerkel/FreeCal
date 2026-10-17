//! FreeCal's core: all domain logic behind one application interface, [`Core`].
//!
//! The frontend talks only to [`Core`]. Besides answering requests, the core
//! pushes [`Signal`]s to a [`SignalSink`].

mod account;
mod calendar;
mod clock;
mod error;
mod event;
mod import;
mod settings;
mod sidebar;
mod signal;
mod store;
mod structured_text;
#[cfg(test)]
mod typescript;
mod view;

use std::fs::{File, TryLockError};
use std::path::Path;

use chrono::NaiveDate;
use std::sync::{Arc, Mutex};

pub use account::{Account, AccountId, Provider};
pub use calendar::{Calendar, CalendarId, Colour};
pub use clock::{Clock, SystemClock};
pub use error::{CoreError, ErrorCode};
pub use event::{Event, EventDraft, EventId, Occurrence, When};
pub use import::{
    ExistingEvents, ImportFile, ImportId, ImportOutcome, ImportPreview, ImportedEvent,
    ImportedWhen, MAX_IMPORT_ENTRIES, MAX_IMPORT_SIZE, PinnedZone,
};
pub use settings::{Appearance, Settings, SettingsUpdate};
pub use sidebar::SidebarState;
pub use signal::{Signal, SignalSink};
pub use structured_text::{Piece, StructuredText, link_allowed};
pub use view::View;

use calendar::calendar_name;
use event::{StoredRange, StoredWhen};
use store::{EventFields, Store, StoredEvent};

pub type Result<T> = std::result::Result<T, CoreError>;

/// The core application interface.
pub struct Core {
    /// Held for the lifetime of `Core`, so that only one running FreeCal
    /// opens the Local Store (ADR 0005). The OS releases it when the process
    /// ends, even after a crash.
    _lock: File,
    store: Mutex<Store>,
    clock: Arc<dyn Clock>,
    signals: Arc<dyn SignalSink>,
    /// The local date as [`Core::check_date`] last saw it.
    date: Mutex<NaiveDate>,
}

impl Core {
    /// Opens the Local Store in `data_dir`, creating it on first launch.
    /// Refuses with [`CoreError::LocalStoreInUse`] while another running
    /// FreeCal has it open.
    pub fn open(
        data_dir: &Path,
        clock: Arc<dyn Clock>,
        signals: Arc<dyn SignalSink>,
    ) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let lock = lock_local_store(data_dir)?;
        let store = Store::open(&data_dir.join("freecal.sqlite3"))?;
        let date = Mutex::new(clock.now().date_naive());
        Ok(Self {
            _lock: lock,
            store: Mutex::new(store),
            clock,
            signals,
            date,
        })
    }

    /// The current local date, on which the views open.
    pub fn today(&self) -> NaiveDate {
        self.clock.now().date_naive()
    }

    /// Sends "Date changed" when the local date differs from the one last
    /// seen. The shell calls this every few seconds, which notices midnight
    /// and also waking up on a new day, when timers set for midnight would
    /// have slept through it.
    pub fn check_date(&self) {
        let today = self.today();
        let changed = {
            let mut seen = self.date.lock().unwrap();
            std::mem::replace(&mut *seen, today) != today
        };
        if changed {
            self.signals.send(Signal::DateChanged { date: today });
        }
    }

    /// The view FreeCal opens in: month on first launch, then the last-used one.
    pub fn last_used_view(&self) -> Result<View> {
        self.store().last_used_view()
    }

    pub fn set_last_used_view(&self, view: View) -> Result<()> {
        self.store().set_last_used_view(view)
    }

    /// The sidebar as it was left: on first launch shown at the default width.
    pub fn sidebar(&self) -> Result<SidebarState> {
        self.store().sidebar()
    }

    pub fn set_sidebar(&self, sidebar: SidebarState) -> Result<()> {
        self.store().set_sidebar(sidebar)
    }

    pub fn settings(&self) -> Result<Settings> {
        self.store().settings()
    }

    /// Changes the settings `update` gives and keeps the others. Returns the
    /// settings as they are now.
    pub fn update_settings(&self, update: SettingsUpdate) -> Result<Settings> {
        let store = self.store();
        if let Some(appearance) = update.appearance {
            store.set_appearance(appearance)?;
        }
        store.settings()
    }

    pub fn list_accounts(&self) -> Result<Vec<Account>> {
        self.store().list_accounts()
    }

    pub fn remove_account(&self, id: AccountId) -> Result<()> {
        let account = self.store().find_account(id)?;
        match account.ok_or(CoreError::AccountNotFound(id))?.provider {
            Provider::Local => Err(CoreError::LocalAccountCannotBeRemoved),
        }
    }

    /// Every Calendar of every Account, in the order they were added.
    pub fn list_calendars(&self) -> Result<Vec<Calendar>> {
        self.store().list_calendars()
    }

    /// Creates a Calendar in the Local Account. Server Accounts' Calendars
    /// are added from their Available Calendars instead (ticket 14).
    pub fn create_calendar(
        &self,
        account_id: AccountId,
        name: &str,
        colour: Colour,
    ) -> Result<Calendar> {
        let name = calendar_name(name)?;
        let created = {
            let store = self.store();
            let account = store.find_account(account_id)?;
            match account
                .ok_or(CoreError::AccountNotFound(account_id))?
                .provider
            {
                Provider::Local => store.insert_calendar(account_id, name, &colour)?,
            }
        };
        self.signals.send(Signal::CalendarsChanged);
        Ok(created)
    }

    pub fn rename_calendar(&self, id: CalendarId, name: &str) -> Result<()> {
        let name = calendar_name(name)?;
        self.store().set_calendar_name(id, name)?;
        self.signals.send(Signal::CalendarsChanged);
        Ok(())
    }

    pub fn recolour_calendar(&self, id: CalendarId, colour: Colour) -> Result<()> {
        self.store().set_calendar_colour(id, &colour)?;
        self.signals.send(Signal::CalendarsChanged);
        Ok(())
    }

    /// Deletes a Local Calendar with all its Events.
    pub fn delete_calendar(&self, id: CalendarId) -> Result<()> {
        self.store().delete_calendar(id)?;
        self.signals.send(Signal::CalendarsChanged);
        Ok(())
    }

    /// Shows or hides a Calendar's Events.
    pub fn set_calendar_shown(&self, id: CalendarId, shown: bool) -> Result<()> {
        self.store().set_calendar_shown(id, shown)?;
        self.signals.send(Signal::CalendarsChanged);
        Ok(())
    }

    /// Shows one Calendar and hides every other Calendar in all Accounts.
    pub fn show_only_calendar(&self, id: CalendarId) -> Result<()> {
        self.store().show_only_calendar(id)?;
        self.signals.send(Signal::CalendarsChanged);
        Ok(())
    }

    /// The Occurrences of shown Calendars on the dates `from` up to `to`
    /// (exclusive), in the Display Time Zone and ordered by start.
    pub fn occurrences(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<Occurrence>> {
        let zone = self.clock.time_zone();
        let events = self
            .store()
            .shown_events_in(&StoredRange::new(from, to, zone))?;
        let occurrences = events
            .into_iter()
            .map(|event| {
                let occurrence = Occurrence {
                    event_id: event.id,
                    calendar_id: event.calendar_id,
                    title: event.title,
                    when: event.when.to_when(zone)?,
                };
                Ok((event.start_utc, occurrence))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Occurrence::ordered(occurrences))
    }

    /// An Event with everything its details and editor show.
    pub fn event(&self, id: EventId) -> Result<Event> {
        let stored = self.store().find_event(id)?;
        self.to_event(id, stored)
    }

    /// Creates an Event in a writable Calendar.
    pub fn create_event(&self, draft: EventDraft) -> Result<Event> {
        let calendar_id = draft.calendar_id;
        let created = {
            let store = self.store();
            writable(&store, calendar_id)?;
            let fields = EventFields {
                title: draft.title,
                when: StoredWhen::new(draft.when, self.clock.time_zone())?,
                location: draft.location,
                description: draft.description,
                description_html: false,
                zone_pinned: false,
            };
            let id = store.insert_event(calendar_id, &fields)?;
            (
                id,
                StoredEvent {
                    calendar_id,
                    fields,
                },
            )
        };
        self.signals.send(Signal::OccurrencesChanged {
            calendar_ids: vec![calendar_id],
        });
        self.to_event(created.0, created.1)
    }

    /// Changes an Event, possibly moving it to another writable Calendar.
    /// A timed Event keeps its Event Time Zones, and what the user left as
    /// it was stays exactly as stored: an unchanged time its wall time and an
    /// unchanged description its original bytes, even if it was HTML.
    pub fn edit_event(&self, id: EventId, draft: EventDraft) -> Result<Event> {
        let zone = self.clock.time_zone();
        let (before, after) = {
            let store = self.store();
            let before = store.find_event(id)?;
            writable(&store, before.calendar_id)?;
            writable(&store, draft.calendar_id)?;

            let old = &before.fields;
            let when = old.when.edit(draft.when, zone)?;
            let (description, description_html) = if draft.description == description_of(old).text {
                (old.description.clone(), old.description_html)
            } else {
                (draft.description, false)
            };
            let after = StoredEvent {
                calendar_id: draft.calendar_id,
                fields: EventFields {
                    title: draft.title,
                    zone_pinned: old.zone_pinned && when == old.when,
                    when,
                    location: draft.location,
                    description,
                    description_html,
                },
            };
            store.update_event(id, &after)?;
            (before.calendar_id, after)
        };
        let mut calendar_ids = vec![before];
        if after.calendar_id != before {
            calendar_ids.push(after.calendar_id);
        }
        self.signals
            .send(Signal::OccurrencesChanged { calendar_ids });
        self.to_event(id, after)
    }

    /// Moves an Event to another time, as a drag or a resize in the grid
    /// does, and leaves everything else as stored. `when` is in the Display
    /// Time Zone; a timed Event keeps its Event Time Zones.
    pub fn reschedule_event(&self, id: EventId, when: When) -> Result<Event> {
        let zone = self.clock.time_zone();
        let after = {
            let store = self.store();
            let mut event = store.find_event(id)?;
            writable(&store, event.calendar_id)?;
            let edited = event.fields.when.edit(when, zone)?;
            // A time the user changed is theirs, no longer a pinned one.
            event.fields.zone_pinned &= edited == event.fields.when;
            event.fields.when = edited;
            store.update_event(id, &event)?;
            event
        };
        self.signals.send(Signal::OccurrencesChanged {
            calendar_ids: vec![after.calendar_id],
        });
        self.to_event(id, after)
    }

    /// Deletes an Event from a writable Calendar.
    pub fn delete_event(&self, id: EventId) -> Result<()> {
        let calendar_id = {
            let store = self.store();
            let calendar_id = store.find_event(id)?.calendar_id;
            writable(&store, calendar_id)?;
            store.delete_event(id)?;
            calendar_id
        };
        self.signals.send(Signal::OccurrencesChanged {
            calendar_ids: vec![calendar_id],
        });
        Ok(())
    }

    /// Previews importing `file`, an `.ics` file or a `.zip` of them, into a
    /// Calendar: how many Events are new, how many it already has (same
    /// UID), and which zones floating times were pinned to. Stores nothing.
    pub fn preview_import(
        &self,
        file: &ImportFile,
        calendar_id: CalendarId,
    ) -> Result<ImportPreview> {
        let existing = {
            let store = self.store();
            store.find_calendar(calendar_id)?;
            store.event_uids(calendar_id)?
        };
        import::preview(file, calendar_id, &existing, self.clock.time_zone())
    }

    /// Runs a previewed import into a writable Calendar, doing with the
    /// Events it already has what `existing` says. The import is one Change
    /// Journal unit, which [`Core::undo_import`] undoes in one step.
    pub fn run_import(
        &self,
        preview: &ImportPreview,
        existing: ExistingEvents,
    ) -> Result<ImportOutcome> {
        let outcome = {
            let store = self.store();
            writable(&store, preview.calendar_id)?;
            import::run(&store, preview, existing, self.clock.now().timestamp())?
        };
        if outcome.imported + outcome.updated > 0 {
            self.signals.send(Signal::OccurrencesChanged {
                calendar_ids: vec![preview.calendar_id],
            });
        }
        Ok(outcome)
    }

    /// Undoes an import exactly: the Events it added are deleted and the
    /// ones it updated get their previous version back. An import is undone
    /// once, and only while every Calendar it changes Events in is writable.
    pub fn undo_import(&self, id: ImportId) -> Result<()> {
        let calendar_ids = import::undo(&self.store(), id)?;
        self.signals
            .send(Signal::OccurrencesChanged { calendar_ids });
        Ok(())
    }

    fn to_event(&self, id: EventId, stored: StoredEvent) -> Result<Event> {
        let description = description_of(&stored.fields);
        let fields = stored.fields;
        Ok(Event {
            id,
            calendar_id: stored.calendar_id,
            title: fields.title,
            when: fields.when.to_when(self.clock.time_zone())?,
            location: StructuredText::from_plain_text(&fields.location),
            description,
            pinned_zone: fields
                .zone_pinned
                .then_some(fields.when.start_zone)
                .flatten(),
        })
    }

    /// The Local Store, locked. Signals are sent only after the lock is
    /// released, so a subscriber can read straight away.
    fn store(&self) -> std::sync::MutexGuard<'_, Store> {
        self.store.lock().unwrap()
    }
}

/// Takes the exclusive lock on the Local Store in `data_dir`, which
/// `freecal.lock` next to it stands for.
fn lock_local_store(data_dir: &Path) -> Result<File> {
    let lock = File::options()
        .write(true)
        .create(true)
        .truncate(false)
        .open(data_dir.join("freecal.lock"))?;
    match lock.try_lock() {
        Ok(()) => Ok(lock),
        Err(TryLockError::WouldBlock) => Err(CoreError::LocalStoreInUse),
        Err(TryLockError::Error(error)) => Err(error.into()),
    }
}

/// Refuses changes to the Events of a missing or Read-only Calendar.
pub(crate) fn writable(store: &Store, id: CalendarId) -> Result<()> {
    if store.find_calendar(id)?.read_only {
        Err(CoreError::CalendarReadOnly(id))
    } else {
        Ok(())
    }
}

fn description_of(fields: &EventFields) -> StructuredText {
    StructuredText::from_description(&fields.description, fields.description_html)
}
