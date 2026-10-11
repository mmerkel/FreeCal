// PROTOTYPE (ticket 40), never merge: compares FullCalendar themes in the real
// month view. Six variants, switchable via `?variant=` and the bar at the bottom.
// The theme classes are prefixed per theme (`fc-breezy-…`, `fc-forma-…`), so
// all of their stylesheets can be loaded at once.
import breezy from 'fullcalendar/themes/breezy';
import classic from 'fullcalendar/themes/classic';
import forma from 'fullcalendar/themes/forma';
import monarch from 'fullcalendar/themes/monarch';
import pulse from 'fullcalendar/themes/pulse';
import 'fullcalendar/themes/breezy/theme.css';
import 'fullcalendar/themes/breezy/palettes/indigo.css';
import 'fullcalendar/themes/classic/theme.css';
import 'fullcalendar/themes/classic/palette.css';
import 'fullcalendar/themes/forma/theme.css';
import 'fullcalendar/themes/forma/palettes/blue.css';
import 'fullcalendar/themes/monarch/theme.css';
import 'fullcalendar/themes/monarch/palettes/blue.css';
import 'fullcalendar/themes/pulse/theme.css';
import 'fullcalendar/themes/pulse/palettes/blue.css';

export interface ThemeVariant {
  key: string;
  name: string;
  plugin: typeof breezy;
  /** Extra options, for the breezy variants that add the dot by hand. */
  options?: Record<string, unknown>;
}

export const variants: ThemeVariant[] = [
  {
    key: 'A',
    name: 'breezy (as now) + dot, time at the end',
    plugin: breezy,
    options: { listItemEventBeforeClass: 'prototype-dot' },
  },
  {
    key: 'B',
    name: 'breezy + dot, time first',
    plugin: breezy,
    options: {
      listItemEventBeforeClass: 'prototype-dot',
      listItemEventInnerClass: 'prototype-start',
      listItemEventTimeClass: 'prototype-time-first',
    },
  },
  { key: 'C', name: 'classic', plugin: classic },
  { key: 'D', name: 'forma (blue)', plugin: forma },
  { key: 'E', name: 'monarch (blue)', plugin: monarch },
  { key: 'F', name: 'pulse (blue)', plugin: pulse },
];

export function currentVariant(): ThemeVariant {
  const key = new URLSearchParams(location.search).get('variant');
  return variants.find((variant) => variant.key === key) ?? variants[0];
}

export function showVariant(key: string) {
  const url = new URL(location.href);
  url.searchParams.set('variant', key);
  history.replaceState(null, '', url);
}
