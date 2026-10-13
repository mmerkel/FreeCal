/**
 * What FullCalendar reports when the user drops or resizes an Occurrence, as
 * the Event and `When` to reschedule it to.
 */
import type { EventId, When } from '../core/types';
import { whenAfterDrag, type Shift } from './when';

/** The parts of FullCalendar's `EventApi` that a drop or resize needs. */
export interface GridEvent {
  id: string;
  allDay: boolean;
  start: Date | null;
  end: Date | null;
  /** The grid gives each Occurrence its `when` here. */
  extendedProps: Record<string, unknown>;
}

export interface Reschedule {
  id: EventId;
  when: When;
}

/** FullCalendar keeps the ID as a string; it came from an Occurrence. */
export function idOf(event: Pick<GridEvent, 'id'>): EventId {
  return Number(event.id) as EventId;
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
  return { id: idOf(event), when };
}
