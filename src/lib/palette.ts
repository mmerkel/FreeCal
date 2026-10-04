import type { MessageKey } from '../i18n';
import type { Calendar } from '../core/types';

/**
 * The colours a Calendar can be given, in rows of 8 by hue. They are Google
 * Calendar's colours, so Calendars from there look the same.
 */
const COLOURS = {
  raspberry: '#ad1457',
  rose: '#d81b60',
  red: '#d50000',
  pink: '#e67c73',
  orange: '#f4511e',
  pumpkin: '#ef6c00',
  amber: '#f09300',
  yellow: '#f6bf26',
  mustard: '#e4c441',
  lime: '#c0ca33',
  pistachio: '#7cb342',
  green: '#0b8043',
  lightGreen: '#33b679',
  teal: '#009688',
  lightBlue: '#039be5',
  blue: '#4285f4',
  indigo: '#3f51b5',
  lavender: '#7986cb',
  lilac: '#b39ddb',
  mauve: '#9e69af',
  purple: '#8e24aa',
  brown: '#795548',
  stone: '#a79b8e',
  grey: '#616161',
} as const;

type ColourName = keyof typeof COLOURS;

export interface Swatch {
  colour: string;
  name: MessageKey;
}

/** The swatches in a Calendar's menu, in the order above. */
export const SWATCHES: Swatch[] = Object.entries(COLOURS).map(([name, colour]) => ({
  colour,
  name: `colour.${name as ColourName}`,
}));

/**
 * The order new Calendars take the colours in. Each is as far as possible,
 * in OKLab, from all before it, so the first few Calendars are easy to tell
 * apart. The browns and greys come last.
 */
const NEW_CALENDAR_ORDER: ColourName[] = [
  'lightBlue', 'red', 'yellow', 'purple', 'green', 'pink', 'pistachio', 'lilac',
  'indigo', 'mauve', 'raspberry', 'amber', 'teal', 'orange', 'lavender', 'rose',
  'lime', 'lightGreen', 'blue', 'pumpkin', 'mustard', 'brown', 'stone', 'grey',
];

/** The colour for a new Calendar: the first in turn that the fewest `calendars` have. */
export function newCalendarColour(calendars: Calendar[]): string {
  const uses = (colour: string) =>
    calendars.filter((calendar) => calendar.colour === colour).length;
  return NEW_CALENDAR_ORDER.map((name) => COLOURS[name]).reduce((best, colour) =>
    uses(colour) < uses(best) ? colour : best,
  );
}
