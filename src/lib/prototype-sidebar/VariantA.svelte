<!--
  PROTOTYPE. A: GNOME/libadwaita-like. Header bar with title and "+",
  boxed-list cards per Account, always-visible "⋮" per row, popover menu,
  AdwAlertDialog-style confirmation (centred, two pill buttons side by side).
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
  <header>
    <span></span>
    <h1>Calendars</h1>
    <button class="flat round" title="New calendar" onclick={() => (creating = true)}><Icon name="plus" /></button>
  </header>

  {#each accounts as account (account.id)}
    <h2>{accountName(account)}</h2>
    <ul class="boxed">
      {#each calendars.filter((c) => c.accountId === account.id) as calendar (calendar.id)}
        <li>
          <input
            type="checkbox"
            class="check"
            style:--colour={calendar.colour}
            checked={calendar.shown}
            onchange={(e) => core.setCalendarShown(calendar.id, e.currentTarget.checked)}
          />
          {#if renaming === calendar.id}
            <!-- svelte-ignore a11y_autofocus -->
            <input class="entry" value={calendar.name} autofocus onkeydown={(e) => { if (e.key === 'Enter') { core.renameCalendar(calendar.id, e.currentTarget.value); renaming = undefined; } if (e.key === 'Escape') renaming = undefined; }} />
          {:else}
            <span class="name">{calendar.name}</span>
          {/if}
          <button class="flat round" title="Options for {calendar.name}" onclick={(e) => { e.stopPropagation(); menuFor = menuFor?.id === calendar.id ? undefined : calendar; }}>
            <Icon name="more" />
          </button>
          {#if menuFor?.id === calendar.id}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <div class="popover" onclick={(e) => e.stopPropagation()}>
              <div class="swatches">
                {#each PALETTE.slice(0, 8) as colour}
                  <button class="swatch" style:background={colour} aria-label={colour} onclick={() => { core.recolourCalendar(calendar.id, colour); menuFor = undefined; }}>
                    {#if colour === calendar.colour}<Icon name="check" size={12} />{/if}
                  </button>
                {/each}
              </div>
              <button class="item" onclick={() => { renaming = calendar.id; menuFor = undefined; }}>Rename…</button>
              <button class="item" onclick={() => { deleting = calendar; menuFor = undefined; }}>Delete…</button>
            </div>
          {/if}
        </li>
      {/each}
      {#if creating}
        <li>
          <!-- svelte-ignore a11y_autofocus -->
          <input class="entry" placeholder="Calendar name" autofocus onkeydown={(e) => { if (e.key === 'Enter') { core.createCalendar(account.id, e.currentTarget.value, PALETTE[6]); creating = false; } if (e.key === 'Escape') creating = false; }} />
        </li>
      {/if}
    </ul>
  {/each}
</aside>

<dialog bind:this={dialog} onclose={() => (deleting = undefined)}>
  <h3>Delete Calendar?</h3>
  <p>“{deleting?.name}” and all its Events will be deleted. This can’t be undone.</p>
  <div class="responses">
    <button class="pill" onclick={() => (deleting = undefined)}>Cancel</button>
    <button class="pill destructive" onclick={() => { if (deleting) core.deleteCalendar(deleting.id); deleting = undefined; }}>Delete</button>
  </div>
</dialog>

<style>
  .sidebar { width: 15rem; background: #ebebed; color: #2e2e32; font: 14px/1.4 'Adwaita Sans', Cantarell, 'Noto Sans', sans-serif; overflow-y: auto; border-right: 1px solid #0000001a; }
  header { display: grid; grid-template-columns: 34px 1fr 34px; align-items: center; height: 46px; padding: 0 6px; }
  h1 { margin: 0; text-align: center; font-size: 14px; font-weight: 700; }
  h2 { margin: 14px 18px 6px; font-size: 13px; font-weight: 700; opacity: 0.55; }
  .boxed { list-style: none; margin: 0 12px; padding: 0; background: #fff; border-radius: 12px; box-shadow: 0 0 0 1px #00000014, 0 1px 3px #00000014; }
  .boxed li { position: relative; display: flex; align-items: center; gap: 10px; min-height: 44px; padding: 0 6px 0 12px; }
  .boxed li + li { border-top: 1px solid #0000000f; }
  .name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .check { appearance: none; width: 18px; height: 18px; margin: 0; border-radius: 6px; border: 2px solid var(--colour); display: grid; place-content: center; cursor: pointer; }
  .check:checked { background: var(--colour); }
  .check:checked::after { content: ''; width: 9px; height: 5px; border: solid #fff; border-width: 0 0 2px 2px; transform: rotate(-45deg) translate(1px, -1px); }
  .flat { all: unset; display: grid; place-content: center; width: 34px; height: 34px; cursor: pointer; color: inherit; }
  .round { border-radius: 50%; }
  .flat:hover { background: #0000000f; }
  .entry { flex: 1; min-width: 0; font: inherit; padding: 6px 8px; border: none; border-radius: 6px; background: #0000000d; outline: 2px solid #3584e480; }
  .popover { position: absolute; right: 4px; top: 40px; z-index: 10; width: 190px; padding: 6px; background: #fff; border-radius: 12px; box-shadow: 0 0 0 1px #00000017, 0 4px 14px #00000026; }
  .swatches { display: grid; grid-template-columns: repeat(4, 1fr); gap: 6px; padding: 6px 6px 10px; border-bottom: 1px solid #0000000f; margin-bottom: 4px; }
  .swatch { all: unset; width: 28px; height: 28px; border-radius: 50%; display: grid; place-content: center; color: #fff; cursor: pointer; justify-self: center; }
  .item { all: unset; display: block; box-sizing: border-box; width: 100%; padding: 8px 10px; border-radius: 6px; cursor: pointer; }
  .item:hover { background: #0000000f; }
  dialog { width: 300px; padding: 24px 24px 20px; border: none; border-radius: 14px; text-align: center; font: 14px/1.45 'Adwaita Sans', Cantarell, 'Noto Sans', sans-serif; color: #2e2e32; box-shadow: 0 0 0 1px #00000017, 0 8px 32px #00000040; }
  dialog::backdrop { background: #00000040; }
  h3 { margin: 0 0 10px; font-size: 18px; }
  p { margin: 0 0 22px; }
  .responses { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .pill { font: inherit; font-weight: 700; padding: 9px; border: none; border-radius: 999px; background: #0000001a; cursor: pointer; }
  .destructive { background: #e01b241a; color: #c01c28; }
</style>
