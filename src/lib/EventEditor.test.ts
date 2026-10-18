import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import hostile from '../../fixtures/hostile-event.json';
import type { Calendar, EventDraft } from '../core/types';
import { accountId, calendarId } from '../test/ids';
import EventEditor from './EventEditor.svelte';

const calendars: Calendar[] = [
  {
    id: calendarId(1),
    accountId: accountId(1),
    name: 'Home',
    colour: '#3366cc',
    shown: true,
    readOnly: false,
  },
  {
    id: calendarId(2),
    accountId: accountId(1),
    name: 'Work',
    colour: '#dc3912',
    shown: false,
    readOnly: false,
  },
];

const initial: EventDraft = {
  calendarId: calendarId(1),
  title: '',
  when: { kind: 'timed', start: '2026-10-05T09:00:00', end: '2026-10-05T10:30:00' },
  location: '',
  description: '',
  repeat: null,
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
  await user.type(screen.getByRole('textbox', { name: 'Location' }), 'Main Street 1');
  await user.type(screen.getByRole('textbox', { name: 'Description' }), 'Bring{Enter}the card');
  await user.click(screen.getByRole('button', { name: 'Save' }));

  expect(onsave).toHaveBeenCalledWith({
    calendarId: 2,
    title: 'Dentist',
    when: { kind: 'timed', start: '2026-10-05T09:00:00', end: '2026-10-05T11:00:00' },
    location: 'Main Street 1',
    description: 'Bring\nthe card',
    repeat: null,
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
  expect(screen.getByRole('textbox', { name: 'Location' })).toHaveValue(hostile.location);
  expect(screen.getByRole('textbox', { name: 'Description' })).toHaveValue(
    hostile.expected.html.text,
  );
  expect(document.querySelector('img, script, b, i')).toBeNull();

  await user.click(screen.getByRole('button', { name: 'Save' }));
  await waitFor(() => expect(onsave).toHaveBeenCalled());
  expect(onsave.mock.lastCall![0].location).toBe(hostile.location);
  expect(onsave.mock.lastCall![0].description).toBe(hostile.expected.html.text);
});

test('a new Event doesn’t repeat, and choosing a frequency saves its rule', async () => {
  const user = userEvent.setup();
  const { onsave } = show();
  const choice = screen.getByRole('combobox', { name: 'Repeat' });
  expect(choice).toHaveDisplayValue('Doesn’t repeat');
  expect(screen.queryByRole('spinbutton', { name: 'Repeat every' })).toBeNull();

  await user.selectOptions(choice, 'Weekly');
  const every = screen.getByRole('spinbutton', { name: 'Repeat every' });
  await user.clear(every);
  await user.type(every, '2');
  await user.click(screen.getByRole('button', { name: 'Save' }));

  expect(onsave).toHaveBeenCalledWith(
    expect.objectContaining({
      repeat: { frequency: 'weekly', interval: 2, end: { kind: 'never' } },
    }),
  );
});

test('a repeat ends on a date or after a number of times', async () => {
  const user = userEvent.setup();
  const { onsave } = show();
  await user.selectOptions(screen.getByRole('combobox', { name: 'Repeat' }), 'Daily');
  const ends = screen.getByRole('combobox', { name: 'Ends' });

  await user.selectOptions(ends, 'On date');
  await fireEvent.input(screen.getByLabelText('Last date'), { target: { value: '2026-10-20' } });
  await user.click(screen.getByRole('button', { name: 'Save' }));
  await user.selectOptions(ends, 'After');
  const times = screen.getByRole('spinbutton', { name: 'Number of times' });
  await user.clear(times);
  await user.type(times, '5');
  await user.click(screen.getByRole('button', { name: 'Save' }));

  expect(onsave.mock.calls.map(([draft]) => draft.repeat)).toEqual([
    { frequency: 'daily', interval: 1, end: { kind: 'until', date: '2026-10-20' } },
    { frequency: 'daily', interval: 1, end: { kind: 'count', count: 5 } },
  ]);
});

test('a repeat without any occurrence can’t be saved', async () => {
  const user = userEvent.setup();
  show();
  await user.selectOptions(screen.getByRole('combobox', { name: 'Repeat' }), 'Daily');

  await user.clear(screen.getByRole('spinbutton', { name: 'Repeat every' }));

  expect(screen.getByRole('status')).toHaveTextContent('Repeat every 1 or more');
  expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
  await user.type(screen.getByRole('spinbutton', { name: 'Repeat every' }), '1');
  await user.selectOptions(screen.getByRole('combobox', { name: 'Ends' }), 'On date');
  await fireEvent.input(screen.getByLabelText('Last date'), { target: { value: '2026-10-04' } });
  expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
});

test('a Recurring Event’s rule starts in the editor and is saved as it was', async () => {
  const user = userEvent.setup();
  const repeat = { frequency: 'monthly', interval: 3, end: { kind: 'count', count: 4 } } as const;
  const { onsave } = show({ mode: 'edit', initial: { ...initial, repeat } });

  expect(screen.getByRole('combobox', { name: 'Repeat' })).toHaveDisplayValue('Monthly');
  expect(screen.getByRole('spinbutton', { name: 'Repeat every' })).toHaveValue(3);
  expect(screen.getByText('months')).toBeInTheDocument();
  expect(screen.getByRole('combobox', { name: 'Ends' })).toHaveDisplayValue('After');
  expect(screen.getByRole('spinbutton', { name: 'Number of times' })).toHaveValue(4);
  await user.click(screen.getByRole('button', { name: 'Save' }));

  expect(onsave).toHaveBeenCalledWith(expect.objectContaining({ repeat }));
});

test('a custom rule says what is kept', () => {
  const repeat = { frequency: 'weekly', interval: 2, end: { kind: 'never' } } as const;
  show({ mode: 'edit', initial: { ...initial, repeat }, customRepeat: true });

  expect(screen.getByText(/also repeats by rules FreeCal can’t edit/)).toBeInTheDocument();
});

test('an Exception has no repeat', () => {
  show({ mode: 'edit', repeatable: false });

  expect(screen.queryByRole('combobox', { name: 'Repeat' })).toBeNull();
});
