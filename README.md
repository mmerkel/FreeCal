# FreeCal

An offline-first desktop calendar for Linux. FreeCal shows Local, CalDAV and Google Calendars in one Google-Calendar-like view, keeps a local copy of every Calendar so it works without a network, and syncs both ways when it can. When a change made in FreeCal and a change on the server can't both be kept, it asks instead of guessing, and every change it sends to a server can be undone.

## Status

Early development. FreeCal launches and shows an empty month view, but it can't hold or sync Events yet. It is not ready for use.

## Building and running

You need:

- Rust (stable, 1.92 or newer)
- Node.js (a current LTS release) and npm
- The system packages Tauri needs on Linux, such as WebKitGTK. See Tauri's [prerequisites](https://v2.tauri.app/start/prerequisites/).
- OpenSSL's development files (`libssl-dev` on Debian and Ubuntu, `openssl-devel` on Fedora), for connections to CalDAV servers
- For the tests, the [Radicale](https://radicale.org) CalDAV server, installed for `python3` (`pip install --user radicale`). The tests start it themselves; `FREECAL_RADICALE_PYTHON` names another Python to run it with.

Then:

```sh
npm install
npx tauri dev      # run with live reload
npx tauri build    # build a release
```

Checks:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all --check
npm run lint
npm run check
npm test
```

## How it is built

A Rust core holds all domain logic and the Local Store (one SQLite database). A thin Tauri shell exposes it to a Svelte 5 frontend, which draws the calendar with FullCalendar.

- [`.scratch/freecal-v1/spec.md`](.scratch/freecal-v1/spec.md): what v1 does, and the decisions behind it
- [`.scratch/freecal-v1/issues/`](.scratch/freecal-v1/issues/): the tickets
- [`docs/adr/`](docs/adr/): architecture decisions
- [`GLOSSARY.md`](GLOSSARY.md): the project's terms (Account, Calendar, Event, Occurrence, ...)

Most of the code is written with [Claude Code](https://claude.com/claude-code), an AI coding assistant, working from the spec and tickets above. A human writes the spec, makes the design decisions and reviews every change. The agent's workflow (specs and tickets, code review, TDD, domain modelling) comes from Matt Pocock's [skills](https://github.com/mattpocock/skills). Commits it helped with carry a `Co-Authored-By: Claude` trailer.

## Licence

Copyright © 2026 Matthias Merkel

FreeCal is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. See [`LICENSE`](LICENSE).

The agent skills in [`.claude/skills/`](.claude/skills/) are Matt Pocock's, except `run-freecal`, and are under the MIT License; see [`.claude/skills/LICENSE-mattpocock-skills`](.claude/skills/LICENSE-mattpocock-skills).
