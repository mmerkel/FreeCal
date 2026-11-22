# Critiques of the Calendar History design (branch `explore/unknown-fields-and-change-journal`)

A review of 2026-11-21 compared the branch's ADRs 0017 to 0020 with the design they replace (ADRs 0009, 0014 and 0015, as on `main`). Each numbered point below is one critique together with the alternative proposed for it, to be grilled in a session of its own. Read "Shared context" first, then only your point.

How to start a session: "Grill me on point N of `.scratch/freecal-v1/critiques.md`." The outcome of a point is a decision. If the alternative is taken, the affected ADRs, the spec, `GLOSSARY.md` and the tickets listed under the point are reworded. If it is rejected, the reason goes into the ADR's "Considered Options".

## Shared context

**The two designs.**

- **On `main`** (ADRs 0009, 0014, 0015, read with `git show main:docs/adr/<file>`): Event Rows hold the current Events. The Change Journal stores, per Journal Event Modification, the previous Event Version and a hash of the new one. Unsent Changes are a stored list (`unsent_change`). The Mirror holds each Server Object's text as received, and the CalDAV writer patches FreeCal's fields into it. A Conflict is a row of `conflict` with copies of the Server Version and the Last Agreed Version (`conflict_event`), and it holds back only its UID. Undo goes per Event: hash check, later changes undone first, and a Dangerous Undo that asks first. Ctrl+Z undoes the user's own latest action. Write Permissions guard who may write what. About 4,000 lines of tested code implement this (`core/src/journal/`, `core/src/store/sql/`, `core/src/sync/caldav/`).
- **On the branch** (ADRs 0017 to 0020): one table of Event Versions that never change, Snapshots linked to the ones they arose from (the Calendar History), and per Calendar a User's Pointer, a Server's Pointer and a Reference Snapshot. What FreeCal shows, what it has to send, the Conflicts and undo are all read from that one structure. Unimplemented Fields live as a tree on every Event Version, and the Mirror's text goes. ETags say whether the server changed. A Calendar is Tracking while it has no Conflict and isn't Held during Undo. Undo moves the User's Pointer to the parent Snapshot, and Incoming Syncs are steps in the stack. The Store Check records the invariants as named queries that the tests run. Tickets 142 to 166 build it; nothing is built yet.

**What the review kept.** The core idea is better than `main`'s: one source of truth instead of five stores kept in step, invariants enforced by the Local Store (rows that never change, unique indexes, foreign keys) and the Store Check, and Unimplemented Fields that move, undo and compare with the Event. ADR 0010's objection to finding what to send by comparison (FreeCal normalises what it stores) doesn't apply, as both sides are FreeCal's own parsed form.

**What the review found overall.**

- "Fewer exceptions" holds only in part. These go: Write Permissions, Unguarded Writes, Sent Event Rows, Propagate's special cases, the Dangerous Undo thresholds and hashes. These arrive: the Reference Snapshot, Held during Undo, Join vs merge, rewriting links for the server's form of FreeCal's own write, the Current-Version Marks as derived state, the Mass-Change Guard reading ancestry, calendar-level entries on "the first Exception", and an exception in the 90-day retention rule. About half of ADR 0019's special cases stem from undo moving the User's Pointer (point 1).
- Cost: the branch replaces tested code with about 25 tickets. Worth it for the storage model; less clearly for the undo semantics.

**Files to read.** `docs/adr/0017-*.md` to `0020-*.md`, `GLOSSARY.md` (Calendar History, Snapshot, Snapshot Transition, Join, Reference Snapshot, Tracking, Held during Undo, Current-Version Mark, Store Check, Unimplemented Fields), `.scratch/freecal-v1/spec.md`, and the tickets named under each point. The prototype is `core/prototypes/calendar-history.PROTOTYPE.html`.

---

## 1. Ctrl+Z undoes syncs → Reverting Undo (decided: taken, as Undo)

**Critique.** ADR 0020: Ctrl+Z undoes the newest Snapshot of any Calendar, Incoming Syncs included. `main` promised that Ctrl+Z undoes the user's own latest action. On the branch, what Ctrl+Z reaches depends on when the last poll ran. A stray Ctrl+Z takes back a colleague's change and sends that at once. Until ticket 126 it can also move a meeting on a server that notifies the attendees (RFC 6638). Undoing one's own action from two hours ago first undoes, and sends, every Incoming Sync since. No other calendar app or collaborative editor works this way.

Moving the pointer back is also what forces:

- Held during Undo: its state, bar, Status Mark, "Continue" and ticket 165
- the rules for when the redo path ends and which Snapshots are dropped
- redo asking "which Calendar?" and "which way?" at a fork
- the Server's Pointer moving backwards after an undo is sent
- ADR 0019's invariant "the user's side behind the server means an undo"
- the question when a Snapshot also spans a read-only Calendar

**Alternative A: Reverting Undo** (proposed new term, not yet agreed): an undo recorded as a new Snapshot that takes back one of the user's own earlier Snapshots. ADR 0020 sketches it under "Left open" (taking back one earlier Snapshot, as a new Snapshot); A makes it the default for Ctrl+Z.

- The User's Pointer stays. It is still the Snapshot FreeCal shows, and it now only moves forward; so does the Server's Pointer. The redo path goes.
- New stored state: one field on a Snapshot, "takes back Snapshot X", part of its origin, written once and never changed. No pointer.
- **Ctrl+Z** takes back the newest Snapshot of the user's own, across all Calendars, that isn't itself an undo or redo and hasn't been taken back yet. Incoming Syncs and Joins are skipped.
- **Redo** takes back the newest Reverting Undo, offered only while the user hasn't made a change since. Syncs in between don't end it.
- **Across Calendars**, the time a Snapshot was made gives the order for both undo and redo, so redo never asks which Calendar.
- For each Event of the Snapshot taken back: if it is still as that Snapshot left it, it goes back to its version before; if it changed since (a later edit, a Follow or a Join), the whole undo is refused. It is refused whole, as `main`'s ADR 0009 argued for user actions: undoing half of a "this and following" split loses or doubles Occurrences.
- Taking back a sync stays possible from the Calendar History, as a Reverting Undo of a sync Snapshot, not through Ctrl+Z.
- The Store Check can verify the field with one query: a Reverting Undo takes back a Snapshot of the user's own, on its own line.

**With Incoming Syncs**, a Reverting Undo is an ordinary change by the user: sent like an edit, compared like an edit. Nothing waits.

| Scenario | Branch | Alternative A |
|---|---|---|
| User edits X, a colleague's change to Y comes in, Ctrl+Z | First Ctrl+Z takes back the colleague's Y and sends that; second takes back X | Takes back X; Y stays |
| User edits X, a colleague changes X, Ctrl+Z | Takes back the colleague's change, then X | Refused: "X changed on the server since" |
| Undo, then the server changes the same Event before it is sent | Conflict, and Held during Undo | Conflict, as for any edit |
| Undo, then the server changes another Event | Held until redo to the end, "Continue" or an edit | Comes in at once |
| Undo, the server changes the same Event, then Redo | Redo works, as the server's change waited | Redo refused |
| Undo of the user's action from two hours ago | Every Incoming Sync since is undone and sent first | Only that action, if its Events haven't changed since |
| Undo of the colleague's change itself | Ctrl+Z | From the Calendar History only |

**Problems A brings that the branch doesn't have.**

1. Undo can be refused. With no merging per field (spec), a change to an unrelated field of the same Event blocks it too. Never in a Local Calendar; now and then in a busy shared one.
2. Ctrl+Z can get stuck: a refused newest Snapshot is refused again on every press, and older actions are out of reach. A fix ("Skip", recorded as a Reverting Undo that takes back nothing and marks the Snapshot as given up) is a second kind of record. **Strongest con of A.**
3. Redo isn't guaranteed; a sync between undo and redo can refuse it. In practice the window is seconds.
4. A refusal needs a message of its own, such as "Couldn't undo 'Move X': X changed on the server since", with a button to open X. The branch instead needs the bar and Status Mark of Held during Undo.
5. "The user's own Snapshot" needs a definition for imports, restores and merges (ending a Conflict); for a merge, it is taken back against which parent?

**The same on both.** An undo not yet sent races the server like an edit and can become a Conflict. Undoing an import already sent meets the Mass-Change Guard, which can treat it as approved by reading the Snapshot's origin. Undoing a move into a read-only Calendar needs a rule.

**Reviewer's view.** A gives up guaranteed undo and redo in return for the server's changes never waiting and Ctrl+Z never taking back someone else's change. Right for a synced calendar: a refusal is visible and harmless; a Ctrl+Z that quietly overwrites a colleague's change on the server is neither. Test with the prototype against the scenarios above before deciding.

**Questions to grill.** Stop and say so, or skip, when Ctrl+Z is refused? Is a "Skip" record acceptable? Which origins count as the user's own (imports, restores, merges)? Is a Reverting Undo itself undone with Ctrl+Z, or only redone? Does the Calendar History offer taking back a sync, and with what confirmation? What becomes of ADR 0020's other "Left open" note (swapping Snapshots), which A makes unnecessary? Does the Reference Snapshot stay (it should: Joins still keep the server's Snapshot out of the user's ancestry)?

**Affected if taken.** ADRs 0019 (Held during Undo, the invariant, the Server's Pointer moving backwards, Outgoing Sync) and 0020 (largely rewritten); spec sections on undo and Held during Undo; `GLOSSARY.md` (Held during Undo goes, Reverting Undo added); tickets 147, 148, 158, 165, and those naming Held during Undo (grep for it).

**Outcome (grilling of 2026-11-21): A taken, as "Undo"** (no new term). Played through in the prototype, walkthroughs 1 to 12.

- An Undo is a Snapshot whose origin names the one it takes back. Both pointers only move forwards. The redo path, Held during Undo (ticket 165 `wontfix`), the Server's Pointer moving backwards and ADR 0019's "behind means undone" go.
- Ctrl+Z takes back the newest of the user's own Snapshots in effect, across all Calendars: actions, imports, restores, merges and "Stop waiting". It passes over the First Sync, Incoming Syncs, Joins, Undos and Snapshots that changed nothing on the user's side, such as a merge where the user kept their own version.
- Refused whole when anything the Snapshot changed has changed since, checked per Occurrence by ADR 0019's comparison (now ticket 167, before ticket 147). The toast names the later change, the server's or the user's own, and offers "Open X" and "Undo both changes". There is no "Skip": a stuck Ctrl+Z is unstuck by "Undo both changes", and older changes can be undone from the Calendar History.
- Redo reverses the newest Undo, however it was made, until the user's next own change; syncs don't end it. When refused, it offers "Redo over the server's change".
- The Calendar History is one list of all Calendars, with filters. Any Snapshot can be undone from it, whole, syncs included. No undo asks first, which also settles ADR 0019's Join sentence. Undo for one Calendar and "Go back to before this" go.
- The Mass-Change Guard counts every Undo, an import's too. "Stop waiting and mark read-only" is a Snapshot with an origin of its own, not a merge. A Snapshot spanning a Calendar marked read-only is refused without a button.
- The swap note in ADR 0020's "Left open" moved to "Considered Options". The Reference Snapshot stays, with a Join as the reason the last common Snapshot can't serve.
- Undoing a merge takes back only the Events that were in Conflict, as a merge also takes in the server's other changes and Ctrl+Z shouldn't take back a colleague's change. Decided for now; to discuss again in ticket 134, with Ctrl+Z passing over merges as the alternative.

Reworded: ADRs 0018, 0019 and 0020 (now `0020-undo-is-a-snapshot.md`), the spec (US-146, US-150, new US-155, "Calendar History", Tracking, the tests, the main menu), `GLOSSARY.md` (Undo added; Undo History, Snapshot, Join, Tracking and Reference Snapshot reworded; Held during Undo removed; Dangerous Undo stays until ticket 147 removes it from the code), and tickets 134, 138, 147, 148, 151, 152, 153, 158, 159, 161, 162, 165, 166 and the new 167.

---

## 2. One Conflict freezes the whole Calendar → end Conflicts automatically (decided: taken; the Calendar History becomes one line)

**Critique.** ADR 0019 ("Conflicts"): while a Calendar has a Conflict, nothing is sent and nothing from the server is taken in. `main` (ADR 0014) held back only the UID in Conflict. On the branch, one disputed Event in a busy shared Calendar stops all incoming and outgoing changes until the user acts; ignoring the bar for a day can mean missing a moved meeting. ADR 0019 rejected the per-Event option ("A Conflict held per Event, with the rest of the Calendar syncing on") because each Event would need a mark and "the pointers no longer describe a whole Calendar": a modelling reason, not one the user would accept. Its own Consequences admit it: "While a Conflict is open, the whole Calendar is stale in both directions."

**Alternative B.** With a history that never changes, nothing is lost by ending a Conflict at once. The comparison of the two sides that finds a Conflict makes the merge Snapshot right away, with the server's version winning, tells the user (toast, Event's History, Problems), and offers "Restore my version". That requires reversing ADR 0020's "An Event's History shows versions but offers no way to restore one". Sync never stops, and the Conflict state, its bar, dialog and Status Mark go.

- **Strongest con:** the user's edit disappears from the grid until they react, which breaks the spec's principle that FreeCal isn't guessing when both sides changed.
- **Fallback, if that principle is firm:** hold only the Events in Conflict and Join the rest. The Reference Snapshot then can't move for the whole Calendar, so it needs a per-Event form (for example, the Last Agreed Version kept per Event in Conflict), which is part of what the branch set out to remove.

**Questions to grill.** Is "FreeCal isn't guessing" firm for Conflicts? Server wins or user wins by default? How insistent is the notice, and how long is "Restore my version" offered? Does restoring make a new Snapshot (fits point 1's Reverting Undo)? How does B interact with the Mass-Change Guard and with Events with guests? If the fallback: what replaces the Reference Snapshot per Event, and does that bring back a Conflict table?

**Affected if taken.** ADR 0019 ("Conflicts", "Tracking and the Reference Snapshot"), ADR 0020 (restoring from an Event's History), spec Conflict sections, `GLOSSARY.md` (Tracking, Conflict, Server Version, User's Version), tickets 21, 22, 133 to 138, 162.

**Outcome (grilling of 2026-11-22): B taken, server wins, and the Calendar History becomes one line.** The design before is kept by the tags `design/snapshot-tree` (this branch at `e7b3e45`) and `design/change-journal` (`main` at `4857548`); the reworked ADRs cite them.

- **Conflicts end at once, the server's version winning.** An Incoming Sync makes one **Server Intake** (new term) with the server's changes that have no Conflict, and one **Server Event Replacement** (new term) per Event in Conflict, in which that Event takes the Server Version; its origin names the Event Version the server held before the fetch, the Last Agreed Version. Nothing is sent for it. Edit against remote delete follows the same rule. A Server Intake or Server Event Replacement that would change nothing isn't made. User wins was rejected: it overwrites a colleague's change on the server on a guess.
- **The user can't miss it.** The Conflict stays until the user resolves it: the Conflict badge on the chip, the Status Mark, the tray state and a bar that can't be dismissed ("2 changes in “Work” were replaced by the server's: Review"); no dialog; one desktop notification per Calendar with the window closed, a toast with it open. The texts say "replaced by the server's", not "isn't syncing". Rejected: a notice alone (missed while in the background), a ghost chip of the User's Version (two chips per Event, a whole series doubled), a grace period, asking when the window is focused, and the per-Event hold (its per-Event state would drive sync, where the review state drives only the display).
- **Resolution** (reworded): keeping the server's version, restoring one's own (an Undo of the Server Event Replacement), or changing the Event again. Each writes a row of `resolution`, written once; open Conflicts are the Server Event Replacements without one. So a redo of a restore leaves the Conflict resolved. A missed write leaves a Conflict showing, never hides one; the Store Check checks that every Undo of a Server Event Replacement has its row. Changing one's mind after a restore is redo, or an Undo of the restore from the Calendar History.
- **Ctrl+Z** passes over Server Intakes (a First Sync is one, from nothing), Server Event Replacements and Undos. A refusal because of a Server Event Replacement offers "Restore my version" in the toast.
- **One line for all Calendars.** With Undo a Snapshot and Conflicts ended at once, the server's line had no job left: the Reference Snapshot was always the Server's Pointer's Snapshot. There is one line of Snapshots, each arising from the one before, of all Calendars together. The Server's Pointer, server Snapshots, merges, Joins, the Reference Snapshot, Tracking and the User's Pointer (always the newest Snapshot) go. The server's Current-Version Mark becomes the stored record of what the server holds; the base of the three-way comparison is the server's marks before the fetch. A partly sent Outgoing Sync moves the marks of what was sent. Strongest con: nothing cross-checks the server's marks any more.
- **Storage.** No Snapshot Transitions and no link table: an Event Version carries `begin` and `end` (Snapshots). `end IS NULL` is the user's current version, enforced by one partial unique index per Calendar, UID and recurrence ID, so the user's Current-Version Mark is no longer stored. Undo and redo write copies (rows compare by content anyway). Rejected: a table of stretches with rows reused (the unique index can't span two tables). `begin` is nullable with `ON DELETE SET NULL` (began before the oldest kept Snapshot); `end` has `ON DELETE RESTRICT`. Dropping after 90 days deletes Event Versions ended more than 90 days ago with no server's mark and not named by a Server Event Replacement, then the Snapshots older than 90 days that no kept `end` names. The trigger allows `end` from NULL to a Snapshot, `begin` to NULL, and the server's mark. The server's form of FreeCal's own write is still put in place of the sent version (as a new row with its `begin` and `end`); the separate Snapshot was rejected again, now because every write to a normalising server would make Undo refuse ("changed on the server since").
- **Records.** ADRs 0018 and 0019 rewritten in place, ADR 0020 adjusted; the prototype is reworked to the line model as a step of its own after the documents.

Terms: Server Intake, Server Event Replacement added; Conflict, Resolution, Snapshot, Calendar History, Current-Version Mark, Last Agreed Version reworded; Join, User's Pointer, Server's Pointer, Reference Snapshot, Tracking, Snapshot Transition and merge removed.

---

## 3. Losing the server's text risks losing data → keep the raw text as a write guard

**Critique.** ADR 0017: the Mirror's text and the patching writer go; a writer builds each Server Object from Event Versions alone. On `main`, a property the reader didn't understand survived every write. On the branch, anything the reader fails to carry into the tree of Unimplemented Fields is lost at the next write of that Event (ADR 0017, Consequences). The only mitigation is "the reader's tests must cover every kind of property, parameter and component". Servers produce more variety than tests do, and the loss is silent and reaches other clients.

**Alternative C.** Keep the server's raw text on the server's side of an Event Version (or beside the CalDAV table's row), never compared and never part of an Event Version's equality. When reading, check that writing the tree back loses nothing against that text (a round trip: reader, then writer, then compare in a normalised form). If something would be lost, treat that Server Object like an unreadable one: its Events are shown, but FreeCal doesn't write it until an update of the reader handles it, and the Log says why.

- **Strongest con:** a second format comes back into the Local Store, which ADR 0017 tried to keep out, even if only as a check. It also costs storage and a round trip per fetched resource.
- A lighter variant: run the round-trip check without storing the text, and only refuse writes (mark the Server Object) when it fails. Nothing extra stored; the check happens only at fetch time.

**Questions to grill.** Store the text, or only the check's outcome? What counts as "nothing lost" (property order, folding, `DTSTAMP`)? Should an Event whose Server Object fails the check be editable, with the edit waiting? What does the user see? Does the same apply to Google's JSON later?

**Affected if taken.** ADR 0017 (Consequences, Considered Options), ADR 0019 ("Server Objects"), tickets 143, 144, 149; the Store Check if a mark is stored.

---

## 4. The First Sync locks the window → staging table

**Critique.** ADR 0018 ("The pointers"): until its First Sync has finished, nothing of a Calendar is stored; the First Sync runs in a modal dialog that locks the window, and holds what it fetched in memory. A large Calendar on a slow server blocks the whole app for as long as it loads; the only way out is Cancel; an interruption (network, FreeCal closing) loses everything fetched and doesn't resume. A loading step shouldn't lock the window.

**Alternative D.** Store the First Sync's pages in a staging table outside the Calendar History, keyed by the Calendar being added. Nothing else reads it. The Calendar is created, with its first Snapshot, only when the staging is complete, in one Database Transaction, as now. The dialog no longer has to be modal, and an interrupted First Sync resumes from the staging table.

- **Strongest con:** one more table with a lifecycle of its own (when stale staging is dropped, what if the account goes, a resume after a server change meanwhile), and the window needs some way to show a Calendar that is loading but not stored, which ADR 0018 rejected as "lines in the sidebar that aren't Calendars".
- Smaller step without the table: keep the in-memory First Sync but make the dialog non-modal. ADR 0018 rejected that because FreeCal has no such kind of dialog yet.

**Questions to grill.** How long do First Syncs actually take on the target servers (Google, Nextcloud with years of events)? Is resuming worth a table? Where does a loading Calendar show? What happens to staged data when the user cancels or the account is removed?

**Affected if taken.** ADR 0018 ("The pointers", Considered Options, Consequences), spec section on adding Available Calendars, tickets 155, 156, 164.

---

## 5. The Store Check runs only in tests → run it in use too

**Critique.** ADR 0018 ("Where the invariants are recorded"): the tests run the Store Check after each scenario. On the branch, what to send is derived (ADR 0019, Consequences: "Any fault that changes one side's Event Versions becomes writes to the server"). A broken invariant in a user's Local Store, from a bug the tests missed, becomes writes to the server, with only the Mass-Change Guard in between.

**Alternative E.** Also run the Store Check, or a cheap subset, at startup and in debug builds, and before an Outgoing Sync for the Calendar it sends. On a violation: write the Log, pause that Calendar's Outgoing Syncs with a Sync Error, and tell the user.

- **Strongest con:** startup time on large Local Stores, and a new question of what FreeCal does when the check fails (a paused Calendar the user can't fix themselves).

**Questions to grill.** Which queries are cheap enough for every start or every Outgoing Sync? Pause only Outgoing Syncs, or the Calendar? What can the user do about it: report, restore a Backup? Is this a Sync Error, a Problem, or both?

**Affected if taken.** ADR 0018 ("Where the invariants are recorded"), ADR 0019 (Sync Errors), spec sections on Sync Errors and Problems, `GLOSSARY.md` (Sync Error), a new ticket.
