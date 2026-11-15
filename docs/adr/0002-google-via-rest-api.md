# Google via its Calendar REST API, not Google's CalDAV endpoint

Google offers a CalDAV endpoint, which would let one CalDAV sync engine serve every Provider. We use the Google Calendar REST API instead and accept a second sync code path behind the common Provider interface. The REST API gives Google-only features that are part of "feels like Google Calendar" (event colours, Meet links), and Google's CalDAV support has known gaps and quirks.

## Consequences

- Google-only fields are kept on export and in Backups as iCalendar `X-` properties.
- Automated tests run against a fake Google provider that mimics the REST API (sync tokens, ETags, 410 "token expired"), because CI can't use real Google servers.

## Amendments

- 2026-11-15, grilling on Unimplemented Fields (ADR 0017): the decision was reopened and stands. `docs/research/google-caldav.md` holds the evidence: Google's CalDAV endpoint rewrites what a client sends, carries no per-event colours, gives conference details only as description text, and leaves no control over guest mail. Incremental sync tokens are no longer a reason, as both APIs have them. Keeping a second code path behind the Provider interface also keeps Microsoft Graph possible as a third. The cost grows with it: each neutral name of the Unimplemented Fields needs a converter per Provider. "Snapshots" above became "Backups" with the renaming of that term.
