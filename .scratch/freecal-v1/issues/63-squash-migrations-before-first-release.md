# 63: Squash schema migrations into one baseline before the first release

**What to build:** Replace the migrations in `MIGRATIONS` (`core/src/store.rs`) with one migration that creates the current schema directly, so the first release ships one clean baseline instead of development history. After that release, migrations are fixed (ADR 0008). Do this as the last schema change before the first release is tagged; a schema change added later goes into the baseline, not after it.

**Blocked by:** none

**Status:** ready-for-agent

- [ ] Before squashing, a test confirms the schema of a new Local Store matches the one migrated through every current step (compare `sqlite_master`), so the baseline is known to equal what's being replaced
- [ ] `MIGRATIONS` holds one `Migration::Sql` creating the current tables and indexes; a new Local Store ends at `user_version = 1` with the Local Account and starting Calendars as before
- [ ] `event_times_as_wall_time` is removed with the step-by-step migrations and their tests (`core/tests/event_times.rs` builds a version 5 Local Store); `recompute_instants` stays if time zone rule updates still need it (ADR 0007)
- [ ] A Local Store with a higher `user_version` (a development one from before the squash) is still refused with `NewerLocalStore`
- [ ] Development Local Stores are deleted before running the squashed build
