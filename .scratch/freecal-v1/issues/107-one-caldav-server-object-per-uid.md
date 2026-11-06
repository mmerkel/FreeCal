# 107: One CalDAV Server Object per UID; duplicates are Ignored Resources

**What to build:** Agreed on 2026-11-06 (ADR 0015, which supersedes ADR 0011's handover; research in `docs/research/caldav-duplicate-uids.md`). Today a second CalDAV resource holding a UID that another already holds is mirrored without Event Rows, and takes over the UID's Event Rows from its mirrored text when the first one goes (`settle`, `core/src/sync/caldav/mod.rs`). Instead, it becomes an Ignored Resource (ticket 106), and the Local Store enforces one CalDAV Server Object per UID in a Calendar.

- **One per UID:** a unique index on `(calendar_id, uid)` in `caldav_server_object`, in the baseline schema. UIDs are compared exactly.
- **The kept resource keeps its UID for as long as it holds it.** A fetched resource holding a UID that a mirrored resource holds is an Ignored Resource with that UID in `caldav_ignored_resource`, and gets one Log line in ticket 102's format with the reason. A new resource never pushes out the kept one.
- **Fixed order among candidates of one Incoming Sync:** of several resources fetched in one Incoming Sync holding one UID no mirrored resource holds (a First Sync, or a takeover), the one holding the single Event or the Recurring Event is mirrored, then the lowest href by byte comparison; the others are Ignored Resources.
- **Takeover in the same Incoming Sync:** when the kept resource is removed, changes its UID, or comes to hold several UIDs or nothing FreeCal keeps, the remote layer fetches the Ignored Resources holding that UID in a second request, before the Journal Transaction. The first by the fixed order is mirrored and propagated to the UID's Event Rows; the others stay ignored with their new ETags. If none can be read, the UID's Event Rows are deleted. A kept resource that becomes unreadable keeps the UID.
- **What goes:** `settle`, the ranking in `Store::corresponding_server_object` (`core/src/store/sql/mod.rs`), the early return in `event::propagate_to_event_rows` for a Server Object that isn't the Corresponding one, `holds_non_exception` and its column, and the tests of the handover in `core/tests/caldav_sync.rs` and `core/src/sync/caldav/incoming.rs`. The moved-resource rule (`moved`) stays.
- **References:** comments and `CODING_STANDARDS.md` citing ADRs 0010, 0011 or 0013 cite ADR 0015 instead, and say nothing about the handover.

**Blocked by:** 106

**Status:** ready-for-agent

**Effort:** M

- [ ] Writing a second CalDAV Server Object with one UID in a Calendar fails in the Local Store
- [ ] A First Sync bringing two resources with one UID mirrors the one holding the Recurring Event, else the lower href, and ignores and logs the other
- [ ] A resource arriving later with a UID already held is ignored and logged, even if it ranks first by the fixed order; the Event Rows don't change
- [ ] The kept resource removed, with and without a sync token: the Ignored Resource takes over in the same Incoming Sync, and the Event stays shown with its content
- [ ] The kept resource's UID changed, and the kept resource coming to hold two UIDs: the same takeover for the old UID
- [ ] Of two Ignored Resources taking over, the fixed order decides; the other stays ignored
- [ ] No Ignored Resource readable on takeover: the UID's Event Rows are deleted
- [ ] A kept resource that becomes unreadable keeps the UID and its Event Rows
- [ ] A kept resource moved to another href keeps its Event Rows
- [ ] `settle`, `holds_non_exception` and the ranking are gone; no code chooses between two Server Objects with one UID outside the fixed order
