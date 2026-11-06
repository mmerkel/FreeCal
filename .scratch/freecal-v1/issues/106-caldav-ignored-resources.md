# 106: CalDAV remembers Ignored Resources

**What to build:** Agreed on 2026-11-06 (ADR 0015; `GLOSSARY.md`: Ignored Resource). Since ticket 102, a CalDAV resource holding several UIDs isn't mirrored. Without a record of it, a server without sync tokens has it fetched and logged on every sync, because the listing compares only with the Mirror. FreeCal now remembers each such resource as an Ignored Resource, with its ETag, and fetches it again only when it changes.

- A new table `caldav_ignored_resource`: the Calendar, the href, the ETag, and the UID, which is empty here. Ticket 107 fills it for a resource holding a UID that another resource already holds. Unique on `(calendar_id, href)`, deleted with its Calendar. Change the baseline schema (ADR 0008); there are no real Local Stores yet.
- Only the Sync Engine writes it, under its Write Permission, from `core/src/store/sql/mirror/caldav.rs`. It is not part of the Mirror: no query of the Mirror reads it.
- The listing (`core/src/caldav.rs`, compared with what FreeCal holds near line 295) treats an Ignored Resource whose ETag is unchanged as known: it isn't fetched or logged again. One whose ETag changed is fetched and read again: if it can now be mirrored, it leaves the table and is mirrored; if it is still ignored, its ETag is updated and one Log line is written. With a sync token, the server reports it as changed or removed like any resource.
- A removed href that is an Ignored Resource leaves the table. An href is never both mirrored and ignored; the code that moves a resource between the two does both in one Database Transaction.

**Blocked by:** 102

**Status:** ready-for-agent

**Effort:** M

- [ ] An Incoming Sync without a sync token fetches a resource holding two UIDs once and writes one Log line; the next sync fetches it no more and logs nothing
- [ ] Changed on the server, it is fetched again and logged again; once it holds one UID, it is mirrored and its Events show, and it leaves `caldav_ignored_resource`
- [ ] Removed from the server, it leaves `caldav_ignored_resource`
- [ ] With a sync token, the same, from the server's reported changes
- [ ] Deleting the Calendar deletes its Ignored Resources
- [ ] `caldav_ignored_resource` is written only by functions taking the Sync Engine's Write Permission
