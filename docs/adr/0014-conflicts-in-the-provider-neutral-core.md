# Conflicts are found in the Provider-neutral core, per Occurrence

A Conflict is the state of an Event, so it is found, held and resolved by Provider-neutral code next to the Change Journal, not by the Sync Engine. For each Calendar and UID a server changed, the Sync Engine reads the Server Version, and the previous Server Version from the Mirror's data before it is overwritten. A neutral step then goes through the Occurrences: one with no Unsent Change is propagated to the Event Rows; one with an Unsent Change that the server didn't change keeps the User's Version, to be sent later; one changed on both sides is a Conflict. A change to the Recurring Event itself, on either side, counts as a change to every Occurrence. Without a readable previous Server Version, every Occurrence counts as changed by the server. Once any Occurrence is in Conflict, the Conflict covers the whole Event, so holding, showing and resolving it stay per Event.

A Conflict keeps a copy of the Server Version, as Event fields, in `conflict` and `conflict_server_event`, so the core never asks the Sync Engine for it and an open Conflict stays complete even if its Server Object later becomes unreadable. A later Incoming Sync that changes the same UID replaces the copy. Conflicts are written in step with Event Rows and Unsent Changes, so the Change Journal's Write Permission guards them. Changes to different Occurrences of one Event are merged without asking: both can be kept, so FreeCal isn't guessing. Field-level merge of two changes to one Event stays out of scope (spec).

## Considered Options

- **Conflicts in the Sync Engine, per Provider**: "a server change to a Server Object while one of its Combined Event Keys is unsent" (ADR 0011) would have been written once per Provider, though the badge, the list, both versions, the common version from the Change Journal and resolving are all about Events.
- **Per UID**: one rule without comparing Server Versions, but an unsent change to one Exception would conflict with a server change to another Exception of the same Recurring Event.
- **The Server Version read again from the Mirror while the Conflict is open**: nothing stored twice, but the core would call into the Sync Engine.
- **The Server Version as one JSON value on `conflict`**: one table less, but a second serialisation of Event fields that SQL can't check.

## Consequences

- An Incoming Sync propagates through the neutral step: Propagate a Server Object writes only the Occurrences the rules above let through.
- `conflict_server_event` is a third copy of the Event columns, next to `event` and `journal_event_modification`.
- On CalDAV, where a Server Version comes whole, finding what the server changed takes the comparison with the previous Server Version; on Google, a changed Google Server Object is one Occurrence or the Recurring Event.
