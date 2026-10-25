## Agent skills

### Issue tracker

Issues and specs live as local markdown files under `.scratch/<feature>/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Default vocabulary: needs-triage, needs-info, ready-for-agent, ready-for-human, wontfix. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.

## Code conventions

- In a design trade-off, choose long-term consistency over short-term ease, since agents write all the code: one rule for every case, invariants the compiler or the Local Store enforces. Name the trade-off when choosing.
- Review rules live in `CODING_STANDARDS.md`. Treat every stored field as untrusted while writing code: see "Untrusted content" in `.scratch/freecal-v1/spec.md`.
- The TypeScript types of the core interface are generated from the Rust ones into `src/core/types.generated.ts`, with serialized samples in `src/core/samples.generated.ts`; never edit those files. IDs are branded in TypeScript; tests make them with `src/test/ids.ts`. After changing a Rust interface type, run `FREECAL_WRITE_TYPES=1 cargo test -p freecal-core`; plain `cargo test` fails while the file is out of date.
- Checks: the pre-commit hook (`.husky/pre-commit`) lists and runs them; `npm run format` fixes formatting.
