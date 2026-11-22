# Unimplemented Fields live on the Event Version, as a tree in canonical form

This is the ADR ticket 129 asks for. With ADRs 0018 to 0020 it comes from the grilling of 2026-11-15 on the branch `explore/unknown-fields-and-change-journal`, which was free to reverse earlier decisions where that made the code simpler and more consistent. The code follows with later tickets, after a prototype; until then it keeps ADRs 0009, 0014 and 0015.

Until now, what a server holds about an Event beyond the fields FreeCal manages lived only in the Mirror, as the text received, and the CalDAV writer patched FreeCal's fields into that text. A resource built without a Server Object lost those fields: an Event moved to another Calendar, the undo of a DELETE already sent, and the new Recurring Event of "this and following events" (ticket 129). We now keep them as Unimplemented Fields on every Event Version (ADR 0018), so they are recorded, compared, restored and moved with the Event, and a writer always builds a Server Object from Event Versions alone. The Mirror's text and the patching writer go.

**The format.** The Unimplemented Fields of one Event Version are a tree of named entries, a Rust type stored through serde in one column, in a canonical form: two trees are equal exactly when their stored forms are, whatever order or layout the server sent. An entry's name says where it is understood:

- A name with a format's prefix (`ical:`, `google:`, later perhaps Microsoft Graph's) holds what only the reader and writer of that format understand. Everything read from iCalendar text is `ical:`, from a CalDAV resource, an `.ics` file or a feed alike. `caldav:` is reserved for what only CalDAV holds and is assigned to nothing yet. Each Provider names the prefixes it holds: Local and ICS feed `ical:`, CalDAV `ical:` and `caldav:`, Google `google:` (decided 2026-11-16). A structured item that doesn't convert to another Provider without loss, such as an alarm or an attachment, stays whole under its format's name.
- A neutral name holds a field whose meaning and value are the same across Providers, such as status, show-as, visibility, sequence, created and updated, or `RDATE`. Neutral names are assigned when Google's reader is built, not before.

A move or an import into a Calendar carries the neutral entries and those with a prefix the target's Provider holds, and drops the rest. That is one rule for every Provider: a move between a Local and a CalDAV Calendar drops nothing, and one from Google into a Local Calendar drops the `google:` entries, which no export or Backup of a Local Calendar could write. When a field changes home (from a format's name to a neutral one, or to a column of its own once FreeCal manages it), a migration rewrites every Event Version, in Rust, and what doesn't convert stays under its format's name. Code reads only the current form.

**What is in the tree.** Only what FreeCal doesn't manage. The values of managed properties are not repeated. Parameters FreeCal doesn't know on a managed property (`LANGUAGE`, `ALTREP`, `X-` parameters) are one entry under the property's name, without its value; an edit of that managed field drops them, as they described the old value. `RANGE=THISANDFUTURE` changes what an Exception means, so it becomes a managed field (ticket 75), not an entry. An Exception the user makes by changing one Occurrence takes the Recurring Event's entries, and so does the new Recurring Event of "this and following events", both without the calendar-level ones (decided 2026-11-16). The properties a writer sets fresh on every write (`DTSTAMP`, `LAST-MODIFIED`) and time zone definitions are not kept. `SEQUENCE` is kept as an entry and written back unchanged: FreeCal changes no Event with guests, and who raises it is decided with the attendee features (ticket 126; decided 2026-11-16). What a CalDAV resource holds at the calendar level, outside its Events, sits under an `ical:` name on the resource's single Event or Recurring Event, or on its first Exception when it holds only Exceptions.

**Part of the version.** The Unimplemented Fields are fully part of the Event Version: a change to them alone is a change, makes a Snapshot when it comes from a server, shows in the Calendar History as "changed in fields FreeCal doesn't show", and is undone with the rest. In an Automatic Resolution and an Undo (ADRs 0019 and 0020) they are one part beside the managed fields. If only one side changed them, that side's are taken; if both did, the server's win, in an Undo the current version's.

## Considered Options

- **The Mirror's text, patched** (ADRs 0010, 0015): exact text kept, but the fields exist only where a Server Object does, which is ticket 129's three cases, and every further case needs its own rule for where the text comes from.
- **The Provider's raw data in a column of the Event Version** (iCalendar text or Google's JSON): nothing lost and no second schema, but the Event table would hold two formats, comparison would depend on order and layout, and a move between Providers would carry nothing.
- **Neutral names only**: one schema, but alarms, attachments and custom properties convert between iCalendar and Google's JSON only with loss, and the loss would happen on reading, before any move asked for it.
- **Provider names only**: no neutral schema to design, but a move between Providers would drop the status, the visibility and the like, which both sides hold in the same way.
- **Google through its CalDAV endpoint, so that one format serves all**: `docs/research/google-caldav.md`. Google's CalDAV rewrites what a client sends and lacks colours, conference data and control over guest mail; ADR 0002 stands.
- **The whole managed property kept beside FreeCal's field**, to keep its parameters: the same value stored twice, with a rule needed for when they disagree.
- **A placeholder row for the Recurring Event of a resource holding only Exceptions**, to carry the calendar-level entries: a row that is no Event, which every reader of Events would have to skip.
- **A Conflict when both sides changed the Unimplemented Fields**: the user would be asked to choose between two versions of fields FreeCal doesn't show.
- **A rule in the history for values in an old home**: a migration rewrites them instead, so that only one form is ever read.

## Consequences

- The server's original text is no longer in the Local Store, neither for writing nor for inspection. A property FreeCal's reader fails to carry into the tree is lost on the next write of that Event; the reader's tests must cover every kind of property, parameter and component.
- A field can have three homes over its life: a format's name, a neutral name, a column. Each change is a migration over all Event Versions.
- Neutral names are a second schema that SQL can't check. Whether a name is truly neutral shows only once Google's reader exists; with three Providers each neutral name needs three converters.
- An edit of a managed field silently drops that property's unknown parameters, such as the `LANGUAGE` of a title.
- An undo can take back another client's change to an alarm, and after a merge in which the server's Unimplemented Fields won, an undo may not bring an older alarm back.
- Changes made on a server to fields the user can't see appear in the Calendar History as changes.
- Which Exception is "the first" is arbitrary, and the calendar-level entries move when that Exception is deleted.
