# Coding standards

The rules a review checks a diff against. They are judgement calls; mechanical rules are enforced by the checks in `.husky/pre-commit` instead.

## Words

- The core's push messages are always called Signals, never "events" (calendar Events) or "notifications" (desktop notifications).
- Domain terms follow `GLOSSARY.md`.

## Frontend

- Every UI string goes through the i18n layer (`t` in `src/i18n`).
- Colours, spacing and controls that more than one component uses live once in `src/app.css` as tokens or classes (spec: "Visual style"). A component that copies another component's control styles is a finding.
- A side effect that relies on the DOM showing new `$state`, such as showing the window once the theme is applied, runs after `await tick()`. Its test checks what is visible at the moment of the effect, not only the order of the calls.

## Untrusted content

Every stored field is untrusted, whatever its origin (spec: "Untrusted content" in `.scratch/freecal-v1/spec.md`):

- `.ics`, XML and SQL are produced only through a library's writer or bound parameters.
- The CSP never gets looser for scripts or remote content.
- Links open in the system browser through the shell (`openLink`), never in the window.
- A field that gains a new way of being shown, such as links or markup pieces, gets a matching payload and `expected` entry in `fixtures/hostile-event.json`.

## Dependencies

- Every npm dependency is GPLv3-compatible (ADR 0003). `cargo deny check licenses` checks the Rust ones.
