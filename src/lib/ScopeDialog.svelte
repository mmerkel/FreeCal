<!--
  Asks which Occurrences of a Recurring Event a change to one of them
  changes: this event, this and following events, or all events, and whether
  the Exceptions with values of their own in that scope take the change too.
  It opens over the editor on Save, and over the grid when an Occurrence is
  dropped.
-->
<script module lang="ts">
  import type { Scope } from '../core/types';

  /**
   * The change asked about: an edit, an edit of the repeat rule, which
   * can't be for "this event" alone, or a drag or a resize.
   */
  export type ScopeChange = 'edit' | 'editRepeat' | 'move';

  /** What the user chose: the scope, and whether its Exceptions with values of their own change too. */
  export interface ScopeChoice {
    scope: Scope;
    exceptionsToo: boolean;
  }
</script>

<script lang="ts">
  import type { ExceptionCounts } from '../core/types';
  import { t, tCount } from '../i18n';
  import Dialog, { enterSubmits } from './Dialog.svelte';

  interface Props {
    change: ScopeChange;
    /** The Exceptions with values of their own each scope reaches. */
    counts: ExceptionCounts;
    /** The user chose, or `null` to cancel. */
    onchoose: (choice: ScopeChoice | null) => void;
  }

  let { change, counts, onchoose }: Props = $props();

  const id = $props.id();
  const questions = {
    edit: 'scope.editWhich',
    editRepeat: 'scope.editWhichOfAll',
    move: 'scope.moveWhich',
  } as const;
  const scopes = [
    { value: 'this', label: 'scope.this' },
    { value: 'thisAndFollowing', label: 'scope.thisAndFollowing' },
    { value: 'all', label: 'scope.all' },
  ] as const;
  // The dialog is shown anew for each change, so its first value is enough.
  // svelte-ignore state_referenced_locally
  const alone = change !== 'editRepeat';
  const first: Scope = alone ? 'this' : 'thisAndFollowing';
  let scope = $state(first);
  let exceptionsToo = $state(false);
  /** The count the checkbox names: the chosen scope's, for "this event" all of them. */
  const count = $derived(scope === 'thisAndFollowing' ? counts.thisAndFollowing : counts.all);
  const reaches = $derived(scope !== 'this' && count > 0);

  function confirm(submitted: SubmitEvent) {
    submitted.preventDefault();
    onchoose({ scope, exceptionsToo: reaches && exceptionsToo });
  }
</script>

<Dialog title={t('scope.title')} onclose={() => onchoose(null)}>
  <form id="{id}-form" class="form" onsubmit={confirm}>
    <fieldset class="choices">
      <legend>{t(questions[change])}</legend>
      {#each scopes as choice (choice.value)}
        <label class="check">
          <input
            type="radio"
            name="{id}-scope"
            value={choice.value}
            disabled={choice.value === 'this' && !alone}
            data-initial-focus={choice.value === first ? '' : undefined}
            bind:group={scope}
            onkeydown={enterSubmits}
          />
          {t(choice.label)}
        </label>
      {/each}
    </fieldset>

    {#if counts.all > 0}
      <label class="check">
        <input
          type="checkbox"
          disabled={!reaches}
          bind:checked={exceptionsToo}
          onkeydown={enterSubmits}
        />
        {tCount('scope.exceptionsToo', count)}
      </label>
    {/if}
  </form>

  {#snippet actions()}
    <button type="button" class="button outline" onclick={() => onchoose(null)}>
      {t('common.cancel')}
    </button>
    <button type="submit" class="button solid" form="{id}-form">
      {t(change === 'move' ? 'scope.move' : 'scope.change')}
    </button>
  {/snippet}
</Dialog>
