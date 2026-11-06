//! Recurrence IDs as obtained (ADR 0012). Every Event Row, Unsent Change
//! and Server Object keeps the recurrence ID of an Exception as it was
//! obtained: a date, a wall time with no zone, or a wall time in the zone it
//! was obtained in. Whether two of them name the same Occurrence depends on
//! the Recurring Event, so it is worked out when comparing, against the
//! Recurring Event, through their Normal Forms, which exist only in memory.
//! Without a Recurring Event, a recurrence ID counts as obtained (ticket 88).

use chrono_tz::Tz;

use super::rule::start_of_id;
use super::when::{instant, wall_text};
use crate::import::{ImportedWhen, Time};

/// A recurrence ID as obtained: from a file or a server, from FreeCal in the
/// Normal Form for the Exceptions it makes, or read back from a stored field
/// or from the window, untrusted. It has no `PartialEq`, so that no code
/// compares recurrence IDs as text: they compare by their [`NormalForm`],
/// which takes the Recurring Event, or its absence.
#[derive(Debug, Clone)]
pub(crate) struct RecurrenceId(String);

impl RecurrenceId {
    /// The recurrence ID `text`, as a stored field or the window gives it.
    pub fn obtained(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// The recurrence ID that names `original`, the original start of an
    /// Occurrence as a file or a server gives it: a date, a wall time with
    /// no zone, or one in the zone it was given in, written after it as
    /// `2026-10-12T03:00:00[America/New_York]`.
    pub fn of_original(original: &Time) -> Self {
        Self(match original {
            Time::Date(date) => date.to_string(),
            Time::Wall(wall, None) => wall_text(*wall),
            Time::Wall(wall, Some(zone)) => format!("{}[{}]", wall_text(*wall), zone.name()),
        })
    }

    /// The text as obtained, which the Local Store writes.
    pub fn as_obtained(&self) -> &str {
        &self.0
    }

    /// The Normal Form of the Occurrence it names of a Recurring Event at
    /// `recurring`. With no Recurring Event, the recurrence ID as obtained,
    /// written as [`Self::of_original`] writes it. None when it isn't a
    /// recurrence ID, such as one with a zone FreeCal doesn't know.
    pub fn normal_form(&self, recurring: Option<&ImportedWhen>) -> Option<NormalForm> {
        let original = original_of(&self.0)?;
        Some(match recurring {
            Some(recurring) => in_normal_form(&original, recurring),
            None => NormalForm(Self::of_original(&original).0),
        })
    }
}

/// A recurrence ID in the Normal Form (ADR 0012): the Occurrence's wall time
/// in the Event Time Zone of its Recurring Event's start, or its date when
/// that is all-day. It exists only in memory, computed against one
/// Recurring Event, so two compare by Occurrence when they were computed
/// against the same one. FreeCal writes the recurrence IDs of the
/// Exceptions it makes in it, and the window gets an Occurrence's as its ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NormalForm(String);

impl NormalForm {
    /// `text` that the Event module computed in the Normal Form, such as a
    /// start the repeat rule gives.
    pub(super) fn computed(text: String) -> Self {
        Self(text)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The text, as the window gets it.
    pub fn into_string(self) -> String {
        self.0
    }

    /// The recurrence ID to store for it, as FreeCal writes those it makes.
    pub fn to_recurrence_id(&self) -> RecurrenceId {
        RecurrenceId(self.0.clone())
    }
}

/// The Normal Form of the Occurrence of a Recurring Event at `recurring`
/// that starts at `original`. A time without a zone is taken as given in
/// the Event Time Zone; a date for a timed Event means the Occurrence on
/// that date.
pub(crate) fn in_normal_form(original: &Time, recurring: &ImportedWhen) -> NormalForm {
    NormalForm(match (*recurring, original) {
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
    })
}

/// The original start the recurrence ID `text`, as
/// [`RecurrenceId::of_original`] writes it, names; none when it isn't one.
fn original_of(text: &str) -> Option<Time> {
    let (wall, zone) = match text.strip_suffix(']').and_then(|rest| rest.split_once('[')) {
        Some((wall, zone)) => (wall, Some(zone.parse::<Tz>().ok()?)),
        None => (text, None),
    };
    let start = start_of_id(wall)?;
    match (wall.contains('T'), zone) {
        (true, zone) => Some(Time::Wall(start, zone)),
        (false, None) => Some(Time::Date(start.date())),
        (false, Some(_)) => None,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveDateTime};
    use chrono_tz::America::New_York;
    use chrono_tz::Europe::Berlin;

    use super::*;
    use crate::event::when::WALL_TIME;

    fn wall(text: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(text, WALL_TIME).unwrap()
    }

    /// A Recurring Event at 9:00 in Berlin on 12 October 2026.
    fn timed() -> ImportedWhen {
        ImportedWhen::Timed {
            start: wall("2026-10-12T09:00:00"),
            start_zone: Berlin,
            end: wall("2026-10-12T10:00:00"),
            end_zone: Berlin,
        }
    }

    fn all_day() -> ImportedWhen {
        let date = NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
        ImportedWhen::AllDay {
            start: date,
            end: date.succ_opt().unwrap(),
        }
    }

    impl RecurrenceId {
        /// Whether it and `other` name the same Occurrence of a Recurring
        /// Event at `recurring`, or, with none, the same original start as
        /// obtained, as the pairing of Exceptions compares them. One that
        /// isn't a recurrence ID names none.
        fn same_occurrence(&self, other: &Self, recurring: Option<&ImportedWhen>) -> bool {
            match (self.normal_form(recurring), other.normal_form(recurring)) {
                (Some(one), Some(other)) => one == other,
                _ => false,
            }
        }
    }

    fn normal_form(text: &str, recurring: Option<&ImportedWhen>) -> Option<String> {
        let normal_form = RecurrenceId::obtained(text).normal_form(recurring);
        normal_form.map(NormalForm::into_string)
    }

    /// A stored field is untrusted: only a zone FreeCal knows reads.
    #[test]
    fn reads_back_as_written_and_only_with_a_zone_freecal_knows() {
        let wall = wall("2026-10-12T03:00:00");
        for original in [
            Time::Date(wall.date()),
            Time::Wall(wall, None),
            Time::Wall(wall, Some(New_York)),
        ] {
            let text = RecurrenceId::of_original(&original).0;
            assert_eq!(original_of(&text), Some(original), "{text}");
        }
        assert_eq!(
            normal_form("2026-10-12T3:0:0[America/New_York]", None).as_deref(),
            Some("2026-10-12T03:00:00[America/New_York]")
        );
        for unreadable in [
            "2026-10-12T03:00:00[<img src=x>]",
            "2026-10-12T03:00:00[]",
            "2026-10-12[America/New_York]",
            "2026-10-12T03:00:00[America/New_York",
            "<b>",
        ] {
            assert_eq!(
                normal_form(unreadable, Some(&timed())),
                None,
                "{unreadable}"
            );
        }
    }

    /// Ticket 88's pairing: a time in another zone at the same instant, a
    /// floating time as wall time in the Event Time Zone, a date for a
    /// timed Recurring Event as its Occurrence on that date, and a time for
    /// an all-day one as its date.
    #[test]
    fn names_the_same_occurrence_as_ticket_88_pairs_them() {
        let pairs = [
            (
                "2026-10-12T03:00:00[America/New_York]",
                "2026-10-12T09:00:00",
                timed(),
            ),
            (
                "2026-10-12T09:00:00[Europe/Berlin]",
                "2026-10-12T09:00:00",
                timed(),
            ),
            (
                "2026-10-12T09:00:00",
                "2026-10-12T09:00:00[Europe/Berlin]",
                timed(),
            ),
            ("2026-10-12", "2026-10-12T09:00:00", timed()),
            (
                "2026-10-12T15:00:00[America/New_York]",
                "2026-10-12",
                all_day(),
            ),
            ("2026-10-12T09:00:00", "2026-10-12", all_day()),
        ];
        for (one, other, recurring) in pairs {
            let (one, other) = (RecurrenceId::obtained(one), RecurrenceId::obtained(other));
            assert!(
                one.same_occurrence(&other, Some(&recurring)),
                "{one:?} {other:?}"
            );
        }
        let in_new_york = RecurrenceId::obtained("2026-10-12T03:00:00[America/New_York]");
        assert!(!in_new_york.same_occurrence(
            &RecurrenceId::obtained("2026-10-12T03:00:00"),
            Some(&timed())
        ));
    }

    /// Without their Recurring Event, two recurrence IDs obtained in
    /// different forms pair only when they are written alike.
    #[test]
    fn without_a_recurring_event_counts_as_obtained() {
        let in_new_york = RecurrenceId::obtained("2026-10-12T03:00:00[America/New_York]");
        let in_berlin = RecurrenceId::obtained("2026-10-12T09:00:00[Europe/Berlin]");
        assert!(!in_new_york.same_occurrence(&in_berlin, None));
        assert!(in_new_york.same_occurrence(
            &RecurrenceId::obtained("2026-10-12T3:0:0[America/New_York]"),
            None
        ));
        assert!(in_new_york.same_occurrence(&in_berlin, Some(&timed())));
        let unreadable = RecurrenceId::obtained("<b>");
        assert!(!unreadable.same_occurrence(&unreadable, None));
    }
}
