# 107: One CalDAV Server Object per UID; duplicates are Ignored Resources

**What to build:** Agreed on 2026-11-06 (ADR 0015, which supersedes ADR 0011's handover; research in `docs/research/caldav-duplicate-uids.md`). Today a second CalDAV resource holding a UID that another already holds is mirrored without Event Rows, and takes over the UID's Event Rows from its mirrored text when the first one goes (`settle`, `core/src/sync/caldav/mod.rs`). Instead, it becomes an Ignored Resource (ticket 106), and the Local Store enforces one CalDAV Server Object per UID in a Calendar.

- **One per UID:** a unique index on `(calendar_id, uid)` in `caldav_server_object`, in the baseline schema. UIDs are compared exactly.
- **The kept resource keeps its UID for as long as it holds it.** A fetched resource holding a UID that a mirrored resource holds is an Ignored Resource with that UID in `caldav_ignored_resource`, and gets one Log line in ticket 102's format with the reason. A new resource never pushes out the kept one.
- **Fixed order among candidates of one Incoming Sync:** of several resources fetched in one Incoming Sync holding one UID no mirrored resource holds (a First Sync, or a takeover), the one holding the single Event or the Recurring Event is mirrored, then the lowest href by byte comparison; the others are Ignored Resources.
- **Takeover in the same Incoming Sync:** when the kept resource is removed, changes its UID, or comes to hold several UIDs or nothing FreeCal keeps, the remote layer fetches the Ignored Resources holding that UID in a second request, before the Journal Transaction. The first by the fixed order is mirrored and propagated to the UID's Event Rows; the others stay ignored with their new ETags. If none can be read, the UID's Event Rows are deleted. A kept resource that becomes unreadable keeps the UID.
- **What goes:** `settle`, the ranking in `Store::corresponding_server_object` (`core/src/store/sql/mod.rs`), the early return in `event::propagate_to_event_rows` for a Server Object that isn't the Corresponding one, `holds_non_exception` and its column, and the tests of the handover in `core/tests/caldav_sync.rs` and `core/src/sync/caldav/incoming.rs`. The moved-resource rule (`moved`) stays.
- **References:** comments and `CODING_STANDARDS.md` citing ADRs 0010, 0011 or 0013 cite ADR 0015 instead, and say nothing about the handover.

**Blocked by:** 106

**Status:** done

**Effort:** M

- [x] Writing a second CalDAV Server Object with one UID in a Calendar fails in the Local Store
- [x] A First Sync bringing two resources with one UID mirrors the one holding the Recurring Event, else the lower href, and ignores and logs the other
- [x] A resource arriving later with a UID already held is ignored and logged, even if it ranks first by the fixed order; the Event Rows don't change
- [x] The kept resource removed, with and without a sync token: the Ignored Resource takes over in the same Incoming Sync, and the Event stays shown with its content
- [x] The kept resource's UID changed, and the kept resource coming to hold two UIDs: the same takeover for the old UID
- [x] Of two Ignored Resources taking over, the fixed order decides; the other stays ignored
- [x] No Ignored Resource readable on takeover: the UID's Event Rows are deleted
- [x] A kept resource that becomes unreadable keeps the UID and its Event Rows
- [x] A kept resource moved to another href keeps its Event Rows
- [x] `settle`, `holds_non_exception` and the ranking are gone; no code chooses between two Server Objects with one UID outside the fixed order

## Comments

2026-11-06, implemented:

- `caldav_server_object` has a unique index on `(calendar_id, uid)` and no `holds_non_exception`; `caldav_ignored_resource.uid` holds the UID kept by another. Baseline schema changed.
- The Incoming Sync (`mirror_and_propagate`, `core/src/sync/caldav/incoming.rs`) first removes every Server Object that stops keeping its UID, so that two resources swapping UIDs in one sync never hit the unique index, then decides per UID (`keep`). A UID left without a keeper loses its Event Rows (`delete_if_unkept`, in place of `settle`).
- The remote layer's second request is `takeover` in `core/src/caldav.rs`, called from `Client::changes` after the first multiget.
- The First Sync is paged, one Journal Transaction per page, so a resource on a later page would otherwise count as "arriving later". During a First Sync the kept resource therefore competes by the fixed order with the candidates (they all belong to one Incoming Sync). If it loses, it becomes an Ignored Resource.
- The moved rule: when the kept resource is removed and a new href (not an Ignored Resource) holds its UID in the same sync, the first such href by the fixed order is the kept resource moved. It wins over Ignored Resources even when one of those ranks first. The ticket doesn't say which rule wins here; this reading keeps "the kept resource keeps its UID".
- `propagate_to_event_rows` keeps a `debug_assert` in place of the early return.
- Beyond the ticket: the test proxy from ticket 106 (`RewritingProxy`) reused pooled connections that Radicale had closed. Its thread panicked, and the two `ignored_resources` tests failed about half the time on the previous commit too. It now opens a fresh connection per request.
- Left open:
  - An Ignored Resource fetched again for a takeover that the server no longer has is not forgotten. On a server with sync tokens it stays remembered until the server reports it removed.
  - A First Sync resumed days later still lets the kept resource compete, as it is the same First Sync.

New terms:

- **Keep a UID** (new term): of the resources holding a UID in a Calendar, be the one CalDAV Server Object holding it, its Corresponding Server Object (`kept_uid`, `kept_by`, `Ignored::UidKept`, `delete_if_unkept`).
- **Take over** (new term): an Ignored Resource becoming the one keeping a UID once the kept one stops holding it (`takeover`).
- **Fixed order** (new term): the order among resources of one Incoming Sync holding one UID, the one holding the single Event or Recurring Event first, then the lowest href (`fixed_order`).
- **Candidate** (new term): a fetched resource holding one UID, competing to keep it (`Candidate`); the decision is a `Keeper`.
- 2026-11-07 (grilling for ticket 108, ADR 0015's amendment): the moved rule above changes. A CalDAV Server Object is its Calendar and UID, so a moved resource is a takeover and competes with the Ignored Resources by the fixed order. The takeover's second request goes: Ignored Resources whose UID a changed or removed Server Object keeps are fetched in the first request. See ticket 108.
