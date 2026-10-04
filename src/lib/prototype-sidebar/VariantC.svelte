<!--
  PROTOTYPE. C: neutral flat (Notion Calendar / shadcn). Compact rows with a
  colour dot instead of a checkbox (filled = shown, ring = hidden; clicking
  the row toggles), small uppercase section labels, ghost "+" and "⋮",
  dropdown with icons, alert dialog card with outline/solid buttons.
-->
<script lang="ts">
  import type { Calendar } from '../../core/types';
  import Icon from './Icon.svelte';
  import { PALETTE, accountName, type VariantProps } from './shared';

  let { core, accounts, calendars }: VariantProps = $props();

  let menuFor = $state<Calendar>();
  let renaming = $state<number>();
  let creating = $state(false);
  let deleting = $state<Calendar>();
  let dialog: HTMLDialogElement;

  $effect(() => {
    if (deleting) dialog.showModal();
    else dialog.close();
  });
</script>

<svelte:window onclick={() => (menuFor = undefined)} />

<aside class="sidebar">
  {#each accounts as account (account.id)}
    <div class="section">
      <span>{accountName(account)}</span>
      <button class="ghost" title="New calendar" onclick={() => (creating = true)}><Icon name="plus" size={14} /></button>
    </div>
    <ul>
      {#each calendars.filter((c) => c.accountId === account.id) as calendar (calendar.id)}
        <li class:open={menuFor?.id === calendar.id} class:hidden={!calendar.shown}>
          {#if renaming === calendar.id}
            <span class="dot" style:--colour={calendar.colour}></span>
            <!-- svelte-ignore a11y_autofocus -->
            <input class="entry" value={calendar.name} autofocus onkeydown={(e) => { if (e.key === 'Enter') { core.renameCalendar(calendar.id, e.currentTarget.value); renaming = undefined; } if (e.key === 'Escape') renaming = undefined; }} />
          {:else}
            <button class="row" role="switch" aria-checked={calendar.shown} onclick={() => core.setCalendarShown(calendar.id, !calendar.shown)}>
              <span class="dot" style:--colour={calendar.colour}></span>
              <span class="name">{calendar.name}</span>
            </button>
          {/if}
          <button class="ghost more" title="Options for {calendar.name}" onclick={(e) => { e.stopPropagation(); menuFor = menuFor?.id === calendar.id ? undefined : calendar; }}>
            <Icon name="more" size={14} />
          </button>
          {#if menuFor?.id === calendar.id}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <div class="menu" onclick={(e) => e.stopPropagation()}>
              <div class="label">Colour</div>
              <div class="swatches">
                {#each PALETTE.slice(0, 8) as colour}
                  <button class="swatch" class:current={colour === calendar.colour} style:background={colour} aria-label={colour} onclick={() => { core.recolourCalendar(calendar.id, colour); menuFor = undefined; }}></button>
                {/each}
              </div>
              <hr />
              <button class="item" onclick={() => { renaming = calendar.id; menuFor = undefined; }}><Icon name="pencil" size={14} /> Rename</button>
              <button class="item" onclick={() => (menuFor = undefined)}><Icon name="eye" size={14} /> Show only this</button>
              <hr />
              <button class="item danger" onclick={() => { deleting = calendar; menuFor = undefined; }}><Icon name="trash" size={14} /> Delete</button>
            </div>
          {/if}
        </li>
      {/each}
      {#if creating}
        <li>
          <span class="dot" style:--colour={PALETTE[6]}></span>
          <!-- svelte-ignore a11y_autofocus -->
          <input class="entry" placeholder="Calendar name" autofocus onkeydown={(e) => { if (e.key === 'Enter') { core.createCalendar(account.id, e.currentTarget.value, PALETTE[6]); creating = false; } if (e.key === 'Escape') creating = false; }} />
        </li>
      {/if}
    </ul>
  {/each}
</aside>

<dialog bind:this={dialog} onclose={() => (deleting = undefined)}>
  <h3>Delete “{deleting?.name}”?</h3>
  <p>This Calendar and all its Events will be permanently deleted. This can’t be undone.</p>
  <div class="actions">
    <button class="outline" onclick={() => (deleting = undefined)}>Cancel</button>
    <button class="solid-danger" onclick={() => { if (deleting) core.deleteCalendar(deleting.id); deleting = undefined; }}>Delete</button>
  </div>
</dialog>

<style>
  .sidebar { width: 14rem; padding: 12px 8px; background: #fafafa; border-right: 1px solid #e5e5e5; color: #171717; font: 13px/1.4 Inter, 'Noto Sans', system-ui, sans-serif; overflow-y: auto; }
  .section { display: flex; justify-content: space-between; align-items: center; padding: 4px 4px 4px 8px; color: #737373; font-size: 11px; font-weight: 600; letter-spacing: 0.04em; text-transform: uppercase; }
  ul { list-style: none; margin: 0 0 12px; padding: 0; }
  li { position: relative; display: flex; align-items: center; gap: 8px; height: 28px; padding: 0 2px 0 8px; border-radius: 6px; }
  li:hover, li.open { background: #f0f0f0; }
  .row { all: unset; flex: 1; min-width: 0; display: flex; align-items: center; gap: 10px; height: 100%; cursor: pointer; }
  .dot { flex: none; width: 10px; height: 10px; border-radius: 50%; background: var(--colour); box-shadow: inset 0 0 0 2px var(--colour); }
  li.hidden .dot { background: transparent; }
  li.hidden .name { color: #a3a3a3; }
  .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ghost { all: unset; display: grid; place-content: center; width: 22px; height: 22px; border-radius: 5px; color: #737373; cursor: pointer; }
  .ghost:hover { background: #e5e5e5; color: #171717; }
  .more { opacity: 0; }
  li:hover .more, li.open .more, .more:focus-visible { opacity: 1; }
  .entry { flex: 1; min-width: 0; height: 22px; font: inherit; padding: 0 6px; border: 1px solid #d4d4d4; border-radius: 5px; background: #fff; outline: 2px solid #a3a3a366; }
  .menu { position: absolute; right: 0; top: 26px; z-index: 10; width: 180px; padding: 4px; background: #fff; border: 1px solid #e5e5e5; border-radius: 8px; box-shadow: 0 4px 12px #0000001a; }
  .label { padding: 6px 8px 4px; color: #737373; font-size: 11px; font-weight: 600; }
  .swatches { display: grid; grid-template-columns: repeat(8, 1fr); gap: 4px; padding: 2px 8px 6px; }
  .swatch { all: unset; width: 14px; height: 14px; border-radius: 50%; cursor: pointer; }
  .swatch.current, .swatch:hover { box-shadow: 0 0 0 2px #fff, 0 0 0 3px #171717; }
  hr { margin: 4px -4px; border: none; border-top: 1px solid #e5e5e5; }
  .item { all: unset; display: flex; align-items: center; gap: 8px; box-sizing: border-box; width: 100%; padding: 6px 8px; border-radius: 4px; cursor: pointer; }
  .item:hover { background: #f5f5f5; }
  .danger { color: #dc2626; }
  .danger:hover { background: #fef2f2; }
  dialog { width: 400px; padding: 24px; border: 1px solid #e5e5e5; border-radius: 10px; color: #171717; font: 14px/1.5 Inter, 'Noto Sans', system-ui, sans-serif; box-shadow: 0 10px 30px #0000002e; }
  dialog::backdrop { background: #00000066; }
  h3 { margin: 0 0 8px; font-size: 16px; font-weight: 600; }
  p { margin: 0 0 20px; color: #737373; }
  .actions { display: flex; justify-content: flex-end; gap: 8px; }
  .outline, .solid-danger { height: 34px; padding: 0 14px; border-radius: 6px; font: 500 13px Inter, 'Noto Sans', system-ui, sans-serif; cursor: pointer; }
  .outline { border: 1px solid #e5e5e5; background: #fff; color: #171717; }
  .outline:hover { background: #f5f5f5; }
  .solid-danger { border: none; background: #dc2626; color: #fff; }
  .solid-danger:hover { background: #b91c1c; }
</style>
