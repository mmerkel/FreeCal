<!--
  Asks before a Dangerous Undo or redo of Session Undo, naming why it is
  dangerous (spec: "Change Journal"): the later changes it also undoes, with
  the titles of their Events, how many of its Events were changed on the
  server, and how many Events it changes when that is more than the
  Mass-Change Guard allows. "Cancel", which Escape also means, has the focus.
-->
<script lang="ts">
  import type { DangerousUndo } from '../core/types';
  import { t, tCount } from '../i18n';
  import Dialog from './Dialog.svelte';

  interface Props {
    danger: DangerousUndo;
    /** The change it undoes or redoes, as `actionText` names it. */
    action: string;
    redo: boolean;
    onanswer: (approved: boolean) => void;
  }

  let { danger, action, redo, onanswer }: Props = $props();

  const later = $derived(danger.laterJournalTransactions);
  const titles = $derived(
    later.map((transaction) =>
      transaction.titles
        .map((title) => t('undo.ask.laterTitle', { title: title || t('event.untitled') }))
        .join(', '),
    ),
  );
</script>

<Dialog
  title={t(redo ? 'undo.ask.redoTitle' : 'undo.ask.undoTitle', { action })}
  role="alertdialog"
  onclose={() => onanswer(false)}
>
  {#if later.length > 0}
    <p>{tCount('undo.ask.later', later.length)}</p>
    <ul>
      {#each titles as title, index (index)}
        <li>{title}</li>
      {/each}
    </ul>
  {/if}
  {#if danger.serverChanges > 0}
    <p>{tCount('undo.ask.server', danger.serverChanges)}</p>
  {/if}
  {#if danger.exceedsMassChangeGuard}
    <p>{tCount('undo.ask.events', danger.events)}</p>
  {/if}
  {#snippet actions()}
    <button type="button" class="button outline" data-initial-focus onclick={() => onanswer(false)}>
      {t('common.cancel')}
    </button>
    <button type="button" class="button solid" onclick={() => onanswer(true)}>
      {t(redo ? 'toast.redo' : 'toast.undo')}
    </button>
  {/snippet}
</Dialog>
