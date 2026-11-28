# 216: The Store Check reads every Event Version; the writer reads its own text back

**What to build:** Checks of `CODING_STANDARDS.md`'s "Conformance": the Local Store holds only values its readers read, and what the CalDAV writer writes conforms to RFC 5545 and RFC 4791, except for the content of properties and components FreeCal doesn't manage. Today nothing checks either; this ticket makes every test through the core interface check both, and ends the breaches of a second copy of a property RFC 5545 allows once and of `METHOD` in a CalDAV resource (`.scratch/freecal-v1/conformance.md`). Runs after 179, whose Store Check it extends, and before 180.

- **The Store Check reads every Event Version** (`store/check.rs`, beside 179's queries, in Rust where SQL can't read a value): its times and zones, its rule (one FreeCal expands, or none), its excluded dates and its recurrence ID read as FreeCal stores them, and its managed text holds no control character but a tab and line ends. Each violation names the row and the rule it breaks, never the value. The Harness and debug builds run it as 179 runs the rest; 186 adds the Unimplemented Fields.
- **Control characters from an edit:** the core leaves them out of managed text an edit through the core interface brings, as the reader does for a text it reads, so the check above holds for every origin.
- **The writer reads its own text back:** in tests and debug builds, `write_resource` (`ical/write/mod.rs`) reads what it wrote with 175's reader and panics on any RFC Violation. So every test that writes a resource, 187's Sender and ticket 09's export included, checks the text it produced.
- **No second copy of a once-only property:** the reader leaves out a second copy of a property RFC 5545 allows once in a `VEVENT` (a second `DESCRIPTION`, `RRULE`, `UID`, `DTSTART`…), reports it as the RFC Violation `Repeated` as today, and lists the rule in `DROPPED`; it no longer goes into the Unimplemented Fields. The writer writes no such second copy from a tree either (the Local Store is untrusted) and returns each as Left Out. ADR 0017's "What the reader can't represent exactly" follows: its examples of a second `DESCRIPTION` and a second `RRULE` go, and a value form FreeCal doesn't model stays.
- **No `METHOD`** (decided 2026-11-28 with the user): the reader leaves out a `VCALENDAR`'s `METHOD`, which says how a text was sent, not what the Event is (RFC 4791 §4.1 forbids it in a CalDAV resource), and lists the rule in `DROPPED`; the writer writes none from a tree and returns it as Left Out.
- **Untrusted content:** the check's report and the panic name rows, rules and RFC Violations, never stored values.

Spec: "Local Store"; "Untrusted content". ADRs 0017 ("What the reader can't represent exactly"), 0018 ("Where the invariants are recorded"). Layout: "Tables (end state)"; "Seams for `/tdd`", 1 and 2; "By phase", P2, 216.

**Blocked by:** 179

**Status:** ready-for-agent

**Effort:** M

- [ ] The Store Check reports an Event Version whose time, zone, rule, excluded dates or recurrence ID doesn't read, or whose managed text holds a control character; a test per kind breaks it by hand and sees it reported
- [ ] An edit through the core interface with a control character in the title, Location and description stores it without (test through the core interface)
- [ ] `write_resource` reads its text back in tests and debug builds and panics on an RFC Violation; a test feeds it a tree that would break the RFC and sees the panic or the Left Out
- [ ] A second `DESCRIPTION`, `RRULE`, `UID` and `DTSTART` in a text read: left out, reported as `Repeated`, listed in `DROPPED`; the Server Texts' check of 175 still passes
- [ ] A tree holding a second copy of a once-only property is written without it, and the writer returns it as Left Out
- [ ] `METHOD` in a text read: left out and listed in `DROPPED`; one in a tree isn't written and is returned as Left Out; the Server Text with `METHOD:PUBLISH` still passes 175's check
- [ ] ADR 0017's "What the reader can't represent exactly" follows, and its "What is in the tree" names `METHOD` among what isn't kept; `.scratch/freecal-v1/conformance.md` strikes items 1 and 2 and the part of item 3 this ticket ends
- [ ] Every test through the core interface passes with the checks on

## Comments

- 2026-11-28: From the conformance discussion with the user (not a review), `.scratch/freecal-v1/conformance.md`. The user agreed: the store and the writer conform, the one exception being the content of properties and components FreeCal doesn't manage; a second copy of a once-only property isn't written. The user also agreed that the reader leaves it out, so the Local Store doesn't hold it; strongest con: a broken text's second `DESCRIPTION` is lost at the read already, and a change to it alone on the server is no longer seen. And that the reader leaves out `METHOD`; strongest con: the server whose text carries `METHOD:PUBLISH` gets it removed on FreeCal's next write, and an `.ics` round trip is no longer exact.
