<!--
  The modal "Working…" dialog. It blocks the window while a long command such
  as an import runs, so that no edit can come in between, and can't be
  closed: it goes once its owner stops showing it.
-->
<script lang="ts">
  import { t } from '../i18n';
  import Dialog from './Dialog.svelte';

  interface Props {
    /** What is being done, such as "Importing “work.ics”…". */
    message: string;
  }

  let { message }: Props = $props();
</script>

<Dialog title={t('working.title')} onclose={() => {}}>
  <p aria-busy="true">{message}</p>
</Dialog>
