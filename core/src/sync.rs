//! The sync engine's incoming half: what changed in a Calendar on its
//! server, whatever its Provider, applied to its local copy in one Change
//! Journal unit.

use crate::event::Loaded;
use crate::journal::Unit;
use crate::remote::Changes;
use crate::{CalendarId, Result};

/// Applies `changes` to the Calendar `calendar_id` in `unit`, and records
/// its new sync state with them. An Event the Calendar has with the UID of
/// a changed one takes its new version (ADR 0007's rules 2 and 3 apply);
/// the others are added. A removed object's Event goes with its Exceptions.
pub(crate) fn apply(unit: &mut Unit, calendar_id: CalendarId, changes: &Changes) -> Result<()> {
    unit.writable(calendar_id)?;
    for event in &changes.changed {
        // The object may hold an Event of another UID now.
        if let Some(server) = &event.server_data
            && let Some((_, uid)) = unit.event_at(calendar_id, &server.href)?
            && uid != event.uid
        {
            Loaded::of_uid(unit, calendar_id, &uid)?.delete_all(unit)?;
        }
        Loaded::of_uid(unit, calendar_id, &event.uid)?.take(unit, event)?;
    }
    // After the changed ones, so that an Event whose object moved to
    // another address is changed, not deleted and added again.
    for href in &changes.removed {
        if let Some((_, uid)) = unit.event_at(calendar_id, href)? {
            Loaded::of_uid(unit, calendar_id, &uid)?.delete_all(unit)?;
        }
    }
    unit.synced(calendar_id, changes.state.token.as_deref())
}
