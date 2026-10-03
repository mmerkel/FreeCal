# 22: Mass-Change Guard

**What to build:** If one sync would delete or change more than 10 Events or more than 5% of a Calendar, in either direction, FreeCal pauses that Calendar and asks first. The paused Calendar shows a Sync Error such as "Paused: 120 Events would be deleted. Review?", and the user approves or rejects the paused changes. Imports skip the guard because the preview is the approval.

**Blocked by:** 10, 17

**Status:** ready-for-agent

- [ ] The thresholds (more than 10 Events or more than 5% of a Calendar) are checked before a sync is applied, for pushes and pulls
- [ ] Exceeding them pauses the Calendar as a Sync Error with an explanatory message
- [ ] Approve applies the paused changes; reject discards them as chosen, and sync continues
- [ ] Imports confirmed through the preview skip the guard
- [ ] Written test-first, including the exact threshold boundaries
