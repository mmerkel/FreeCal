<!--
  A menu of actions, shown while its owner shows it. It opens in the top
  layer, so no scrolling or clipping parent cuts it off, and CSS anchor
  positioning places it under `anchor` (see `.menu` in app.css). Its items are
  any elements with a `menuitem`, `menuitemradio` or `menuitemcheckbox` role.
  The first item gets focus; the arrow keys, Home and End move between items.
  Escape, Tab and a click or right-click outside it ask the owner to close it
  through `onclose`. That click does nothing else: while the menu is open,
  every mouse press, release and click outside it is stopped before it
  reaches anything, as if a sheet lay over the rest of the window, and a wheel
  or touch outside it doesn't scroll, as in native menus. When it closes with
  focus inside it, focus returns to where it was when it opened, normally the
  button that opened it.
-->
<script lang="ts">
  import { untrack, type Snippet } from 'svelte';

  interface Props {
    label: string;
    /** The element the menu opens against, normally the button that opens it. */
    anchor: HTMLElement;
    onclose: () => void;
    children: Snippet;
  }

  let { label, anchor, onclose, children }: Props = $props();

  const id = $props.id();
  const anchorName = `--menu-${id}`;

  /** The mouse events a click or right-click outside the open menu is made of. */
  const SWALLOWED = [
    'pointerdown',
    'mousedown',
    'pointerup',
    'mouseup',
    'click',
    'auxclick',
    'dblclick',
    'contextmenu',
  ];

  /** The events that scroll what is under them. */
  const SCROLLS = ['wheel', 'touchmove'];

  function items(menu: HTMLElement): HTMLElement[] {
    return [...menu.querySelectorAll<HTMLElement>('[role^="menuitem"]')];
  }

  function open(element: HTMLDivElement) {
    const returnTo = document.activeElement;
    // Read once: the menu stays where it opened, and an owner's update that
    // touches `anchor` doesn't reopen it and move its focus.
    const anchoredTo = untrack(() => anchor);
    anchoredTo.style.setProperty('anchor-name', anchorName);
    element.showPopover();
    items(element)[0]?.focus();

    // On `window` in the capture phase, so nothing outside the menu sees the
    // press. The click or right-click that opened the menu has already
    // passed `window` by the time the menu appears, so it can't close it.
    function outside(pressed: Event) {
      if (element.contains(pressed.target as Node)) return;
      pressed.preventDefault();
      pressed.stopPropagation();
      if (pressed.type === 'click' || pressed.type === 'contextmenu') onclose();
    }
    for (const type of SWALLOWED) window.addEventListener(type, outside, true);

    // Not passive, or the scroll couldn't be cancelled.
    function scroll(scrolled: Event) {
      if (!element.contains(scrolled.target as Node)) scrolled.preventDefault();
    }
    for (const type of SCROLLS) {
      window.addEventListener(type, scroll, { capture: true, passive: false });
    }

    return () => {
      for (const type of SWALLOWED) window.removeEventListener(type, outside, true);
      for (const type of SCROLLS) window.removeEventListener(type, scroll, true);
      anchoredTo.style.removeProperty('anchor-name');
      const focus = document.activeElement;
      if (!focus || focus === document.body || element.contains(focus)) {
        if (returnTo instanceof HTMLElement) returnTo.focus();
      }
    };
  }

  function move(key: KeyboardEvent & { currentTarget: HTMLElement }) {
    const all = items(key.currentTarget);
    const current = all.indexOf(document.activeElement as HTMLElement);
    const last = all.length - 1;
    const next = {
      ArrowDown: current === last ? 0 : current + 1,
      ArrowRight: current === last ? 0 : current + 1,
      ArrowUp: current <= 0 ? last : current - 1,
      ArrowLeft: current <= 0 ? last : current - 1,
      Home: 0,
      End: last,
    }[key.key];

    if (next !== undefined) {
      key.preventDefault();
      all[next]?.focus();
    } else if (key.key === 'Escape') {
      key.preventDefault();
      key.stopPropagation();
      onclose();
    } else if (key.key === 'Tab') {
      onclose();
    }
  }
</script>

<div
  class="menu"
  role="menu"
  aria-label={label}
  tabindex="-1"
  popover="manual"
  style:position-anchor={anchorName}
  onkeydown={move}
  {@attach open}
>
  {@render children()}
</div>
