# 88: Exceptions without their Recurring Event

**What to build:** An object or `.ics` file that holds Exceptions without their Recurring Event, such as an invitation to one Occurrence, is kept: each Exception is an Event of its own, as GLOSSARY has it, and joins its Recurring Event when that arrives. Today `import::assemble` leaves them out, so they never appear.

**Blocked by:** 87

**Status:** ready-for-agent

**Effort:** M

- [ ] From a server, Exceptions without their Recurring Event are stored with their recurrence IDs and their object; they show in the grid at their own times and open as Events of their own
- [ ] From an import, the same; an import that later brings the Recurring Event (or a sync whose object now holds it) pairs them with its Occurrences by recurrence ID, and an Exception that matches no Occurrence stays an Event of its own. Where the Recurring Event brings its own Exception for the same Occurrence, that one replaces the standalone one, so the Occurrence has one Exception (ticket 87's rule)
- [ ] Their recurrence IDs are normalised as for any Exception; with no Recurring Event to take the zone from, a timed one keeps the zone it was given in, and is normalised again when it joins
- [ ] Core tests for each point, from a server and from a file

Out of scope: editing them as Occurrences before their Recurring Event arrives (they edit as single Events).
