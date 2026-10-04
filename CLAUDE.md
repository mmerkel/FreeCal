## Agent skills

### Issue tracker

Issues and specs live as local markdown files under `.scratch/<feature>/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Default vocabulary: needs-triage, needs-info, ready-for-agent, ready-for-human, wontfix. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.

## Code conventions

- Svelte code uses Svelte 5 runes only (`$state`, `$derived`, `$effect`, `$props`). No Svelte 4 syntax: no `export let`, `$:`, stores for component state, `on:` directives or slots.
- The core's push messages are always called Signals, never "events" (calendar Events) or "notifications" (desktop notifications).
- Every UI string goes through the i18n layer (`t` in `src/i18n`).
- The TypeScript types of the core interface are generated from the Rust ones into `src/core/types.generated.ts`; never edit that file. After changing a Rust interface type, run `FREECAL_WRITE_TYPES=1 cargo test -p freecal-core`; plain `cargo test` fails while the file is out of date.
- Every dependency must be GPLv3-compatible (ADR 0003).
- Treat every stored field as untrusted: no `{@html}` or `innerHTML`, no `.ics`, XML or SQL built from strings, and never loosen the CSP for scripts or remote content. See "Untrusted content" in `.scratch/freecal-v1/spec.md`.
- Checks: `cargo test`, `cargo clippy --all-targets`, `npm run check`, `npm test`.
