import { expect, test } from 'vitest';
import hostile from '../../fixtures/hostile-event.json';
import { errorText } from '.';

test('an error about a file names the file, as text', () => {
  expect(errorText({ code: 'importNotCalendar', file: 'takeout.zip/Calendar/{error}.ics' })).toBe(
    '“takeout.zip/Calendar/{error}.ics” isn’t a calendar file FreeCal can read',
  );
});

test('the hostile file name stays text in an import error', () => {
  expect(errorText({ code: 'importNotCalendar', file: hostile.fileName })).toBe(
    hostile.expected.importNotCalendar,
  );
});

test('an error without a file is translated from its code alone', () => {
  expect(errorText({ code: 'calendarReadOnly' })).toBe('the calendar is read-only');
});
