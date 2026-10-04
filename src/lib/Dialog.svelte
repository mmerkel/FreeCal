<!--
  The one modal dialog every part of FreeCal uses. It opens as soon as it is
  shown and closes when its owner stops showing it; Escape asks the owner to
  close it through `onclose`. Focus goes to the element marked
  `data-initial-focus` (or the first control) and returns afterwards to where
  it was before.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    /** Called on Escape. The owner then stops showing the dialog. */
    onclose: () => void;
    /** `alertdialog` for a confirmation that interrupts the user. */
    role?: 'dialog' | 'alertdialog';
    children: Snippet;
    /** The buttons, shown in a row on the right. */
    actions: Snippet;
  }

  let { title, onclose, role = 'dialog', children, actions }: Props = $props();

  const id = $props.id();

  function open(dialog: HTMLDialogElement) {
    const returnTo = document.activeElement;
    dialog.showModal();
    dialog.querySelector<HTMLElement>('[data-initial-focus]')?.focus();
    return () => {
      const focus = document.activeElement;
      if (!focus || focus === document.body || dialog.contains(focus)) {
        if (returnTo instanceof HTMLElement) returnTo.focus();
      }
    };
  }
</script>

<dialog
  {role}
  aria-labelledby="{id}-title"
  aria-describedby="{id}-body"
  oncancel={(cancelled) => {
    cancelled.preventDefault();
    onclose();
  }}
  {@attach open}
>
  <h2 id="{id}-title">{title}</h2>
  <div id="{id}-body" class="body">
    {@render children()}
  </div>
  <div class="actions">
    {@render actions()}
  </div>
</dialog>

<style>
  dialog {
    box-sizing: border-box;
    width: min(25rem, calc(100vw - 2rem));
    padding: var(--space-12);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
    box-shadow: var(--shadow-dialog);
    color: var(--foreground);
    font-size: var(--text-md);
    line-height: 1.5;
  }

  dialog::backdrop {
    background: var(--backdrop);
  }

  h2 {
    margin: 0 0 var(--space-4);
    font-size: var(--text-lg);
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  .body {
    margin-bottom: var(--space-10);
    color: var(--foreground-muted);
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
