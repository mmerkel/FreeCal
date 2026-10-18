<!--
  "Manage calendars…" for a server Account: one checklist of its Calendars,
  checked, and the Available Calendars its server offers now, unchecked. Save
  removes the unchecked Calendars from FreeCal and adds the checked Available
  ones. Right after the Account is added, it lists what adding found instead
  of asking the server again, all checked.
-->
<script lang="ts">
  import { untrack } from 'svelte';
  import type { CoreApi } from '../core/CoreApi';
  import type { Account, AvailableCalendar, Calendar, CalendarId } from '../core/types';
  import { errorText, t, tCount } from '../i18n';
  import Dialog from './Dialog.svelte';

  interface Props {
    core: CoreApi;
    account: Account;
    /** The Account's name as the sidebar shows it. */
    accountName: string;
    /** The Account's Calendars. */
    calendars: Calendar[];
    /** The Available Calendars adding the Account found, if it was just added. */
    found?: AvailableCalendar[];
    /** Why the last save failed, if it did. */
    failure?: string;
    onsave: (removed: CalendarId[], added: string[]) => void;
    oncancel: () => void;
  }

  let { core, account, accountName, calendars, found, failure, onsave, oncancel }: Props = $props();

  let available = $state(untrack(() => found));
  let readFailure = $state<string>();
  /** The Calendars the user unchecked. */
  let removed = $state<CalendarId[]>([]);
  /** The `url`s of the Available Calendars the user checked. */
  let added = $state(untrack(() => found?.map((calendar) => calendar.url) ?? []));

  const removing = $derived(
    calendars.filter((calendar) => removed.includes(calendar.id)).map((calendar) => calendar.id),
  );

  // Asked once, as the dialog opens.
  untrack(() => {
    if (found) return;
    core.availableCalendars(account.id).then(
      (read) => (available = read),
      (error) => {
        // Removing needs no server, so the Account's Calendars stay listed.
        readFailure = t('manage.readFailed', { error: errorText(error) });
        available = [];
      },
    );
  });

  function toggle(id: CalendarId, kept: boolean) {
    removed = kept ? removed.filter((other) => other !== id) : [...removed, id];
  }
</script>

{#snippet item(name: string, colour: string | null)}
  <!-- Without a colour of its own, it gets the core's grey once added. -->
  <span class="calendar-dot" style:--colour={colour ?? '#808080'} aria-hidden="true"></span>
  <bdi class="name">{name}</bdi>
{/snippet}

<Dialog title={t('manage.title', { account: accountName })} onclose={oncancel}>
  <div class="form">
    {#if readFailure}
      <p class="problem" role="alert">{readFailure}</p>
    {/if}
    {#if !available}
      <p aria-busy="true">{t('manage.reading')}</p>
    {:else if calendars.length + available.length === 0 && !readFailure}
      <p>{t('manage.none')}</p>
    {:else}
      <fieldset class="choices">
        <legend class="field-label">{t('manage.legend')}</legend>
        {#each calendars as calendar (calendar.id)}
          <label class="check">
            <input
              type="checkbox"
              checked={!removed.includes(calendar.id)}
              onchange={(changed) => toggle(calendar.id, changed.currentTarget.checked)}
            />
            {@render item(calendar.name, calendar.colour)}
          </label>
        {/each}
        {#each available as calendar (calendar.url)}
          <label class="check">
            <input type="checkbox" value={calendar.url} bind:group={added} />
            {@render item(calendar.name, calendar.colour)}
          </label>
        {/each}
      </fieldset>
      {#if removing.length > 0}
        <p class="note">{tCount('manage.removing', removing.length)}</p>
      {/if}
    {/if}

    {#if failure}
      <p class="problem" role="alert">{failure}</p>
    {/if}
  </div>

  {#snippet actions()}
    <button type="button" class="button outline" onclick={oncancel}>{t('common.cancel')}</button>
    <button
      type="button"
      class="button solid"
      disabled={!available}
      onclick={() => onsave(removing, added)}
    >
      {t('manage.save')}
    </button>
  {/snippet}
</Dialog>

<style>
  .name {
    min-width: 0;
    overflow-wrap: anywhere;
  }
</style>
