<!--
  PROTOTYPE. B: Google Calendar-like. Big floating "Create" pill on top,
  collapsible Account sections with "+", coloured square checkboxes, row
  hover pill with "⋮", Material menu with a 12-colour grid, Material 3 dialog
  (left-aligned title, text buttons on the right).
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
  let collapsed = $state(new Set<number>());
  let dialog: HTMLDialogElement;

  $effect(() => {
    if (deleting) dialog.showModal();
    else dialog.close();
  });
</script>

<svelte:window onclick={() => (menuFor = undefined)} />

<aside class="sidebar">
  <button class="create" onclick={() => (creating = true)}><Icon name="plus" size={22} /> <span>Create</span></button>

  {#each accounts as account (account.id)}
    <div class="section">
      <button class="section-title" onclick={() => { collapsed.has(account.id) ? collapsed.delete(account.id) : collapsed.add(account.id); collapsed = new Set(collapsed); }}>
        <span>{accountName(account)}</span>
        <span class="chevron" class:up={!collapsed.has(account.id)}><Icon name="chevron" size={18} /></span>
      </button>
      <button class="icon" title="New calendar" onclick={() => (creating = true)}><Icon name="plus" size={18} /></button>
    </div>
    {#if !collapsed.has(account.id)}
      <ul>
        {#each calendars.filter((c) => c.accountId === account.id) as calendar (calendar.id)}
          <li class:open={menuFor?.id === calendar.id}>
            <label>
              <input type="checkbox" style:--colour={calendar.colour} checked={calendar.shown} onchange={(e) => core.setCalendarShown(calendar.id, e.currentTarget.checked)} />
              {#if renaming === calendar.id}
                <!-- svelte-ignore a11y_autofocus -->
                <input class="entry" value={calendar.name} autofocus onkeydown={(e) => { if (e.key === 'Enter') { core.renameCalendar(calendar.id, e.currentTarget.value); renaming = undefined; } if (e.key === 'Escape') renaming = undefined; }} />
              {:else}
                <span class="name">{calendar.name}</span>
              {/if}
            </label>
            <button class="icon more" title="Options for {calendar.name}" onclick={(e) => { e.stopPropagation(); menuFor = menuFor?.id === calendar.id ? undefined : calendar; }}>
              <Icon name="more" size={18} />
            </button>
            {#if menuFor?.id === calendar.id}
              <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
              <div class="menu" onclick={(e) => e.stopPropagation()}>
                <button class="item" onclick={() => (menuFor = undefined)}>Display this only</button>
                <button class="item" onclick={() => { renaming = calendar.id; menuFor = undefined; }}>Rename</button>
                <button class="item" onclick={() => { deleting = calendar; menuFor = undefined; }}>Delete</button>
                <div class="palette">
                  {#each PALETTE as colour}
                    <button class="swatch" style:background={colour} aria-label={colour} onclick={() => { core.recolourCalendar(calendar.id, colour); menuFor = undefined; }}>
                      {#if colour === calendar.colour}<Icon name="check" size={12} />{/if}
                    </button>
                  {/each}
                </div>
              </div>
            {/if}
          </li>
        {/each}
        {#if creating}
          <li>
            <!-- svelte-ignore a11y_autofocus -->
            <input class="entry" placeholder="Name" autofocus onkeydown={(e) => { if (e.key === 'Enter') { core.createCalendar(account.id, e.currentTarget.value, PALETTE[6]); creating = false; } if (e.key === 'Escape') creating = false; }} />
          </li>
        {/if}
      </ul>
    {/if}
  {/each}
</aside>

<dialog bind:this={dialog} onclose={() => (deleting = undefined)}>
  <h3>Delete calendar?</h3>
  <p>You will no longer have access to “{deleting?.name}” and its events.</p>
  <div class="actions">
    <button class="text" onclick={() => (deleting = undefined)}>Cancel</button>
    <button class="text" onclick={() => { if (deleting) core.deleteCalendar(deleting.id); deleting = undefined; }}>Delete</button>
  </div>
</dialog>

<style>
  .sidebar { width: 16rem; padding: 8px 0 8px; background: #fff; color: #1f1f1f; font: 14px/20px Roboto, 'Noto Sans', Arial, sans-serif; overflow-y: auto; }
  .create { display: flex; align-items: center; gap: 12px; margin: 8px 16px 20px; padding: 0 24px 0 16px; height: 56px; border: none; border-radius: 16px; background: #fff; color: #3c4043; font: 500 14px Roboto, 'Noto Sans', sans-serif; box-shadow: 0 1px 3px #3c404352, 0 4px 8px 3px #3c404326; cursor: pointer; }
  .create:hover { background: #f6fafe; }
  .section { display: flex; align-items: center; padding: 0 8px 0 16px; height: 40px; }
  .section-title { all: unset; flex: 1; display: flex; justify-content: space-between; align-items: center; font-weight: 500; font-size: 16px; cursor: pointer; padding-right: 4px; }
  .chevron { display: grid; transition: transform 0.15s; }
  .chevron.up { transform: rotate(180deg); }
  ul { list-style: none; margin: 0; padding: 0 0 8px; }
  li { position: relative; display: flex; align-items: center; height: 32px; padding: 0 4px 0 16px; border-radius: 0 16px 16px 0; margin-right: 8px; }
  li:hover, li.open { background: #f1f3f4; }
  label { flex: 1; display: flex; align-items: center; gap: 14px; min-width: 0; cursor: pointer; }
  .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  input[type='checkbox'] { appearance: none; flex: none; width: 18px; height: 18px; margin: 0; border: 2px solid var(--colour); border-radius: 2px; display: grid; place-content: center; cursor: pointer; }
  input[type='checkbox']:checked { background: var(--colour); }
  input[type='checkbox']:checked::after { content: ''; width: 10px; height: 5px; border: solid #fff; border-width: 0 0 2px 2px; transform: rotate(-45deg) translate(1px, -1px); }
  .icon { all: unset; display: grid; place-content: center; width: 32px; height: 32px; border-radius: 50%; color: #444746; cursor: pointer; }
  .icon:hover { background: #1f1f1f14; }
  .more { opacity: 0; }
  li:hover .more, li.open .more, .more:focus-visible { opacity: 1; }
  .entry { flex: 1; min-width: 0; font: inherit; padding: 4px 8px; border: none; border-bottom: 2px solid #0b57d0; background: #f1f3f4; outline: none; }
  .menu { position: absolute; left: 24px; top: 30px; z-index: 10; width: 220px; padding: 8px 0; background: #fff; border-radius: 4px; box-shadow: 0 2px 6px 2px #3c404326, 0 1px 2px #3c40434d; }
  .item { all: unset; display: block; box-sizing: border-box; width: 100%; padding: 8px 16px; cursor: pointer; }
  .item:hover { background: #1f1f1f14; }
  .palette { display: grid; grid-template-columns: repeat(6, 1fr); gap: 8px; padding: 12px 16px 4px; border-top: 1px solid #e0e0e0; margin-top: 8px; }
  .swatch { all: unset; width: 22px; height: 22px; border-radius: 50%; display: grid; place-content: center; color: #fff; cursor: pointer; }
  .swatch:hover { box-shadow: 0 0 0 2px #fff, 0 0 0 3px #0003; }
  dialog { width: 312px; padding: 24px; border: none; border-radius: 28px; background: #f0f4f9; color: #1f1f1f; font: 14px/20px Roboto, 'Noto Sans', sans-serif; box-shadow: 0 4px 8px 3px #00000026, 0 1px 3px #0000004d; }
  dialog::backdrop { background: #00000052; }
  h3 { margin: 0 0 16px; font: 400 24px/32px Roboto, 'Noto Sans', sans-serif; }
  p { margin: 0 0 24px; color: #444746; }
  .actions { display: flex; justify-content: flex-end; gap: 8px; }
  .text { padding: 10px 12px; border: none; border-radius: 20px; background: none; color: #0b57d0; font: 500 14px Roboto, 'Noto Sans', sans-serif; cursor: pointer; }
  .text:hover { background: #0b57d014; }
</style>
