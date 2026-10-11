# Event times are stored as wall time and time zone

A timed Event is stored as its wall-clock time plus the IANA time zone it is given in, for example 9:00 `Europe/Berlin`, because that is what the user means and what iCalendar, CalDAV and Google store. UTC instants are derived from it for range queries and ordering only. FreeCal's time zone rules are built into the app, so the Local Store records which rules the instants were computed with and recomputes them when an update brings new rules. A change such as the EU abolishing daylight saving time then keeps a summer meeting at 9:00 instead of moving it by an hour. All-day Events are stored as dates.

Every timed Event has a time zone. FreeCal has no floating times (a wall time without a zone, shown as that time wherever the user is): few calendar apps offer them, and an Event that moves when the user travels needs explaining. A floating time that arrives through import or CalDAV sync is pinned once:

1. To the time zone of the Calendar it arrives in (CalDAV's `calendar-timezone`, RFC 4791), otherwise to the Display Time Zone at that moment.
2. The pinned zone is kept, also when the Display Time Zone changes and when the server sends a new version of the Event that is still floating.
3. If only one of start and end is floating, it gets the zone of the other.
4. The original server data keeps its floating time. Only when the user changes the Event's time does FreeCal write the time with an explicit zone; a move to another Calendar or Account writes it with the pinned zone.
5. The Event's details and editor say that the original has no time zone, and the import preview says how many Events were pinned to which zone.

## Considered Options

- **UTC instants as the truth, with the zone recorded beside them** (as built in ticket 03): range queries are trivial, but a change of time zone rules moves every future Event, recurrence has to convert back to wall time, and a time in the hour skipped when clocks go forward can't be kept as entered.
- **Floating times as a third kind next to timed and all-day Events**: faithful to iCalendar, but their UTC instants change with the Display Time Zone, and users rarely expect them. Apple Calendar offers them only behind a setting. They can still be added later as an explicit time zone choice.
