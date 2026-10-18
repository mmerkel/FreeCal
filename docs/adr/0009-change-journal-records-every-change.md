# The Change Journal records every change to an Event, and undo goes per Event

The Change Journal records every change to the local copy of an Event, whatever its source: the user's edits, moves and deletes in any Calendar (Local Calendars too), imports, Conflict resolutions, Snapshot restores, undos, and the changes an incremental sync pulls from a server. Each entry holds the Event's previous version and a hash of its new version. A change can then be undone exactly when the Event still matches that hash. If it doesn't, the Journal says which later changes came between, and those are undone first. There is no gap to guess about, so undo never has to match Events by ID and hope, as ticket 58's first version did. The same entries give Conflicts the common version both sides started from.

Undo goes per Event, not as one global stack. Undoing an entry also undoes the later entries for the same Events, and leaves the rest alone. The user sees one timeline. Ctrl+Z undoes their own latest action in this session, and older entries are undone from the list or from an Event's History. Entries of a unit (an import, a sync of one Calendar) can be undone one by one or together.

An undo is benign and runs at once when it touches only what the user picked, came from their own actions, and stays within the Mass-Change Guard's thresholds. Otherwise it is dangerous and asks first. That is the case when it also undoes later changes, reverts a change that came from a server, or touches more than 10 Events or more than 5% of a Calendar. That confirmation is the Mass-Change Guard's approval. Every undo is itself an entry, so a wrong undo can be undone too.

A Calendar's first sync is the baseline and is not recorded: undoing it would delete the whole Calendar on the server. Entries are kept for 90 days, whether or not FreeCal is restarted.

## Considered Options

- **Only changes FreeCal sends to a server, plus imports** (the first spec): edits in Local Calendars and changes pulled from servers left gaps, so undoing an import had to guess about Events changed since. It deleted an imported Event the user had moved, or brought back one deleted since, without saying so.
- **A global undo stack**: simple to explain, but undoing last week's import would first undo every unrelated change made since, in every Calendar. Ctrl+Z still behaves like a stack for the user's own actions in this session.
- **Clearing the Journal when FreeCal quits**: it would save a few megabytes, but the Journal exists to repair mistakes noticed days later. FreeCal also keeps running in the tray, so it quits at random times.
- **Overwriting Events changed outside FreeCal on undo**: with incoming syncs recorded, only a Local Store edited by another program can still have an Event that matches no entry. Undo leaves such an Event alone and says that its current version isn't in the Change Journal.

## Consequences

- Undoing a change pulled from a server writes the previous version back to the server, so it overwrites someone else's change, or the user's change made on another device. Hence the confirmation.
- Read-only Calendars record incoming changes too, so they can be seen. Undo skips their Events and names the Calendar.
- An import that created Calendars removes them on undo if they are empty by then.
