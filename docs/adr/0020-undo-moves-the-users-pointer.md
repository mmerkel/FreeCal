# Undo is a stack, and moves the User's Pointer

With ADR 0018 this supersedes ADR 0009, and reverses its choice of undo per Event. It comes from the grilling of 2026-11-15 (see ADR 0017); the code follows with later tickets, after a prototype.

Undo per Event had to work out which later changes stood in the way, undo those first, check hashes, and classify the result as benign or dangerous. With the Calendar History (ADR 0018), an undo moves a Calendar's User's Pointer to the parent Snapshot and writes nothing else; if the Calendar is Tracking its server, the difference between the pointers is then sent like any other (ADR 0019). We take that simple form first and judge it in use.

- **Ctrl+Z** undoes the newest Snapshot of any Calendar, Incoming Syncs included, and asks before undoing one that came from a server. A Snapshot spanning several Calendars is undone whole.
- **Undo for one Calendar** moves only that Calendar's User's Pointer. Undoing a move there leaves the other Calendar as it is, so the Event is then in both, and FreeCal says so.
- **At a merge**, undo asks which side to go back through, the user's or the server's. **Redo** moves the pointer forward again, and asks which way where two Snapshots arose from the one it is at.
- **The redo path ends whenever the User's Pointer leaves its line**: by a change the user makes, by an Incoming Sync that moves the User's Pointer, and at a fork by redo, for the way not taken. The Snapshots that are then neither an ancestor of a pointer nor beyond the User's Pointer are dropped. That is one rule, which the Store Check asks as one query (ADR 0018).
- Undo can't go before a Calendar's first Snapshot (its First Sync, or the empty one of a Local Calendar).
- An undo is not itself recorded. After the send that follows it, the Server's Pointer moves back with the User's.
- While a Calendar isn't Tracking its server, undo moves the User's Pointer and sends nothing.
- The Dangerous Undo's thresholds go; the confirmation before undoing a sync stays, and the Mass-Change Guard checks the send that follows (ADR 0019).

## Considered Options

- **Undo per Event** (ADR 0009): an old import can be undone without touching what came after it in other Events, but it needs the hash check, the search for changes in between, the rule for undoing a Journal Transaction whole, and the Dangerous Undo.
- **Taking back one Snapshot from the middle** (a reverse change as a new Snapshot): selective undo within the new model, at the price of a second kind of undo that can fail or conflict.
- **Ctrl+Z over the user's own actions only**, skipping syncs: closer to other programs, but the pointer would have to jump over Snapshots, which is the selective undo above. Left for after practical use.
- **Recording each undo as a Snapshot**: the history would show it, but redo would be a second undo, and the pointer could no longer simply move.
- **Keeping the redo path after an unrelated change**: the history would fork on the user's side, with a choice at every redo.

## Consequences

- There is no selective undo. Undoing an old change in a Calendar takes back everything since in that Calendar, changes from the server included, and the following send overwrites the server with it.
- What Ctrl+Z reaches depends on when the last poll ran, as syncs are steps in the stack.
- An unrelated edit after an accidental undo loses the redo, and so does an Incoming Sync that brings a change before the redo.
- Choosing the server's way at a redo fork drops the user's own undone Snapshots for good.
- Undoing one Calendar's part of a move or of an import leaves the other part: one UID in two Calendars, or half an import.
- An Event's History shows versions but offers no way to restore one.
