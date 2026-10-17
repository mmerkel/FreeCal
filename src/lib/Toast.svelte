<!--
  A message at the bottom of the window about something that just happened,
  with an optional action such as Undo. It stays until dismissed or replaced.
-->
<script lang="ts">
  import X from '@lucide/svelte/icons/x';
  import { t } from '../i18n';

  interface Props {
    message: string;
    /** The action's button, if there is one. */
    action?: { label: string; run: () => void };
    ondismiss: () => void;
  }

  let { message, action, ondismiss }: Props = $props();
</script>

<div class="toast" role="status">
  <p>{message}</p>
  {#if action}
    <button type="button" class="button outline" onclick={action.run}>{action.label}</button>
  {/if}
  <button
    type="button"
    class="icon-button"
    aria-label={t('toast.dismiss')}
    title={t('toast.dismiss')}
    onclick={ondismiss}
  >
    <X size={14} aria-hidden="true" />
  </button>
</div>

<style>
  .toast {
    position: fixed;
    bottom: var(--space-12);
    left: 50%;
    display: flex;
    align-items: center;
    gap: var(--space-6);
    box-sizing: border-box;
    max-width: calc(100vw - 2rem);
    padding: var(--space-4) var(--space-4) var(--space-4) var(--space-7);
    transform: translateX(-50%);
  }

  p {
    margin: 0;
    overflow-wrap: anywhere;
  }
</style>
