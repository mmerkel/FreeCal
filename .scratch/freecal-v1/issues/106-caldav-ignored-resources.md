# 106: CalDAV remembers Ignored Resources

**What to build:** Agreed on 2026-11-06 (ADR 0015; `GLOSSARY.md`: Ignored Resource). Since ticket 102, a CalDAV resource holding several UIDs isn't mirrored. Without a record of it, a server without sync tokens has it fetched and logged on every sync, because the listing compares only with the Mirror. FreeCal now remembers each such resource as an Ignored Resource, with its ETag, and fetches it again only when it changes.

- A new table `caldav_ignored_resource`: the Calendar, the href, the ETag, and the UID, which is empty here. Ticket 107 fills it for a resource holding a UID that another resource already holds. Unique on `(calendar_id, href)`, deleted with its Calendar. Change the baseline schema (ADR 0008); there are no real Local Stores yet.
- Only the Sync Engine writes it, under its Write Permission, from `core/src/store/sql/mirror/caldav.rs`. It is not part of the Mirror: no query of the Mirror reads it.
- The listing (`core/src/caldav.rs`, compared with what FreeCal holds near line 295) treats an Ignored Resource whose ETag is unchanged as known: it isn't fetched or logged again. One whose ETag changed is fetched and read again: if it can now be mirrored, it leaves the table and is mirrored; if it is still ignored, its ETag is updated and one Log line is written. With a sync token, the server reports it as changed or removed like any resource.
- A removed href that is an Ignored Resource leaves the table. An href is never both mirrored and ignored; the code that moves a resource between the two does both in one Database Transaction.

**Blocked by:** 102

**Status:** done

**Effort:** M

- [x] An Incoming Sync without a sync token fetches a resource holding two UIDs once and writes one Log line; the next sync fetches it no more and logs nothing
- [x] Changed on the server, it is fetched again and logged again; once it holds one UID, it is mirrored and its Events show, and it leaves `caldav_ignored_resource`
- [x] Removed from the server, it leaves `caldav_ignored_resource`
- [x] With a sync token, the same, from the server's reported changes
- [x] Deleting the Calendar deletes its Ignored Resources
- [x] `caldav_ignored_resource` is written only by functions taking the Sync Engine's Write Permission

## Comments

2026-11-06, implemented:

- `caldav_ignored_resource` is in the baseline schema; its `uid` is NULL until ticket 107. Three triggers make the Local Store refuse an href both mirrored and ignored, beyond the ticket's "one Database Transaction": an invariant the Local Store enforces, at the cost of three near-identical triggers.
- The Incoming Sync's known set is the Server Objects plus the Ignored Resources (`Core::sync_calendar`, `core/src/lib.rs`), so the listing skips an unchanged Ignored Resource and reports a gone one as removed.
- Beyond the ticket: an Ignored Resource that comes to hold nothing FreeCal keeps (only a VTODO, say) is forgotten; one FreeCal can't read keeps its old ETag, so it is fetched again, as an unreadable Server Object is.
- Radicale refuses to serve a resource holding two UIDs, so the end-to-end tests (`ignored_resources` in `core/tests/caldav_sync.rs`) reach Radicale through a proxy that rewrites one resource into two UIDs and counts its fetches.
- Left open: a server that gives no ETag has every resource fetched on each sync, Ignored Resources included, as before.
- `GLOSSARY.md`'s "Write Permission" now names the Ignored Resources under the Sync Engine's, as ADR 0015 does.

New terms:

- **Known Server Resources** (approved, renamed from "known set"): the Server Objects FreeCal holds and the Ignored Resources it remembers, by address, which the listing compares with.
- **Forget an Ignored Resource** (approved): remove it from `caldav_ignored_resource`, as gone, mirrored or holding nothing FreeCal keeps (`forget_ignored`, `forget_ignored_resource`); remembering one is `ignore`, `ignore_resource`.
- **Rewriting proxy** (new term, tests only): `RewritingProxy`, a stand-in server in front of Radicale that rewrites its answers.
