# iCalendar and CalDAV fields

Every iCalendar component, property and common parameter (RFC 5545, with the additions of RFC 7986, 9073, 9074 and 6638 and the common `X-` extensions), and the WebDAV/CalDAV properties a calendar client meets (RFC 4918, 4791, 6578, 6638, 5397 and Apple's extensions).

**FreeCal** gives what FreeCal does with the field:

- **rw**: read into the Local Store and written back when sending a change.
- **read**: read but never written.
- **write**: written but not read.
- **none**: ignored. A `VEVENT` keeps the properties FreeCal doesn't know: an edit rewrites only FreeCal's own properties and the revision properties (`Group` and `Revision` in `core/src/sync/caldav/resource/write.rs`).

**Importance** is how much a calendar client that shows and edits events needs the field:

1. **Essential**: no event can be shown or synced correctly without it.
2. **High**: in most events; users notice when it's lost.
3. **Medium**: common in real calendars, or needed for one feature (reminders, invitations).
4. **Low**: occasional; most clients only keep it.
5. **Rare**: hardly used by any client or server.

| Field | Kind | Meaning | FreeCal | Importance |
| --- | --- | --- | --- | --- |
| `VCALENDAR` | component | The outer object around every iCalendar stream. | rw | 1 Essential |
| `VEVENT` | component | An event: something that takes up time on the calendar. | rw | 1 Essential |
| `UID` | property | The ID of an event, the same in every calendar and every copy; exceptions share it with their recurring event. | rw | 1 Essential |
| `DTSTART` | property | When the event starts: a date (all-day) or a date-time (UTC, floating or with a `TZID`). | rw | 1 Essential |
| `DTEND` | property | When the event ends (exclusive). Mutually exclusive with `DURATION`. | rw | 1 Essential |
| `DURATION` | property | How long the event lasts, in place of `DTEND`. | rw (read; written as `DTEND`) | 1 Essential |
| `SUMMARY` | property | The event's title. | rw | 1 Essential |
| `VTIMEZONE` | component | The definition of a time zone a `TZID` names: its offsets and when they change. | rw (FreeCal writes one for every zone it uses) | 1 Essential |
| `TZID` | property / parameter | The ID of a time zone: a property of `VTIMEZONE`, a parameter of date-time properties. | rw | 1 Essential |
| `VALUE` | parameter | The type of a property's value, e.g. `DATE` for all-day times. | rw | 1 Essential |
| `VERSION` | property | The iCalendar version, always `2.0`. | write | 1 Essential |
| `PRODID` | property | The product that made the `VCALENDAR`. | write | 1 Essential |
| `getetag` | DAV property | The version of a resource; sent in `If-Match` to avoid overwriting a newer one. | read | 1 Essential |
| `calendar-data` | CalDAV property | The iCalendar text of a resource, in a `REPORT` answer. | rw | 1 Essential |
| `current-user-principal` | DAV property | The URL of the logged-in user's principal (RFC 5397). | read | 1 Essential |
| `calendar-home-set` | CalDAV property | The collection under which the user's calendars live. | read | 1 Essential |
| `resourcetype` | DAV property | What a collection is; a calendar has `C:calendar`. | read | 1 Essential |
| `RRULE` | property | The rule an event repeats by (`FREQ`, `INTERVAL`, `COUNT`, `UNTIL`, `BYDAY`, `BYMONTHDAY`, `WKST`, …). | rw (rules FreeCal can't expand are kept, the event shown as its first occurrence) | 2 High |
| `RECURRENCE-ID` | property | Which occurrence of a recurring event an exception replaces. | rw | 2 High |
| `EXDATE` | property | Occurrences removed from a recurring event. | rw | 2 High |
| `LOCATION` | property | Where the event takes place, as text. | rw | 2 High |
| `DESCRIPTION` | property | The event's notes, as plain text. | rw | 2 High |
| `SEQUENCE` | property | How often the event was revised in a way that matters to attendees. | rw | 2 High |
| `DTSTAMP` | property | When this copy was created; without `METHOD`, when it was last changed. Required. | write | 2 High |
| `LAST-MODIFIED` | property | When the event was last changed. | write | 2 High |
| `STANDARD` / `DAYLIGHT` | component | The winter and summer offsets of a `VTIMEZONE`. | write | 2 High |
| `TZOFFSETFROM` / `TZOFFSETTO` | property | The UTC offsets before and after a change in a `VTIMEZONE`. | write | 2 High |
| `TZNAME` | property | The abbreviation of a `VTIMEZONE` offset, e.g. `CEST`. | write | 2 High |
| `X-WR-TIMEZONE` | property (Apple/Google) | The default time zone of a `VCALENDAR`; used for floating times. | read | 2 High |
| `displayname` | DAV property | The name of a calendar. | read | 2 High |
| `sync-token` | DAV property | The state of a collection; `sync-collection` reports the changes since a token (RFC 6578). | read | 2 High |
| `supported-calendar-component-set` | CalDAV property | Which components a calendar takes (`VEVENT`, `VTODO`, …). | read | 2 High |
| `current-user-privilege-set` | DAV property | What the user may do on a calendar, e.g. `write`. | read | 2 High |
| `STATUS` | property | `TENTATIVE`, `CONFIRMED` or `CANCELLED` for an event. | none | 2 High |
| `VALARM` | component | A reminder of an event: `ACTION`, `TRIGGER`, `DESCRIPTION`, … | none | 2 High |
| `TRIGGER` | property | When a `VALARM` goes off, relative to the start or end, or absolute. | none | 2 High |
| `ACTION` | property | What a `VALARM` does: `DISPLAY`, `AUDIO` or `EMAIL`. | none | 2 High |
| `ORGANIZER` | property | Who invited the attendees of a meeting. | none | 2 High |
| `ATTENDEE` | property | A person invited to the event, with `PARTSTAT`, `ROLE`, `CN`, `RSVP`, … | read (as Guests) | 2 High |
| `calendar-user-address-set` | CalDAV property | The user's own addresses (RFC 6638); FreeCal's Own Addresses. | read | 2 High |
| `TRANSP` | property | Whether the event blocks time: `OPAQUE` (busy) or `TRANSPARENT` (free). | none | 3 Medium |
| `CLASS` | property | `PUBLIC`, `PRIVATE` or `CONFIDENTIAL`: what others may see. | none | 3 Medium |
| `CREATED` | property | When the event was first created. | none | 3 Medium |
| `RDATE` | property | Extra occurrences of a recurring event; in a `VTIMEZONE`, more change dates. | write (in `VTIMEZONE` only) | 3 Medium |
| `URL` | property | A link about the event. | none | 3 Medium |
| `CATEGORIES` | property | Tags of the event. | none | 3 Medium |
| `X-ALT-DESC` | property (Microsoft) | The description as HTML, which Outlook and Thunderbird show in place of `DESCRIPTION`. | none (dropped when the description is edited) | 3 Medium |
| `X-LIC-LOCATION` | property (libical) | The IANA name of a `VTIMEZONE` with a custom `TZID`. | read | 3 Medium |
| `calendar-color` | DAV property (Apple) | The color of a calendar. | read | 3 Medium |
| `calendar-timezone` | CalDAV property | The default time zone of a calendar, as a `VTIMEZONE`. | read | 3 Medium |
| `CN` | parameter | The display name of an `ATTENDEE` or `ORGANIZER`. | none | 3 Medium |
| `PARTSTAT` | parameter | An attendee's answer: `NEEDS-ACTION`, `ACCEPTED`, `DECLINED`, `TENTATIVE`, `DELEGATED`. | none | 3 Medium |
| `ROLE` | parameter | `CHAIR`, `REQ-PARTICIPANT`, `OPT-PARTICIPANT` or `NON-PARTICIPANT`. | none | 3 Medium |
| `RSVP` | parameter | Whether the organizer wants an answer. | none | 3 Medium |
| `RANGE` | parameter | `THISANDFUTURE` on `RECURRENCE-ID`: the exception changes all later occurrences too. | none | 3 Medium |
| `METHOD` | property | The iTIP method of a scheduling message: `REQUEST`, `REPLY`, `CANCEL`, … Not allowed in CalDAV resources. | none | 3 Medium |
| `VTODO` | component | A task, with `DUE`, `COMPLETED`, `PERCENT-COMPLETE`, … | none | 3 Medium |
| `X-MICROSOFT-CDO-BUSYSTATUS` | property (Microsoft) | Outlook's free/busy state: `FREE`, `TENTATIVE`, `BUSY`, `OOF`. | none | 3 Medium |
| `X-APPLE-TRAVEL-ADVISORY-BEHAVIOR`, `X-APPLE-STRUCTURED-LOCATION` | property (Apple) | Apple's travel time and geocoded location. | none | 3 Medium |
| `ALTREP` | parameter | A link to another form of a text, e.g. an HTML description. | none | 4 Low |
| `LANGUAGE` | parameter | The language of a text. | none | 4 Low |
| `ATTACH` | property | A file attached to the event, as a URI or inline. | none | 4 Low |
| `PRIORITY` | property | Priority 0 to 9 (1 highest). | none | 4 Low |
| `COMMENT` | property | A comment, mostly in scheduling replies. | none | 4 Low |
| `CONTACT` | property | Contact information for the event. | none | 4 Low |
| `GEO` | property | Latitude and longitude of the event. | none | 4 Low |
| `RELATED-TO` | property | A related component, e.g. a task's parent. | none | 4 Low |
| `REPEAT` | property | How often a `VALARM` repeats after it goes off. | none | 4 Low |
| `ACKNOWLEDGED` | property | When a `VALARM` was dismissed (RFC 9074). | none | 4 Low |
| `RELATED` | parameter | Whether a `TRIGGER` counts from the start or the end. | none | 4 Low |
| `COLOR` | property | The color of an event or calendar (RFC 7986). | none | 4 Low |
| `CONFERENCE` | property | A video or phone conference link (RFC 7986). | none | 4 Low |
| `NAME` | property | The name of a `VCALENDAR` (RFC 7986). | none | 4 Low |
| `X-WR-CALNAME`, `X-WR-CALDESC` | property (Apple/Google) | The name and description of a `VCALENDAR`. | none | 4 Low |
| `REFRESH-INTERVAL`, `X-PUBLISHED-TTL` | property | How often a subscribed calendar should be fetched again. | none | 4 Low |
| `SOURCE` | property | Where a subscribed calendar can be fetched again (RFC 7986). | none | 4 Low |
| `TZURL` | property | Where the latest form of a `VTIMEZONE` lives. | none | 4 Low |
| `X-MOZ-*` | property (Mozilla) | Thunderbird's state, e.g. `X-MOZ-LASTACK`, `X-MOZ-SNOOZE-TIME`, `X-MOZ-GENERATION`. | none | 4 Low |
| `X-MICROSOFT-CDO-*` (others) | property (Microsoft) | Outlook's all-day flag, importance, appointment sequence, … | none | 4 Low |
| `X-GOOGLE-CONFERENCE` | property (Google) | The Google Meet link. | none | 4 Low |
| `DELEGATED-TO`, `DELEGATED-FROM`, `MEMBER`, `CUTYPE`, `SENT-BY`, `DIR` | parameter | More about an attendee or organizer: delegation, groups, kind, who sent for them, directory entry. | none | 4 Low |
| `SCHEDULE-AGENT`, `SCHEDULE-STATUS`, `SCHEDULE-FORCE-SEND` | parameter | Whether the server or the client sends invitations, and how they fared (RFC 6638). | none | 4 Low |
| `schedule-inbox-URL`, `schedule-outbox-URL` | CalDAV property | Where the server delivers and takes invitations (RFC 6638). | none | 4 Low |
| `getctag` | DAV property (CalendarServer) | The state of a collection, before `sync-token`. | none | 4 Low |
| `calendar-description` | CalDAV property | The description of a calendar. | none | 4 Low |
| `calendar-order` | DAV property (Apple) | The order of the calendars in a client. | none | 4 Low |
| `getcontenttype` | DAV property | The media type of a resource, `text/calendar`. | none | 4 Low |
| `owner` | DAV property | The principal that owns a calendar. | none | 4 Low |
| `max-resource-size`, `min-date-time`, `max-date-time`, `max-instances`, `max-attendees-per-instance` | CalDAV property | Limits of a calendar server. | none | 4 Low |
| `supported-calendar-data` | CalDAV property | The media types a calendar takes. | none | 4 Low |
| `RESOURCES` | property | Equipment or rooms needed. | none | 5 Rare |
| `IMAGE` | property | A picture for the event or calendar (RFC 7986). | none | 5 Rare |
| `EXRULE` | property | A rule for removed occurrences; deprecated by RFC 5545. | none | 5 Rare |
| `REQUEST-STATUS` | property | The result of a scheduling request. | none | 5 Rare |
| `VJOURNAL` | component | A journal entry. | none | 5 Rare |
| `VFREEBUSY` | component | Free/busy time, with `FREEBUSY` periods. | none | 5 Rare |
| `FREEBUSY` | property | Busy or free periods in a `VFREEBUSY`. | none | 5 Rare |
| `FBTYPE` | parameter | The kind of a `FREEBUSY` period. | none | 5 Rare |
| `DUE`, `COMPLETED`, `PERCENT-COMPLETE` | property | A task's due date, completion and progress. | none | 5 Rare (for events) |
| `CALSCALE` | property | The calendar system; only `GREGORIAN` exists. | none | 5 Rare |
| `ENCODING`, `FMTTYPE` | parameter | The encoding and media type of an inline `ATTACH`. | none | 5 Rare |
| `RELTYPE` | parameter | The kind of a `RELATED-TO`: `PARENT`, `CHILD`, `SIBLING`. | none | 5 Rare |
| `VAVAILABILITY`, `AVAILABLE` | component | When a user is available (RFC 7953). | none | 5 Rare |
| `VLOCATION`, `VRESOURCE`, `PARTICIPANT` | component | Structured locations, resources and participants (RFC 9073). | none | 5 Rare |
| `STYLED-DESCRIPTION`, `STRUCTURED-DATA` | property | A rich-text description and attached structured data (RFC 9073). | none | 5 Rare |
| `PROXIMITY` | property | An alarm that goes off on arriving or leaving (RFC 9074). | none | 5 Rare |
| `TZUNTIL`, `TZID-ALIAS-OF` | property | When a `VTIMEZONE` stops being valid, and other names of it (RFC 7808). | none | 5 Rare |
| `ESTIMATED-DURATION` | property | How long a task is expected to take (RFC 9253 draft). | none | 5 Rare |
