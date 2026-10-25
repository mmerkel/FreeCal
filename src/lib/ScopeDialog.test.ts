// The recurrence scope question: its choices, which one is preselected, and
// the checkbox for Exceptions with values of their own.
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import type { ExceptionCounts } from '../core/types';
import ScopeDialog, { type ScopeChange } from './ScopeDialog.svelte';

function show(change: ScopeChange, counts: ExceptionCounts = { thisAndFollowing: 0, all: 0 }) {
  const onchoose = vi.fn();
  render(ScopeDialog, { change, counts, onchoose });
  return onchoose;
}

const radio = (name: string) => screen.getByRole('radio', { name });
const exceptionsToo = () => screen.getByRole('checkbox', { name: /changed on (its|their) own/ });

test('the scopes come in order, with "This event" chosen and focused, so Enter confirms', async () => {
  const user = userEvent.setup();
  const onchoose = show('edit');

  expect(screen.getAllByRole('radio').map((choice) => choice.getAttribute('value'))).toEqual([
    'this',
    'thisAndFollowing',
    'all',
  ]);
  expect(radio('This event')).toBeChecked();
  expect(radio('This event')).toHaveFocus();
  expect(screen.getByRole('button', { name: 'Change' })).toHaveClass('solid');
  await user.keyboard('{Enter}');

  expect(onchoose).toHaveBeenCalledExactlyOnceWith({ scope: 'this', exceptionsToo: false });
});

test('a changed repeat rule can’t be for this event alone', async () => {
  const user = userEvent.setup();
  const onchoose = show('editRepeat');

  expect(radio('This event')).toBeDisabled();
  expect(radio('This and following events')).toBeChecked();
  expect(radio('This and following events')).toHaveFocus();
  await user.click(screen.getByRole('button', { name: 'Change' }));

  expect(onchoose).toHaveBeenCalledExactlyOnceWith({
    scope: 'thisAndFollowing',
    exceptionsToo: false,
  });
});

test('a move confirms with "Move", and Cancel chooses nothing', async () => {
  const user = userEvent.setup();
  const onchoose = show('move');

  expect(screen.getByRole('button', { name: 'Move' })).toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Cancel' }));

  expect(onchoose).toHaveBeenCalledExactlyOnceWith(null);
});

test('without Exceptions of their own in any scope there is no checkbox', () => {
  show('edit');

  expect(screen.queryByRole('checkbox')).toBeNull();
});

test('the checkbox counts the Exceptions in the chosen scope, unchecked, greyed out for this event', async () => {
  const user = userEvent.setup();
  const onchoose = show('edit', { thisAndFollowing: 1, all: 3 });

  expect(exceptionsToo()).toBeDisabled();
  expect(exceptionsToo()).not.toBeChecked();
  await user.click(radio('This and following events'));
  expect(exceptionsToo()).toBeEnabled();
  expect(exceptionsToo()).toHaveAccessibleName('Also change the 1 event changed on its own');
  await user.click(radio('All events'));
  expect(exceptionsToo()).toHaveAccessibleName('Also change the 3 events changed on their own');
  await user.click(exceptionsToo());
  await user.click(screen.getByRole('button', { name: 'Change' }));

  expect(onchoose).toHaveBeenCalledExactlyOnceWith({ scope: 'all', exceptionsToo: true });
});

test('greyed out, the checkbox keeps its value and is ignored', async () => {
  const user = userEvent.setup();
  const onchoose = show('move', { thisAndFollowing: 0, all: 2 });

  await user.click(radio('All events'));
  await user.click(exceptionsToo());
  await user.click(radio('This and following events'));
  expect(exceptionsToo()).toBeDisabled();
  expect(exceptionsToo()).toBeChecked();
  await user.click(screen.getByRole('button', { name: 'Move' }));
  await user.click(radio('This event'));
  expect(exceptionsToo()).toBeDisabled();
  expect(exceptionsToo()).toBeChecked();
  await user.click(screen.getByRole('button', { name: 'Move' }));

  expect(onchoose.mock.calls).toEqual([
    [{ scope: 'thisAndFollowing', exceptionsToo: false }],
    [{ scope: 'this', exceptionsToo: false }],
  ]);
});
