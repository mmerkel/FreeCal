<!--
  About FreeCal: the name, the version, the licence and the source code. The
  link opens in the system browser through the shell, never in the window.
-->
<script lang="ts">
  import { t } from '../i18n';
  import Dialog from './Dialog.svelte';

  interface Props {
    /** Shown once the shell has said it. */
    version?: string;
    /** Why the link couldn't be opened, if it couldn't. */
    failure?: string;
    onopenlink: (url: string) => void;
    onclose: () => void;
  }

  let { version, failure, onopenlink, onclose }: Props = $props();

  const SOURCE_CODE = 'https://github.com/mmerkel/FreeCal';
</script>

<Dialog title={t('about.title')} {onclose}>
  <p class="name">{t('about.name')}</p>
  {#if version}
    <p>{t('about.version', { version })}</p>
  {/if}
  <p>{t('about.licence')}</p>
  <p>{t('about.copyright')}</p>
  <p>
    <button type="button" class="link" title={SOURCE_CODE} onclick={() => onopenlink(SOURCE_CODE)}>
      {t('about.source')}
    </button>
  </p>
  {#if failure}
    <p class="failure" role="alert">{failure}</p>
  {/if}
</Dialog>

<style>
  p + p {
    margin-top: var(--space-2);
  }

  .name {
    color: var(--foreground);
    font-weight: 600;
  }

  .failure {
    color: var(--danger);
  }

  .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--foreground);
    font: inherit;
    text-decoration: underline;
    cursor: pointer;
  }
</style>
