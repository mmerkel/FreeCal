// The question before a Dangerous Undo: each reason it names, and its answers.
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import type { DangerousUndo } from '../core/types';
import { journalTransactionId } from '../test/ids';
import UndoDialog from './UndoDialog.svelte';

const harmless: DangerousUndo = {
  laterJournalTransactions: [],
  serverChanges: 0,
  exceedsMassChangeGuard: false,
  events: 1,
};

function show(danger: DangerousUndo, redo = false) {
  const onanswer = vi.fn();
  render(UndoDialog, { danger, action: 'deleting “⁨Lunch⁩”', redo, onanswer });
  return { onanswer, dialog: screen.getByRole('alertdialog') };
}

test('names the later changes it also undoes, with their Events', () => {
  const { dialog } = show({
    ...harmless,
    laterJournalTransactions: [
      { id: journalTransactionId(5), madeAt: 0, titles: ['Standup', ''] },
      { id: journalTransactionId(6), madeAt: 0, titles: ['Retro'] },
    ],
  });

  expect(dialog).toHaveAccessibleName('Undo deleting “⁨Lunch⁩”?');
  expect(dialog).toHaveTextContent('This also undoes 2 later changes:');
  expect(screen.getAllByRole('listitem').map((item) => item.textContent)).toEqual([
    '“⁨Standup⁩”, “⁨(No title)⁩”',
    '“⁨Retro⁩”',
  ]);
});

test('names the changes from the server and how many Events it changes', () => {
  const { dialog } = show({
    ...harmless,
    serverChanges: 3,
    exceedsMassChangeGuard: true,
    events: 412,
  });

  expect(dialog).toHaveTextContent('3 of these were changed on the server.');
  expect(dialog).toHaveTextContent('This changes 412 events.');
  expect(dialog).not.toHaveTextContent('later');
});

test('Cancel has the focus; Escape and Cancel decline, Redo approves a redo', async () => {
  const user = userEvent.setup();
  const { onanswer, dialog } = show({ ...harmless, serverChanges: 1 }, true);

  expect(dialog).toHaveAccessibleName('Redo deleting “⁨Lunch⁩”?');
  expect(screen.getByRole('button', { name: 'Cancel' })).toHaveFocus();
  await user.keyboard('{Escape}');
  expect(onanswer).toHaveBeenLastCalledWith(false);

  await user.click(screen.getByRole('button', { name: 'Redo' }));
  expect(onanswer).toHaveBeenLastCalledWith(true);
});
