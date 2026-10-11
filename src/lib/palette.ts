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

/** Text on a filled Event: the near-black of the light theme, or white. */
const DARK_TEXT = '#171717';
const LIGHT_TEXT = '#ffffff';

/**
 * The text colour for an Event filled with `colour` (`#rrggbb`): near-black or
 * white, whichever has the higher WCAG contrast ratio with it. It is worked out
 * from the colour itself, so colours from CalDAV or Google work too.
 */
export function textColourOn(colour: string): string {
  const background = relativeLuminance(colour);
  const contrast = (text: string) => {
    const [lighter, darker] = [background, relativeLuminance(text)].sort((a, b) => b - a);
    return (lighter + 0.05) / (darker + 0.05);
  };
  return contrast(DARK_TEXT) >= contrast(LIGHT_TEXT) ? DARK_TEXT : LIGHT_TEXT;
}

/** WCAG 2's relative luminance of a `#rrggbb` colour. */
function relativeLuminance(colour: string): number {
  const [red, green, blue] = [1, 3, 5].map((at) => {
    const channel = parseInt(colour.slice(at, at + 2), 16) / 255;
    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
}
