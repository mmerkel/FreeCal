//! iCalendar text: the reader and the writer of one CalDAV resource
//! (ADR 0017).

pub(crate) mod read;
#[cfg(test)]
pub(crate) mod server_texts;
mod windows_zones;
pub(crate) mod write;

use chrono::NaiveDateTime;

use crate::unimplemented::{Component, EntryValue, Parameter, UnimplementedFields};

/// Whether `name` can name a property, parameter or component: an
/// iCalendar name of letters, digits and dashes.
pub(crate) fn is_name(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// Each `TZID` a property of `tree` gives, nested ones too, unquoted, with
/// the wall times of that property's value FreeCal can read.
pub(crate) fn tree_tzids(tree: &UnimplementedFields) -> Vec<(String, Vec<NaiveDateTime>)> {
    let mut properties: Vec<(&[Parameter], &str)> = Vec::new();
    let mut components: Vec<&Component> = Vec::new();
    for entry in tree.entries() {
        match &entry.value {
            EntryValue::Property { parameters, value } => properties.push((parameters, value)),
            EntryValue::Parameters(_) => {}
            EntryValue::Component {
                properties: p,
                components: c,
            } => {
                properties.extend(
                    p.iter()
                        .map(|p| (p.parameters.as_slice(), p.value.as_str())),
                );
                components.extend(c);
            }
        }
    }
    while let Some(component) = components.pop() {
        let p = component.properties.iter();
        properties.extend(p.map(|p| (p.parameters.as_slice(), p.value.as_str())));
        components.extend(&component.components);
    }
    let mut tzids = Vec::new();
    for (parameters, value) in properties {
        let times: Vec<NaiveDateTime> = value
            .split(',')
            .filter_map(|time| NaiveDateTime::parse_from_str(time.trim(), "%Y%m%dT%H%M%S").ok())
            .collect();
        let named = parameters
            .iter()
            .filter(|p| p.name.eq_ignore_ascii_case("TZID"));
        for tzid in named.flat_map(|p| &p.values) {
            tzids.push((read::unquoted(tzid).to_owned(), times.clone()));
        }
    }
    tzids
}
