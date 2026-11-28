//! The Unimplemented Fields of one Event Version (ADR 0017): a tree of
//! named entries, stored through serde in its canonical form.
//!
//! An entry's name starts with the prefix of the format that understands
//! it: `ical:` for everything read from iCalendar text ([`ICAL`]);
//! `caldav:` ([`CALDAV`]) is reserved and holds nothing yet. Neutral names
//! come with Google's reader (ticket 210). Each Provider holds the prefixes
//! [`prefixes`] names; a move or an import into a Calendar keeps only those
//! of its Provider.
//!
//! An entry holds an iCalendar property whole, the parameters FreeCal
//! doesn't know of a managed property without its value, or a component
//! whole, with its own properties and components. Values are kept as the
//! `icalendar` parser gives them: text values unescaped, others as written.
//!
//! **Canonical form.** Two trees are equal exactly when their serialised
//! forms ([`UnimplementedFields::canonical`]) are. Every tree is canonical
//! from its making ([`UnimplementedFields::new`], deserialising too):
//! - names of entries, properties, parameters and components are in upper
//!   case after the prefix, as iCalendar's names don't depend on case;
//! - a parameter's values are unquoted and kept in their order;
//! - order doesn't carry meaning between different names, so entries,
//!   properties, parameters and components are sorted by name;
//! - order does carry meaning among several of one name (two `VALARM`s, two
//!   `ATTENDEE`s), so those keep the order they were read in.
//!
//! Folding and line endings are gone before the tree is made: the reader
//! builds it from unfolded content lines.
//!
//! The tree is server data (spec: "Untrusted content"): it is never shown
//! and never crosses the core interface.

use serde::{Deserialize, Serialize};

use crate::Provider;

/// The prefix of what is read from iCalendar text.
pub(crate) const ICAL: &str = "ical:";

/// The prefix of what only CalDAV holds; assigned to nothing yet.
#[allow(dead_code, reason = "reserved by ADR 0017; nothing holds it yet")]
pub(crate) const CALDAV: &str = "caldav:";

/// The prefixes of the entries Calendars of `provider` hold.
#[allow(dead_code, reason = "until ticket 186: a move keeps these")]
pub(crate) fn prefixes(provider: Provider) -> &'static [&'static str] {
    match provider {
        Provider::Local => &[ICAL],
        Provider::CalDav => &[ICAL, CALDAV],
    }
}

/// The Unimplemented Fields of one Event Version, in canonical form.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "Vec<Entry>", into = "Vec<Entry>")]
pub(crate) struct UnimplementedFields {
    entries: Vec<Entry>,
}

/// One named entry of [`UnimplementedFields`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Entry {
    /// With its format's prefix, such as `ical:X-MOZ-GENERATION`.
    pub(crate) name: String,
    pub(crate) value: EntryValue,
}

/// What an [`Entry`] holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum EntryValue {
    /// A property whole: one FreeCal doesn't manage, or one of a managed
    /// property that the reader can't represent exactly.
    Property {
        parameters: Vec<Parameter>,
        value: String,
    },
    /// The parameters FreeCal doesn't know of a managed property, without
    /// its value, which the managed field holds.
    Parameters(Vec<Parameter>),
    /// A component whole, such as an alarm.
    Component {
        properties: Vec<Property>,
        components: Vec<Component>,
    },
}

/// An iCalendar property inside a component of the tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Property {
    pub(crate) name: String,
    pub(crate) parameters: Vec<Parameter>,
    pub(crate) value: String,
}

/// An iCalendar component inside a component of the tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Component {
    pub(crate) name: String,
    pub(crate) properties: Vec<Property>,
    pub(crate) components: Vec<Component>,
}

/// An iCalendar parameter, with its values unquoted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Parameter {
    pub(crate) name: String,
    pub(crate) values: Vec<String>,
}

impl UnimplementedFields {
    /// The tree of `entries`, in canonical form.
    pub(crate) fn new(entries: Vec<Entry>) -> Self {
        let mut entries: Vec<Entry> = entries.into_iter().map(Entry::canonical).collect();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Self { entries }
    }

    #[allow(dead_code, reason = "until ticket 176 writes them")]
    pub(crate) fn entries(&self) -> &[Entry] {
        &self.entries
    }

    #[allow(dead_code, reason = "until ticket 176 writes them")]
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Adds `entry`, after any of the same name.
    pub(crate) fn push(&mut self, entry: Entry) {
        let mut entries = std::mem::take(&mut self.entries);
        entries.push(entry);
        *self = Self::new(entries);
    }

    /// The serialised form, which the Local Store holds; equal for two
    /// trees exactly when they are equal.
    #[allow(dead_code, reason = "until ticket 177 compares by it")]
    pub(crate) fn canonical(&self) -> String {
        serde_json::to_string(self).expect("a tree of strings serialises")
    }
}

impl From<Vec<Entry>> for UnimplementedFields {
    fn from(entries: Vec<Entry>) -> Self {
        Self::new(entries)
    }
}

impl From<UnimplementedFields> for Vec<Entry> {
    fn from(fields: UnimplementedFields) -> Self {
        fields.entries
    }
}

impl Entry {
    fn canonical(self) -> Self {
        let name = match self.name.split_once(':') {
            Some((prefix, name)) => format!("{prefix}:{}", name.to_ascii_uppercase()),
            None => self.name.to_ascii_uppercase(),
        };
        let value = match self.value {
            EntryValue::Property { parameters, value } => EntryValue::Property {
                parameters: canonical_parameters(parameters),
                value,
            },
            EntryValue::Parameters(parameters) => {
                EntryValue::Parameters(canonical_parameters(parameters))
            }
            EntryValue::Component {
                properties,
                components,
            } => EntryValue::Component {
                properties: canonical_properties(properties),
                components: canonical_components(components),
            },
        };
        Self { name, value }
    }
}

fn canonical_parameters(parameters: Vec<Parameter>) -> Vec<Parameter> {
    sorted(parameters.into_iter().map(|parameter| {
        Parameter {
            name: parameter.name.to_ascii_uppercase(),
            values: parameter
                .values
                .into_iter()
                .map(|value| match value.strip_prefix('"') {
                    Some(quoted) => quoted.strip_suffix('"').unwrap_or(quoted).to_owned(),
                    None => value,
                })
                .collect(),
        }
    }))
}

fn canonical_properties(properties: Vec<Property>) -> Vec<Property> {
    sorted(properties.into_iter().map(|property| Property {
        name: property.name.to_ascii_uppercase(),
        parameters: canonical_parameters(property.parameters),
        value: property.value,
    }))
}

fn canonical_components(components: Vec<Component>) -> Vec<Component> {
    sorted(components.into_iter().map(|component| Component {
        name: component.name.to_ascii_uppercase(),
        properties: canonical_properties(component.properties),
        components: canonical_components(component.components),
    }))
}

/// Sorted by name; several of one name keep their order (a stable sort).
fn sorted<T: Named>(items: impl Iterator<Item = T>) -> Vec<T> {
    let mut items: Vec<T> = items.collect();
    items.sort_by(|a, b| a.name().cmp(b.name()));
    items
}

trait Named {
    fn name(&self) -> &str;
}

impl Named for Parameter {
    fn name(&self) -> &str {
        &self.name
    }
}

impl Named for Property {
    fn name(&self) -> &str {
        &self.name
    }
}

impl Named for Component {
    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn parameter(name: &str, values: &[&str]) -> Parameter {
        Parameter {
            name: name.to_owned(),
            values: values.iter().map(|value| (*value).to_owned()).collect(),
        }
    }

    pub(crate) fn property(name: &str, parameters: Vec<Parameter>, value: &str) -> Property {
        Property {
            name: name.to_owned(),
            parameters,
            value: value.to_owned(),
        }
    }

    fn alarm(properties: Vec<Property>) -> Component {
        Component {
            name: "valarm".to_owned(),
            properties,
            components: Vec::new(),
        }
    }

    fn entry(name: &str, value: EntryValue) -> Entry {
        Entry {
            name: name.to_owned(),
            value,
        }
    }

    fn x_property(name: &str, value: &str) -> Entry {
        entry(
            name,
            EntryValue::Property {
                parameters: vec![
                    parameter("x-b", &["\"one\"", "two"]),
                    parameter("X-A", &["a"]),
                ],
                value: value.to_owned(),
            },
        )
    }

    fn alarms(first: &str, second: &str) -> Entry {
        let action = |name: &str| property(name, Vec::new(), "DISPLAY");
        entry(
            "ical:VEVENT-ALARMS",
            EntryValue::Component {
                properties: vec![property("x-z", Vec::new(), "z"), action("Action")],
                components: vec![
                    alarm(vec![
                        property("TRIGGER", Vec::new(), first),
                        action("ACTION"),
                    ]),
                    alarm(vec![
                        action("action"),
                        property("trigger", Vec::new(), second),
                    ]),
                ],
            },
        )
    }

    #[test]
    fn order_between_names_and_letter_case_of_names_make_no_difference() {
        let one = UnimplementedFields::new(vec![
            x_property("ical:X-ONE", "1"),
            alarms("-PT5M", "-PT1H"),
            entry(
                "ical:summary",
                EntryValue::Parameters(vec![parameter("language", &["de"])]),
            ),
        ]);
        let mut other = UnimplementedFields::new(vec![entry(
            "ical:SUMMARY",
            EntryValue::Parameters(vec![parameter("LANGUAGE", &["\"de\""])]),
        )]);
        other.push(alarms("-PT5M", "-PT1H"));
        other.push(x_property("ical:x-one", "1"));

        assert_eq!(one, other);
        assert_eq!(one.canonical(), other.canonical());
    }

    #[test]
    fn several_of_one_name_keep_their_order() {
        let ab = UnimplementedFields::new(vec![
            x_property("ical:X-A", "a"),
            x_property("ical:X-A", "b"),
        ]);
        let ba = UnimplementedFields::new(vec![
            x_property("ical:X-A", "b"),
            x_property("ical:X-A", "a"),
        ]);
        assert_ne!(ab.canonical(), ba.canonical());
        assert_ne!(
            UnimplementedFields::new(vec![alarms("-PT5M", "-PT1H")]).canonical(),
            UnimplementedFields::new(vec![alarms("-PT1H", "-PT5M")]).canonical(),
        );
    }

    #[test]
    fn different_content_gives_different_forms() {
        let base = UnimplementedFields::new(vec![x_property("ical:X-A", "a")]);
        for other in [
            vec![x_property("ical:X-A", "A")],
            vec![x_property("ical:X-B", "a")],
            vec![x_property("ical:X-A", "a"), x_property("ical:X-A", "a")],
            vec![entry(
                "ical:X-A",
                EntryValue::Parameters(vec![parameter("X-A", &["a"])]),
            )],
            vec![],
        ] {
            assert_ne!(
                base.canonical(),
                UnimplementedFields::new(other).canonical()
            );
        }
    }

    #[test]
    fn the_stored_form_reads_back_canonical() {
        let fields = UnimplementedFields::new(vec![x_property("ical:X-A", "a"), alarms("a", "b")]);
        let stored = fields.canonical();
        let read: UnimplementedFields = serde_json::from_str(&stored).unwrap();
        assert_eq!(read, fields);

        let shuffled = r#"[{"name":"ical:x-b","value":{"property":{"parameters":[],"value":"b"}}},
            {"name":"ical:X-A","value":{"property":{"parameters":[],"value":"a"}}}]"#;
        let read: UnimplementedFields = serde_json::from_str(shuffled).unwrap();
        assert_eq!(read.entries()[0].name, "ical:X-A");
        assert_eq!(read.entries()[1].name, "ical:X-B");
    }

    #[test]
    fn each_provider_holds_ical() {
        assert_eq!(prefixes(Provider::Local), [ICAL]);
        assert_eq!(prefixes(Provider::CalDav), [ICAL, CALDAV]);
    }
}
