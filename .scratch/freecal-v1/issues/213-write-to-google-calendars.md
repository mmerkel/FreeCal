# 213: Write to Google Calendars

**What to build:** Writable Google Calendars get the same editing, Outgoing Syncs, Calendar History, Conflicts and Mass-Change Guard as CalDAV. The writing boxes of old ticket 26 (`git show design/snapshot-line:.scratch/freecal-v1/issues/26-write-to-google.md`); its login expiry and exports are 214's.

- **Google's writer** (`google/write.rs`) builds an event resource's JSON from an Event Version's managed fields and its Unimplemented Fields, the neutral entries and the `google:` ones: the counterpart of 210's reader, as 176's writer is of 175's. What it builds reads back as the Event Version it was built from.
- **An Outgoing Sync** sends what differs between the Current-Version Marks (192), through 201's Mass-Change Guard. Every update and delete is conditional on the stored ETag; the closing Database Transaction records the outcome as for CalDAV, and a created resource gets its `google_server_object` row from the answer.
- **The server's form:** Google answers a write with the resource as it stored it. If that differs from what was sent, 193's function of `history/` puts it in the sent Event Version's place. No read-back is needed.
- **Conflicts, Undo and the Mass-Change Guard** need no Google code; they are tested against the fake Google server all the same.
- **The interface the two Providers' Sync Engines share** is designed here, now that both read and write; record it in `redesign-layout.md`.
- **Writability:** 212's blanket read-only mark goes; a Calendar is marked when added only if Google doesn't let the user change it (200's rule), and removing the mark asks Google.
- **An Exception without its Recurring Event** (Google can't create one without the Recurring Event's resource), imported or moved into a Google Calendar: decided here, for example sent as a single Event; the choice goes into this ticket's text.

Spec: US-93; "Sync engine"; "Writability"; "Untrusted content". ADRs 0017, 0018 ("Who may write"), 0019 ("An Outgoing Sync"), 0020. Layout: "By phase", "P5 and P6".

**Blocked by:** 193, 201, 212

**Status:** ready-for-agent

**Effort:** M

- [ ] Conditional create, update and delete with ETags; what FreeCal doesn't show is written back from the Unimplemented Fields; writer and reader round-trip (unit tests)
- [ ] Google's answer to a write takes the sent Event Version's place through 193's function when it differs
- [ ] A Google Calendar is marked read-only only when Google refuses changes; removing the mark asks Google
- [ ] An Exception without its Recurring Event is handled as decided here
- [ ] Tests through the core interface against the fake Google server, test-first: edits, a refused conditional write, Undo, a Conflict and its restore, a Server Event Merge, the Mass-Change Guard's pause, changes made offline sent later; the Store Check after each
