# Google Calendar over CalDAV

Researched 2026-11-15 for ADR 0002 (Google via its REST API): how does Google's CalDAV endpoint really behave, and what would FreeCal gain or lose by using it as the Provider for Google instead of the Calendar REST API? Source links point at the commit cloned that day; Thunderbird links point at the comm-central tip of that day. Nothing was tested against a Google account. "Unknown" means no primary source was found.

Each claim is marked as one of:

- **Documented**: stated by Google on a page it owns.
- **Observed**: seen by a named client, in its code or bug tracker, with the date. Old observations may no longer hold.
- **Unknown**: no primary source.

## Summary

| # | Question | Answer | Confidence |
|---|---|---|---|
| 1 | Fidelity of a PUT | Google does not store the resource it is given: it converts to its own event model and generates the iCalendar on GET. It adds STATUS, TRANSP, timestamps, the Calendar's owner as attendee and, when the event has no VALARM, the Calendar's default reminders. It rewrites DTSTAMP, wants HTML in DESCRIPTION, drops `ACTION:AUDIO` alarms and VTODO/VJOURNAL. Custom `X-` properties were dropped in 2008 and relied on in 2012; today's rule is unknown, as is the fate of CATEGORIES, ATTACH, RDATE, unknown parameters, absolute triggers. | Observed (mostly 2012 to 2014), partly documented, much unknown |
| 2 | Sync | `sync-collection` (RFC 6578) is supported and mandatory after the first sync; ctag and ETags exist; `If-Match` is the only `If*` header supported. Token lifetime and the response to an expired token are not documented. `If-None-Match: *` on an existing UID was seen to give HTTP 500. | Documented; expiry unknown |
| 3 | Colours and conferences | Per-event colours are not carried over CalDAV. Conference details arrive as generated text in the description (2017 announcement); a dedicated property in the CalDAV output could not be confirmed. No source shows a way to create a Meet link over CalDAV. | Observed (colours, 2024 and 2026); documented in part (conference text); creation unknown |
| 4 | Guests | Inbox exists but is always empty; invitations land directly in the events collection; no free-busy. No control like `sendUpdates` is documented. One 2014 report: a PARTSTAT change mails the organiser, `SCHEDULE-AGENT=CLIENT` is ignored, adding an invitation sends nothing. Whether a PUT with new ATTENDEEs mails the guests today is unknown. | Documented (inbox); observed 2014 (mail); otherwise unknown |
| 5 | Google-only things | Default reminders are applied by the server when a new event has no VALARM (workaround: a dummy `ACTION:NONE` alarm). Birthday events arrive as ordinary events with nothing marking them. No source shows guest permissions, extended properties, event types or visibility `default` in the CalDAV output. | Observed (reminders 2012 to 2014, birthdays 2025); otherwise unknown |
| 6 | Discovery | Principal `https://apidata.googleusercontent.com/caldav/v2/CALENDAR_ID/user`, collection `.../CALENDAR_ID/events`. No MKCALENDAR, no WebDAV ACL. Shared Calendars are listed only after the user ticks them on a Google settings page (`syncselect`). The v1 endpoint `https://www.google.com/calendar/dav` is deprecated. | Documented; shared Calendars observed (2021 to 2026) |
| 7 | Operations | OAuth 2.0 only (Basic gives 401); every client found uses the full scope `https://www.googleapis.com/auth/calendar`; whether a read-only scope works is undocumented. Same quota as the REST API (600 requests per minute per user per project). Each developer must enable the "CalDAV API" in a Google Cloud project. The v2 guide was last updated 2026-10-09; no deprecation notice for v2. | Documented; read-only scope unknown |
| 8 | REST comparison | `events.patch` changes only the fields sent; arrays are replaced whole. `events.insert` cannot set `organizer`, `hangoutLink`, `recurringEventId` and other read-only fields; `events.import` sets `iCalUID` and `organizer`. REST gives each Exception its own resource with `recurringEventId` and `originalStartTime`; CalDAV gives one resource per UID holding the Recurring Event and its Exceptions. | Documented |
| 9 | Other clients | CalDAV: Thunderbird (built in), Evolution/EDS, GNOME Online Accounts, DAVx5, vdirsyncer, SyncEvolution, Apple Calendar. REST: Thunderbird's add-on "Provider for Google Calendar", KDE's Google resource. The CalDAV clients record 400/403/409/500 responses without explanation, rate-limit trouble and missing shared Calendars. | Observed (source code, 2026-11) |

## 1. Fidelity of a PUT

### Documented by Google

The [CalDAV API Developer's Guide](https://developers.google.com/workspace/calendar/caldav/v2/guide) (last updated 2026-10-09) lists what is not supported, per specification:

> Doesn't support arbitrary (user-defined) WebDAV properties.

> Doesn't support the AUDIO action.

> Doesn't support VTODO or VJOURNAL data.

> Doesn't support the Apple iCal extension to allow user-settable URL properties.

> Although our CalDAV implementation doesn't cover every specification, it works correctly for many clients, including Apple Calendar.

The first quote is about WebDAV properties (PROPPATCH), not iCalendar `X-` properties. The guide says nothing about `X-` properties, parameters, CATEGORIES, ATTACH, RDATE, ORGANIZER, SEQUENCE or DTSTAMP.

### Observed

| What | Behaviour | Source, date |
|---|---|---|
| GET after PUT | Not the bytes sent. A bare VEVENT came back with added DTSTAMP, CREATED, LAST-MODIFIED, `STATUS:CONFIRMED`, `TRANSP:OPAQUE` and two VALARMs; UID and `SEQUENCE:1` were kept. | [Mozilla bug 1018834](https://bugzilla.mozilla.org/show_bug.cgi?id=1018834) comment 9, 2014-06-11 |
| PRODID and wrapper | Output carries `PRODID:-//Google Inc//Google Calendar 70.9054//EN`, `CALSCALE`, `X-WR-CALNAME`, `X-WR-TIMEZONE`; UIDs Google makes end in `@google.com`. | [davx5-ose#1339](https://github.com/bitfireAT/davx5-ose/issues/1339), 2025-03-10 |
| VALARM, none sent | Google adds the Calendar's default reminders: `ACTION:EMAIL` with `TRIGGER:-P0DT0H10M0S` and `ACTION:DISPLAY` with `TRIGGER:-P0DT0H30M0S` in the report. | Bug 1018834 (2014-06-11); [bug 1059597](https://bugzilla.mozilla.org/show_bug.cgi?id=1059597) (2014-08-28) |
| VALARM, relative | Returned in Google's own duration form (`-P0DT0H30M0S`). Whether several alarms, and absolute triggers, survive: unknown. REST allows at most 5 overrides, methods `email` and `popup`, 0 to 40320 minutes before the start ([Events resource](https://developers.google.com/workspace/calendar/api/v3/reference/events), 2026-07-07), so an absolute trigger or one after the start has no place in Google's model. | Bug 1018834; inference from the REST model |
| `X-` properties | 2008 (v1 endpoint): "google drops all X-MOX props", so dismissed alarms could not be stored. 2012: SyncEvolution relies on the server "preserving X- extensions" for its `X-SYNCEVOLUTION-RECURRENCE-ID`, but lists `X-EVOLUTION-ALARM-UID` (inside VALARM) as not preserved. 2021: a Thunderbird developer suspects the dismissal state "gets lost on the server". The two disagree; today's rule is unknown. | [Mozilla bug 451821](https://bugzilla.mozilla.org/show_bug.cgi?id=451821) (2008-08-23); SyncEvolution [CalDAVSource.h#L156](https://gitlab.freedesktop.org/SyncEvolution/syncevolution/-/blob/dac2d9e25d44f234fda00b44542fcecbf53589a5/src/backends/webdav/CalDAVSource.h#L156), [synccompare.pl#L482-491](https://gitlab.freedesktop.org/SyncEvolution/syncevolution/-/blob/dac2d9e25d44f234fda00b44542fcecbf53589a5/test/synccompare.pl#L482-491); [Mozilla bug 1709055](https://bugzilla.mozilla.org/show_bug.cgi?id=1709055) comment 6 (2021-06-09) |
| DTSTAMP | "Google insists on replacing the original DTSTAMP and checks it (409, "Can only store an event with a newer DTSTAMP")". | SyncEvolution [CalDAVSource.cpp#L454-463](https://gitlab.freedesktop.org/SyncEvolution/syncevolution/-/blob/dac2d9e25d44f234fda00b44542fcecbf53589a5/src/backends/webdav/CalDAVSource.cpp#L454-463), comment from about 2012 |
| SEQUENCE | Client must "increase sequence number if smaller or equal to sequence on server. Server rejects update otherwise"; SEQUENCE and TRANSP listed as not preserved. | SyncEvolution [CalDAVSource.cpp#L599-601](https://gitlab.freedesktop.org/SyncEvolution/syncevolution/-/blob/dac2d9e25d44f234fda00b44542fcecbf53589a5/src/backends/webdav/CalDAVSource.cpp#L599-601), synccompare.pl as above, about 2012 |
| ATTENDEE | "Google adds calendar owner as attendee of meetings, regardless whether it was on the original attendee list". Output form: `ATTENDEE;CUTYPE=INDIVIDUAL;ROLE=REQ-PARTICIPANT;PARTSTAT=ACCEPTED;CN=...;X-NUM-GUESTS=0:mailto:...`. | synccompare.pl as above (about 2012); davx5-ose#1339 (2025-03-10) |
| ORGANIZER | Events without a real organiser come back with `ORGANIZER;CN=Unknown Organizer:mailto:unknownorganizer@calendar.google.com`. Whether a client-supplied ORGANIZER is kept: unknown. | davx5-ose#1339 (2025-03-10) |
| DESCRIPTION | "Google expects item descriptions to be bare HTML in violation of spec, rather than using the standard Alternate Text Representation"; Thunderbird rewrites descriptions for Google. | Thunderbird [CalDavRequest.sys.mjs L1260-1280](https://hg.mozilla.org/comm-central/file/tip/calendar/providers/caldav/modules/CalDavRequest.sys.mjs), tip 2026-11-15 |
| VTIMEZONE | A redundant VTIMEZONE is added for all-day events. | SyncEvolution [synccompare.pl#L373-376](https://gitlab.freedesktop.org/SyncEvolution/syncevolution/-/blob/dac2d9e25d44f234fda00b44542fcecbf53589a5/test/synccompare.pl#L373-376), about 2012 |
| UID and href | The href is `.../events/<name>.ics`; Evolution does "not try twice with Google, it's either with ".ics" extension or not there". A PUT of a UID that another app already stored is merged into the existing event ("server merges the data (Google)"). A UID with a leading space led to a resource that could not be found (vdirsyncer#734, title only, not read). Whether Google ever changes a client's UID: unknown. | EDS [e-cal-backend-caldav.c#L1719-1721](https://gitlab.gnome.org/GNOME/evolution-data-server/-/blob/3b587d11d904ca1a0ecd09a5baaf89df8c99d1f1/src/calendar/backends/caldav/e-cal-backend-caldav.c#L1719-1721); SyncEvolution [CalDAVSource.cpp#L495](https://gitlab.freedesktop.org/SyncEvolution/syncevolution/-/blob/dac2d9e25d44f234fda00b44542fcecbf53589a5/src/backends/webdav/CalDAVSource.cpp#L495) |
| Exception without its Recurring Event | "Google gets confused by a single detached event without parent" (a RECURRENCE-ID component with no master). | SyncEvolution CalDAVSource.h#L156 as above, about 2012 |
| Recurring Event with Exceptions | Synced "fine both ways" in DAVx5's test. | [synctools#285](https://github.com/bitfireAT/synctools/issues/285), 2026-04-15 |
| Silent partial store | As a guest, a PUT changing PARTSTAT and RRULE returned success; Google stored the PARTSTAT and kept the old RRULE. | [Mozilla bug 2086042](https://bugzilla.mozilla.org/show_bug.cgi?id=2086042), 2026-10-27, unconfirmed |
| Events someone else organises | Writes are refused: 403 "You don't have access to change that event" (2012), HTTP 400 which Evolution answers by "Pretend success ... and save only locally (like added reminders)", 409 "Can't delete a recurring event except on its organizer's calendar". Dismissing a reminder on such an event fails in Thunderbird. | SyncEvolution [CalDAVSource.cpp#L678](https://gitlab.freedesktop.org/SyncEvolution/syncevolution/-/blob/dac2d9e25d44f234fda00b44542fcecbf53589a5/src/backends/webdav/CalDAVSource.cpp#L678), [#L864](https://gitlab.freedesktop.org/SyncEvolution/syncevolution/-/blob/dac2d9e25d44f234fda00b44542fcecbf53589a5/src/backends/webdav/CalDAVSource.cpp#L864); EDS [e-cal-backend-caldav.c#L1943-1946](https://gitlab.gnome.org/GNOME/evolution-data-server/-/blob/3b587d11d904ca1a0ecd09a5baaf89df8c99d1f1/src/calendar/backends/caldav/e-cal-backend-caldav.c#L1943-1946); [Mozilla bug 1974438](https://bugzilla.mozilla.org/show_bug.cgi?id=1974438) (2025-06-27), [bug 1974641](https://bugzilla.mozilla.org/show_bug.cgi?id=1974641) (2025-06-29) |
| Unexplained rejections | "It's generally a known issue that Google calendar rejects some events without much further notice" (vdirsyncer maintainer, 2017). HTTP 400 when changing only the title of an event created on Android (2026, open). | [vdirsyncer#692](https://github.com/pimutils/vdirsyncer/issues/692) (2017-12-19); [Evolution#3364](https://gitlab.gnome.org/GNOME/evolution/-/issues/3364) (2026-07-30) |

### Unknown

CATEGORIES, ATTACH, RDATE, unknown parameters, several VALARMs, absolute triggers, and custom `X-` properties on the v2 endpoint today. Thunderbird's REST add-on stores its categories in a shared extended property (`X-MOZ-CATEGORIES`, [items.js](https://github.com/kewisch/gdata-provider/blob/eff914999cd6c1f57c02fc6749c89fd973beba18/src/background/items.js)), which shows that Google's event model has no categories field, but not what the CalDAV endpoint does with the property. DAVx5 has an open issue "CalDAV attachment support" ([davx5-ose#2031](https://github.com/bitfireAT/davx5-ose/issues/2031), not read).

## 2. Sync

### Documented by Google

From the CalDAV guide (2026-10-09), on RFC 6578:

> Client applications must switch to this mode of operation after the initial sync.

On the ctag: it "changes when anything in the calendar has changed". On RFC 4918:

> Doesn't support the HTTP methods LOCK, UNLOCK, COPY, MOVE, MKCOL, or the If* header (except for If-Match).

Read literally, `If-None-Match` is not supported. REPORT is supported: "All reports except free-busy-query are implemented."

The guide does not say how long a sync token lives or what the server returns for an expired one. RFC 6578 section 3.8 requires a failure with the `DAV:valid-sync-token` precondition; whether Google does that is unknown. For REST the rule is documented ([sync guide](https://developers.google.com/workspace/calendar/api/guides/sync), 2026-11-07): "Sometimes sync tokens are invalidated by the server, for various reasons including token expiration or changes in related ACLs", answered with 410 and followed by a full sync.

### Observed

| What | Behaviour | Source, date |
|---|---|---|
| sync-collection | Enabled on the v2 endpoint in September 2013; Google requires the `sync-level` element (400 without it). | [Mozilla bug 920755](https://bugzilla.mozilla.org/show_bug.cgi?id=920755), 2013-09-25 |
| `If-None-Match: *` | A PUT for a UID Google already holds returned a bare HTTP 500 instead of 412 or 409 `no-uid-conflict`. | [Mozilla bug 2080670](https://bugzilla.mozilla.org/show_bug.cgi?id=2080670), 2026-10-12, unconfirmed |
| `If-Match` | Sent by vdirsyncer with ETags of the form `"63647421014"`; an update with it returned 409 for some events, reproducible "by altering an existing recurring event locally". One user's workaround: delete and create again. | [vdirsyncer#692](https://github.com/pimutils/vdirsyncer/issues/692) (2017); [vdirsyncer#963](https://github.com/pimutils/vdirsyncer/issues/963) (2022-01-18, open, comment 2024-06-26); [vdirsyncer#613](https://github.com/pimutils/vdirsyncer/issues/613) (comment 2022-01-18) |
| Listing and fetch disagree | Google returns 404 on GET or multiget for items which a calendar-query lists; some items have no VEVENT. | SyncEvolution [CalDAVSource.cpp#L223-228](https://gitlab.freedesktop.org/SyncEvolution/syncevolution/-/blob/dac2d9e25d44f234fda00b44542fcecbf53589a5/src/backends/webdav/CalDAVSource.cpp#L223-228), about 2012 |
| Rate limits break sync | Thunderbird forces a one-time full resync of Google Calendars because "Some users' calendars got into a bad state due to Google rate-limit problems". | Thunderbird [CalCalendarManager.sys.mjs L645-711](https://hg.mozilla.org/comm-central/file/tip/calendar/base/src/CalCalendarManager.sys.mjs), tip 2026-11-15; [Mozilla bug 1789424](https://bugzilla.mozilla.org/show_bug.cgi?id=1789424) (2022-09-06) |
| Reminder edits | In REST, "changing reminders does not also change the updated property" (Events resource). Whether the CalDAV ETag changes then: unknown. | Documented for REST only |

## 3. Colours and conferences

| What | Behaviour | Confidence, source |
|---|---|---|
| Per-event colour, read | Not shown in Thunderbird over CalDAV. | Observed: [Mozilla bug 1893099](https://bugzilla.mozilla.org/show_bug.cgi?id=1893099), 2024-04-24 |
| Per-event colour, write | DAVx5 sends a colour; after the next sync it is gone. Maintainer: "Most likely specific colors for events are not supported by Google via their CalDAV backend ... it gets back the event without the stored color." | Observed: [davx5-ose#2728](https://github.com/bitfireAT/davx5-ose/issues/2728), 2026-07-30 |
| Conference, read | "whenever someone uses a third-party CalDAV-based calendaring client to access an event on Google Calendar, we will add automatically generated text to the event's description. This text will include instructions on how to join the event through Meet or Hangouts." | Documented: [Google Workspace Updates](https://workspaceupdates.googleblog.com/2017/08/making-it-easier-to-join-meetings-from.html), 2017-08-10 |
| Conference as a property | Google's ICS export is said to carry `X-GOOGLE-CONFERENCE:<url>`, and a 2025 Workspace Updates post is said to put the Meet link into the location for other calendar apps. Neither was confirmed in a CalDAV GET or read at the source. | Unknown (secondary leads only) |
| Conference, create | No source describes a way. In REST it needs `conferenceData.createRequest` plus the request parameter `conferenceDataVersion=1` ([events.insert](https://developers.google.com/workspace/calendar/api/v3/reference/events/insert), 2026-07-07); CalDAV has no place for either. RFC 7986 defines COLOR and CONFERENCE properties; no source shows Google reading them. | Unknown; absence inferred |

The generated description text means a client that writes an event back unchanged sends that text to Google as the description. What Google does with it is unknown.

## 4. Guests

### Documented by Google

From the CalDAV guide (2026-10-09), on RFC 6638:

> Supports a trivial "inbox," which is always empty.

> Invitations you receive are automatically delivered into your "events" collection rather than being placed into your "inbox."

> Doesn't support free-busy lookup.

Nothing is said about an outbox, about `SCHEDULE-AGENT`, or about when mail is sent. There is no counterpart to REST's `sendUpdates` (`all`, `externalOnly`, `none`; [events.patch](https://developers.google.com/workspace/calendar/api/v3/reference/events/patch), 2026-07-01).

### Observed

| What | Behaviour | Source, date |
|---|---|---|
| Mail on PUT | Adding or deleting an invitation over CalDAV sent nothing; changing the own PARTSTAT made Google mail the organiser. `SCHEDULE-AGENT=CLIENT` on ORGANIZER "didn't change Google's behaviour". OPTIONS did not advertise `calendar-auto-schedule`. | [Mozilla bug 997784](https://bugzilla.mozilla.org/show_bug.cgi?id=997784), 2014-04-17, unconfirmed |
| How a guest answers | By a PUT of the event with the own ATTENDEE's PARTSTAT changed (there is no inbox to act on). It can fail: HTTP 400 when accepting invitations from Microsoft 365. | Bug 997784; [Mozilla bug 2076603](https://bugzilla.mozilla.org/show_bug.cgi?id=2076603), [bug 2058863](https://bugzilla.mozilla.org/show_bug.cgi?id=2058863) (2026) |
| Side effects | An accepted invitation was stored with public visibility. A guest's other changes are dropped without an error (bug 2086042, section 1). | [Mozilla bug 1952157](https://bugzilla.mozilla.org/show_bug.cgi?id=1952157), 2025-03-06 |
| Attendees not saved | Reported in 2021 (title only, not read). | [Mozilla bug 1693903](https://bugzilla.mozilla.org/show_bug.cgi?id=1693903) |
| Scheduling modes in vdirsyncer | A pull request "add Google Calendar scheduling modes" was closed; vdirsyncer "does not make changes to items". | [vdirsyncer#1230](https://github.com/pimutils/vdirsyncer/pull/1230), 2026-09-11 |

### Unknown

Whether a PUT that adds ATTENDEEs to an event the user organises sends invitation mail today, and whether anything suppresses it.

## 5. Google-only things

| Thing | Over CalDAV | Confidence, source |
|---|---|---|
| Default reminders | No marker like REST's `reminders.useDefault`. The server adds the default reminders to a new event without VALARM, and they come back as plain VALARMs. Workaround from 2012: "Avoid that by adding a dummy VALARM with ACTION=NONE. Must include a fake TRIGGER. This is what iOS does." Removing the defaults in Google's web UI also stops it. | Observed: SyncEvolution [CalDAVSource.cpp#L405-408](https://gitlab.freedesktop.org/SyncEvolution/syncevolution/-/blob/dac2d9e25d44f234fda00b44542fcecbf53589a5/src/backends/webdav/CalDAVSource.cpp#L405-408) (about 2012); Mozilla bug 1059597 (2014-08-28). Whether it still holds: unknown |
| Event types | Birthdays appear in the primary Calendar since November 2024 as ordinary events (`CLASS:PRIVATE`, `TRANSP:TRANSPARENT`, unknown organiser); the maintainer found no way to tell them apart. Their reminders cannot be dismissed in Thunderbird. Focus time, out of office, working location: unknown. In REST, `eventType` "cannot be modified after the event is created". | Observed: [davx5-ose#1339](https://github.com/bitfireAT/davx5-ose/issues/1339) (2025-03-10); [Mozilla bug 1971160](https://bugzilla.mozilla.org/show_bug.cgi?id=1971160) (2025) |
| Guest permissions (`guestsCanModify`, `guestsCanInviteOthers`, `guestsCanSeeOtherGuests`) | No source shows them in the CalDAV output. | Unknown |
| Extended properties | No source shows them. | Unknown |
| Visibility `default` | REST has `default`, `public`, `private`, `confidential`. Over CalDAV only CLASS is seen (`CLASS:PRIVATE` on birthdays); how `default` is written, and whether it differs from `public`, is unknown. | Unknown |
| Per-attendee extra guests | `X-NUM-GUESTS` parameter on ATTENDEE. | Observed: davx5-ose#1339 (2025-03-10) |
| Tasks | Not available: "Doesn't support VTODO or VJOURNAL data". | Documented: CalDAV guide |

Thunderbird's REST add-on shows which Google fields have no iCalendar home: it keeps them as `X-GOOGLE-CONFDATA`, `X-DEFAULT-ALARM` and `X-GOOGLE-EVENT-TYPE` on its own items ([items.js](https://github.com/kewisch/gdata-provider/blob/eff914999cd6c1f57c02fc6749c89fd973beba18/src/background/items.js)). These names are the add-on's, not Google's.

## 6. Discovery

### Documented by Google

From the CalDAV guide (2026-10-09):

- Principal: `https://apidata.googleusercontent.com/caldav/v2/CALENDAR_ID/user`. Collection: `https://apidata.googleusercontent.com/caldav/v2/CALENDAR_ID/events`. The primary Calendar's ID is the user's email address.
- > Warning: The old https://www.google.com/calendar/dav endpoint is deprecated and is no longer supported; use it at your own risk.
- > Doesn't support the HTTP method MKCALENDAR.
- > Doesn't support WebDAV Access Control (rfc3744).
- Requests with `calendar-proxy-read-for` or `calendar-proxy-write-for` "will fail with an iOS UserAgent".
- PROPPATCH is supported, but not for user-defined properties.

So a Calendar cannot be created over CalDAV. Deleting one is not mentioned.

### Observed

| What | Behaviour | Source, date |
|---|---|---|
| Discovery start | Thunderbird, DAVx5 and GNOME Online Accounts all start at `.../caldav/v2/<email>/user`. | Thunderbird [CalDavProvider.sys.mjs L267-284](https://hg.mozilla.org/comm-central/file/tip/calendar/providers/caldav/CalDavProvider.sys.mjs); DAVx5 [OAuthGoogle.kt#L19-L31](https://github.com/bitfireAT/davx5-ose/blob/8415799527c45b8e605ad29d7f0ee03f8bfccc89/core/src/main/kotlin/at/bitfire/davdroid/network/OAuthGoogle.kt#L19-L31); GOA [goagoogleprovider.c#L341](https://gitlab.gnome.org/GNOME/gnome-online-accounts/-/blob/fa76dfad7c018c10c5efbb0ba911364452215da4/src/goabackend/goagoogleprovider.c#L341) |
| Shared Calendars | Not listed until the user ticks them at `https://calendar.google.com/calendar/syncselect`, which vdirsyncer calls "a secret settings page". One 2026 report says only owned Calendars are seen. | [vdirsyncer config.rst#L272-L375](https://github.com/pimutils/vdirsyncer/blob/f8328610e458fbcdb92e20c14412911afc1d974c/docs/config.rst#L272-L375); [Mozilla bug 1735525](https://bugzilla.mozilla.org/show_bug.cgi?id=1735525) (2021-10-13, comment 2023-01-12); [bug 1988835](https://bugzilla.mozilla.org/show_bug.cgi?id=1988835) (2025-09-16); [vdirsyncer#1206](https://github.com/pimutils/vdirsyncer/issues/1206) (2026-01-15, open) |
| Collection names | "Neither Google nor iCloud use URI, only something like path"; the collection identifier of a secondary Calendar is a generated ID. | EDS [e-cal-backend-caldav.c#L1130](https://gitlab.gnome.org/GNOME/evolution-data-server/-/blob/3b587d11d904ca1a0ecd09a5baaf89df8c99d1f1/src/calendar/backends/caldav/e-cal-backend-caldav.c#L1130); vdirsyncer [tutorial.rst#L191](https://github.com/pimutils/vdirsyncer/blob/f8328610e458fbcdb92e20c14412911afc1d974c/docs/tutorial.rst#L191) |
| OPTIONS | "Some servers, notably Google, allow OPTIONS when not authorized". | EDS [e-cal-backend-caldav.c#L360](https://gitlab.gnome.org/GNOME/evolution-data-server/-/blob/3b587d11d904ca1a0ecd09a5baaf89df8c99d1f1/src/calendar/backends/caldav/e-cal-backend-caldav.c#L360) |
| v1 endpoint | Evolution still uses `https://www.google.com/calendar/dav/` when the account has no OAuth, v2 otherwise. | EDS [module-google-backend.c#L504-511](https://gitlab.gnome.org/GNOME/evolution-data-server/-/blob/3b587d11d904ca1a0ecd09a5baaf89df8c99d1f1/src/modules/google-backend/module-google-backend.c#L504-511) |

### Unknown

Which privileges Google reports in `current-user-privilege-set` for a read-only shared Calendar (Evolution reads that property to decide whether a Calendar is writable, [e-cal-backend-caldav.c#L280-293](https://gitlab.gnome.org/GNOME/evolution-data-server/-/blob/3b587d11d904ca1a0ecd09a5baaf89df8c99d1f1/src/calendar/backends/caldav/e-cal-backend-caldav.c#L280-293), for every server; nothing there is specific to Google). Whether a Calendar's colour and name can be read and written as properties. Whether DELETE on a collection works.

## 7. Operations

| What | Fact | Confidence, source |
|---|---|---|
| Authentication | HTTPS and OAuth 2.0 only: "Attempting to connect over HTTP or using Basic Authentication results in an HTTP 401". | Documented: CalDAV guide (2026-10-09) |
| Passwords | "CalDAV, CardDAV, IMAP, POP and Google Sync will no longer work when signing in with just a password"; final date 2025-05-01 for Workspace. The post mentions app passwords as a fallback for apps without OAuth, which contradicts the guide's 401 for Basic; DAVx5 states "Since June 2023, Google has disabled all authentication methods except OAuth". Whether an app password still works over CalDAV: unknown. | Documented, conflicting: [Workspace Updates](https://workspaceupdates.googleblog.com/2023/09/winding-down-google-sync-and-less-secure-apps-support.html) (updated 2025-04-29); observed: [DAVx5 "Tested with Google"](https://www.davx5.com/tested-with/google) (June 2023) |
| Scope | The guide names none; its auth link leads to the REST scope list ([auth page](https://developers.google.com/workspace/calendar/api/auth), 2026-09-03). Evolution, GNOME Online Accounts, DAVx5 and vdirsyncer all request `https://www.googleapis.com/auth/calendar`. Whether `calendar.readonly` or `calendar.events` is accepted by the CalDAV endpoint: unknown. | Observed: EDS [e-oauth2-service-google.c#L184-194](https://gitlab.gnome.org/GNOME/evolution-data-server/-/blob/3b587d11d904ca1a0ecd09a5baaf89df8c99d1f1/src/libedataserver/e-oauth2-service-google.c#L184-194), GOA [goagoogleprovider.c#L137-138](https://gitlab.gnome.org/GNOME/gnome-online-accounts/-/blob/fa76dfad7c018c10c5efbb0ba911364452215da4/src/goabackend/goagoogleprovider.c#L137-138), DAVx5 OAuthGoogle.kt, vdirsyncer [google.py#L170-L173](https://github.com/pimutils/vdirsyncer/blob/f8328610e458fbcdb92e20c14412911afc1d974c/vdirsyncer/storage/google.py#L170-L173) |
| Project setup | The "CalDAV API" must be enabled in a Google Cloud project, separately from the Calendar API. An OAuth consent screen left in "Testing" mode makes tokens expire after 7 days. | Documented: CalDAV guide; observed: vdirsyncer config.rst, DAVx5 page |
| Quota | "The CalDAV API has the same quota limits as the Calendar API": 10,000 requests per minute per project, 600 per minute per user per project, sliding window; 1,000,000 requests per day per project as billing threshold; "Exceeding the quota request limits is planned to incur charges to your Google Cloud billing account later in 2026". | Documented: CalDAV guide; [quota page](https://developers.google.com/workspace/calendar/api/guides/quota) (2026-11-11) |
| Rate-limit errors | REST documents 403 `usageLimits` or 429. Over CalDAV a 403 with the same `usageLimits` JSON is seen; vdirsyncer retries on it. Evolution notes that a 404 "counts to the Error requests quota limit". | Documented for REST; observed: vdirsyncer [http.py#L177-L183](https://github.com/pimutils/vdirsyncer/blob/f8328610e458fbcdb92e20c14412911afc1d974c/vdirsyncer/http.py#L177-L183); EDS e-cal-backend-caldav.c#L1719-1721 |
| Other errors | 400 without detail, 403, 409, and 500 with an empty `<D:error/>` body. A size limit exists ("A provided value exceeds the allowed size limit", bug title only, not read). | Observed: [vdirsyncer#613](https://github.com/pimutils/vdirsyncer/issues/613) (2017); [Mozilla bug 1872262](https://bugzilla.mozilla.org/show_bug.cgi?id=1872262) |
| Cookies | "Google might set cookies on client applications that access data using CalDAV." | Documented: CalDAV guide |
| Maintenance | v2 guide updated 2026-10-09, with a banner on usage limits "As of May 1, 2026"; no deprecation notice for v2. The conference text for CalDAV clients was added in 2017. Client maintainers' view: "Synchronizing with Google accounts is not officially supported neither by DAVx5 nor by Google" (2023); "they are not up to fix some of the bugs it has" (2025); "I don't have anybody to contact at Google to fix this" (vdirsyncer, 2017). | Documented; observed: DAVx5 page, davx5-ose#1339, vdirsyncer#613 |

## 8. REST comparison

All documented by Google.

| Topic | REST API | CalDAV |
|---|---|---|
| Partial update | `events.patch`: "only specified fields change"; "Array fields, if specified, overwrite the existing arrays"; `null` deletes a field. Costs three quota units. ([events.patch](https://developers.google.com/workspace/calendar/api/v3/reference/events/patch), 2026-07-01; [performance guide](https://developers.google.com/workspace/calendar/api/guides/performance)) | None. A PUT replaces the whole resource, so every field the client did not understand must be sent back or is lost. |
| Fields not settable on `events.insert` | The request body has no `organizer`, `creator`, `created`, `updated`, `etag`, `htmlLink`, `hangoutLink`, `recurringEventId`, `locked`, `privateCopy`. Of `id` and `iCalUID`, "only one of them should be supplied at event creation time"; `iCalUID` "must be supplied when importing events via the import method"; `organizer` is "Read-only, except when importing an event". ([events.insert](https://developers.google.com/workspace/calendar/api/v3/reference/events/insert), [Events resource](https://developers.google.com/workspace/calendar/api/v3/reference/events), 2026-07-07) | The client chooses the UID; ORGANIZER handling unknown (section 1). |
| Layout of Exceptions | The Recurring Event is one resource with `recurrence[]` ("List of RRULE, EXRULE, RDATE and EXDATE lines"). Each Exception is its own resource with its own `id`, `recurringEventId` and `originalStartTime`, edited at `/events/instanceId`; a cancelled occurrence is a resource with `status: cancelled`. All share one `iCalUID`. ([recurring events guide](https://developers.google.com/workspace/calendar/api/guides/recurringevents), 2026-09-17) | One resource per UID, holding the Recurring Event and its Exceptions as components with RECURRENCE-ID (RFC 4791). Cancelled occurrences are expected as EXDATE; not verified. |
| Sync | `nextSyncToken`; 410 when it is invalid, then a full sync. ([sync guide](https://developers.google.com/workspace/calendar/api/guides/sync), 2026-11-07) | sync-collection; expiry behaviour undocumented. |
| Guest mail | `sendUpdates` on every write. With `none`, "some emails might still be sent"; Google advises `events.import` instead of `none` for copies. | No control. |
| Reminders | `reminders.useDefault` or up to 5 `overrides`. | VALARMs only; defaults applied by the server (section 5). |
| Google-only fields | `colorId`, `eventLabelId`, `conferenceData`, `extendedProperties` (private and shared, [guide](https://developers.google.com/workspace/calendar/api/guides/extended-properties), 2026-09-11), `eventType` ([guide](https://developers.google.com/workspace/calendar/api/guides/event-types), 2026-09-11), `guestsCan*`, `attachments`. | Not found in any CalDAV output (sections 3 and 5). |

## 9. What other clients do

| Client | API for Google Calendars | Evidence |
|---|---|---|
| Thunderbird, built in | CalDAV v2 with OAuth; detects Google by host name | [CalDavProvider.sys.mjs L267-284](https://hg.mozilla.org/comm-central/file/tip/calendar/providers/caldav/CalDavProvider.sys.mjs), [calProviderDetectionUtils.sys.mjs L60](https://hg.mozilla.org/comm-central/file/tip/calendar/base/modules/utils/calProviderDetectionUtils.sys.mjs), tip 2026-11-15 |
| Thunderbird add-on "Provider for Google Calendar" | REST: PATCH for updates, `events/import` for invitations, `conferenceDataVersion=1`, `sendUpdates=all` only when a setting is on | [calendar.js#L210-L219](https://github.com/kewisch/gdata-provider/blob/eff914999cd6c1f57c02fc6749c89fd973beba18/src/background/calendar.js#L210-L219), [#L288-L291](https://github.com/kewisch/gdata-provider/blob/eff914999cd6c1f57c02fc6749c89fd973beba18/src/background/calendar.js#L288-L291) |
| Evolution / Evolution Data Server | CalDAV (v2 with OAuth, v1 without); tasks through Google's Tasks REST API | [module-google-backend.c#L504-511](https://gitlab.gnome.org/GNOME/evolution-data-server/-/blob/3b587d11d904ca1a0ecd09a5baaf89df8c99d1f1/src/modules/google-backend/module-google-backend.c#L504-511) |
| GNOME Online Accounts (and GNOME Calendar, which reads through Evolution Data Server; that link was not checked in GNOME Calendar's source) | CalDAV v2 | [goagoogleprovider.c#L341](https://gitlab.gnome.org/GNOME/gnome-online-accounts/-/blob/fa76dfad7c018c10c5efbb0ba911364452215da4/src/goabackend/goagoogleprovider.c#L341) |
| DAVx5 | CalDAV v2 | [OAuthGoogle.kt#L19-L31](https://github.com/bitfireAT/davx5-ose/blob/8415799527c45b8e605ad29d7f0ee03f8bfccc89/core/src/main/kotlin/at/bitfire/davdroid/network/OAuthGoogle.kt#L19-L31) |
| vdirsyncer | CalDAV v2 (`GoogleCalendarStorage` is a `CalDAVStorage`) | [google.py#L170-L173](https://github.com/pimutils/vdirsyncer/blob/f8328610e458fbcdb92e20c14412911afc1d974c/vdirsyncer/storage/google.py#L170-L173) |
| SyncEvolution | CalDAV | [CalDAVSource.cpp](https://gitlab.freedesktop.org/SyncEvolution/syncevolution/-/blob/dac2d9e25d44f234fda00b44542fcecbf53589a5/src/backends/webdav/CalDAVSource.cpp) |
| KDE (Akonadi Google resource) | REST through libkgapi, with REST sync tokens | [calendarhandler.cpp#L122-126](https://invent.kde.org/pim/kdepim-runtime/-/blob/da5fc23cac74827099c2325c015201c44b9ca5ac/resources/google-groupware/calendarhandler.cpp#L122-126) |
| Apple Calendar | CalDAV, according to Google ("works correctly for many clients, including Apple Calendar"; the 2017 post names "Apple's calendar clients"). Closed source; its workarounds are unknown, apart from SyncEvolution's note that the `ACTION:NONE` alarm "is what iOS does". | CalDAV guide; Workspace Updates 2017-08-10 |

Why the Thunderbird add-on stayed on REST: the proposal to move its users to CalDAV was closed WONTFIX in 2019. Its author: "There are a few features of calendaring that are available in the gdata provider but not exposed through the caldav protocol. This includes but is not limited to tasks." ([Mozilla bug 1570933](https://bugzilla.mozilla.org/show_bug.cgi?id=1570933), 2019-08-02)

Google-specific code in the CalDAV clients, in short:

- Evolution Data Server keeps an `is_google` flag ([e-cal-backend-caldav.c#L104-107](https://gitlab.gnome.org/GNOME/evolution-data-server/-/blob/3b587d11d904ca1a0ecd09a5baaf89df8c99d1f1/src/calendar/backends/caldav/e-cal-backend-caldav.c#L104-107)): one href attempt only, pretended success on HTTP 400 for events organised by others.
- Thunderbird: HTML description rewrite, forced resync after rate-limit damage.
- vdirsyncer: retry on 403 `usageLimits`; documents that VTODO is rejected and that shared Calendars need the `syncselect` page; notes "Google has deprecated the previous APIs used for this without providing a suitable replacement".
- SyncEvolution (about 2012): dummy `ACTION:NONE` alarm, DTSTAMP and SEQUENCE bumping, 404 on listed items.
- DAVx5: no Google-specific sync code found; states that it is "not officially supported".

## What this means for ADR 0002

ADR 0002 chose REST for event colours, Meet links, incremental sync tokens and to avoid the CalDAV endpoint's known gaps. The facts above, sorted by what FreeCal would gain and lose with Google over CalDAV.

Gain:

- One Provider implementation for Google and the CalDAV servers, and one layout of Recurring Events and Exceptions (one resource per UID) instead of two.
- The client chooses the UID, and a Recurring Event with its Exceptions is written in one request.
- Incremental sync exists on both sides (sync-collection, documented as mandatory after the first sync), so that reason of the ADR does not separate the two APIs, apart from the undocumented expiry behaviour.
- Other open-source clients use the same endpoint (Thunderbird, Evolution, DAVx5, vdirsyncer), so their recorded quirks apply directly.
- No difference in quota or in project setup effort: both need a Google Cloud project, an OAuth client and the same request limits.

Lose:

- Per-event colours (observed 2024 and 2026).
- Meet links as data: they arrive as generated description text, and no way to create one is known.
- Control over guest mail (`sendUpdates`), and `events.import` for copies.
- Partial updates: every write is a whole-resource PUT against a server that regenerates the resource and may drop what it does not model, so the fields FreeCal does not understand are at risk; with REST, `events.patch` leaves them alone.
- The marker for default reminders, event types (birthdays cannot be told apart), guest permissions, extended properties, and probably visibility `default`. The ADR's consequence "Google-only fields kept as iCalendar `X-` properties" would have nothing to keep, because the fields never arrive.
- Reliable writes to events organised by someone else: 400, 403, 409 and silent partial stores are recorded, as recently as October 2026.
- Shared Calendars without a manual step on Google's `syncselect` page.
- Creating a Calendar (no MKCALENDAR), free-busy, tasks.
- Documented error behaviour: REST documents 410, 403 `usageLimits`, 429; over CalDAV, clients report bare 400 and 500 responses.

The same on both sides: OAuth 2.0 with the full `calendar` scope in every client found, the request limits, HTML descriptions (a property of Google's event model).

## Unknown, to be settled only by a test against a real Google account

1. Which of these survive PUT then GET on v2 today: custom `X-` properties (on VEVENT and inside VALARM), CATEGORIES, ATTACH, RDATE, unknown parameters, several VALARMs, an absolute TRIGGER, a trigger after the start.
2. Whether Google keeps a client-supplied ORGANIZER, SEQUENCE and UID, and whether the DTSTAMP and SEQUENCE checks of 2012 still exist.
3. Whether the `ACTION:NONE` dummy alarm still suppresses default reminders, and how an event with default reminders differs in the output from one with the same reminders set by hand.
4. How long a sync token lives and what an expired one returns (403 `valid-sync-token`, 400, 410 or other).
5. Whether `If-None-Match: *` works at all, and what a UID conflict returns.
6. Whether the ETag changes when only reminders change.
7. Whether a PUT with new ATTENDEEs sends invitation mail, whether a DELETE sends cancellations, and whether `SCHEDULE-AGENT` is honoured.
8. Which property, if any, carries the Meet link (`X-GOOGLE-CONFERENCE`, CONFERENCE, LOCATION, or only description text), and what happens when the generated description text is sent back.
9. Whether COLOR (RFC 7986) or any other property is read or written for per-event colours.
10. How visibility `default`, guest permissions, extended properties, focus time, out of office and working location appear, if at all.
11. Which privileges are reported for a read-only shared Calendar, and what a PUT to it returns.
12. Whether the CalDAV endpoint accepts `calendar.readonly` or `calendar.events` scopes, and whether an app password still works.
13. Whether a Calendar's name and colour can be changed by PROPPATCH, and whether DELETE on a collection works.
14. How a cancelled occurrence and an Exception of a Recurring Event organised by someone else are laid out in the resource.
