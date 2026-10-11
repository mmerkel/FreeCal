<!--
  PROTOTYPE (ticket 40), never merge: the floating bar that cycles through the
  theme variants (← and → also work) and flips light and dark. Its strings
  skip the i18n layer on purpose.
-->
<script lang="ts">
  import { showVariant, variants, type ThemeVariant } from './prototypeThemes';

  interface Props {
    current: ThemeVariant;
    onchange: (variant: ThemeVariant) => void;
  }

  let { current, onchange }: Props = $props();

  function step(by: number) {
    const index = variants.findIndex((variant) => variant.key === current.key);
    const next = variants[(index + by + variants.length) % variants.length];
    showVariant(next.key);
    onchange(next);
  }

  function flipScheme() {
    const root = document.documentElement;
    root.dataset.colorScheme = root.dataset.colorScheme === 'dark' ? 'light' : 'dark';
  }

  function onkeydown(event: KeyboardEvent) {
    const target = event.target as HTMLElement;
    if (target.closest('input, textarea, [contenteditable], [role="dialog"]')) return;
    if (event.key === 'ArrowLeft') step(-1);
    if (event.key === 'ArrowRight') step(1);
  }
</script>

<svelte:window {onkeydown} />

<div class="prototype-switcher">
  <button onclick={() => step(-1)}>←</button>
  <button onclick={() => step(1)}>→</button>
  <button onclick={flipScheme}>light/dark</button>
  <span>{current.key} ({current.name})</span>
</div>

<style>
  .prototype-switcher {
    position: fixed;
    bottom: 12px;
    left: 260px;
    z-index: 1000;
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 6px 12px;
    border-radius: 999px;
    background: #111;
    color: #ff0;
    font: 13px monospace;
    box-shadow: 0 2px 8px #0008;
  }

  button {
    border: 1px solid #ff0;
    border-radius: 999px;
    background: none;
    color: inherit;
    font: inherit;
    padding: 2px 8px;
  }
</style>
