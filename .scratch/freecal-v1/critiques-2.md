# Critiques of the Calendar History design, second review (branch `explore/unknown-fields-and-change-journal`)

A second review of 2026-11-22 compared the documents (`GLOSSARY.md`, ADRs, spec, tickets) of three versions: `design/change-journal` (`main` at `4857548`, ADRs 0009, 0014, 0015), `design/snapshot-tree` (`e7b3e45`, ADRs 0017 to 0020 as a tree with two pointers) and the branch at `2f24150` (ADRs 0017 to 0020 as one line of Snapshots with the server's marks). It follows the first review, `.scratch/freecal-v1/critiques.md`, and uses the same form: each numbered point is one critique together with the alternative proposed for it, to be grilled in a session of its own. Read "Shared context" first, then only your point.

**Status (2026-11-26):** point 1 decided with point 5 of `critiques.md` (outcome recorded there); points 2 and 3 decided together (outcome under point 2); point 4 decided (outcome there; its Last Agreed Version reversed by point 7); point 7, raised by the user on 2026-11-26, decided (outcome there); point 5 decided on 2026-11-28 (outcome there); point 6 done on 2026-11-28 (outcome there); the scorecard is updated for point 3 of `critiques.md` and points 2 and 3 here. Two findings of this review belong to points of the first review and were added there instead of here: the round-trip check on reading (to point 3 of `critiques.md`) and running checks before an Outgoing Sync (to point 5). Point 1 below should be grilled together with point 5 of `critiques.md`.

How to start a session: "Grill me on point N of `.scratch/freecal-v1/critiques-2.md`." The outcome of a point is a decision. If the alternative is taken, the affected ADRs, the spec, `GLOSSARY.md` and the tickets listed under the point are reworded. If it is rejected, the reason goes into the ADR's "Considered Options".

## Shared context

**The three versions.**

- **A, `design/change-journal`**: Event Rows, the Change Journal with previous Event Versions and hashes, a stored list of Unsent Changes, the Mirror's text patched by the CalDAV writer, a Conflict table holding back only its UID, undo per Event with the Dangerous Undo, Write Permissions. Built and tested.
- **B, `design/snapshot-tree`**: a tree of Snapshots per Calendar, a User's Pointer, a Server's Pointer, a Reference Snapshot, Joins and merges; a Conflict stops the whole Calendar syncing until the user merges. Undo already a Snapshot of its own.
- **C, the branch now**: one line of Snapshots across all Calendars; Event Versions with `begin` and `end`; the server's Current-Version Mark as the record of what the server holds; Conflicts end at once with a Server Event Replacement, or an Automatic Resolution; Unimplemented Fields on every Event Version. Tickets 142 to 168 build it; nothing is built yet.

**Scorecard** (1 worst, 3 best among the three; a judgement from the documents, as B and C aren't built):

| Criterion | A | B | C |
|---|---|---|---|
| Code quality, conceptual economy | 2: five stores kept in step | 1: most concepts (two pointers, Reference Snapshot, Tracking, Join vs merge, Snapshot Transitions, link table) | **3**: one table of Event Versions, the rest derived |
| Robustness: invariants enforced | 2: Write Permissions, a Conflict rule run on every write | 2: strong schema, marks derived from pointers | **3**: Event Versions that never change, unique indexes, foreign keys, the Store Check; zero-length Event Versions by a trigger |
| Robustness: data safety | **3**: the Mirror's text keeps what FreeCal doesn't understand (except ticket 129's cases); never sends a guess | 2: no Mirror text; Conflicts never guessed | 2 (was 1–2, before point 3 of `critiques.md`): unknown and partly understood parts kept in the tree, ticket 129's cases too; a reader bug loses data once and the round-trip check shows it; Mark Reconciliation checks the server's marks weekly; a Server Event Merge sends a guess, limited to title, Location, description and Unimplemented Fields |
| Maintainability | 2: a new field goes into five places | 1 | **3**: a field goes into one place; a schema change touches schema, Store Check and an ADR |
| Exception cases | 2: hashes, Sent Event Rows, Propagate's special cases, Dangerous Undo | 1: Join vs merge, Reference Snapshot moves, Tracking | 2: fewer, but new ones (below); point 2 removed about as many as it added |
| UX, undo | 1–2: per Event, undoing later changes first, Dangerous Undo dialogs | 2 | **3**: Ctrl+Z only takes back the user's own changes; refusals name the later change |
| UX, Conflicts | 2: never guesses, only the UID waits, but the colleague's change stays unseen until resolved | 1: one Conflict freezes the whole Calendar both ways | **3** (was 2–3): sync never stops; changes to different field groups are combined, with a toast and an Undo per Event; the user's edit leaves the grid only for the same group, the times against another group, or a series against one Occurrence |
| Cost and risk to reach | **3**: exists, about 4,000 tested lines | n/a | 1: about 27 tickets replace tested code |

**What the review concluded.** B is dominated by C: once an Undo was a Snapshot of its own, the server's line had no job left (point 2 of `critiques.md`). B's one remaining advantage, the Server's Pointer cross-checking the server's marks, is what point 1 below is about. C is better than A in storage, maintainability and undo, and weaker in data safety: points 3 and 5 of `critiques.md` and point 1 here close that gap.

**After point 3 of `critiques.md` (2026-11-22).** On what FreeCal doesn't manage, C now keeps what A keeps, and also keeps it in ticket 129's cases, where A loses it; only a bug in the reader still loses data, once per bug and reported in the Log and the Event's details, where A loses nothing. The rest of the gap is the server's marks: point 1 here with point 5 of `critiques.md`. The check added one exception, a display-only mark beside the ETag; the other rows are unchanged.

**C's exceptions, as the review counted them.** The server's form of FreeCal's own write replaces the sent row; `begin` is emptied when its Snapshot is dropped; the retention rule's conditions; the Mass-Change Guard reading origins; the Automatic Resolution as a Server Event Replacement plus a second Snapshot (gone with point 2, which added zero-length Event Versions, the copy at a Server Event Replacement, the follow-ups' reference and the times as a group apart); `resolution` as the one state of sync outside the Calendar History; the First Sync modal, in memory and not undoable; Ctrl+Z stopping at a Snapshot spanning a read-only Calendar; calendar-level entries on "the first Exception"; two rules when both sides changed the Unimplemented Fields (gone with points 2 and 3).

**Files to read.** `docs/adr/0017-*.md` to `0020-*.md`, `GLOSSARY.md` (Conflict, Resolution, Automatic Resolution, Server Intake, Server Event Replacement, Current-Version Mark, Snapshot, Calendar History), `.scratch/freecal-v1/spec.md`, `.scratch/freecal-v1/critiques.md`, and the tickets named under each point. Older versions: `git show design/snapshot-tree:<file>`, `git show design/change-journal:<file>`.

---

## 1. Nothing cross-checks the server's marks → reconcile them with a full fetch now and then (decided: taken, weekly; see point 5 of `critiques.md`)

**Critique.** ADR 0019 (Consequences): "The server's Current-Version Mark is the only record of what the server holds, and nothing cross-checks it." The dangerous fault is a mark sitting on the user's version while the server holds something else: the change is never sent, no Unsent Mark shows, and the Mass-Change Guard can't catch a send that never happens. `If-Match` on the stored ETag protects only writes that are sent. On `design/snapshot-tree` the Server's Pointer gave a second record to compare with; the line has none.

**Alternative F: Mark Reconciliation** (proposed new term, not yet agreed: a full fetch of a Calendar compared with the server's marks). When the server resets a sync token, and otherwise at a fixed interval (for example weekly), fetch every resource and compare it with the versions carrying the server's mark. This is the path ADR 0019 already has for servers without ETags. A difference is written to the Log and taken in as an ordinary Server Intake (or a Conflict, by the usual comparison).

- **Strongest con:** bandwidth and time on large Calendars, and a second trigger for "fetch everything" with its own schedule.
- A lighter variant: reconcile only on a sync-token reset, which costs nothing extra but may never run on a server that keeps its tokens.

**Questions to grill.** How often, and in the background only? Does a difference found say anything to the user beyond the Log? Does a difference count as a fault to report (a Problem), since it means a bug? Does it run together with point 5 of `critiques.md` (the Store Check before an Outgoing Sync)?

**Affected if taken.** ADR 0019 ("Whether the server changed", Consequences), spec "Sync engine", tickets 151, 152, 167, a new ticket; `GLOSSARY.md` if the term is agreed.

---

## 2. The Automatic Resolution is a modelling artifact → a server-only Event Version, and field-level merging (decided: taken, as a zero-length Event Version and the Server Event Merge)

**Outcome (2026-11-22).** Taken in this form, recorded in ADRs 0017 to 0020, `GLOSSARY.md`, the spec and tickets 21, 133, 135, 147, 152, 159, 167, 168 (now `168-server-event-merge.md`) and 171:

- **The Incoming Sync Snapshot** (new term, replacing Server Intake) records every server version fetched. The server's version of an Event merged or in Conflict begins and ends with it, a **zero-length Event Version**, in no Snapshot's state, carrying the server's mark; an insert trigger allows that only at an Incoming Sync Snapshot.
- After it, one Snapshot per Event: a **Server Event Merge** (new term, short form Merge) with the combination, sent and undoable alone, or a **Server Event Replacement** with a copy of the server's version, to which the mark moves. Each refers to its Incoming Sync Snapshot by a foreign key in the Snapshot table. The Automatic Resolution goes, with its Snapshot and its row of `resolution`.
- **Field groups,** by default: the times (with time zone and rule), title, Location, description, Unimplemented Fields. The times combine only with the Unimplemented Fields; nothing combines across a Recurring Event and its Exceptions. One function names every managed field's group. The same rule decides an Undo, which settles **point 3**: Unimplemented Fields changed on both sides are a Conflict, or refuse the Undo.
- A toast after an Incoming Sync with Merges ("Undo" for one, "Show" for several); an Event's History shows the zero-length row before a Merge as "on the server, before combining".
- Rejected (in ADR 0019's Considered Options): G with a flag; two lines, which check nothing against the server; the combination inside the Incoming Sync Snapshot, which can't be undone per Event; every group combining freely; opt-in merging.
- The cost named: a Server Event Merge sends a guess, against ADR 0019's reason for taking the server's version, so the groups were narrowed.

**Critique.** The user changes an Event's title and a colleague adds an alarm. Nobody would call that a Conflict, yet ADR 0019 makes a Conflict, a Server Event Replacement Snapshot, an Automatic Resolution Snapshot and a row of `resolution` for it. The reason is a storage one (`critiques.md`, point 2, "Settled with the prototype"): the server's mark needs a row holding exactly the server's content, and the combined version isn't that. The Calendar History then shows "replaced by the server's version" and "Automatic Resolution" for an ordinary concurrent change.

**Alternative G: server-only Event Version** (proposed new term, not yet agreed: a row carrying only the server's mark, never current on the user's side). Let the server's mark sit on such a row. The combination is then part of the Server Intake: no Conflict, no Resolution, no extra Snapshots, and the Automatic Resolution and ticket 168 go.

- It also makes **field-level merging** of the managed fields cheap, by groups (times with their time zone and rule, title, Location, description): each group from the side that changed it, a Conflict only when both changed the same group. That is one rule for every field, where today the managed fields are one part and the Unimplemented Fields another. The spec now puts field-level merging out of scope, "planned later as an opt-in setting".
- **Strongest con:** it breaks "each row is current for one unbroken stretch of the line" (ADR 0018), and an empty `begin` already means "began before the oldest kept Snapshot", so a server-only row needs telling apart (a flag, or a CHECK on both marks).
- Field-level merging on its own, without the server-only row, also works on today's model: the Automatic Resolution would simply apply more often.

**Questions to grill.** Is the noise in the Calendar History worth a new kind of row? Which field groups, and does a change to the rule count against every Occurrence's times? Default or opt-in? Does an Event's History show the server-only row?

**Affected if taken.** ADRs 0017 ("Part of the version"), 0018 ("Event Versions", "Keeping and dropping"), 0019 ("Whether the server changed", "An Incoming Sync"), 0020 ("Where both changed"); spec US-73 and "Out of Scope"; `GLOSSARY.md` (Conflict, Resolution, Automatic Resolution); tickets 133, 152, 167, 168.

---

## 3. Two rules when both sides changed the Unimplemented Fields → one rule (decided with point 2: one rule by field groups, both changed is a Conflict or a refused Undo)

**Critique.** ADR 0017 ("Part of the version") and ADR 0020 ("Where both changed"): in an Automatic Resolution the server's Unimplemented Fields win, in an Undo the current version's. That is two rules for one situation, against `CLAUDE.md`'s "one rule for every case", and no ADR gives the reason.

**Alternative H.** One rule for both: the server's win, as in sync, because a wrong guess then reaches nobody else (ADR 0019's argument for Conflicts).

- **Strongest con:** an Undo would then overwrite what the user's side currently has with the server's, which for an Undo, "take back what I did, keep the rest", feels wrong; the current rule may be right and only needs its reason written down.

**Questions to grill.** Is the difference deliberate? If so, which sentence states why? If not, which rule wins?

**Affected if taken.** ADRs 0017 and 0020; tickets 147, 168.

---

## 4. The 90-day retention is the likeliest place for bugs → one reachability rule, tested with generated histories (decided: the rule taken, over Snapshots; scenarios, not generated histories)

**Outcome (2026-11-22).** Recorded in ADRs 0018 ("Keeping and dropping") and 0019, `GLOSSARY.md` (Last Agreed Version, Server Event Replacement), the spec and tickets 21, 133, 135, 137, 146, 147, 148, 151 and 152:

- **One rule over Snapshots.** A Snapshot is kept while a root reaches it. Roots: the Snapshots of the last 90 days, open Conflicts (Server Event Replacements without a Resolution), the Snapshot with which a version carrying the server's mark ended. Edges: an Undo to the Snapshot it takes back, a Server Event Merge or Replacement to its Incoming Sync Snapshot, a Replacement through its Resolution to the Snapshot that ended the Conflict. An Event Version goes with its `end` Snapshot (`ON DELETE CASCADE`), so a kept Snapshot keeps every Event Version ending with it. A new reference is a new root or edge, never a condition.
- **Enforced by the Local Store.** The edges are deferred foreign keys (`DEFERRABLE INITIALLY DEFERRED`, replacing `ON DELETE RESTRICT`), so a drop missing one is refused at commit; two triggers refuse deleting an Event Version with the server's mark and an open Server Event Replacement. Only keeping too much passes.
- **A bug found on the way:** the old conditions dropped a Replacement's zero-length Event Version once it had lost the server's mark, while an open Conflict or an Undo kept the Replacement, breaking the Store Check's invariant. A kept Incoming Sync Snapshot now keeps it.
- **The Last Agreed Version is no longer stored** (reversed on 2026-11-26 by point 7: it is stored per Conflict Event). Only the Conflict view used it, and for a Recurring Event it is several Event Versions, which one column can't name. The Conflict shows the two versions with the fields where they differ (ticket 135, retitled); the user knows which change was theirs. Rejected: a table naming the Last Agreed Version's rows; storing which field groups each side changed.
- **Tests:** one scenario per root and edge, and one where a missed edge is refused at commit. Generated histories rejected (in ADR 0018's Considered Options): with the foreign keys and triggers, the Local Store refuses every mistake in the drop but keeping too much.
- The cost named: after a long time offline, a kept Snapshot keeps all Event Versions that ended with it, e.g. a large import's earlier versions.

**Critique.** ADR 0018 ("Keeping and dropping") has four conditions on Event Versions, two on Snapshots, `ON DELETE SET NULL` on `begin` and `ON DELETE RESTRICT` on `end` and on an Undo's reference, and it already changed once after the prototype (the Undo outliving what it took back). Each new kind of reference (a Resolution, an origin naming the Last Agreed Version) adds a condition. Scenario tests cover the cases someone thought of.

**Alternative I.** State retention as one rule: a row is kept while it is reachable from a kept Snapshot, a server's mark, an open Conflict or a kept Undo; everything else older than 90 days goes. Let the foreign keys carry as much of it as they can, and test it with generated histories (random edits, syncs, Undos, Conflicts, drops) followed by the Store Check.

- **Strongest con:** generated tests need a generator of valid histories, a piece of test code of its own, and their failures are harder to read than a scenario's.

**Questions to grill.** Can every condition be phrased as reachability? Which references become foreign keys? Is a property-based test crate acceptable in the core?

**Affected if taken.** ADR 0018 ("Keeping and dropping"), ticket 148, `CODING_STANDARDS.md` if generated tests become a rule.

---

## 5. Removing a Calendar has no rule in the Calendar History → refuse Undos that span a removed Calendar (decided: not taken; an Undo takes back what it can and asks first, and removal deletes the Calendar's history)

**Outcome (2026-11-28).** Alternative J not taken; the user widened the point to read-only Calendars and Conflicts. Recorded in ADRs 0017, 0018 ("Snapshots", "Removing a Calendar") and 0020 ("Asking first", Ctrl+Z, "Read-only Calendars"), `GLOSSARY.md` (Undo), the spec (US-150, "Calendar History", "Choosing server Calendars", "Sidebar", "Toasts", Seam 1) and tickets 133, 146, 147, 151, 158, 159, 167 and the new 172:

- **An Undo takes back what it can,** one rule for every cause: an Event is left out when it is in a Conflict (changed since), or its Calendar is read-only or removed. The Undo is a Snapshot of its own with what it took back. This reverses ADR 0020's "refused whole" and its rejected option "taking back what can be taken back", whose objection, half a move left without a word, asking first answers.
- **Asking first, in every case:** where anything would be left out, a dialog lists it with why, before anything is written, with "Undo the others", "Undo all over the later changes" and "Cancel"; the same from Ctrl+Z and the Calendar History. A move with one side that can't be written is taken back on the other after asking: restored in its origin (staying in a read-only target too), or deleted from its target when the origin is gone. Redo brings it back.
- **Ctrl+Z** passes over a Snapshot only when every Event still in effect is in a read-only or removed Calendar, checked at each press, and otherwise stops: undoes or asks. No "Skip to the change before" button: Ctrl+Z goes back in order, and the Calendar History reaches older changes. Rejected: passing over Conflicts silently, or once the user was asked (a stored answer).
- **Removing a Calendar** deletes its Event Versions, Conflicts and Unsent Changes, drops Snapshots left empty, and can't be undone; its dialog says so (ticket 172). The data-protecting triggers let a removal through. An Undo recognises a move whose one side went with a removed Calendar from the move recorded on the Snapshot, so it is never taken for a create or a delete; an action of the user now records which it was. Rejected (ADR 0018, Considered Options): removal as a Snapshot with the Calendar kept as removed, and a stored list of the Calendars each Snapshot spanned.
- The costs named: Ctrl+Z can open a dialog; an Undo can leave one UID in two Calendars after asking; a removed Calendar can't be named in an Undo.

**Critique.** The spec lets the user remove a server Calendar from FreeCal ("Removing a server Calendar from FreeCal deletes only its local copy"), and a Local Calendar can be deleted. ADR 0009 had a rule for that; ADRs 0018 to 0020 have none. With one line across all Calendars, a move out of a removed Calendar leaves a Snapshot spanning only the other one; undoing it from the Calendar History would quietly become a delete there. It is also open what happens to Snapshots that only spanned the removed Calendar, and to its open Conflicts.

**Alternative J.** Removing a Calendar deletes its Event Versions and its rows of `conflict`; Snapshots left without Event Versions are dropped; an Undo of a Snapshot that spanned a removed Calendar is refused whole, saying which Calendar is gone, as for a read-only one (ADR 0020). The Calendar History shows such a Snapshot without "Undo".

- **Strongest con:** the Snapshot must remember which Calendars it spanned, which today is read from its Event Versions alone; that is a stored list or a table, the kind of second record ADR 0018 removed.

**Questions to grill.** Stored list of Calendars per Snapshot, or a marker Event Version left behind? Is removing a Calendar itself a Snapshot (and undoable)? What happens to the Unsent Changes of a removed server Calendar?

**Affected if taken.** ADRs 0018 and 0020; spec "Accounts and Calendars", "Calendar History"; tickets 146, 147, 159, and the tickets that remove Calendars and Accounts.

---

## 6. The spec and the glossary lag behind the ADRs → reword them (done: reworded as proposed)

**Outcome (2026-11-28).** Checked again after all other points were decided, so the con below no longer held; the user agreed to every wording proposed. Recorded in the spec, `GLOSSARY.md` and tickets 126, 146, 151, 152, 159 and 162:

- **US-83** now asks to see the Calendar History of all Calendars or of one; undoing from it is US-155. **US-81** says a Snapshot of all Calendars, so that any change can be seen and undone. **US-74 and US-75** say what FreeCal does first and how the user decides afterwards; US-73 was already reworded by point 2, and US-76 and US-77 needed nothing.
- **What C costs the user** is a paragraph after the Solution: an edit colliding with a server change leaves the grid until restored; an Undo can ask first; a Calendar being added shows empty until its First Sync, and an Event created meanwhile may duplicate one; removing a Calendar can't be undone. The window lock and Ctrl+Z stopping at a read-only Calendar were already gone (point 4 of `critiques.md`, point 5 here).
- **Also lagging, found while checking:** the spec's import paragraph still took a Calendar back to before an import, and the Solution spoke of "its Calendar History"; ticket 159 offered no "Undo" on a Calendar's first Snapshot, against the spec and ADR 0018; ticket 126 still named "Go back to before this".
- **`GLOSSARY.md`:** the Snapshot entry was already right (points 2 and 7). Entries of the old design say "Goes when the code follows ADR 0018" (0019, 0020): the Change Journal's seven, Record a Journal Event Modification, Sent Event Rows and Propagate a Server Object. Those revised say "Until the code follows ADR …" and what follows: Event Row, Server Object and its CalDAV and Google forms, Mirror, Ignored Resource, Known Server Resources, Corresponding Server Object and Sender. Unsent Change is defined by the difference from the server's version, stored as a list only until ADR 0019 is built.
- **Ticket files renamed:** `146-snapshots-on-one-line-and-store-check`, `151-servers-current-version-mark`, `152-comparison-and-incoming-sync-snapshots`, `162-conflict-bar-and-toast`. No ticket linked them by file name.

**Critique.** Found while comparing; none of it needs a grilling, only agreement on the wording.

- **US-83** ("take the Calendar back to before any change in it") contradicts ADR 0020 ("no taking a Calendar back to before a Snapshot"); ticket 159 still cites it.
- **US-81** says a Snapshot is "of its Events", one Calendar's; a Snapshot now spans all Calendars.
- **US-73 to US-77** say "I want to … choose one": FreeCal now chooses first and the user resolves afterwards. The Solution paragraph already says so; the stories don't.
- The spec should state plainly what C costs the user: an offline move can leave the grid until resolved; Ctrl+Z can stop at a Snapshot spanning a read-only Calendar; adding a large Calendar locks the window (point 4 of `critiques.md`).
- `GLOSSARY.md`, **Snapshot**: doesn't name the Server Event Replacement, the Automatic Resolution or "Stop waiting and mark read-only" among what makes one; ADR 0018 does.
- `GLOSSARY.md` still defines the Change Journal's terms (Change Journal, Journal Transaction, Journal Event Modification, Transaction Origin, Dangerous Undo, Write Permission, Guard); ADR 0018 keeps them until the code changes. Mark each "until ADR 0018 is built".
- Ticket file names keep removed terms: `146-snapshots-and-users-pointer`, `151-servers-pointer`, `152-reference-snapshot-and-comparison`, `162-conflict-bar-and-stopped-tracking-dialog`. Renaming changes links in other tickets.

**Strongest con of rewording now:** points 1 to 5 here and points 3 to 5 of `critiques.md` may reword the same passages again.

**Affected.** `.scratch/freecal-v1/spec.md`, `GLOSSARY.md`, tickets 146, 151, 152, 159, 162 and the tickets linking to them.

---

## 7. The Server Event Merge and Replacement as Snapshots of their own, and the `resolution` table → fold both into the Incoming Sync Snapshot and keep a table of Conflicts (raised by the user on 2026-11-26; decided: taken, as design N′)

**Outcome (2026-11-26).** Taken. The design before is tagged `design/server-event-snapshots`. Recorded in ADRs 0018, 0019 ("The Conflict tables") and 0020 ("Limited to one Event"), `GLOSSARY.md`, the spec and tickets 15, 21, 22, 133, 134, 135, 136, 137, 146, 147, 148, 152, 158, 159, 160, 168 and 171:

- **One Snapshot per Incoming Sync and Calendar.** An Event in Conflict takes the server's version in the Incoming Sync Snapshot itself, current at once and carrying both marks, with no copy and no zero-length Event Version (a Server Event Replacement). An Event combined gets its zero-length server's version and its combination there too (a Server Event Merge). Both are now what an Incoming Sync Snapshot does to an Event, not Snapshots. Zero-length Event Versions are needed only for Merges.
- **What goes:** the Snapshot column pointing to the Incoming Sync Snapshot, with its CHECK, its edge in the drop's rule and its Store Check rule; the `resolution` table and its edge; the exception that an Incoming Sync Snapshot is made even where it changes nothing.
- **An Undo limited to one Event** (one UID, in every Calendar the Snapshot spans, so a move is never split), recorded beside the Snapshot it takes back. It restores a replaced Event and takes back a Merge; "Undo this Event" offers it for any Snapshot in the Calendar History. Ctrl+Z is never limited; a Snapshot is in effect while some UID it changed isn't taken back. A whole Undo of an Incoming Sync Snapshot takes back its Merges and Replacements and resolves its Conflicts.
- **The Conflict tables.** `conflict`: the Incoming Sync Snapshot, the Calendar, the UID and the time it was resolved (empty while open, set once). `conflict_event`, a **Conflict Event** (`GLOSSARY.md`): one row per single Event found on either side or in the Last Agreed Version, with that Last Agreed Version, empty where none existed or the base is untrusted. The User's and Server's Versions are derived from the Event Versions ending and beginning with the Incoming Sync Snapshot; how a Conflict ended isn't stored. A resolved Conflict stays as long as its Incoming Sync Snapshot.
- **The Last Agreed Version is stored again,** reversing point 4: one row per single Event names the several Event Versions of a Recurring Event, which one column couldn't, and with Conflicts decided by field groups, what each side changed is what the Conflict is about (ticket 135 retitled back).
- **Retention:** roots are the Snapshots of the last 90 days, the Incoming Sync Snapshots of open Conflicts and each Snapshot with which a version carrying the server's mark ended; edges are an Undo to what it takes back and an Incoming Sync Snapshot, through its Conflict Events, to the Snapshots with which their Last Agreed Versions ended. Triggers refuse deleting a version carrying the server's mark and an open Conflict.
- **Strongest con, named:** a missed insert into `conflict` hides the Conflict, and no query can find it afterwards, as an Event in Conflict and one the server alone changed leave the same Event Versions. Mitigated by one writer (the branch of the comparison that decides a Conflict), the insert trigger, and tests reaching every path to a Conflict.
- Rejected (in ADR 0019's Considered Options): a table of open Conflicts only (the only trace of a Conflict would vanish when resolved); keeping `resolution` beside `conflict` (nothing reads how a Conflict ended); storing the User's and Server's Versions in the Conflict Event (derived); Undos limited to Incoming Sync Snapshots only, or to a Calendar and UID (splits a move).

**Critique.** Since point 2, an Incoming Sync writes up to three kinds of Snapshot: the Incoming Sync Snapshot, then one Server Event Merge or Server Event Replacement per Event, each pointing back to it by a column of its own with a CHECK, an edge and a Store Check rule. A Replacement copies the server's version and moves the mark; the Incoming Sync Snapshot holds a zero-length row for it, and it is made even where it changes nothing. They were Snapshots of their own only so that each could be undone alone. Open Conflicts are read as Replacements without a row of `resolution`, an inverted record whose ending is stored in detail though nothing reads it.

**Alternative N′.** Fold the Merges and Replacements into the Incoming Sync Snapshot, and make an Undo limitable to one Event so each can still be undone alone. Record Conflicts directly in a table, one row per Conflict with a resolved time, and one per single Event with its Last Agreed Version.

**Affected.** As listed in the outcome.
