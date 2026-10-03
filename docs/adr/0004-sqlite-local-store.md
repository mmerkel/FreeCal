# SQLite as the single local store

FreeCal is offline-first. One SQLite database holds the local copy of every Calendar, the queue of pending changes, the Change Journal and the settings. Snapshots are compressed `.ics` files stored next to it. SQLite's transactions mean a crash mid-sync can't leave a half-written state. Each Event's original server data is stored as received, next to the fields FreeCal uses, so fields FreeCal doesn't understand (attendees, conference data, custom properties) survive every write, and Change Journal entries can restore exact previous versions.

## Considered Options

- **A folder of `.ics` files (vdir)**: readable by other tools such as khal, but there are no transactions, and the pending-change queue and Journal would need a separate mechanism.
- **An embedded key-value store (redb, sled)**: no advantage over SQLite here, and harder to inspect while debugging.
