//! The Mass-Change Guard's thresholds (spec: "Sync"; ADR 0009): the one
//! rule for how many changed Events of a Calendar are unusually many. An
//! undo past them is a Dangerous Undo; ticket 22's sync pauses past them.

/// More changed Events than this in a Calendar can be unusually many.
const EVENTS: usize = 10;

/// More than this share of a Calendar's Events, in percent, can be
/// unusually many.
const PERCENT: usize = 5;

/// Whether changing `changed` Events of a Calendar that has
/// `calendar_events` is past the thresholds: more than 10 Events, and more
/// than 5% of the Calendar, so that a small Calendar isn't guarded at every
/// change and a large one still is past 10.
pub(crate) fn exceeded(changed: usize, calendar_events: usize) -> bool {
    changed > EVENTS && changed * 100 > calendar_events * PERCENT
}

#[cfg(test)]
mod tests {
    use super::exceeded;

    #[test]
    fn more_than_10_events_and_more_than_5_percent_of_the_calendar() {
        assert!(!exceeded(10, 10));
        assert!(exceeded(11, 11));
        assert!(exceeded(11, 219));
        assert!(!exceeded(11, 220));
        assert!(exceeded(412, 8_000));
        assert!(!exceeded(400, 8_000));
        assert!(!exceeded(1, 3));
    }
}
