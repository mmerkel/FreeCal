## Agent skills

### Issue tracker

Issues and specs live as local markdown files under `.scratch/<feature>/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Default vocabulary: needs-triage, needs-info, ready-for-agent, ready-for-human, wontfix. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.

## Talking to the user

- The user knows the domain, the ADRs, the specs and the tickets, but not the code in detail. When you mention code, first say what it does in `GLOSSARY.md` terms, then give the identifier or `file:line` as a pointer, e.g. "the step that renormalises the Combined Event Keys when an Exception joins its Recurring Event (`JournalTransaction::rejoin_combined_event_keys`, `core/src/journal/mod.rs`)", not just "`rejoin_combined_event_keys`". Don't assume the user remembers a file's structure or an earlier diff.
- Internal vocabulary needs agreement, as user-facing vocabulary does. Terms for the code and the ADRs live in the "Internals" section of `GLOSSARY.md`. Before using a term that is in neither section, propose it marked as new, e.g. "New term: **Send Batch**, the Unsent Changes sent in one request. OK?", and add it to Internals once the user agrees. Name types, modules and tables after agreed terms. When the user isn't in the loop (`/implement`), use the term marked "(new term)" and list it in the report; `/code-review` lists every new term for the user to approve or rename.

## Code conventions

- In a design trade-off, choose long-term consistency over short-term ease, since agents write all the code: one rule for every case, invariants the compiler or the Local Store enforces. Name the trade-off when choosing.
- Review rules live in `CODING_STANDARDS.md`. Treat every stored field as untrusted while writing code: see "Untrusted content" in `.scratch/freecal-v1/spec.md`.
- The TypeScript types of the core interface are generated from the Rust ones into `src/core/types.generated.ts`, with serialized samples in `src/core/samples.generated.ts`; never edit those files. IDs are branded in TypeScript; tests make them with `src/test/ids.ts`. After changing a Rust interface type, run `FREECAL_WRITE_TYPES=1 cargo test -p freecal-core`; plain `cargo test` fails while the file is out of date.
- Checks: the pre-commit hook (`.husky/pre-commit`) lists and runs them; `npm run format` fixes formatting.
