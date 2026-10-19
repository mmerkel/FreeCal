<!--
  The one modal dialog every part of FreeCal uses. It opens as soon as it is
  shown and closes when its owner stops showing it; Escape and the ✕ at the
  top right ask the owner to close it through `onclose`. A confirmation
  (`alertdialog`) has no ✕, because an explicit choice is its point. Focus
  goes to the element marked `data-initial-focus` (or the ✕) and returns
  afterwards to where it was before. The body scrolls when the dialog
  doesn't fit the window.
-->
<script lang="ts">
  import X from '@lucide/svelte/icons/x';
  import type { Snippet } from 'svelte';
  import { t } from '../i18n';

  interface Props {
    title: string;
    /** Called on Escape and ✕. The owner then stops showing the dialog. Without it, the dialog can't be closed. */
    onclose?: () => void;
    /** `alertdialog` for a confirmation that interrupts the user. */
    role?: 'dialog' | 'alertdialog';
    /** Whether a click on the backdrop closes the dialog too. */
    backdropCloses?: boolean;
    children: Snippet;
    /** Icon buttons shown left of the ✕ in a row above the title. */
    tools?: Snippet;
    /** The buttons, shown in a row on the right. */
    actions?: Snippet;
  }

  let {
    title,
    onclose,
    role = 'dialog',
    backdropCloses = false,
    children,
    tools,
    actions,
  }: Props = $props();

  const id = $props.id();
  const closable = $derived(role === 'dialog' && onclose !== undefined);
  /** Whether the press that may end in a click began on the backdrop. */
  let pressedBackdrop = false;

  function open(dialog: HTMLDialogElement) {
    const returnTo = document.activeElement;
    dialog.showModal();
    (
      dialog.querySelector<HTMLElement>('[data-initial-focus]') ??
      dialog.querySelector<HTMLElement>('.close')
    )?.focus();
    return () => {
      const focus = document.activeElement;
      if (!focus || focus === document.body || dialog.contains(focus)) {
        if (returnTo instanceof HTMLElement) returnTo.focus();
      }
    };
  }
</script>

<!-- The dialog element itself has no padding, so a click on it, not on its
     content, is a click on the backdrop. Only a press that also began there
     counts, so that selecting text past the edge doesn't close it. -->
<dialog
  {role}
  aria-labelledby="{id}-title"
  aria-describedby="{id}-body"
  oncancel={(cancelled) => {
    cancelled.preventDefault();
    onclose?.();
  }}
  onpointerdown={(pressed) => (pressedBackdrop = pressed.target === pressed.currentTarget)}
  onclick={(clicked) => {
    if (backdropCloses && pressedBackdrop && clicked.target === clicked.currentTarget) onclose?.();
  }}
  {@attach open}
>
  <div class="frame">
    {#snippet close()}
      <button
        type="button"
        class="icon-button close"
        aria-label={t('common.close')}
        title={t('common.close')}
        onclick={() => onclose?.()}
      >
        <X size={16} aria-hidden="true" />
      </button>
    {/snippet}
    {#if tools}
      <div class="tools">
        {@render tools()}
        {#if closable}{@render close()}{/if}
      </div>
    {/if}
    <div class="heading">
      <h2 id="{id}-title">{title}</h2>
      {#if closable && !tools}{@render close()}{/if}
    </div>
    <div id="{id}-body" class="body">
      {@render children()}
    </div>
    {#if actions}
      <div class="actions">
        {@render actions()}
      </div>
    {/if}
  </div>
</dialog>

<style>
  dialog {
    box-sizing: border-box;
    width: min(25rem, calc(100vw - 2rem));
    max-height: calc(100vh - 2rem);
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
    box-shadow: var(--shadow-dialog);
    color: var(--foreground);
    font-size: var(--text-md);
    line-height: 1.5;
  }

  /* Shown modal, the dialog is a column whose body takes the leftover height. */
  dialog[open] {
    display: flex;
  }

  dialog::backdrop {
    background: var(--backdrop);
  }

  .frame {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-height: 0;
    padding: var(--space-12);
  }

  .tools {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin: calc(-1 * var(--space-6)) calc(-1 * var(--space-6)) var(--space-2) 0;
  }

  .heading {
    display: flex;
    align-items: flex-start;
    gap: var(--space-4);
    margin-bottom: var(--space-4);
  }

  /* In the same corner as the tools row's ✕. */
  .heading .close {
    margin: calc(-1 * var(--space-6)) calc(-1 * var(--space-6)) 0 0;
  }

  h2 {
    flex: 1;
    margin: 0;
    font-size: var(--text-lg);
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  .body {
    min-height: 0;
    overflow-y: auto;
    color: var(--foreground-muted);
  }

  .body:has(+ .actions) {
    margin-bottom: var(--space-10);
  }

  .body :global(p) {
    margin: 0;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: var(--space-4);
  }
</style>
