//! The TypeScript copy of the core interface's types, generated with ts-rs so
//! that the frontend's types can't drift from these. `cargo test` fails while
//! the generated file is out of date; run it with `FREECAL_WRITE_TYPES=1` to
//! write the file.

use std::path::Path;

use ts_rs::{Config, TS};

use crate::{
    Account, AccountId, Calendar, CalendarId, Colour, Description, Event, EventDraft, EventId,
    Occurrence, Piece, Provider, Signal, When,
};

const GENERATED: &str = "../src/core/types.generated.ts";

fn declaration<T: TS>(config: &Config) -> String {
    let docs = T::docs().unwrap_or_default();
    format!("{docs}export {}\n", T::decl(config))
}

fn generated() -> String {
    let config = Config::new().with_large_int("number");
    let declarations = [
        declaration::<AccountId>(&config),
        declaration::<Provider>(&config),
        declaration::<Account>(&config),
        declaration::<CalendarId>(&config),
        declaration::<Colour>(&config),
        declaration::<Calendar>(&config),
        declaration::<EventId>(&config),
        declaration::<When>(&config),
        declaration::<EventDraft>(&config),
        declaration::<Event>(&config),
        declaration::<Occurrence>(&config),
        declaration::<Description>(&config),
        declaration::<Piece>(&config),
        declaration::<Signal>(&config),
    ];
    let file = format!(
        "// Generated from the Rust core by core/src/typescript.rs. Don't edit;\n\
         // run `FREECAL_WRITE_TYPES=1 cargo test -p freecal-core` instead.\n\n{}",
        declarations.join("\n")
    );
    file.lines()
        .map(|line| format!("{}\n", line.trim_end()))
        .collect()
}

#[test]
fn the_typescript_types_are_up_to_date() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(GENERATED);
    let expected = generated();
    if std::env::var_os("FREECAL_WRITE_TYPES").is_some() {
        std::fs::write(&path, &expected).expect("write the TypeScript types");
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        committed == expected,
        "{GENERATED} is out of date; run `FREECAL_WRITE_TYPES=1 cargo test -p freecal-core`"
    );
}
