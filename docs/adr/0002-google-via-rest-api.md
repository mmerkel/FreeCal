# Google via its Calendar REST API, not Google's CalDAV endpoint

Google offers a CalDAV endpoint, which would let one CalDAV sync engine serve every Provider. We use the Google Calendar REST API instead and accept a second sync code path behind the common Provider interface. The REST API gives Google-only features that are part of "feels like Google Calendar" (event colours, Meet links, incremental sync tokens), and Google's CalDAV support has known gaps and quirks.

## Consequences

- Google-only fields are kept on export and in Snapshots as iCalendar `X-` properties.
- Automated tests run against a fake Google provider that mimics the REST API (sync tokens, ETags, 410 "token expired"), because CI can't use real Google servers.
