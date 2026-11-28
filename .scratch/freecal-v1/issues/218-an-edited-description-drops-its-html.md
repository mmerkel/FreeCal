# 218: An edited description drops its HTML

**What to build:** Ticket 122's rule that an edited description drops `X-ALT-DESC`, so Outlook and Thunderbird don't keep showing the old HTML in its place, lives only in the patching writer (`Group::write` and `Group::of`, `core/src/sync/caldav/resource/write.rs`), which 187 deletes. From 186 on, the Unimplemented Fields keep an `ical:X-ALT-DESC` entry like any property FreeCal doesn't otherwise handle, and the new writer (176) writes it back whole, so after 187 a description edited in FreeCal goes to the server beside the stale HTML. Found in the review of 177, which groups `ical:X-ALT-DESC` with the description in the Event Comparison (`entry_group`, `core/src/comparison/mod.rs`).

- **Where:** wherever the user's edit of an Event's description makes its new Event Version (186's path through the `SnapshotWriter`): when the description changes, the new version's Unimplemented Fields lose their `ical:X-ALT-DESC` entries. One function, used by every edit that can change the description (a single Event, each scope of a Recurring Event); an Undo restores a whole version and needs nothing.
- **Not here:** a description that came in as HTML (`description_html`) keeps its own rule; an Incoming Sync never drops anything.

Spec: "Untrusted content"; "Sync engine". ADRs 0017 ("Part of the version"). Layout: "By phase", P2, 186 and 187.

**Blocked by:** 186, 187

**Status:** needs-triage

**Effort:** S

- [ ] Editing the description of an Event with an `X-ALT-DESC` sends a resource without it; editing its title keeps it (tests through the core interface)
- [ ] Each scope of a Recurring Event's edit follows the same rule, with a test
- [ ] 177's Event Comparison combines one side's edited description (without its HTML) with an alarm the other side added, already tested in `the_html_description_goes_with_the_description`; a test through the edit shows the edit gives that shape

## Comments

- 2026-11-28: From the review of 177. Strongest con: an `X-ALT-DESC` a client keeps in sync with the description on purpose is lost on FreeCal's edit, where a smarter rule would rewrite it from the plain text; ticket 122 chose dropping for the same reason.
