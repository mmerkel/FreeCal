<script lang="ts">
  import type { CoreApi } from './core/CoreApi';
  import type { Account, Calendar, Signal } from './core/types';
  import { t } from './i18n';
  import CalendarGrid from './lib/CalendarGrid.svelte';
  import Sidebar from './lib/Sidebar.svelte';

  interface Props {
    core: CoreApi;
  }

  let { core }: Props = $props();

  interface Directory {
    accounts: Account[];
    calendars: Calendar[];
  }

  let today = $state<Temporal.PlainDate>();
  let directory = $state<Directory>();
  let startFailure = $state<string>();
  let readFailure = $state<string>();

  $effect(() => {
    let stopped = false;
    let unsubscribe: (() => void) | undefined;
    // Reads can overtake each other; only the latest one may land.
    let latestRead = 0;

    async function readDirectory() {
      const read = ++latestRead;
      const [accounts, calendars] = await Promise.all([core.listAccounts(), core.listCalendars()]);
      if (!stopped && read === latestRead) {
        directory = { accounts, calendars };
        readFailure = undefined;
      }
    }

    function onSignal(signal: Signal) {
      switch (signal.kind) {
        case 'calendarsChanged':
          readDirectory().catch((error) => (readFailure = String(error)));
          break;
      }
    }

    async function start() {
      // Subscribe first, then read, so that no change falls in the gap.
      const stop = await core.subscribe(onSignal);
      if (stopped) return stop();
      unsubscribe = stop;
      const [currentDate] = await Promise.all([core.today(), readDirectory()]);
      today = currentDate;
    }

    start().catch((error) => (startFailure = String(error)));
    return () => {
      stopped = true;
      unsubscribe?.();
    };
  });
</script>

{#if startFailure}
  <p role="alert">{t('app.loadFailed', { error: startFailure })}</p>
{:else if today && directory}
  <div class="app">
    <Sidebar {core} accounts={directory.accounts} calendars={directory.calendars} />
    <main class="grid">
      {#if readFailure}
        <p role="alert">{t('app.readFailed', { error: readFailure })}</p>
      {/if}
      <CalendarGrid {today} />
    </main>
  </div>
{:else}
  <p>{t('app.loading')}</p>
{/if}

<style>
  .app {
    display: flex;
    height: 100vh;
  }

  .grid {
    flex: 1;
    min-width: 0;
    padding: 0.75rem;
  }
</style>
