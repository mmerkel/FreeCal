import { expect, test } from 'vitest';
import type { Repeat } from '../core/types';
import { describeRepeat } from './repeat';

const weekly: Repeat = { frequency: 'weekly', interval: 1, end: { kind: 'never' } };

test('a repeat says how often', () => {
  expect(describeRepeat(weekly, false)).toBe('Every week');
  expect(describeRepeat({ ...weekly, frequency: 'daily', interval: 3 }, false)).toBe(
    'Every 3 days',
  );
  expect(describeRepeat({ ...weekly, frequency: 'monthly' }, false)).toBe('Every month');
  expect(describeRepeat({ ...weekly, frequency: 'yearly', interval: 2 }, false)).toBe(
    'Every 2 years',
  );
});

test('a repeat says when it ends', () => {
  expect(describeRepeat({ ...weekly, end: { kind: 'count', count: 1 } }, false)).toBe(
    'Every week, once',
  );
  expect(describeRepeat({ ...weekly, end: { kind: 'count', count: 10 } }, false)).toBe(
    'Every week, 10 times',
  );
  expect(describeRepeat({ ...weekly, end: { kind: 'until', date: '2026-12-31' } }, false)).toBe(
    'Every week, until Thursday, December 31, 2026',
  );
});

test('a custom rule says that it has more than the editor shows', () => {
  expect(describeRepeat(weekly, true)).toBe('Every week, by further rules');
  expect(describeRepeat(null, true)).toBe('Repeats by a custom rule');
  expect(describeRepeat(null, false)).toBeUndefined();
});
