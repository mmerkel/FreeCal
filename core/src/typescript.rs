//! The TypeScript copy of the core interface's types, generated with ts-rs so
//! that the frontend's types can't drift from these. Next to the types goes
//! one serialized sample of each type and variant, which `npm run check`
//! checks against the types, so that serde's output and the declared types
//! can't disagree either. `cargo test` fails while the generated files are out
//! of date; run it with `FREECAL_WRITE_TYPES=1` to write them.

use std::any::TypeId;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use chrono::NaiveDate;
use serde::Serialize;
use ts_rs::{Config, TS, TypeVisitor};

use crate::{
    Account, AccountId, Calendar, CalendarId, Colour, Description, Event, EventDraft, EventId,
    Occurrence, Piece, Provider, Signal, View, When,
};

const TYPES: &str = "../src/core/types.generated.ts";
const SAMPLES: &str = "../src/core/samples.generated.ts";

const HEADER: &str = "// Generated from the Rust core by core/src/typescript.rs. Don't edit;\n\
                      // run `FREECAL_WRITE_TYPES=1 cargo test -p freecal-core` instead.\n\n";

/// Declares each visited type and every type it uses, once, by name. The
/// derived `visit_dependencies` visits in an order that changes between
/// builds, so the names give the order.
struct Declarations<'a> {
    config: &'a Config,
    seen: HashSet<TypeId>,
    declarations: BTreeMap<String, String>,
}

impl TypeVisitor for Declarations<'_> {
    fn visit<T: TS + 'static + ?Sized>(&mut self) {
        // Types without an output path are TypeScript's own, like `string`.
        if T::output_path().is_none() || !self.seen.insert(TypeId::of::<T>()) {
            return;
        }
        let docs = T::docs().unwrap_or_default();
        self.declarations.insert(
            T::ident(self.config),
            format!("{docs}export {}\n", T::decl(self.config)),
        );
        T::visit_dependencies(self);
    }
}

/// The interface's TypeScript declarations, and the names they declare.
fn types() -> (String, Vec<String>) {
    let config = Config::new().with_large_int("number");
    let mut types = Declarations {
        config: &config,
        seen: HashSet::new(),
        declarations: BTreeMap::new(),
    };
    // The types the commands and Signals take and return.
    types.visit::<Account>();
    types.visit::<Calendar>();
    types.visit::<Occurrence>();
    types.visit::<Event>();
    types.visit::<EventDraft>();
    types.visit::<Signal>();
    types.visit::<View>();
    let names = types.declarations.keys().cloned().collect();
    let declarations: Vec<_> = types.declarations.into_values().collect();
    (generated(&declarations.join("\n")), names)
}

/// The samples of one type: its name, `name: [..]` and its line in the
/// `Samples` type.
fn samples<T: TS + Serialize>(values: &[T]) -> (String, String, String) {
    let name = T::ident(&Config::new());
    let json = serde_json::to_string_pretty(values).expect("serialize the samples");
    (
        name.clone(),
        format!("  {name}: {},\n", json.replace('\n', "\n  ")),
        format!("  {name}: Unbranded<T.{name}>[];\n"),
    )
}

/// The samples file, and the names of the types it has samples of.
fn samples_file() -> (String, Vec<String>) {
    let date = |day| NaiveDate::from_ymd_opt(2026, 10, day).unwrap();
    let all_day = When::AllDay {
        start: date(5),
        end: date(6),
    };
    let timed = When::Timed {
        start: date(5).and_hms_opt(9, 30, 0).unwrap(),
        end: date(5).and_hms_opt(10, 0, 0).unwrap(),
    };
    let text = Piece::Text {
        text: "Agenda: ".to_owned(),
    };
    let link = Piece::Link {
        text: "notes".to_owned(),
        url: "https://example.org/notes".to_owned(),
    };
    let description = Description {
        paragraphs: vec![vec![text.clone(), link.clone()]],
        text: "Agenda: notes (https://example.org/notes)".to_owned(),
    };
    let calendar = Calendar {
        id: CalendarId(2),
        account_id: AccountId(1),
        name: "Home".to_owned(),
        colour: Colour::parse("#3a86ff").unwrap(),
        shown: true,
        read_only: false,
    };
    let entries = [
        samples(&[AccountId(1)]),
        samples(&[Provider::Local]),
        samples(&[Account {
            id: AccountId(1),
            provider: Provider::Local,
        }]),
        samples(&[CalendarId(2)]),
        samples(std::slice::from_ref(&calendar.colour)),
        samples(&[calendar]),
        samples(&[EventId(3)]),
        samples(&[all_day, timed]),
        samples(&[EventDraft {
            calendar_id: CalendarId(2),
            title: "Dentist".to_owned(),
            when: timed,
            location: "Main Street 1".to_owned(),
            description: description.text.clone(),
        }]),
        samples(&[Event {
            id: EventId(3),
            calendar_id: CalendarId(2),
            title: "Dentist".to_owned(),
            when: timed,
            location: "Main Street 1".to_owned(),
            description: description.clone(),
        }]),
        samples(&[Occurrence {
            event_id: EventId(3),
            calendar_id: CalendarId(2),
            title: "Holiday".to_owned(),
            when: all_day,
        }]),
        samples(&[description]),
        samples(&[text, link]),
        samples(&[
            Signal::CalendarsChanged,
            Signal::OccurrencesChanged {
                calendar_ids: vec![CalendarId(2)],
            },
            Signal::DateChanged { date: date(4) },
        ]),
        samples(&[View::Day, View::Week, View::Month]),
    ];
    let mut names = Vec::new();
    let (mut values, mut types) = (String::new(), String::new());
    for (name, value, r#type) in entries {
        names.push(name);
        values.push_str(&value);
        types.push_str(&r#type);
    }
    let file = generated(&format!(
        "import type * as T from './types.generated';\n\
         import type {{ Unbranded }} from './types';\n\n\
         /** One serialized sample of each interface type and variant. */\n\
         export const samples = {{\n{values}}} satisfies Samples;\n\n\
         type Samples = {{\n{types}}};\n"
    ));
    (file, names)
}

fn generated(body: &str) -> String {
    format!("{HEADER}{body}")
        .lines()
        .map(|line| format!("{}\n", line.trim_end()))
        .collect()
}

/// Checks the committed file against `expected`, first writing it when asked.
fn check(relative: &str, expected: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    if std::env::var_os("FREECAL_WRITE_TYPES").is_some() {
        std::fs::write(&path, expected).expect("write the generated file");
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        committed == expected,
        "{relative} is out of date; run `FREECAL_WRITE_TYPES=1 cargo test -p freecal-core`"
    );
}

#[test]
fn the_typescript_types_are_up_to_date() {
    check(TYPES, &types().0);
}

#[test]
fn the_typescript_samples_are_up_to_date() {
    let (file, mut sampled) = samples_file();
    let (_, mut declared) = types();
    sampled.sort();
    declared.sort();
    assert_eq!(sampled, declared, "every interface type needs samples");
    check(SAMPLES, &file);
}
