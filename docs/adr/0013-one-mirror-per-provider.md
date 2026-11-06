# One Mirror per Provider

Each Provider has Mirror tables of its own, shaped for how its server stores Events, and only the Sync Engine writes and reads them. CalDAV's is `caldav_server_object`: the Calendar, href, ETag, the iCalendar text as received, the unreadable mark, the UID, and whether it holds the Event that isn't an Exception (the single Event or the Recurring Event). Google's, with ticket 26, holds one Google event resource per row with its UID, its recurrence ID as obtained, Google's link to its Recurring Event and the JSON as received. They replace `server_object`, whose `data` column held iCalendar or JSON by its `format` column, and `combined_event_key`, which made every Provider describe its Server Objects in one generic shape. This makes ADR 0011's first paragraph literal: how a server lays out Event Rows in resources is held only by the Mirror and the Provider code.

A CalDAV Server Object holds exactly one UID, as RFC 4791 §4.1 requires ("Calendar components in a calendar collection that have different UID property values MUST be stored in separate calendar object resources"). A resource holding several is mirrored with its first UID and without Event Rows, and reported as a problem of its Calendar, as a second Server Object holding a UID already is (ADR 0011). Such resources are rare: SabreDAV (Nextcloud, Baïkal), Radicale, Apple's Calendar Server, Cyrus (Fastmail) and Stalwart refuse to store them, the one documented case (CalDavZAP on DAViCal, 2014) was fixed in the next release, and DAVx5 skips them.

Unsent Changes stay one Provider-neutral table, `unsent_change`, guarded by the Change Journal: they are written in the same Database Transaction as the Event Rows they are about, and they describe Events, including Events that have no Server Object yet. The Sync Engine's code for one Provider lives in its own module (`core/src/sync/caldav/`), with what every Provider shares in `core/src/sync/mod.rs`; a shared interface waits for the second Provider. The Mirror's SQL lives in `core/src/store/sql/mirror/`, one file per Provider.

## Considered Options

- **One `server_object` table for every Provider, with a `format` column and a generic `combined_event_key` table** (ADR 0010 and ADR 0011's first form): one column held two formats, and CalDAV kept, and had to renormalise, recurrence IDs in its keys that it never used to find a Server Object.
- **A (Server Object, UID) key table for CalDAV**: it exists only for resources holding several UIDs, which the standard forbids and most servers refuse.
- **Unsent Changes per Provider, under the Sync Engine**: every change recorded by the Change Journal would have had to call into the Sync Engine, which already builds on the Change Journal.

## Consequences

- The rules that read the Mirror are written per Provider: finding the Corresponding Server Object (by UID and whether it holds the Event that isn't an Exception on CalDAV, by UID and Occurrence on Google), handing a UID's Event Rows to its next Corresponding Server Object, and recognising a Server Object that moved.
- Each Provider's tables have their own migrations.
- Events in a CalDAV resource holding several UIDs aren't shown, though some other clients show them.
- Combined Event Keys are no longer stored in the Mirror; the term names what an Unsent Change is about and what the Sender finds a Server Object by.
