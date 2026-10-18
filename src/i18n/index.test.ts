import { expect, test } from 'vitest';
import hostile from '../../fixtures/hostile-event.json';
import { errorText, tCount } from '.';

test('an error about a file names the file, as text', () => {
  expect(errorText({ code: 'importNotCalendar', file: 'takeout.zip/Calendar/{error}.ics' })).toBe(
    '“\u2068takeout.zip/Calendar/{error}.ics\u2069” isn’t a calendar file FreeCal can read',
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

test('a refused undo says why', () => {
  expect(errorText({ code: 'changedSinceUnit' })).toBe('events it changed were changed since');
  expect(errorText({ code: 'alreadyUndone' })).toBe('it was undone already');
  expect(errorText({ code: 'unitNotFound' })).toBe('the change is no longer in the Change Journal');
});

test('a server’s own text is quoted as text, never read for placeholders', () => {
  expect(errorText({ code: 'serverRefused', status: 403, message: '<b>No</b> {status}' })).toBe(
    'the server refused (status 403): “\u2068<b>No</b> {status}\u2069”',
  );
});

test('the hostile server text stays text in a refusal', () => {
  expect(errorText({ code: 'serverRefused', status: 500, message: hostile.serverMessage })).toBe(
    hostile.expected.serverRefused,
  );
});

test('the hostile redirect address stays text', () => {
  expect(errorText({ code: 'redirectedElsewhere', url: hostile.redirectUrl })).toBe(
    hostile.expected.redirectedElsewhere,
  );
});

test('a refusal without text from the server gives the status alone', () => {
  expect(errorText({ code: 'serverRefused', status: 503, message: '' })).toBe(
    'the server refused (status 503)',
  );
});

test('a count takes its form for one or for any other number', () => {
  expect(tCount('import.newEvents', 1)).toBe('1 new event');
  expect(tCount('import.newEvents', 0)).toBe('0 new events');
  expect(tCount('import.newEvents', 42)).toBe('42 new events');
});

test('a placeholder inside a filled-in value stays as it is', () => {
  expect(tCount('import.pinned', 2, { zone: '{count}' })).toBe(
    '2 events have times without a time zone; they will use {count}.',
  );
});
