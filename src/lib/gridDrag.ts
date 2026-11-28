/**
 * What FullCalendar reports when the user drops or resizes an Occurrence, as
 * the Occurrence and the `When` to reschedule it to.
 */
import type { CombinedEventKey, Occurrence, When } from '../core/types';
import { whenAfterDrag, type Shift } from './when';

/** The parts of FullCalendar's `EventApi` that a drop or resize needs. */
export interface GridEvent {
  id: string;
  allDay: boolean;
  start: Date | null;
  end: Date | null;
  /** The grid gives each Occurrence its key, `recurrenceId` and `when` here. */
  extendedProps: Record<string, unknown>;
}

export interface Reschedule {
  combinedEventKey: CombinedEventKey;
  /** Which Occurrence of a Recurring Event it is; `null` for a single Event. */
  recurrenceId: string | null;
  when: When;
}

/**
 * The ID FullCalendar keeps for an Occurrence: its Event's key with its
 * recurrence ID, so that each Occurrence is told apart. The only place the
 * window makes a string of a key; nothing reads it back.
 */
export function gridId(occurrence: Occurrence): string {
  return JSON.stringify([occurrence.combinedEventKey, occurrence.recurrenceId]);
}

/** The Event of an Occurrence; the grid gives its key in `extendedProps`. */
export function keyOf(event: Pick<GridEvent, 'extendedProps'>): CombinedEventKey {
  return event.extendedProps.combinedEventKey as CombinedEventKey;
}

/** Which Occurrence of a Recurring Event this is; the grid gives it in `extendedProps`. */
export function recurrenceIdOf(event: Pick<GridEvent, 'extendedProps'>): string | null {
  const id = event.extendedProps.recurrenceId;
  return typeof id === 'string' ? id : null;
}

export function rescheduleOfDrop(dropped: {
  event: GridEvent;
  oldEvent: GridEvent;
  delta: Shift;
}): Reschedule {
  return reschedule(dropped.event, dropped.oldEvent, dropped.delta, dropped.delta);
}

export function rescheduleOfResize(resized: {
  event: GridEvent;
  oldEvent: GridEvent;
  startDelta: Shift;
  endDelta: Shift;
}): Reschedule {
  return reschedule(resized.event, resized.oldEvent, resized.startDelta, resized.endDelta);
}

function reschedule(event: GridEvent, oldEvent: GridEvent, startBy: Shift, endBy: Shift) {
  const start = event.start!;
  // The grid sets `forceEventDuration`, which gives every Occurrence an end.
  const end = event.end ?? start;
  const when = whenAfterDrag(oldEvent.extendedProps.when as When, oldEvent.allDay, {
    allDay: event.allDay,
    start,
    end,
    startBy,
    endBy,
  });
  return { combinedEventKey: keyOf(event), recurrenceId: recurrenceIdOf(event), when };
}
