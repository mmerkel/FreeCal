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
mod journal;
mod recurrence;
mod scope;
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
    ExistingEvents, ImportFile, ImportOutcome, ImportPreview, ImportSummary, ImportedEvent,
    ImportedWhen, MAX_IMPORT_ENTRIES, MAX_IMPORT_SIZE, PinnedZone,
};
pub use journal::{Changed, UnitId};
pub use recurrence::{Frequency, Repeat, RepeatEnd};
pub use scope::Scope;
pub use settings::{Appearance, Settings, SettingsUpdate};
pub use sidebar::SidebarState;
pub use signal::{Signal, SignalSink};
pub use structured_text::{Piece, StructuredText, link_allowed};
pub use view::View;

use calendar::calendar_name;
use event::{StoredRange, StoredWhen};
use journal::{Origin, Unit};
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
    /// Its changes leave the Change Journal (ADR 0009).
    pub fn delete_calendar(&self, id: CalendarId) -> Result<()> {
        journal::delete_calendar(&self.store(), id)?;
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
    /// (exclusive), in the Display Time Zone and ordered by start. Recurring
    /// Events are expanded here, in wall time in their Event Time Zone, and
    /// their Exceptions stand in for the Occurrences they changed.
    pub fn occurrences(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<Occurrence>> {
        let zone = self.clock.time_zone();
        let range = StoredRange::new(from, to, zone);
        let mut occurrences = Vec::new();
        let store = self.store();
        for event in store.shown_events_in(&range)? {
            let expanded = match event.fields.rule {
                None => None,
                Some(_) => {
                    let exceptions: Vec<String> = store
                        .exceptions(event.calendar_id, &event.uid)?
                        .into_iter()
                        .filter_map(|(_, fields)| fields.recurrence_id)
                        .collect();
                    recurrence::occurrences_in(&event.fields, &exceptions, &range)?
                }
            };
            let occurrence = |when: &StoredWhen, recurrence_id| -> Result<Occurrence> {
                Ok(Occurrence {
                    event_id: event.id,
                    calendar_id: event.calendar_id,
                    title: event.fields.title.clone(),
                    when: when.to_when(zone)?,
                    recurrence_id,
                })
            };
            match expanded {
                Some(found) => {
                    for (recurrence_id, when) in found {
                        let start_utc = when.instants()?.0;
                        occurrences.push((start_utc, occurrence(&when, Some(recurrence_id))?));
                    }
                }
                None => occurrences.push((
                    event.start_utc,
                    occurrence(&event.fields.when, event.fields.recurrence_id.clone())?,
                )),
            }
        }
        Ok(Occurrence::ordered(occurrences))
    }

    /// An Event with everything its details and editor show.
    pub fn event(&self, id: EventId) -> Result<Event> {
        let stored = self.store().find_event(id)?;
        self.to_event(id, stored)
    }

    /// Creates an Event in a writable Calendar, a Recurring Event when the
    /// draft has a repeat rule.
    pub fn create_event(&self, draft: EventDraft) -> Result<Changed<Event>> {
        let when = StoredWhen::new(draft.when, self.clock.time_zone())?;
        let rule = draft
            .repeat
            .map(|repeat| recurrence::rule_text(repeat, None, &when))
            .transpose()?;
        let event = StoredEvent {
            calendar_id: draft.calendar_id,
            fields: EventFields {
                title: draft.title,
                when,
                location: draft.location,
                description: draft.description,
                description_html: false,
                zone_pinned: false,
                rule,
                exdates: String::new(),
                recurrence_id: None,
            },
        };
        let created = self.record(Origin::UserAction, |unit| {
            let id = unit.create(&uuid::Uuid::new_v4().to_string(), &event)?;
            Ok((id, event))
        })?;
        self.changed_event(created)
    }

    /// Changes an Event, possibly moving it to another writable Calendar.
    /// A timed Event keeps its Event Time Zones, and what the user left as
    /// it was stays exactly as stored: an unchanged time its wall time, an
    /// unchanged description its original bytes, even if it was HTML, and an
    /// unchanged repeat rule its stored rule. A Recurring Event changes with
    /// all its Occurrences, from its first date, and an Exception alone (see
    /// [`Core::edit_occurrence`]).
    pub fn edit_event(&self, id: EventId, draft: EventDraft) -> Result<Changed<Event>> {
        if self.recurring(id)? {
            return self.edit_whole(id, scope::Edit::Draft(draft));
        }
        let zone = self.clock.time_zone();
        let edited = self.record(Origin::UserAction, |unit| {
            let old = unit.find_event(id)?.fields;
            let when = old.when.edit(draft.when, zone)?;
            let rule = draft
                .repeat
                .map(|repeat| recurrence::rule_text(repeat, None, &when))
                .transpose()?;
            let (description, description_html) = if draft.description == description_of(&old).text
            {
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
                    rule,
                    ..old
                },
            };
            unit.replace(id, &after)?;
            Ok((id, after))
        })?;
        self.changed_event(edited)
    }

    /// Moves an Event to another time, as a drag or a resize in the grid
    /// does, and leaves everything else as stored. `when` is in the Display
    /// Time Zone; a timed Event keeps its Event Time Zones. A Recurring Event
    /// moves with all its Occurrences, and an Exception alone.
    pub fn reschedule_event(&self, id: EventId, when: When) -> Result<Changed<Event>> {
        if self.recurring(id)? {
            return self.edit_whole(id, scope::Edit::Reschedule(when));
        }
        let zone = self.clock.time_zone();
        let rescheduled = self.record(Origin::UserAction, |unit| {
            let mut event = unit.find_event(id)?;
            let edited = event.fields.when.edit(when, zone)?;
            // A time the user changed is theirs, no longer a pinned one.
            event.fields.zone_pinned &= edited == event.fields.when;
            event.fields.when = edited;
            unit.replace(id, &event)?;
            Ok((id, event))
        })?;
        self.changed_event(rescheduled)
    }

    /// Whether the Event `id` is a Recurring Event or an Exception.
    fn recurring(&self, id: EventId) -> Result<bool> {
        let fields = self.store().find_event(id)?.fields;
        Ok(fields.rule.is_some() || fields.recurrence_id.is_some())
    }

    fn edit_whole(&self, id: EventId, edit: scope::Edit) -> Result<Changed<Event>> {
        let zone = self.clock.time_zone();
        let edited = self.record(Origin::UserAction, |unit| {
            scope::edit_whole(unit, zone, id, edit)
        })?;
        let value = self.event(edited.value.id)?;
        Ok(Changed {
            unit: edited.unit,
            value,
        })
    }

    /// Changes one Occurrence of a Recurring Event, given by the Event and
    /// the recurrence ID its [`Occurrence`] has, in `scope`: alone, as an
    /// Exception; with every later one, as a Recurring Event of their own;
    /// or with all of them. What the user changed is what differs from the
    /// Occurrence as it was. Returns the Occurrence as it is now.
    pub fn edit_occurrence(
        &self,
        id: EventId,
        recurrence_id: &str,
        scope: Scope,
        draft: EventDraft,
    ) -> Result<Changed<Occurrence>> {
        self.edit_in_scope(id, recurrence_id, scope, scope::Edit::Draft(draft))
    }

    /// Moves one Occurrence of a Recurring Event to another time in `scope`,
    /// as a drag or a resize in the grid does (see [`Core::edit_occurrence`]).
    pub fn reschedule_occurrence(
        &self,
        id: EventId,
        recurrence_id: &str,
        scope: Scope,
        when: When,
    ) -> Result<Changed<Occurrence>> {
        self.edit_in_scope(id, recurrence_id, scope, scope::Edit::Reschedule(when))
    }

    fn edit_in_scope(
        &self,
        id: EventId,
        recurrence_id: &str,
        scope: Scope,
        edit: scope::Edit,
    ) -> Result<Changed<Occurrence>> {
        let zone = self.clock.time_zone();
        let edited = self.record(Origin::UserAction, |unit| {
            scope::edit(unit, zone, id, recurrence_id, scope, edit)
        })?;
        let occurrence = edited.value;
        Ok(Changed {
            unit: edited.unit,
            value: Occurrence {
                event_id: occurrence.id,
                calendar_id: occurrence.calendar_id,
                title: occurrence.title,
                when: occurrence.when.to_when(zone)?,
                recurrence_id: occurrence.recurrence_id,
            },
        })
    }

    /// Deletes an Event from a writable Calendar. For a Recurring Event or
    /// one of its Exceptions, that is the whole Recurring Event with all its
    /// Exceptions.
    pub fn delete_event(&self, id: EventId) -> Result<Changed<()>> {
        self.record(Origin::UserAction, |unit| {
            let event = unit.find_event(id)?;
            let uid = unit.event_uid(id)?;
            let mut deleted = vec![id];
            if event.fields.recurrence_id.is_some() {
                deleted.extend(
                    unit.find_event_by_uid(event.calendar_id, &uid)?
                        .map(|(id, _)| id),
                );
            }
            if event.fields.rule.is_some() || event.fields.recurrence_id.is_some() {
                deleted.extend(
                    unit.exceptions(event.calendar_id, &uid)?
                        .into_iter()
                        .map(|(id, _)| id),
                );
            }
            deleted.sort_unstable_by_key(|id| id.0);
            deleted.dedup();
            deleted.into_iter().try_for_each(|id| unit.delete(id))
        })
    }

    /// Deletes one Occurrence of a Recurring Event, given by the Event and
    /// the recurrence ID its [`Occurrence`] has, and leaves the rest of the
    /// Recurring Event in place: the Occurrence becomes a cancelled
    /// Exception, and an Exception that changed it is deleted. The only
    /// Occurrence of a single Event is the Event.
    pub fn delete_occurrence(&self, id: EventId, recurrence_id: &str) -> Result<Changed<()>> {
        let cancelled =
            recurrence::recurrence_id(recurrence_id).ok_or(CoreError::OccurrenceNotFound(id))?;
        self.record(Origin::UserAction, |unit| {
            let event = unit.find_event(id)?;
            let calendar_id = event.calendar_id;
            let uid = unit.event_uid(id)?;
            let recurring = match &event.fields.recurrence_id {
                Some(_) => unit.find_event_by_uid(calendar_id, &uid)?,
                None => Some((id, event.fields)),
            }
            .filter(|(_, fields)| fields.rule.is_some());
            for (exception, fields) in unit.exceptions(calendar_id, &uid)? {
                let changed = fields.recurrence_id.as_deref();
                if changed.and_then(recurrence::recurrence_id).as_ref() == Some(&cancelled) {
                    unit.delete(exception)?;
                }
            }
            match recurring {
                Some((recurring_id, fields)) => unit.replace(
                    recurring_id,
                    &StoredEvent {
                        calendar_id,
                        fields: EventFields {
                            exdates: recurrence::with_exdate(&fields.exdates, &cancelled),
                            ..fields
                        },
                    },
                ),
                None if unit.exists(id)? => unit.delete(id),
                None => Ok(()),
            }
        })
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
    /// Events it already has what `existing` says, as one Change Journal unit.
    pub fn run_import(
        &self,
        preview: &ImportPreview,
        existing: ExistingEvents,
    ) -> Result<ImportOutcome> {
        let run = self.record(Origin::Import, |unit| import::run(unit, preview, existing))?;
        Ok(ImportOutcome {
            unit: run.unit,
            ..run.value
        })
    }

    /// Undoes a Change Journal unit whole: every Event it changed gets its
    /// previous version back under the same identity. Refused while an Event
    /// changed since, and for a unit undone already. The undo is a unit
    /// itself, and undoing it redoes the change.
    pub fn undo_unit(&self, id: UnitId) -> Result<Changed<()>> {
        let undone = journal::undo(&self.store(), id, self.clock.now().timestamp())?;
        Ok(self.signal(undone))
    }

    /// Runs `change` as one Change Journal unit of `origin` and sends
    /// "Occurrences changed" for the Calendars it changed Events in.
    fn record<T>(
        &self,
        origin: Origin,
        change: impl FnOnce(&mut Unit) -> Result<T>,
    ) -> Result<Changed<T>> {
        let recorded =
            journal::record(&self.store(), origin, self.clock.now().timestamp(), change)?;
        Ok(self.signal(recorded))
    }

    /// Sends "Occurrences changed" for a recorded unit's Calendars, once the
    /// Store lock is released.
    fn signal<T>(&self, recorded: journal::Recorded<T>) -> Changed<T> {
        if !recorded.calendars.is_empty() {
            self.signals.send(Signal::OccurrencesChanged {
                calendar_ids: recorded.calendars,
            });
        }
        Changed {
            unit: recorded.unit,
            value: recorded.value,
        }
    }

    fn changed_event(&self, changed: Changed<(EventId, StoredEvent)>) -> Result<Changed<Event>> {
        let (id, stored) = changed.value;
        self.to_event(id, stored).map(|value| Changed {
            unit: changed.unit,
            value,
        })
    }

    fn to_event(&self, id: EventId, stored: StoredEvent) -> Result<Event> {
        let description = description_of(&stored.fields);
        // An Exception repeats with its Recurring Event, which the editor
        // can change from it.
        let recurring = match stored.fields.recurrence_id {
            Some(_) => {
                let store = self.store();
                let uid = store.event_uid(id)?;
                store.find_event_by_uid(stored.calendar_id, &uid)?
            }
            None => None,
        };
        let (repeat, custom_repeat) = recurrence::repeat_of(
            recurring
                .as_ref()
                .map_or(&stored.fields, |(_, fields)| fields),
        );
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
            repeat,
            custom_repeat,
            recurrence_id: fields.recurrence_id,
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

fn description_of(fields: &EventFields) -> StructuredText {
    StructuredText::from_description(&fields.description, fields.description_html)
}
