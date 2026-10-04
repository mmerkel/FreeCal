import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import hostile from '../../fixtures/hostile-event.json';
import type { Calendar, EventDraft } from '../core/types';
import EventEditor from './EventEditor.svelte';

const calendars: Calendar[] = [
  { id: 1, accountId: 1, name: 'Home', colour: '#3366cc', shown: true, readOnly: false },
  { id: 2, accountId: 1, name: 'Work', colour: '#dc3912', shown: false, readOnly: false },
];

const initial: EventDraft = {
  calendarId: 1,
  title: '',
  when: { kind: 'timed', start: '2026-10-05T09:00:00', end: '2026-10-05T10:30:00' },
  location: '',
  description: '',
};

function show(props: Partial<Parameters<typeof render<typeof EventEditor>>[1] & object> = {}) {
  const onsave = vi.fn(async (_draft: EventDraft) => {});
  const oncancel = vi.fn();
  render(EventEditor, { mode: 'create', initial, calendars, onsave, oncancel, ...props });
  return { onsave, oncancel };
}

test('starts from the draft it is given', () => {
  show();

  expect(screen.getByRole('dialog', { name: 'New event' })).toBeInTheDocument();
  expect(screen.getByRole('textbox', { name: 'Title' })).toHaveFocus();
  expect(screen.getByLabelText('Start date')).toHaveValue('2026-10-05');
  expect(screen.getByLabelText('Start time')).toHaveValue('09:00');
  expect(screen.getByLabelText('End time')).toHaveValue('10:30');
  expect(screen.getByRole('checkbox', { name: 'All day' })).not.toBeChecked();
  expect(screen.getByRole('combobox', { name: 'Calendar' })).toHaveDisplayValue('Home');
});

test('saves every field the user filled in', async () => {
  const user = userEvent.setup();
  const { onsave } = show();

  await user.type(screen.getByRole('textbox', { name: 'Title' }), 'Dentist');
  await fireEvent.input(screen.getByLabelText('End time'), { target: { value: '11:00' } });
  await user.selectOptions(screen.getByRole('combobox', { name: 'Calendar' }), 'Work');
  await user.type(screen.getByRole('textbox', { name: 'Place' }), 'Main Street 1');
  await user.type(screen.getByRole('textbox', { name: 'Description' }), 'Bring{Enter}the card');
  await user.click(screen.getByRole('button', { name: 'Save' }));

  expect(onsave).toHaveBeenCalledWith({
    calendarId: 2,
    title: 'Dentist',
    when: { kind: 'timed', start: '2026-10-05T09:00:00', end: '2026-10-05T11:00:00' },
    location: 'Main Street 1',
    description: 'Bring\nthe card',
  });
});

test('Enter in the title saves', async () => {
  const user = userEvent.setup();
  const { onsave } = show();

  await user.type(screen.getByRole('textbox', { name: 'Title' }), 'Quick{Enter}');

  expect(onsave).toHaveBeenCalledWith(expect.objectContaining({ title: 'Quick' }));
});

test('an all-day Event has dates only, with its last day included', async () => {
  const user = userEvent.setup();
  const { onsave } = show();

  await user.click(screen.getByRole('checkbox', { name: 'All day' }));
  await fireEvent.input(screen.getByLabelText('End date'), { target: { value: '2026-10-06' } });

  expect(screen.queryByLabelText('Start time')).toBeNull();
  await user.click(screen.getByRole('button', { name: 'Save' }));
  expect(onsave).toHaveBeenCalledWith(
    expect.objectContaining({ when: { kind: 'allDay', start: '2026-10-05', end: '2026-10-07' } }),
  );
});

test('an Event that ends before it starts can’t be saved', async () => {
  show();

  await fireEvent.input(screen.getByLabelText('End time'), { target: { value: '08:00' } });

  expect(screen.getByRole('status')).toHaveTextContent('The event can’t end before it starts.');
  expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
});

test('a refused save is reported and the editor stays open with the draft', async () => {
  const user = userEvent.setup();
  const onsave = vi.fn(async () => {
    throw new Error('Calendar 1 is read-only');
  });
  show({ onsave });

  await user.type(screen.getByRole('textbox', { name: 'Title' }), 'Mine');
  await user.click(screen.getByRole('button', { name: 'Save' }));

  expect(await screen.findByRole('alert')).toHaveTextContent('Calendar 1 is read-only');
  expect(screen.getByRole('textbox', { name: 'Title' })).toHaveValue('Mine');
});

test('Cancel and Escape cancel', async () => {
  const user = userEvent.setup();
  const { oncancel } = show();

  await user.click(screen.getByRole('button', { name: 'Cancel' }));
  await user.keyboard('{Escape}');

  expect(oncancel).toHaveBeenCalledTimes(2);
});

test('a new draft from outside doesn’t replace what the user typed', async () => {
  const user = userEvent.setup();
  const onsave = vi.fn(async (_draft: EventDraft) => {});
  const shown = render(EventEditor, {
    mode: 'edit',
    initial: { ...initial, title: 'Old' },
    calendars,
    onsave,
    oncancel: () => {},
  });

  await user.clear(screen.getByRole('textbox', { name: 'Title' }));
  await user.type(screen.getByRole('textbox', { name: 'Title' }), 'Mine');
  await shown.rerender({ initial: { ...initial, title: 'From the server' } });

  expect(screen.getByRole('textbox', { name: 'Title' })).toHaveValue('Mine');
});

test('edits the hostile Event’s fields as plain text', async () => {
  const user = userEvent.setup();
  const onsave = vi.fn(async (_draft: EventDraft) => {});
  const draft = {
    ...initial,
    title: hostile.title,
    location: hostile.location,
    description: hostile.expected.html.text,
  };
  show({ mode: 'edit', initial: draft, onsave });

  expect(screen.getByRole('textbox', { name: 'Title' })).toHaveValue(
    hostile.title.replace(/[\r\n]/g, ''),
  );
  expect(screen.getByRole('textbox', { name: 'Place' })).toHaveValue(hostile.location);
  expect(screen.getByRole('textbox', { name: 'Description' })).toHaveValue(
    hostile.expected.html.text,
  );
  expect(document.querySelector('img, script, b, i')).toBeNull();

  await user.click(screen.getByRole('button', { name: 'Save' }));
  await waitFor(() => expect(onsave).toHaveBeenCalled());
  expect(onsave.mock.lastCall![0].location).toBe(hostile.location);
  expect(onsave.mock.lastCall![0].description).toBe(hostile.expected.html.text);
});
