# 102: CalDAV ignores resources holding several UIDs, and writes them to the Log

**What to build:** Agreed on 2026-11-06 (amends ADR 0013; `GLOSSARY.md`: Log). A CalDAV resource holding several UIDs breaks RFC 4791 §4.1 and most servers refuse to store it. FreeCal no longer mirrors it: it is moved to the edge of the app, where the CalDAV reading step treats it as holding nothing FreeCal keeps and notes it in the Log.

- A CalDAV-only reading step, next to the file reader in `core/src/import/mod.rs` (not in it: one `.ics` file to import legitimately holds many UIDs), turns each fetched resource into one of three: it can't be read, it holds nothing FreeCal keeps, or it holds one UID with its Events. A resource holding several UIDs is "holds nothing FreeCal keeps". The Sync Engine never sees several UIDs.
- A resource FreeCal doesn't mirror yet that holds several UIDs isn't mirrored. A mirrored one that comes to hold several goes as one that now holds nothing goes: its Mirror row is removed and its UID's Event Rows go to the UID's next Corresponding Server Object, or are deleted.
- `of_its_uid` (`core/src/sync/caldav/mod.rs`) and Propagate's "a Server Object holding several UIDs gives no Events" case go. Handing a UID's Event Rows to its next Corresponding Server Object (`settle`) reads that one Server Object through the new step and matches on the three outcomes, instead of wrapping it in a list and filtering.
- The Log: the `log` crate in the core, `tauri-plugin-log` writing a file in the operating system's log directory. Each ignored resource writes one line: the Calendar's ID, the resource's address and its UIDs, each escaped with `{:?}` (untrusted: a newline can't make a second line), each UID cut to 200 characters, at most the first 5 UIDs listed with a count of the others.
- ADR 0013 is amended: such resources are ignored and logged, not mirrored and reported. Its second paragraph and its consequence "Events in a CalDAV resource holding several UIDs aren't shown" stay true.

**Blocked by:** none

**Status:** done

**Effort:** M

- [x] A First Sync and an Incoming Sync bringing a resource holding two UIDs mirror nothing for it and write no Event Rows; another resource with one of its UIDs is that UID's Corresponding Server Object
- [x] A mirrored Server Object that comes to hold two UIDs is removed from the Mirror; its UID's Event Rows go to its next Corresponding Server Object, or are deleted
- [x] Each ignored resource writes one Log line with the Calendar's ID, its escaped address and at most 5 escaped UIDs of at most 200 characters, plus a count of the others
- [x] The file import still reads files holding many UIDs
- [x] `of_its_uid` is gone; `settle` doesn't build a list to read one Server Object
- [x] The Log is written to a file in the operating system's log directory

## Comments

- 2026-11-06: ADR 0015 consolidates ADRs 0010, 0011 and 0013 and supersedes them; cite it instead. Ticket 106 remembers the resources this ticket ignores as Ignored Resources, so that a server without sync tokens doesn't have them fetched and logged on every sync; this ticket needn't. Ticket 107 replaces `settle` with a takeover by Ignored Resources, so keep the change to `settle` here small.
- 2026-11-06: Done. The reading step is `read_caldav_resource` (`core/src/import/mod.rs`), returning `Holds`: `Unreadable`, `Nothing { uids }` or `OneUid { uid, events }`. The CalDAV Incoming Sync (`mirror_and_propagate`, `core/src/sync/caldav/incoming.rs`) writes the Log line only for `Nothing` with UIDs (a resource with no Events, such as only a VTODO, is not logged). The Log is the core's `log` lines at warning and above (other crates' lines are filtered out), in `FreeCal.log` in Tauri's app log directory, restarted at 1 MB with the old file gone. I checked that the file is created by starting the app with a scratch `XDG_DATA_HOME`. The test for a resource holding two UIDs reads the Log through a thread-local test logger. New terms, for approval or renaming:
  - **Holds** (new term): what a fetched CalDAV resource holds of what FreeCal keeps: it can't be read, holds nothing FreeCal keeps, or holds one UID with its Events.
  - **read_caldav_resource(s)** (new term): the CalDAV-only reading step that turns a fetched resource into its Holds; it replaces `read_server_objects`.
  - **log_ignored / ignored_line**, **LOGGED_UIDS / LOGGED_UID_LENGTH** (new terms): write and format the Log line of a CalDAV resource holding several UIDs, and its limits of 5 UIDs and 200 characters each.
  - Left from the review: `Fetched` (`core/src/remote.rs`), shared by every Provider, now carries the CalDAV-only `Holds`; reconsider with Google's Remote (ticket 26). `Holds::Nothing` covers two cases (no Events, several UIDs), as the ticket asks for three outcomes; ticket 106, which remembers only the several-UID kind, may want a variant of its own. A test in `incoming.rs` still cites ADR 0011 for the handover that ticket 107 replaces.
