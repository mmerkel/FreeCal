import { expect, test } from 'vitest';
import { textColourOn } from './palette';

const DARK = '#171717';
const WHITE = '#ffffff';

test('puts dark text on light colours and white text on dark ones', () => {
  expect(textColourOn('#ffffff')).toBe(DARK);
  expect(textColourOn('#000000')).toBe(WHITE);
  expect(textColourOn('#f6bf26')).toBe(DARK); // yellow
  expect(textColourOn('#e4c441')).toBe(DARK); // mustard
  expect(textColourOn('#c0ca33')).toBe(DARK); // lime
  expect(textColourOn('#3366cc')).toBe(WHITE);
  expect(textColourOn('#ad1457')).toBe(WHITE); // raspberry
  expect(textColourOn('#616161')).toBe(WHITE); // grey
});

test('works out a colour it has never seen, such as one from CalDAV', () => {
  expect(textColourOn('#fff3a0')).toBe(DARK);
  expect(textColourOn('#1a237e')).toBe(WHITE);
});
