<!--
  Holds the sidebar at its width, with a handle on its right edge that changes
  the width by dragging or with the arrow keys and resets it on double-click.
  Dragging below the minimum hides the sidebar and keeps the width from
  before the drag.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { t } from '../i18n';
  import { maxWidth, MIN_WIDTH, shownWidth, WIDTH_STEP } from './sidebarWidth';

  interface Props {
    /** The width in `rem`, or `null` for the default. */
    width: number | null;
    /** The width was changed and is to be remembered; `null` is the default. */
    onresize: (width: number | null) => void;
    /** A drag ended below the minimum. */
    onhide: () => void;
    children: Snippet;
  }

  let { width, onresize, onhide, children }: Props = $props();

  /** A drag in progress: where it started, and the width it has reached. */
  interface Drag {
    startX: number;
    startWidth: number;
    /** The width now, unlimited, so that it can fall below the minimum. */
    width: number;
  }

  let windowPixels = $state(window.innerWidth);
  let drag = $state<Drag>();

  /** Pixels to the rem; without a computed font size, the browser default. */
  const remPixels = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
  const windowWidth = $derived(windowPixels / remPixels);
  const max = $derived(maxWidth(windowWidth));
  const current = $derived(shownWidth(drag ? drag.width : width, windowWidth));
  const collapsing = $derived(drag !== undefined && drag.width < MIN_WIDTH);

  function onpointerdown(pointer: PointerEvent) {
    if (pointer.button !== 0) return;
    // Keeps the column-resize cursor, and no text gets selected, wherever
    // the pointer goes during the drag.
    pointer.preventDefault();
    (pointer.currentTarget as Element).setPointerCapture(pointer.pointerId);
    drag = { startX: pointer.clientX, startWidth: current, width: current };
  }

  function onpointermove(pointer: PointerEvent) {
    if (!drag) return;
    drag.width = drag.startWidth + (pointer.clientX - drag.startX) / remPixels;
  }

  /**
   * Saves once, when the drag ends, and only when the shown width changed, so
   * that a click or a jitter keeps a stored width beyond the window's limits.
   */
  function onpointerup() {
    if (!drag) return;
    const ended = drag;
    drag = undefined;
    if (ended.width < MIN_WIDTH) return onhide();
    const resized = shownWidth(ended.width, windowWidth);
    if (resized !== ended.startWidth) onresize(resized);
  }

  function onkeydown(key: KeyboardEvent) {
    const by = { ArrowLeft: -WIDTH_STEP, ArrowRight: WIDTH_STEP }[key.key];
    if (by === undefined) return;
    key.preventDefault();
    const next = shownWidth(current + by, windowWidth);
    if (next !== current) onresize(next);
  }

  /** A value for the separator, without the noise of a fractional drag. */
  function rounded(value: number): number {
    return Math.round(value * 10) / 10;
  }
</script>

<svelte:window
  bind:innerWidth={windowPixels}
  {onpointermove}
  {onpointerup}
  onpointercancel={() => (drag = undefined)}
/>

<div
  class="pane"
  class:collapsing
  class:dragging={drag !== undefined}
  style:width="{collapsing ? 0 : current}rem"
>
  {@render children()}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions: a focusable separator is a splitter, which ARIA counts as interactive -->
  <div
    class="handle"
    role="separator"
    aria-label={t('sidebar.width')}
    aria-orientation="vertical"
    aria-valuenow={rounded(current)}
    aria-valuemin={MIN_WIDTH}
    aria-valuemax={rounded(max)}
    tabindex="0"
    {onpointerdown}
    {onkeydown}
    ondblclick={() => onresize(null)}
  ></div>
</div>

<style>
  .pane {
    position: relative;
    display: flex;
    flex: none;
  }

  /* While a drag is below the minimum, only the handle shows, to say that
     letting go hides the sidebar. */
  .collapsing {
    visibility: hidden;
  }

  .handle {
    position: absolute;
    top: 0;
    right: calc(var(--space-2) * -1);
    bottom: 0;
    z-index: 1;
    width: var(--space-4);
    visibility: visible;
    cursor: col-resize;
    touch-action: none;
  }

  /* The sidebar's 1px border is the line the handle sits on. */
  .handle:hover,
  .handle:focus-visible,
  .dragging .handle {
    outline: none;
    background: linear-gradient(
      to right,
      transparent calc(50% - 1px),
      var(--border-strong) calc(50% - 1px),
      var(--border-strong) calc(50% + 1px),
      transparent calc(50% + 1px)
    );
  }
</style>
