<!--
  Asks before quitting with Unsent Changes still waiting, once the try to
  send them failed (spec: "Quitting"): how many wait, each Calendar with
  why, and that they'll be sent at the next start. "Cancel", which Escape
  also means, has the focus; "Quit anyway" quits.
-->
<script lang="ts">
  import { t, tCount } from '../i18n';
  import Dialog from './Dialog.svelte';
  import type { UnsentAtQuit } from './unsent';

  interface Props {
    unsent: UnsentAtQuit[];
    oncancel: () => void;
    onquit: () => void;
  }

  let { unsent, oncancel, onquit }: Props = $props();

  const count = $derived(unsent.reduce((sum, calendar) => sum + calendar.count, 0));
</script>

<Dialog title={t('quit.title')} role="alertdialog" onclose={oncancel}>
  <p>{tCount('quit.body', count)}</p>
  <ul>
    {#each unsent as waiting (waiting.calendar.id)}
      <li>{t('quit.calendar', { calendar: waiting.calendar.name, reason: waiting.reason })}</li>
    {/each}
  </ul>
  <p>{t('quit.later')}</p>
  {#snippet actions()}
    <button type="button" class="button outline" data-initial-focus onclick={oncancel}>
      {t('common.cancel')}
    </button>
    <button type="button" class="button solid" onclick={onquit}>{t('quit.anyway')}</button>
  {/snippet}
</Dialog>
