//! Whether an edit may be made (spec: "Writability", "Attendees"): the one
//! rule for a Read-only Calendar and an Event with guests. A [`ReadOnly`]
//! is built from a stored Calendar's user mark and Server read-only, and,
//! for an Event, from whether its UID has guests (see [`crate::guests`]). The
//! Change Journal, the Guard of Event Rows, is the only one that asks it to
//! decide on an edit; the Local Store's Calendar reader asks it only for
//! [`crate::Calendar::read_only`]. The window keeps its own copy of the rule
//! as a hint (`whyReadOnly` in `src/lib/windowModel.svelte.ts`).

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
    /// Whether the Events of the UID have guests; false for a whole
    /// Calendar.
    has_guests: bool,
}

impl ReadOnly {
    /// A Calendar's, from its stored user mark and Server read-only.
    pub fn calendar(marked: bool, server_read_only: bool) -> Self {
        ReadOnly {
            marked,
            server_read_only,
            has_guests: false,
        }
    }

    /// The Events of one UID in this Calendar, which have guests or not.
    pub fn with_guests(self, has_guests: bool) -> Self {
        ReadOnly { has_guests, ..self }
    }

    /// Whether the Calendar is a Read-only Calendar: the user marked it or
    /// its server doesn't allow changing it.
    pub fn calendar_read_only(self) -> bool {
        self.marked || self.server_read_only
    }

    /// Whether an edit of `origin` may be made in the Calendar `id`, or why
    /// not: an Incoming Sync and a First Sync may always edit, as they load
    /// what the server has; every other Transaction Origin may not edit a
    /// Read-only Calendar, nor an Event with guests. A Read-only Calendar is
    /// named first, as the user can change that in FreeCal.
    pub fn may_edit(self, id: CalendarId, origin: TransactionOrigin) -> Result<()> {
        if origin.is_incoming_sync() {
            Ok(())
        } else if self.calendar_read_only() {
            Err(CoreError::CalendarReadOnly(id))
        } else if self.has_guests {
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

    #[test]
    fn a_calendar_is_read_only_when_marked_or_server_read_only() {
        assert!(!ReadOnly::calendar(false, false).calendar_read_only());
        assert!(ReadOnly::calendar(true, false).calendar_read_only());
        assert!(ReadOnly::calendar(false, true).calendar_read_only());
        assert!(ReadOnly::calendar(true, true).calendar_read_only());
    }

    #[test]
    fn guests_dont_make_the_calendar_read_only() {
        let event = ReadOnly::calendar(false, false).with_guests(true);
        assert!(!event.calendar_read_only());
    }

    #[test]
    fn a_writable_calendar_without_guests_may_be_edited() {
        for origin in EDITS.into_iter().chain(SYNCS) {
            let calendar = ReadOnly::calendar(false, false);
            assert!(calendar.may_edit(ID, origin).is_ok(), "{origin:?}");
            assert!(calendar.with_guests(false).may_edit(ID, origin).is_ok());
        }
    }

    #[test]
    fn the_users_mark_refuses_every_edit_but_a_syncs() {
        for origin in EDITS {
            let refused = ReadOnly::calendar(true, false).may_edit(ID, origin);
            assert!(
                matches!(refused, Err(CoreError::CalendarReadOnly(ID))),
                "{origin:?}"
            );
        }
        for origin in SYNCS {
            assert!(ReadOnly::calendar(true, false).may_edit(ID, origin).is_ok());
        }
    }

    #[test]
    fn server_read_only_refuses_every_edit_but_a_syncs() {
        for origin in EDITS {
            let refused = ReadOnly::calendar(false, true).may_edit(ID, origin);
            assert!(
                matches!(refused, Err(CoreError::CalendarReadOnly(ID))),
                "{origin:?}"
            );
        }
        for origin in SYNCS {
            assert!(ReadOnly::calendar(false, true).may_edit(ID, origin).is_ok());
        }
    }

    #[test]
    fn guests_refuse_every_edit_but_a_syncs() {
        let event = ReadOnly::calendar(false, false).with_guests(true);
        for origin in EDITS {
            let refused = event.may_edit(ID, origin);
            assert!(
                matches!(refused, Err(CoreError::EventHasGuests)),
                "{origin:?}"
            );
        }
        for origin in SYNCS {
            assert!(event.may_edit(ID, origin).is_ok());
        }
    }

    #[test]
    fn a_read_only_calendar_is_named_before_guests() {
        for (marked, server_read_only) in [(true, false), (false, true)] {
            let event = ReadOnly::calendar(marked, server_read_only).with_guests(true);
            let refused = event.may_edit(ID, TransactionOrigin::UserAction);
            assert!(matches!(refused, Err(CoreError::CalendarReadOnly(ID))));
        }
    }
}
