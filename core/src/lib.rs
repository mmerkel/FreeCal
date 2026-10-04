//! FreeCal's core: all domain logic behind one application interface, [`Core`].
//!
//! The frontend talks only to [`Core`]. Besides answering requests, the core
//! pushes [`Signal`]s to a [`SignalSink`].

mod account;
mod calendar;
mod clock;
mod description;
mod event;
mod signal;
mod store;
#[cfg(test)]
mod typescript;

use std::path::Path;

use chrono::NaiveDate;
use std::sync::{Arc, Mutex};

pub use account::{Account, AccountId, Provider};
pub use calendar::{Calendar, CalendarId, Colour};
pub use clock::{Clock, SystemClock};
pub use description::{Description, Piece, link_allowed};
pub use event::{Event, EventDraft, EventId, Occurrence, When};
pub use signal::{Signal, SignalSink};

use calendar::calendar_name;
use event::{StoredRange, StoredWhen};
use store::{EventFields, StoredEvent, Store};

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("the Local Store failed: {0}")]
    Store(#[from] rusqlite::Error),
    #[error("could not create the data directory: {0}")]
    DataDirectory(#[from] std::io::Error),
    #[error("there is no Account {0:?}")]
    AccountNotFound(AccountId),
    #[error("there is no Calendar {0:?}")]
    CalendarNotFound(CalendarId),
    #[error("Calendar {0:?} is read-only")]
    CalendarReadOnly(CalendarId),
    #[error("there is no Event {0:?}")]
    EventNotFound(EventId),
    #[error("an Event can't end before it starts")]
    EndBeforeStart,
    #[error("the Local Store has an Event time FreeCal can't read: {0:?}")]
    UnreadableEventTime(String),
    #[error("a Calendar needs a name")]
    EmptyCalendarName,
    #[error("{0:?} is not a colour of the form #rrggbb")]
    InvalidColour(String),
    #[error("the Local Account cannot be removed")]
    LocalAccountCannotBeRemoved,
    #[error(
        "the Local Store was written by a newer FreeCal \
         (schema {found}, this build knows {known})"
    )]
    NewerLocalStore { found: i64, known: i64 },
    #[error("the Local Store has an Account with an unknown Provider {0:?}")]
    UnknownProvider(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;

/// The core application interface.
pub struct Core {
    store: Mutex<Store>,
    clock: Arc<dyn Clock>,
    signals: Arc<dyn SignalSink>,
}

impl Core {
    /// Opens the Local Store in `data_dir`, creating it on first launch.
    pub fn open(
        data_dir: &Path,
        clock: Arc<dyn Clock>,
        signals: Arc<dyn SignalSink>,
    ) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let store = Store::open(&data_dir.join("freecal.sqlite3"))?;
        Ok(Self {
            store: Mutex::new(store),
            clock,
            signals,
        })
    }

    /// The current local date, on which the views open.
    pub fn today(&self) -> NaiveDate {
        self.clock.now().date_naive()
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
        let mut occurrences = events
            .into_iter()
            .map(|event| {
                Ok(Occurrence {
                    event_id: event.id,
                    calendar_id: event.calendar_id,
                    title: event.title,
                    when: event.when.to_when(zone)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Occurrence::order(&mut occurrences);
        Ok(occurrences)
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
                when: StoredWhen::from_when(draft.when, self.clock.time_zone())?,
                location: draft.location,
                description: draft.description,
                description_html: false,
            };
            let id = store.insert_event(calendar_id, &fields)?;
            (id, StoredEvent {
                calendar_id,
                fields,
            })
        };
        self.signals.send(Signal::OccurrencesChanged {
            calendar_ids: vec![calendar_id],
        });
        self.to_event(created.0, created.1)
    }

    /// Changes an Event, possibly moving it to another writable Calendar.
    /// What the user left as it was stays exactly as stored: an unchanged
    /// time keeps its time zone and an unchanged description its original
    /// bytes, even if it was HTML.
    pub fn edit_event(&self, id: EventId, draft: EventDraft) -> Result<Event> {
        let zone = self.clock.time_zone();
        let (before, after) = {
            let store = self.store();
            let before = store.find_event(id)?;
            writable(&store, before.calendar_id)?;
            writable(&store, draft.calendar_id)?;

            let old = &before.fields;
            let when = if old.when.to_when(zone).ok() == Some(draft.when) {
                old.when.clone()
            } else {
                StoredWhen::from_when(draft.when, zone)?
            };
            let (description, description_html) =
                if draft.description == description_of(old).text {
                    (old.description.clone(), old.description_html)
                } else {
                    (draft.description, false)
                };
            let after = StoredEvent {
                calendar_id: draft.calendar_id,
                fields: EventFields {
                    title: draft.title,
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

    fn to_event(&self, id: EventId, stored: StoredEvent) -> Result<Event> {
        let description = description_of(&stored.fields);
        let fields = stored.fields;
        Ok(Event {
            id,
            calendar_id: stored.calendar_id,
            title: fields.title,
            when: fields.when.to_when(self.clock.time_zone())?,
            location: fields.location,
            description,
        })
    }

    /// The Local Store, locked. Signals are sent only after the lock is
    /// released, so a subscriber can read straight away.
    fn store(&self) -> std::sync::MutexGuard<'_, Store> {
        self.store.lock().unwrap()
    }
}

/// Refuses changes to the Events of a missing or Read-only Calendar.
fn writable(store: &Store, id: CalendarId) -> Result<()> {
    if store.find_calendar(id)?.read_only {
        Err(CoreError::CalendarReadOnly(id))
    } else {
        Ok(())
    }
}

fn description_of(fields: &EventFields) -> Description {
    if fields.description_html {
        Description::from_html(&fields.description)
    } else {
        Description::from_plain_text(&fields.description)
    }
}
