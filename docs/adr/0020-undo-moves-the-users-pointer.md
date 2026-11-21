# Undo is a stack, and moves the User's Pointer

With ADR 0018 this supersedes ADR 0009, and reverses its choice of undo per Event. It comes from the grilling of 2026-11-15 (see ADR 0017); the code follows with later tickets, after a prototype.

Undo per Event had to work out which later changes stood in the way, undo those first, check hashes, and classify the result as benign or dangerous. With the Calendar History (ADR 0018), an undo moves a Calendar's User's Pointer to the parent Snapshot and writes nothing else; if the Calendar is Tracking its server, the difference between the pointers is then sent like any other (ADR 0019). We take that simple form first and judge it in use.

- **Ctrl+Z** undoes the newest Snapshot of any Calendar, Incoming Syncs included, and asks nothing where there is nothing to choose: a toast after each undo says what was undone, a change on the server too, and offers "Redo"; a toast after each redo says what was redone, without a button (decided 2026-11-21, replacing the question before undoing a Snapshot from a server). A Snapshot spanning several Calendars is undone whole.
- **Undo for one Calendar** moves only that Calendar's User's Pointer. Undoing a move there leaves the other Calendar as it is, so the Event is then in both, and FreeCal says so.
- **At a merge**, which only ending a Conflict makes (ADR 0019), undo asks which side to go back through, the user's or the server's. **Redo** moves the pointer forward again, and asks which way where two Snapshots arose from the one it is at.
- **Which Calendar.** Ctrl+Z takes the Snapshot made last among those a User's Pointer is on. Redo has no such order to follow, as nothing records in which order Snapshots were undone: it redoes when exactly one Snapshot is next beyond the User's Pointers of all Calendars, and otherwise asks which, as it does at a fork (decided 2026-11-16).
- **The redo path ends whenever the User's Pointer leaves its line**: by a change the user makes, by "Continue" while the Calendar is Held during Undo, and at a fork by redo, for the way not taken. An Incoming Sync doesn't end it: while a redo path exists, the server's changes wait (ADR 0019, decided 2026-11-21). The Snapshots that are then neither an ancestor of a pointer nor beyond the User's Pointer are dropped. That is one rule, which the Store Check asks as one query (ADR 0018).
- Undo can't go before a Calendar's first Snapshot (its First Sync or the empty one of a Local Calendar). An import or a restore doesn't create Calendars (ADR 0018, decided 2026-11-21, replacing the half-undo of an import that created Calendars, of 2026-11-16), so Ctrl+Z takes either back whole.
- An undo is not itself recorded. After the Outgoing Sync that follows it, the Server's Pointer moves back with the User's. If the server changes before the redo path ends, the Calendar is Held during Undo (ADR 0019).
- While a Calendar isn't Tracking its server, undo moves the User's Pointer and sends nothing.
- The Dangerous Undo's thresholds go; the confirmation before undoing a sync goes too (decided 2026-11-21), and the Mass-Change Guard checks the Outgoing Sync that follows (ADR 0019).

## Considered Options

- **Undo per Event** (ADR 0009): an old import can be undone without touching what came after it in other Events, but it needs the hash check, the search for changes in between, the rule for undoing a Journal Transaction whole, and the Dangerous Undo.
- **Taking back one Snapshot from the middle** (a reverse change as a new Snapshot): selective undo within the new model, at the price of a second kind of undo that can fail or conflict. Kept as a possibility (see "Left open").
- **Ctrl+Z over the user's own actions only**, skipping syncs: closer to other programs, but the pointer would have to jump over Snapshots. Left for after practical use; a way to do it is kept as a possibility (see "Left open").
- **Recording each undo as a Snapshot**: the history would show it, but redo would be a second undo, and the pointer could no longer simply move.
- **Redo taking the oldest Snapshot beyond any User's Pointer**, the mirror of Ctrl+Z: no question after undoing twice across two Calendars, but it brings back the wrong Calendar's Snapshot after an undo in one Calendar, a change in another and an undo of that, and after an undo for one Calendar.
- **Numbering the undos** (each undo marks the Snapshots it steps back over, and redo takes the highest number): redo would always reverse the last undo without asking, at the price of stored state about undos that has to agree with the pointers. Not taken for now (see "Left open").
- **Asking before undoing a Snapshot from a server** (the first form): a guard against a stray Ctrl+Z taking back someone else's change on the server, but a dialog in the most common undo after a sync. Held during Undo (ADR 0019) lets the toast's "Redo" bring back exactly what was undone, as nothing from the server comes in meanwhile.
- **"Undo" in the toast after a redo**: symmetric, but once the redo path has ended, the server's held changes come in, and the toast's Undo would take those back instead.
- **Keeping the redo path after an unrelated change**: the history would fork on the user's side, with a choice at every redo.

## Consequences

- There is no selective undo. Undoing an old change in a Calendar takes back everything since in that Calendar, changes from the server included, and the following Outgoing Sync overwrites the server with it.
- What Ctrl+Z reaches depends on when the last poll ran, as syncs are steps in the stack.
- A stray Ctrl+Z after a sync takes back someone else's change on the server at once; the toast's "Redo" puts it back, and others may see both.
- An unrelated edit after an accidental undo loses the redo. An Incoming Sync doesn't, but the server's changes wait until the user redoes to the end, continues or makes a change, and the Calendar isn't Tracking meanwhile.
- Redo asks in a common case: after undoing one change in each of two Calendars, both have a Snapshot to bring back.
- Choosing the server's way at a redo fork drops the user's own undone Snapshots for good.
- Undoing one Calendar's part of a move, an import or a restore leaves the other part: one UID in two Calendars, or half an import or restore.
- An Event's History shows versions but offers no way to restore one.

## Left open

Two ways to add what the stack leaves out, noted on 2026-11-16. Neither is decided or built; both work on the Calendar History as it is, need no change to it now, and don't exclude each other. A third note of the same day: if redo's question between Calendars proves a nuisance, numbering the undos (see "Considered Options") removes it, as one column on a Snapshot's link to its Calendar and one query of the Store Check.

- **Taking back one earlier Snapshot, as a new Snapshot.** A new Snapshot at the User's Pointer holds the result of the comparison of the two sides (ADR 0019) with the Snapshot to take back as what both are compared with, the User's Pointer's Snapshot as one side and the parent of the Snapshot to take back as the other. It is refused when that comparison finds a Conflict, which is when an Event the Snapshot changed has changed again since, and when the Snapshot is no ancestor of the User's Pointer. No Snapshot is moved and no parent changes. Fits undoing an old change from a list. Its price: a second kind of undo, one that is recorded, ends the redo path, and is itself undone with Ctrl+Z.
- **Swapping a Snapshot of the user with the sync Snapshot after it, then undoing.** Where the two changed no Event in common, they change places: each keeps what begins and ends in it and gets the other's place among the parents, and a pointer that was on the later one goes to the one now later, whose state is the same. A plain undo then takes the user's change back and keeps the server's. Repeated, it moves a Snapshot further. It is refused where the two share an Event, where a merge Snapshot would be passed (there, undo through the server's side already does it), and where the Server's Pointer, the Reference Snapshot or the Snapshot of a running Outgoing Sync lies strictly between. A Snapshot spanning several Calendars is moved in all of them in one Database Transaction or not at all: moved past another spanning Snapshot in one Calendar only, each would be the other's ancestor. Swaps with sync Snapshots alone can't cause this, as a sync Snapshot is of one Calendar. Fits Ctrl+Z over the user's own actions only: undo stays one kind and redo works. Its price: a Snapshot's parents can change, which nothing else does, and the Calendar History is no longer the order in which things happened: the sync Snapshot then shows a state the server never held.
