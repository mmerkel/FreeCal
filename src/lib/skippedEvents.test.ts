import { expect, test } from 'vitest';
import { samples } from '../core/samples.generated';
import type { Calendar } from '../core/types';
import hostile from '../../fixtures/hostile-event.json';
import { calendarId } from '../test/ids';
import { skippedText } from './skippedEvents';

const work = { ...(samples.Calendar[0] as Calendar), id: calendarId(3), name: 'Work' };

test('names the Event and why it was skipped, in bidi isolates', () => {
  const skipped = (reason: Parameters<typeof skippedText>[0]['reason']) =>
    skippedText({ title: 'Standup', reason }, [work]);

  expect(skipped({ kind: 'notInChangeJournal' })).toBe(
    'Can’t undo “⁨Standup⁩”: its current version isn’t in the Change Journal of the Local Store.',
  );
  expect(skipped({ kind: 'readOnlyCalendar', calendarId: calendarId(3) })).toBe(
    'Can’t undo “⁨Standup⁩”: “⁨Work⁩” is read-only.',
  );
  expect(skipped({ kind: 'readOnlyCalendar', calendarId: calendarId(9) })).toBe(
    'Can’t undo “⁨Standup⁩”: its calendar is read-only.',
  );
  expect(skipped({ kind: 'changedTogetherWith', title: 'Yoga' })).toBe(
    'Can’t undo “\u2068Standup\u2069”: it was changed together with “\u2068Yoga\u2069”, which can’t be undone.',
  );
  expect(skipped({ kind: 'hasGuests' })).toBe(
    'Can’t undo “⁨Standup⁩”: it has guests, so it can’t be changed in FreeCal.',
  );
});

test('a hostile title and Calendar name stay text', () => {
  const calendar = { ...work, name: hostile.calendarName };
  const reason = { kind: 'readOnlyCalendar', calendarId: calendarId(3) } as const;

  expect(skippedText({ title: hostile.title, reason }, [calendar])).toBe(
    hostile.expected.undoSkippedReadOnly,
  );
});
