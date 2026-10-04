<!-- PROTOTYPE. Floating variant switcher; ←/→ also cycle (not while typing). -->
<script lang="ts">
  let { variants, current = $bindable() }: { variants: [string, string][]; current: string } = $props();

  function step(by: number) {
    const index = variants.findIndex(([key]) => key === current);
    current = variants[(index + by + variants.length) % variants.length][0];
    history.replaceState(null, '', `?variant=${current}`);
  }

  function onkeydown(key: KeyboardEvent) {
    const target = key.target as HTMLElement;
    if (target.closest('input, textarea, [contenteditable], dialog[open]')) return;
    if (key.key === 'ArrowLeft') step(-1);
    if (key.key === 'ArrowRight') step(1);
  }
</script>

<svelte:window {onkeydown} />

<div class="switcher">
  <button onclick={() => step(-1)}>←</button>
  <span>{current} ({variants.find(([key]) => key === current)?.[1]})</span>
  <button onclick={() => step(1)}>→</button>
</div>

<style>
  .switcher { position: fixed; bottom: 16px; left: 50%; transform: translateX(-50%); z-index: 1000; display: flex; gap: 12px; align-items: center; padding: 6px 14px; border-radius: 999px; background: #111; color: #fff; font: 13px/1 monospace; box-shadow: 0 4px 16px #0006; }
  button { all: unset; cursor: pointer; padding: 4px 8px; }
</style>
