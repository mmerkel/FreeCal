//! The CalDAV resource to send for one Calendar and UID, built from its
//! Event Rows ([`resource_to_send`]), which the CalDAV Sender (ticket 123)
//! writes to the server. CalDAV keeps one resource per UID (RFC 4791 §4.1,
//! ADR 0015), so a Recurring Event and its Exceptions go in one.
//!
//! With a Server Object, its text is the starting point, and FreeCal
//! changes no more of it than the Event Rows differ from it: in each
//! `VEVENT`, only the groups of FreeCal's Properties whose fields differ
//! from what FreeCal reads from them are written anew ([`write::Group`]),
//! and with them `DTSTAMP`, `LAST-MODIFIED` and a higher `SEQUENCE`
//! ([`write::Revision`]); every other content line, attendees, conference links, alarms, custom
//! and `X-` properties and other components among them, is kept byte for
//! byte, as are its folding and parameters. Only line ends become CRLF, as
//! RFC 5545 requires. `VEVENT`s are paired with Event Rows by Occurrence
//! ([`Loaded::pair_with_server`]); one the Event Rows no longer hold goes,
//! and an Event Row no `VEVENT` holds gets a new one.
//!
//! The Server Object's text and the Event Rows are untrusted (spec:
//! "Untrusted content"); see [`write`] for how what is written stays valid
//! iCalendar.

mod write;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;

use crate::event::{EventFields, Loaded, ServerPairing, as_propagated};
use crate::import::{ImportedEvent, ImportedWhen, read_events};
use crate::{CoreError, Result};
use write::{Group, Revision, Written, new_vevent, vtimezone};

/// The iCalendar text of the CalDAV resource to send for `loaded`, the
/// Event Rows of one Calendar and UID, built on `server`, the text of its
/// CalDAV Server Object, or anew without one, stamping new `VEVENT`s
/// `now`. None when there are no Event Rows: then there is nothing to
/// write, only a Server Object to delete.
pub(crate) fn resource_to_send(
    loaded: &Loaded,
    server: Option<&str>,
    now: DateTime<Utc>,
) -> Result<Option<String>> {
    if !loaded.exists() {
        return Ok(None);
    }
    let text = match server {
        Some(server) => update(loaded, server, now)?,
        None => create(loaded, now)?,
    };
    Ok(Some(text))
}

/// A new `VCALENDAR` holding `loaded`'s Event Rows, the Event that isn't
/// an Exception first, after the `VTIMEZONE`s their times need.
fn create(loaded: &Loaded, now: DateTime<Utc>) -> Result<String> {
    let pairing = loaded.pair_with_server(&[]);
    let events = new_vevents(loaded, &pairing, now)?;
    let mut out = Written::new();
    out.begin("VCALENDAR");
    out.text_property("VERSION", "2.0");
    out.text_property("PRODID", "-//FreeCal//FreeCal//EN");
    for &(zone, from) in events.zones() {
        vtimezone(zone, from, &mut out);
    }
    out.append(&events);
    out.end("VCALENDAR");
    Ok(out.into_text())
}

/// New `VEVENT`s for the Event Rows of `pairing` that no `VEVENT` holds.
fn new_vevents(loaded: &Loaded, pairing: &ServerPairing, now: DateTime<Utc>) -> Result<Written> {
    let mut written = Written::new();
    for row in &pairing.left {
        new_vevent(
            loaded.uid(),
            row,
            pairing.recurring.as_ref(),
            now,
            &mut written,
        )?;
    }
    Ok(written)
}

/// Where a `VEVENT` of the Server Object's text sits: the index of its
/// `VCALENDAR` among the components at the top, and its own among the
/// `VCALENDAR`'s items.
type Place = (usize, usize);

/// `server`, the text of `loaded`'s Server Object, with its `VEVENT`s made
/// to hold `loaded`'s Event Rows. New `VEVENT`s follow the last one it has,
/// and the `VTIMEZONE`s that the times written need and a `VCALENDAR`
/// doesn't have yet come before its first `VEVENT`.
fn update(loaded: &Loaded, server: &str, now: DateTime<Utc>) -> Result<String> {
    let server = server.strip_prefix('\u{feff}').unwrap_or(server);
    let roots = components(server).ok_or(CoreError::UnreadableServerObject)?;
    let calendars: Vec<(usize, &Block)> = roots
        .iter()
        .enumerate()
        .filter_map(|(index, item)| match item {
            Item::Block(block) if block.kind == "VCALENDAR" => Some((index, block)),
            _ => None,
        })
        .collect();
    let Some(&(first_calendar, _)) = calendars.first() else {
        return Err(CoreError::UnreadableServerObject);
    };
    let mut places: Vec<Place> = Vec::new();
    let mut events: Vec<ImportedEvent> = Vec::new();
    for &(root, calendar) in &calendars {
        for (index, item) in calendar.items.iter().enumerate() {
            if item.is_vevent() {
                places.push((root, index));
                events.push(read_vevent(calendar, index)?);
            }
        }
    }
    let pairing = loaded.pair_with_server(&events);
    let mut rewritten: Vec<(Place, Written)> = Vec::new();
    for ((place, event), row) in places.iter().zip(&events).zip(&pairing.paired) {
        if let (Some(row), Item::Block(block)) = (row, &roots[place.0].items()[place.1]) {
            let recurring = pairing.recurring.as_ref();
            rewritten.push((*place, rewrite(block, event, row, recurring, now)?));
        }
    }
    let added = new_vevents(loaded, &pairing, now)?;
    // New VEVENTs follow the last VEVENT, or end the first VCALENDAR.
    let added_at = places
        .last()
        .map_or((first_calendar, None), |&(root, index)| (root, Some(index)));
    let mut out = Written::new();
    for (root, item) in roots.iter().enumerate() {
        let Item::Block(calendar) = item else {
            item.keep(&mut out);
            continue;
        };
        if calendar.kind != "VCALENDAR" {
            item.keep(&mut out);
            continue;
        }
        let mut needed = Written::new();
        for (place, written) in &rewritten {
            if place.0 == root {
                needed.append(written);
            }
        }
        if added_at.0 == root {
            needed.append(&added);
        }
        let mut zones = Written::new();
        for &(zone, from) in needed.zones() {
            if !calendar.has_vtimezone(zone) {
                vtimezone(zone, from, &mut zones);
            }
        }
        let first_vevent = calendar.items.iter().position(Item::is_vevent);
        out.kept(&calendar.begin.physical);
        for (index, item) in calendar.items.iter().enumerate() {
            if Some(index) == first_vevent {
                out.append(&zones);
            }
            if item.is_vevent() {
                if let Some((_, written)) = rewritten.iter().find(|(at, _)| *at == (root, index)) {
                    out.append(written);
                }
            } else {
                item.keep(&mut out);
            }
            if added_at == (root, Some(index)) {
                out.append(&added);
            }
        }
        if first_vevent.is_none() {
            out.append(&zones);
        }
        if added_at == (root, None) {
            out.append(&added);
        }
        out.kept(&calendar.end.physical);
    }
    Ok(out.into_text())
}

/// The `VEVENT` at `index` in `calendar`, read as the import reads files
/// ([`read_events`]) in its `VCALENDAR`, with the zones that defines, but
/// without its other `VEVENT`s. Floating times are pinned to UTC, which
/// [`as_propagated`] puts right for Event Rows pinned elsewhere.
fn read_vevent(calendar: &Block, index: usize) -> Result<ImportedEvent> {
    let mut alone = Written::new();
    alone.kept(&calendar.begin.physical);
    for (at, item) in calendar.items.iter().enumerate() {
        if at == index || !item.is_vevent() {
            item.keep(&mut alone);
        }
    }
    alone.kept(&calendar.end.physical);
    let text = alone.into_text();
    let read =
        read_events("", text.as_bytes(), Tz::UTC).map_err(|_| CoreError::UnreadableServerObject)?;
    let mut events = read.events.into_iter().chain(read.exceptions);
    match (events.next(), events.next()) {
        (Some(event), None) => Ok(event),
        _ => Err(CoreError::UnreadableServerObject),
    }
}

/// `vevent`, which FreeCal reads as `server`, made to hold `row`, an Event
/// Row of the Recurring Event at `recurring`, if it has one: the groups of
/// FreeCal's Properties whose field differs are written anew,
/// and with any of them its [`Revision`] properties, revised `now`: each
/// in place of its first line, or before its first component, such as an
/// alarm, if it had none. Every other line is kept.
fn rewrite(
    vevent: &Block,
    server: &ImportedEvent,
    row: &EventFields,
    recurring: Option<&ImportedWhen>,
    now: DateTime<Utc>,
) -> Result<Written> {
    let read = as_propagated(server, row);
    let groups = Group::ALL
        .into_iter()
        .filter(|group| group.differs(row, &read, recurring))
        .map(Part::Group);
    let mut pending: Vec<Part> = groups.collect();
    if !pending.is_empty() {
        pending.extend(Revision::ALL.map(Part::Revision));
    }
    let changed = pending.clone();
    let write = |part: Part, out: &mut Written| match part {
        Part::Group(group) => group.write(row, recurring, out),
        Part::Revision(revision) => {
            revision.write(server.sequence.saturating_add(1), now, out);
            Ok(())
        }
    };
    let mut out = Written::new();
    out.kept(&vevent.begin.physical);
    for item in &vevent.items {
        match item {
            Item::Line(line) => match Part::of(&line.name()) {
                Some(part) if changed.contains(&part) => {
                    if let Some(at) = pending.iter().position(|left| *left == part) {
                        pending.remove(at);
                        write(part, &mut out)?;
                    }
                }
                _ => out.kept(&line.physical),
            },
            Item::Block(_) => {
                for part in pending.drain(..) {
                    write(part, &mut out)?;
                }
                item.keep(&mut out);
            }
        }
    }
    for part in pending {
        write(part, &mut out)?;
    }
    out.kept(&vevent.end.physical);
    Ok(out)
}

/// What [`rewrite`] writes anew as one: a group of FreeCal's Properties
/// that changed, or a property saying the `VEVENT` was revised.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Group(Group),
    Revision(Revision),
}

impl Part {
    fn of(name: &str) -> Option<Part> {
        Group::of(name)
            .map(Part::Group)
            .or_else(|| Revision::of(name).map(Part::Revision))
    }
}

/// A content line as received: its physical lines without their line
/// ends, the first holding its name.
struct ContentLine<'t> {
    physical: Vec<&'t str>,
}

impl ContentLine<'_> {
    /// The line unfolded: each physical line after the first loses the
    /// space or tab that continues it.
    fn unfolded(&self) -> String {
        let mut text = self.physical[0].to_owned();
        for continued in &self.physical[1..] {
            text.push_str(&continued[1..]);
        }
        text
    }

    /// Its name, in upper case.
    fn name(&self) -> String {
        let first = self.physical[0];
        let end = first.find([';', ':']).unwrap_or(first.len());
        first[..end].trim().to_ascii_uppercase()
    }

    /// Its value: what follows the first colon outside a quoted parameter
    /// value.
    fn value(&self) -> Option<String> {
        let unfolded = self.unfolded();
        let mut quoted = false;
        for (at, c) in unfolded.char_indices() {
            match c {
                '"' => quoted = !quoted,
                ':' if !quoted => return Some(unfolded[at + 1..].to_owned()),
                _ => {}
            }
        }
        None
    }
}

/// A component as received: its `BEGIN` and `END` lines, its kind in upper
/// case, and its properties and components in order.
struct Block<'t> {
    begin: ContentLine<'t>,
    kind: String,
    items: Vec<Item<'t>>,
    end: ContentLine<'t>,
}

impl Block<'_> {
    /// Whether it has a `VTIMEZONE` with `zone`'s IANA name as its `TZID`.
    fn has_vtimezone(&self, zone: Tz) -> bool {
        self.items.iter().any(|item| match item {
            Item::Block(block) if block.kind == "VTIMEZONE" => block.items.iter().any(|item| {
                matches!(item, Item::Line(line) if line.name() == "TZID"
                    && line.value().is_some_and(|tzid| tzid.trim().trim_matches('"') == zone.name()))
            }),
            _ => false,
        })
    }
}

enum Item<'t> {
    Line(ContentLine<'t>),
    Block(Block<'t>),
}

impl<'t> Item<'t> {
    fn is_vevent(&self) -> bool {
        matches!(self, Item::Block(block) if block.kind == "VEVENT")
    }

    /// The items of a component, none for a line.
    fn items(&self) -> &[Item<'t>] {
        match self {
            Item::Block(block) => &block.items,
            Item::Line(_) => &[],
        }
    }

    /// Writes it as received.
    fn keep(&self, out: &mut Written) {
        match self {
            Item::Line(line) => out.kept(&line.physical),
            Item::Block(block) => {
                out.kept(&block.begin.physical);
                for item in &block.items {
                    item.keep(out);
                }
                out.kept(&block.end.physical);
            }
        }
    }
}

/// The components of iCalendar text as received; none when its lines
/// aren't components, such as a property outside one, an `END` that
/// doesn't match its `BEGIN`, a continued line after an empty one, which
/// readers unfold differently, or a control character other than a tab
/// inside a line, such as a carriage return.
fn components(text: &str) -> Option<Vec<Item<'_>>> {
    let mut lines: Vec<ContentLine> = Vec::new();
    let mut after_empty = false;
    for physical in text.split('\n') {
        let physical = physical.strip_suffix('\r').unwrap_or(physical);
        // A carriage return inside a line ends it for some readers, so a
        // line kept with one could carry a property in; RFC 5545 allows no
        // other control character but a tab either.
        if physical.chars().any(|c| c.is_ascii_control() && c != '\t') {
            return None;
        }
        if physical.is_empty() {
            after_empty = true;
        } else if physical.starts_with([' ', '\t']) {
            if after_empty {
                return None;
            }
            lines.last_mut()?.physical.push(physical);
        } else {
            lines.push(ContentLine {
                physical: vec![physical],
            });
            after_empty = false;
        }
    }
    let mut roots = Vec::new();
    let mut open: Vec<(ContentLine, String, Vec<Item>)> = Vec::new();
    for line in lines {
        match line.name().as_str() {
            "BEGIN" => {
                let kind = line.value()?.trim().to_ascii_uppercase();
                open.push((line, kind, Vec::new()));
            }
            "END" => {
                let (begin, kind, items) = open.pop()?;
                if line.value()?.trim().to_ascii_uppercase() != kind {
                    return None;
                }
                let block = Item::Block(Block {
                    begin,
                    kind,
                    items,
                    end: line,
                });
                match open.last_mut() {
                    Some((_, _, items)) => items.push(block),
                    None => roots.push(block),
                }
            }
            _ => open.last_mut()?.2.push(Item::Line(line)),
        }
    }
    open.is_empty().then_some(roots)
}

#[cfg(test)]
mod tests;
