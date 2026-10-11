## Agent skills

### Issue tracker

Issues and specs live as local markdown files under `.scratch/<feature>/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Default vocabulary: needs-triage, needs-info, ready-for-agent, ready-for-human, wontfix. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.

## Code conventions

- Review rules live in `CODING_STANDARDS.md`. Treat every stored field as untrusted while writing code: see "Untrusted content" in `.scratch/freecal-v1/spec.md`.
- The TypeScript types of the core interface are generated from the Rust ones into `src/core/types.generated.ts`, with serialized samples in `src/core/samples.generated.ts`; never edit those files. IDs are branded in TypeScript; tests make them with `src/test/ids.ts`. After changing a Rust interface type, run `FREECAL_WRITE_TYPES=1 cargo test -p freecal-core`; plain `cargo test` fails while the file is out of date.
- Checks: `npm run format:check`, `npm run lint`, `cargo test`, `cargo clippy --all-targets`, `npm run check`, `npm test`. The pre-commit hook (`.husky/pre-commit`) formats staged files and runs the rest when code is staged; `npm run format` fixes formatting.
