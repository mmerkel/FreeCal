/**
 * What FullCalendar reports when the user drops or resizes an Occurrence, as
 * the Occurrence and the `When` to reschedule it to.
 */
import type { EventId, Occurrence, When } from '../core/types';
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
  /** Which Occurrence of a Recurring Event it is; `null` for a single Event. */
  recurrenceId: string | null;
  when: When;
}

/**
 * The ID FullCalendar keeps for an Occurrence: its Event's, followed by its
 * recurrence ID for one of a Recurring Event, so that each is told apart.
 */
export function gridId(occurrence: Occurrence): string {
  const id = String(occurrence.eventId);
  return occurrence.recurrenceId === null ? id : `${id}/${occurrence.recurrenceId}`;
}

/** The Event of an Occurrence, from the ID FullCalendar keeps (see `gridId`). */
export function idOf(event: Pick<GridEvent, 'id'>): EventId {
  return Number.parseInt(event.id, 10) as EventId;
}

/**
 * The Event of an Occurrence and which Occurrence it is, from the ID
 * FullCalendar keeps (see `gridId`).
 */
export function occurrenceOf(id: string): { id: EventId; recurrenceId: string | null } {
  const slash = id.indexOf('/');
  return {
    id: idOf({ id }),
    recurrenceId: slash === -1 ? null : id.slice(slash + 1),
  };
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
  return { id: idOf(event), recurrenceId: recurrenceIdOf(event), when };
}
