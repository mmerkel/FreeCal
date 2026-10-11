import { afterEach, expect, test } from 'vitest';
import { setFirstDayOfWeek, startOfWeek, type FirstDayOfWeek } from './week.svelte';

afterEach(() => setFirstDayOfWeek('monday'));

// 2026-10-04 is a Sunday, 2026-10-05 a Monday and 2026-10-10 a Saturday.
test.each<[FirstDayOfWeek, string, string]>([
  ['monday', '2026-10-04', '2026-09-28'],
  ['monday', '2026-10-05', '2026-10-05'],
  ['monday', '2026-10-10', '2026-10-05'],
  ['sunday', '2026-10-04', '2026-10-04'],
  ['sunday', '2026-10-10', '2026-10-04'],
  ['saturday', '2026-10-04', '2026-10-03'],
  ['saturday', '2026-10-10', '2026-10-10'],
])('with weeks starting on %s, the week of %s starts on %s', (firstDay, date, start) => {
  setFirstDayOfWeek(firstDay);
  expect(startOfWeek(Temporal.PlainDate.from(date)).toString()).toBe(start);
});
