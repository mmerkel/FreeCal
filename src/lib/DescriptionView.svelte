<!--
  Draws a description from the pieces the core built: text and links, never
  markup. A link opens in the system browser; when its text differs from its
  URL, the user is first asked whether to open the full URL.
-->
<script lang="ts">
  import type { Description } from '../core/types';
  import { t } from '../i18n';
  import Dialog from './Dialog.svelte';

  interface Props {
    description: Description;
    onopenlink: (url: string) => void;
  }

  let { description, onopenlink }: Props = $props();

  /** The URL waiting for the user's yes. */
  let confirming = $state<string>();

  function open(text: string, url: string) {
    if (text === url) onopenlink(url);
    else confirming = url;
  }
</script>

<div class="description">
  {#each description.paragraphs as paragraph, index (index)}
    <!-- One line, so that no whitespace gets between the pieces. -->
    <p>{#each paragraph as piece, at (at)}{#if piece.kind === 'link'}<button type="button" class="link" title={piece.url} onclick={() => open(piece.text, piece.url)}>{piece.text}</button>{:else}{piece.text}{/if}{/each}</p>
  {/each}
</div>

{#if confirming !== undefined}
  {@const url = confirming}
  <Dialog role="alertdialog" title={t('link.openTitle')} onclose={() => (confirming = undefined)}>
    <p class="url">{t('link.openBody', { url })}</p>
    {#snippet actions()}
      <button
        type="button"
        class="button outline"
        data-initial-focus
        onclick={() => (confirming = undefined)}
      >
        {t('common.cancel')}
      </button>
      <button
        type="button"
        class="button solid"
        onclick={() => {
          // `url` follows `confirming`, so it is used before that is cleared.
          onopenlink(url);
          confirming = undefined;
        }}
      >
        {t('link.open')}
      </button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .description p {
    min-height: 1lh;
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .link {
    display: inline;
    padding: 0;
    border: none;
    background: none;
    color: inherit;
    font: inherit;
    text-align: start;
    text-decoration: underline;
    cursor: pointer;
  }

  .url {
    overflow-wrap: anywhere;
  }
</style>
