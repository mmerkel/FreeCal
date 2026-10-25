# 63: Squash schema migrations into one baseline before the first release

**What to build:** Replace the migrations in `MIGRATIONS` (`core/src/store.rs`) with one migration that creates the current schema directly, so the first release ships one clean baseline instead of development history. After that release, migrations are fixed (ADR 0008). Do this as the last schema change before the first release is tagged; a schema change added later goes into the baseline, not after it.

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** S

**Closing only on request:** unlike other tickets, an implementer ticks this ticket's boxes and sets it to done only when the user explicitly asks for it, not upon finishing the work. Until then the ticket stays open with every box unticked, even when the work is done, because schema changes may still come before the first release.

- [ ] Before squashing, a test confirms the schema of a new Local Store matches the one migrated through every current step (compare `sqlite_master`), so the baseline is known to equal what's being replaced
- [ ] `MIGRATIONS` holds one `Migration::Sql` creating the current tables and indexes; a new Local Store ends at `user_version = 1` with the Local Account and starting Calendars as before
- [ ] `event_times_as_wall_time` is removed with the step-by-step migrations and their tests (`core/tests/event_times.rs` builds a version 5 Local Store); `recompute_instants` stays if time zone rule updates still need it (ADR 0007)
- [ ] A Local Store with a higher `user_version` (a development one from before the squash) is still refused with `NewerLocalStore`
- [ ] Development Local Stores are deleted before running the squashed build

## Comments

**2026-10-25:** Already squashed: `MIGRATIONS` holds one baseline, a new Local Store ends at `user_version = 1`, and the step-by-step migrations are gone with their tests, `event_times_as_wall_time`, `StoredWhen::from_utc_instants` and the Change Journal's re-hashing. `HASH_FORMAT` starts again at 1, since no Local Store holds hashes of the older formats. Before the old migrations were removed, a test compared a new Local Store's schema with the baseline's (`sqlite_master` SQL without whitespace, each table's columns, foreign keys and indexes, and the starting rows) and passed; it went with them, as it needed them. `recompute_instants` stays. A development Local Store from before (`user_version` 19) is refused with `NewerLocalStore`, so development Local Stores have to be deleted before running this build. Boxes stay unticked and the ticket open per the closing rule above: a schema change made before the first release goes into the baseline.
