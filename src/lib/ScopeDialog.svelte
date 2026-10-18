<!--
  Asks which Occurrences of a Recurring Event a change to one of them
  changes: this event, this and following events, or all events. It opens
  over the editor on Save, and over the grid when an Occurrence is dropped.
-->
<script module lang="ts">
  /**
   * The change asked about: an edit, an edit of the repeat rule, which
   * can't be for "this event" alone, or a drag or a resize.
   */
  export type ScopeChange = 'edit' | 'editRepeat' | 'move';
</script>

<script lang="ts">
  import type { Scope } from '../core/types';
  import { t } from '../i18n';
  import Dialog from './Dialog.svelte';

  interface Props {
    change: ScopeChange;
    /** The user chose a scope, or `null` to cancel. */
    onchoose: (scope: Scope | null) => void;
  }

  let { change, onchoose }: Props = $props();

  const questions = {
    edit: 'scope.editWhich',
    editRepeat: 'scope.editWhichOfAll',
    move: 'scope.moveWhich',
  } as const;
  const alone = $derived(change !== 'editRepeat');
</script>

<Dialog title={t('scope.title')} onclose={() => onchoose(null)}>
  <p class="question">{t(questions[change])}</p>

  {#snippet actions()}
    <button type="button" class="button outline" onclick={() => onchoose(null)}>
      {t('common.cancel')}
    </button>
    <button type="button" class="button outline" onclick={() => onchoose('all')}>
      {t('scope.all')}
    </button>
    <button type="button" class="button outline" onclick={() => onchoose('thisAndFollowing')}>
      {t('scope.thisAndFollowing')}
    </button>
    {#if alone}
      <button
        type="button"
        class="button solid"
        data-initial-focus
        onclick={() => onchoose('this')}
      >
        {t('scope.this')}
      </button>
    {/if}
  {/snippet}
</Dialog>

<style>
  .question {
    margin: 0;
  }
</style>
