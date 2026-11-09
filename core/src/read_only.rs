//! Whether an edit may be made (spec: "Writability", "Attendees"): the one
//! rule for a Read-only Calendar and an Event with guests. A [`ReadOnly`]
//! is built from a stored Calendar's user mark and Server read-only; for an
//! Event, it asks whether its UID has guests (see [`crate::guests`]) only
//! when that decides. The Change Journal, the Guard of Event Rows, is the
//! only one that asks it to decide on an edit; the Local Store's Calendar
//! reader asks it only for [`crate::Calendar::read_only`], and the Sync
//! Engine only for Server read-only. The window keeps its own copy of the
//! rule as a hint (`whyReadOnly` in `src/lib/windowModel.svelte.ts`).

use crate::journal::TransactionOrigin;
use crate::{CalendarId, CoreError, Result};

/// What decides whether the Events of a Calendar, or of one UID in it, may
/// be edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReadOnly {
    /// Whether the user marked the Calendar read-only in FreeCal.
    marked: bool,
    /// Whether the Calendar's server doesn't allow the user to change it.
    server_read_only: bool,
}

impl ReadOnly {
    /// A Calendar's, from its stored user mark and Server read-only.
    pub fn calendar(marked: bool, server_read_only: bool) -> Self {
        ReadOnly {
            marked,
            server_read_only,
        }
    }

    /// Whether the Calendar is a Read-only Calendar: the user marked it or
    /// its server doesn't allow changing it.
    pub fn calendar_read_only(self) -> bool {
        self.marked || self.server_read_only
    }

    /// Whether the Calendar's server doesn't allow the user to change it.
    pub fn server_read_only(self) -> bool {
        self.server_read_only
    }

    /// Whether an edit of `origin` that targets the Calendar `id`, not one
    /// of its UIDs, may be made, or why not, as
    /// [`ReadOnly::may_edit_event`] says for a UID without guests.
    pub fn may_edit(self, id: CalendarId, origin: TransactionOrigin) -> Result<()> {
        self.may_edit_event(id, origin, || Ok(false))
    }

    /// Whether an edit of `origin` may be made to the Events of a UID in
    /// the Calendar `id`, which have guests as `has_guests` reads, or why
    /// not: an Incoming Sync and a First Sync may always edit, as they load
    /// what the server has; every other Transaction Origin may not edit a
    /// Read-only Calendar, nor an Event with guests. A Read-only Calendar is
    /// named first, as the user can change that in FreeCal. `has_guests` is
    /// read only when it decides, so that a sync costs no query for it.
    pub fn may_edit_event(
        self,
        id: CalendarId,
        origin: TransactionOrigin,
        has_guests: impl FnOnce() -> Result<bool>,
    ) -> Result<()> {
        if origin.is_incoming_sync() {
            Ok(())
        } else if self.calendar_read_only() {
            Err(CoreError::CalendarReadOnly(id))
        } else if has_guests()? {
            Err(CoreError::EventHasGuests)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: CalendarId = CalendarId(7);

    const EDITS: [TransactionOrigin; 3] = [
        TransactionOrigin::UserAction,
        TransactionOrigin::Import,
        TransactionOrigin::Undo,
    ];

    const SYNCS: [TransactionOrigin; 2] = [
        TransactionOrigin::IncomingSync,
        TransactionOrigin::FirstSync,
    ];

    /// The ways a Calendar is read-only: the user's mark, Server read-only.
    const READ_ONLY: [(bool, bool); 2] = [(true, false), (false, true)];

    fn no_guests_read() -> Result<bool> {
        panic!("guests read, though they don't decide")
    }

    #[test]
    fn a_calendar_is_read_only_when_marked_or_server_read_only() {
        assert!(!ReadOnly::calendar(false, false).calendar_read_only());
        assert!(ReadOnly::calendar(true, false).calendar_read_only());
        assert!(ReadOnly::calendar(false, true).calendar_read_only());
        assert!(ReadOnly::calendar(true, true).calendar_read_only());
    }

    #[test]
    fn a_writable_calendar_without_guests_may_be_edited() {
        for origin in EDITS.into_iter().chain(SYNCS) {
            let calendar = ReadOnly::calendar(false, false);
            assert!(calendar.may_edit(ID, origin).is_ok(), "{origin:?}");
            assert!(calendar.may_edit_event(ID, origin, || Ok(false)).is_ok());
        }
    }

    #[test]
    fn a_read_only_calendar_refuses_every_edit_but_a_syncs() {
        for (marked, server_read_only) in READ_ONLY {
            let calendar = ReadOnly::calendar(marked, server_read_only);
            for origin in EDITS {
                let refused = calendar.may_edit(ID, origin);
                assert!(
                    matches!(refused, Err(CoreError::CalendarReadOnly(ID))),
                    "{origin:?}"
                );
            }
            for origin in SYNCS {
                assert!(calendar.may_edit(ID, origin).is_ok());
            }
        }
    }

    #[test]
    fn guests_refuse_every_edit_but_a_syncs() {
        let calendar = ReadOnly::calendar(false, false);
        for origin in EDITS {
            let refused = calendar.may_edit_event(ID, origin, || Ok(true));
            assert!(
                matches!(refused, Err(CoreError::EventHasGuests)),
                "{origin:?}"
            );
        }
        for origin in SYNCS {
            assert!(calendar.may_edit_event(ID, origin, no_guests_read).is_ok());
        }
    }

    #[test]
    fn a_read_only_calendar_is_named_before_guests() {
        for (marked, server_read_only) in READ_ONLY {
            let refused = ReadOnly::calendar(marked, server_read_only).may_edit_event(
                ID,
                TransactionOrigin::UserAction,
                no_guests_read,
            );
            assert!(matches!(refused, Err(CoreError::CalendarReadOnly(ID))));
        }
    }
}
