//! Who counts as a Guest (new term: an attendee of an Event who isn't the
//! user, spec: "Attendees"). An Event has guests when the Corresponding
//! Server Object of its UID names a Guest in an `ATTENDEE` of any of its
//! Events. It is then read-only, as an Event of a Read-only Calendar is
//! (US-46): the Change Journal refuses every change to it but an Incoming
//! Sync's. That holds for the whole Recurring Event, as FreeCal writes a
//! UID's Server Object as one piece: an Exception with a Guest makes all of
//! it read-only. An Event that no Server Object holds, such as one of a
//! Local Calendar, has none.
//!
//! How the core knows the user: by an Account's Own Addresses (new term:
//! the calendar user addresses that name the user on its server), read
//! when the Account is added and kept in the Local Store. For CalDAV, the
//! `calendar-user-address-set` of the user's principal (RFC 6638 §2.4.1),
//! read with its `calendar-home-set` in the same request, and the username
//! too when it is an email address, as some servers, such as Radicale,
//! give only the principal's path. For Google (ticket 26), the address of
//! the Google account, which names its primary calendar. An address the
//! server adds later isn't noticed until the Account is added again; the
//! Event then stays read-only, which errs on the safe side.
//!
//! Addresses compare in one form ([`compared`]): without `mailto:` and in
//! lower case, as servers and clients write the same address in either.

use icalendar::parser::{read_components, unfold};

/// An address in the form addresses compare in: trimmed, without
/// surrounding quotes and a `mailto:`, in lower case.
fn compared(address: &str) -> String {
    let address = address.trim().trim_matches('"').trim();
    let address = match address.get(..7) {
        Some(scheme) if scheme.eq_ignore_ascii_case("mailto:") => &address[7..],
        _ => address,
    };
    address.to_lowercase()
}

/// The `ATTENDEE`s of every Event of the iCalendar text `text`, each once,
/// in the form addresses compare in; none for text FreeCal can't read. The
/// attendees of an alarm get its emails and aren't the Event's.
pub(crate) fn attendees(text: &str) -> Vec<String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let unfolded = unfold(text);
    let Ok(roots) = read_components(&unfolded) else {
        return Vec::new();
    };
    let mut attendees = Vec::new();
    for event in roots
        .iter()
        .filter(|root| root.name.as_str().eq_ignore_ascii_case("VCALENDAR"))
        .flat_map(|root| &root.components)
        .filter(|component| component.name.as_str().eq_ignore_ascii_case("VEVENT"))
    {
        for property in &event.properties {
            if property.name.as_str().eq_ignore_ascii_case("ATTENDEE") {
                let address = compared(property.val.as_str());
                if !address.is_empty() && !attendees.contains(&address) {
                    attendees.push(address);
                }
            }
        }
    }
    attendees
}

/// A CalDAV Account's Own Addresses: the addresses its server gives for the
/// user, and `username` when it is an email address, each once, in the
/// form addresses compare in.
pub(crate) fn caldav_own_addresses(given: &[String], username: &str) -> Vec<String> {
    let mut own: Vec<String> = Vec::new();
    let username = username.contains('@').then_some(username);
    for address in given.iter().map(String::as_str).chain(username) {
        let address = compared(address);
        if !address.is_empty() && !own.contains(&address) {
            own.push(address);
        }
    }
    own
}

/// Addresses as the Local Store keeps them: a JSON array.
pub(crate) fn stored(addresses: &[String]) -> String {
    serde_json::to_string(addresses).expect("a list of strings is JSON")
}

/// Whether an Event has guests, from the attendees of its Corresponding
/// Server Object and its Account's Own Addresses as stored: untrusted, so
/// attendees FreeCal can't read count as a Guest, and Own Addresses it
/// can't read as none, which keeps the Event read-only. None for an Event
/// without a Server Object.
pub(crate) fn has_guests(attendees: Option<&str>, own_addresses: Option<&str>) -> bool {
    let Some(attendees) = attendees else {
        return false;
    };
    let Ok(attendees) = serde_json::from_str::<Vec<String>>(attendees) else {
        return true;
    };
    let own: Vec<String> = own_addresses
        .and_then(|own| serde_json::from_str(own).ok())
        .unwrap_or_default();
    attendees.iter().any(|attendee| !own.contains(attendee))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object(events: &str) -> String {
        format!("BEGIN:VCALENDAR\nVERSION:2.0\n{events}END:VCALENDAR\n").replace('\n', "\r\n")
    }

    #[test]
    fn the_attendees_of_every_event_count_in_one_form_and_not_those_of_alarms() {
        let text = object(
            "BEGIN:VEVENT\nUID:a\nATTENDEE;CN=Bob:MAILTO:Bob@Example.com\n\
             ATTENDEE:\"mailto:carol@example.com\"\n\
             BEGIN:VALARM\nACTION:EMAIL\nATTENDEE:mailto:alarm@example.com\nEND:VALARM\n\
             END:VEVENT\n\
             BEGIN:VEVENT\nUID:a\nRECURRENCE-ID:20261012T090000Z\n\
             ATTENDEE:mailto:bob@example.com\nATTENDEE;CUTYPE=ROOM:urn:uuid:Room-1\n\
             END:VEVENT\n",
        );
        assert_eq!(
            attendees(&text),
            ["bob@example.com", "carol@example.com", "urn:uuid:room-1"]
        );
    }

    #[test]
    fn text_freecal_can_t_read_has_no_attendees() {
        assert!(attendees("ATTENDEE:mailto:bob@example.com").is_empty());
    }

    #[test]
    fn a_caldav_account_s_own_addresses_take_the_username_only_when_it_is_an_address() {
        let given = ["mailto:Anna@Example.com".to_owned(), "/anna/".to_owned()];
        assert_eq!(
            caldav_own_addresses(&given, "anna@example.com"),
            ["anna@example.com", "/anna/"]
        );
        assert_eq!(
            caldav_own_addresses(&given, "anna"),
            ["anna@example.com", "/anna/"]
        );
    }

    #[test]
    fn a_guest_is_an_attendee_without_an_own_address() {
        let own = stored(&["anna@example.com".to_owned()]);
        let only_anna = stored(&["anna@example.com".to_owned()]);
        let with_bob = stored(&["anna@example.com".to_owned(), "bob@example.com".to_owned()]);
        assert!(!has_guests(None, Some(&own)));
        assert!(!has_guests(Some("[]"), None));
        assert!(!has_guests(Some(&only_anna), Some(&own)));
        assert!(has_guests(Some(&with_bob), Some(&own)));
        assert!(has_guests(Some(&only_anna), None));
    }

    #[test]
    fn stored_values_freecal_can_t_read_keep_the_event_read_only() {
        assert!(has_guests(Some("not JSON"), None));
        let only_anna = stored(&["anna@example.com".to_owned()]);
        assert!(has_guests(Some(&only_anna), Some("{\"not\": \"a list\"}")));
    }
}
