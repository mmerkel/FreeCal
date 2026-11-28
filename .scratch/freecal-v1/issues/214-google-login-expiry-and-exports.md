# 214: Google login expiry; exports keep Google's fields

**What to build:** When the Google login expires, the user is asked to log in again and sync resumes; exports keep Google-only details, so a Google Event can be restored completely. The other boxes of old ticket 26 (`git show design/snapshot-line:.scratch/freecal-v1/issues/26-write-to-google.md`); its writing is 213's.

- **An expired login** (a refresh token Google refuses, 211) is a Sync Error on the Account whose action is "Log in again" (202's click on the Sync Error mark): 211's browser login for the same Google account, whose tokens replace the old ones in the keyring. Sync resumes without the user doing more; changes made meanwhile wait as when offline. A login with a different Google account is refused with a message.
- **Export and import:** the export (ticket 09) writes `google:` entries as `X-` properties, and an import reads them back as `google:` entries. An import or a restore into a Calendar whose Provider doesn't hold `google:` drops them (186's prefixes, ADR 0017), so the round trip is exact for a Google Calendar.

Spec: US-80, US-93; "Import/export"; "Keyring" under Desktop interfaces; "Untrusted content". ADRs 0017, 0019. Layout: "By phase", "P5 and P6".

**Blocked by:** 09, 213

**Status:** ready-for-agent

**Effort:** M

- [ ] An expired login is a Sync Error on the Account with "Log in again"; after logging in again, sync resumes and waiting changes are sent
- [ ] Logging in again as another Google account is refused
- [ ] Google-only fields are exported as `X-` properties and read back on import, round-tripping exactly for a Google Calendar and dropped for a Local one
- [ ] Tests through the core interface against the fake Google server (expiry, re-login, resumed sync); component tests on the fake core for the action; export round-trip tests
