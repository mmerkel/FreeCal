<!--
  A message at the bottom of the window about something that just happened,
  with an optional action such as Undo (spec: "Toasts"). A confirmation goes
  after 5 seconds, counted again from the start once the pointer leaves it
  and it loses the focus; one that `stays`, reporting a problem, stays until
  dismissed. A new toast is a new component, so that its time starts anew.
-->
<script lang="ts">
  import X from '@lucide/svelte/icons/x';
  import { t } from '../i18n';

  interface Props {
    message: string;
    /** The action's button, if there is one. */
    action?: { label: string; run: () => void };
    /** Whether it stays until dismissed, as one reporting a problem does. */
    stays: boolean;
    ondismiss: () => void;
  }

  let { message, action, stays, ondismiss }: Props = $props();

  /** How long a confirmation shows, in milliseconds. */
  const SHOWN_FOR = 5000;

  let toast: HTMLElement;
  /** Whether the pointer is over the toast, also when it rested there as the toast appeared. */
  let pointedAt = $state(false);
  $effect(() => {
    if (toast.matches(':hover')) pointedAt = true;
  });
  let focused = $state(false);

  $effect(() => {
    if (stays || pointedAt || focused) return;
    const timer = setTimeout(ondismiss, SHOWN_FOR);
    return () => clearTimeout(timer);
  });

  /** Whether the focus left for somewhere outside the toast. */
  function focusLeft(event: FocusEvent) {
    const to = event.relatedTarget;
    focused = to instanceof Node && (event.currentTarget as Node).contains(to);
  }
</script>

<div
  bind:this={toast}
  class="toast"
  role="status"
  onmouseenter={() => (pointedAt = true)}
  onmouseleave={() => (pointedAt = false)}
  onfocusin={() => (focused = true)}
  onfocusout={focusLeft}
>
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
