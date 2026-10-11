<!--
  Holds the sidebar at its width, shown or hidden. Its right edge is a window
  splitter whose visible grip is a tab, which stays at the window's edge while
  the sidebar is hidden. A click on the tab hides and shows the sidebar, a
  drag from anywhere on the edge pulls it in or out, the arrow keys change
  the width and Enter hides and shows it.
-->
<script lang="ts">
  import ChevronLeft from '@lucide/svelte/icons/chevron-left';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import type { Snippet } from 'svelte';
  import type { SidebarState } from '../core/types';
  import { t } from '../i18n';
  import { shortcutHint } from './shortcuts';
  import { draggedWidth, MAX_WIDTH, shownWidth, WIDTH_STEP } from './sidebarWidth';

  interface Props {
    /** The sidebar as remembered; its width is `null` for the default. */
    sidebar: SidebarState;
    /** The sidebar was changed and is to be remembered. */
    onchange: (sidebar: SidebarState) => void;
    children: Snippet;
  }

  let { sidebar, onchange, children }: Props = $props();

  /** A press moving less than this many pixels before release is a click. */
  const CLICK_DISTANCE = 4;

  /** A press on the edge, which becomes a drag once it moves far enough. */
  interface Press {
    startX: number;
    /** Pixels from the sidebar's edge to where it was pressed. */
    grab: number;
    onTab: boolean;
    /** Once it is a drag: the width it shows the sidebar at, `undefined` when hidden. */
    drag?: { width: number | undefined };
  }

  let press = $state<Press>();

  /** Pixels to the rem; without a computed font size, the browser default. */
  const remPixels = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
  const shown = $derived(press?.drag ? press.drag.width !== undefined : sidebar.shown);
  const current = $derived(press?.drag?.width ?? shownWidth(sidebar.width));
  const label = $derived(t(shown ? 'sidebar.hide' : 'sidebar.show'));

  function toggle() {
    onchange({ ...sidebar, shown: !sidebar.shown });
  }

  function onpointerdown(pointer: PointerEvent) {
    if (pointer.button !== 0) return;
    // Keeps the column-resize cursor, and no text gets selected, wherever
    // the pointer goes during the drag.
    pointer.preventDefault();
    (pointer.currentTarget as Element).setPointerCapture(pointer.pointerId);
    const edge = shown ? current * remPixels : 0;
    press = {
      startX: pointer.clientX,
      grab: pointer.clientX - edge,
      onTab: pointer.target instanceof Element && pointer.target.closest('.tab') !== null,
    };
  }

  function onpointermove(pointer: PointerEvent) {
    if (!press) return;
    if (!press.drag && Math.abs(pointer.clientX - press.startX) < CLICK_DISTANCE) return;
    press.drag = { width: draggedWidth((pointer.clientX - press.grab) / remPixels) };
  }

  /**
   * Saves once, when the drag ends, and only when it changed what shows, so
   * that a jitter keeps a stored width beyond the limits. Letting go while
   * hidden keeps the width from before the drag.
   */
  function onpointerup() {
    if (!press) return;
    const { onTab, drag } = press;
    press = undefined;
    if (!drag) {
      if (onTab) toggle();
    } else if (drag.width === undefined) {
      if (sidebar.shown) onchange({ ...sidebar, shown: false });
    } else if (!sidebar.shown || drag.width !== shownWidth(sidebar.width)) {
      onchange({ shown: true, width: drag.width });
    }
  }

  function onkeydown(key: KeyboardEvent) {
    if (key.key === 'Enter') {
      key.preventDefault();
      toggle();
      return;
    }
    const by = { ArrowLeft: -WIDTH_STEP, ArrowRight: WIDTH_STEP }[key.key];
    if (by === undefined || !sidebar.shown) return;
    key.preventDefault();
    const next = shownWidth(current + by);
    if (next !== current) onchange({ shown: true, width: next });
  }
</script>

<svelte:window {onpointermove} {onpointerup} onpointercancel={() => (press = undefined)} />

<div
  class="pane"
  class:hidden={!shown}
  class:dragging={press?.drag !== undefined}
  style:--width="{current}rem"
>
  <div class="clip">
    <div class="content" inert={!shown} aria-hidden={!shown}>
      {@render children()}
    </div>
  </div>
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions: a focusable separator is a splitter, which ARIA counts as interactive -->
  <div
    class="edge"
    role="separator"
    aria-label={t('sidebar.width')}
    aria-orientation="vertical"
    aria-valuenow={shown ? Math.round(current * 10) / 10 : 0}
    aria-valuemin={0}
    aria-valuemax={MAX_WIDTH}
    title={shortcutHint(label, 'sidebar')}
    tabindex="0"
    {onpointerdown}
    {onkeydown}
  >
    <span class="tab" data-focus-ring>
      {#if shown}
        <ChevronLeft size={10} aria-hidden="true" />
      {:else}
        <ChevronRight size={10} aria-hidden="true" />
      {/if}
    </span>
  </div>
</div>

<style>
  .pane {
    position: relative;
    display: flex;
    flex: none;
    width: var(--width);
    /* How long hiding and showing slide. */
    --slide: 120ms;
    transition: width var(--slide) ease-out;
  }

  .hidden {
    width: 0;
  }

  .clip {
    display: flex;
    flex: 1;
    min-width: 0;
    overflow: hidden;
  }

  /* As wide as the sidebar even while it slides, so that it moves rather
     than reflows. Once hidden, it is gone for the pointer as well. */
  .content {
    display: flex;
    flex: none;
    width: var(--width);
    transition: visibility 0s;
  }

  .hidden .content {
    visibility: hidden;
    transition-delay: var(--slide);
  }

  .dragging,
  .dragging .content {
    transition: none;
  }

  @media (prefers-reduced-motion: reduce) {
    .pane,
    .content {
      transition: none;
    }
  }

  /* The splitter: a strip centred on the sidebar's 1px border. */
  .edge {
    position: absolute;
    top: 0;
    right: calc(var(--space-2) * -1);
    bottom: 0;
    z-index: 1;
    width: var(--space-4);
    /* Its tab shows the focus ring (`data-focus-ring`). */
    outline: none;
    cursor: col-resize;
    touch-action: none;
  }

  /* Its grip, sticking out on the grid's side of the line. */
  .tab {
    position: absolute;
    top: 50%;
    left: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 10px;
    height: 40px;
    box-sizing: border-box;
    border: 1px solid var(--border);
    border-left: none;
    border-radius: 0 var(--radius) var(--radius) 0;
    background: var(--surface);
    color: var(--foreground-faint);
    cursor: pointer;
    transform: translateY(-50%);
  }

  .edge:hover,
  .edge:focus-visible,
  .dragging .edge {
    background: linear-gradient(
      to right,
      transparent calc(50% - 1px),
      var(--border-strong) calc(50% - 1px),
      var(--border-strong) calc(50% + 1px),
      transparent calc(50% + 1px)
    );
  }

  .edge:hover .tab,
  .edge:focus-visible .tab,
  .dragging .tab {
    border-color: var(--border-strong);
    background: var(--hover-strong);
    color: var(--foreground);
  }
</style>
