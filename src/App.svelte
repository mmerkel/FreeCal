<script lang="ts">
  import type { CoreApi } from './core/CoreApi';
  import type { Account, Calendar, Signal } from './core/types';
  import { t } from './i18n';
  import CalendarGrid from './lib/CalendarGrid.svelte';
  import PrototypeSwitcher from './lib/prototype-sidebar/PrototypeSwitcher.svelte';
  import VariantA from './lib/prototype-sidebar/VariantA.svelte';
  import VariantB from './lib/prototype-sidebar/VariantB.svelte';
  import VariantC from './lib/prototype-sidebar/VariantC.svelte';

  // PROTOTYPE: sidebar style variants, see src/lib/prototype-sidebar/README.md.
  const VARIANTS: [string, string][] = [['A', 'GNOME/libadwaita'], ['B', 'Google Calendar'], ['C', 'Neutral flat']];
  const THEMES: Record<string, 'pulse' | 'monarch' | 'breezy'> = { A: 'pulse', B: 'monarch', C: 'breezy' };
  let variant = $state(new URLSearchParams(location.search).get('variant') ?? 'A');

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
    {#if variant === 'A'}<VariantA {core} accounts={directory.accounts} calendars={directory.calendars} />
    {:else if variant === 'B'}<VariantB {core} accounts={directory.accounts} calendars={directory.calendars} />
    {:else}<VariantC {core} accounts={directory.accounts} calendars={directory.calendars} />{/if}
    <PrototypeSwitcher variants={VARIANTS} bind:current={variant} />
    <main class="grid">
      {#if readFailure}
        <p role="alert">{t('app.readFailed', { error: readFailure })}</p>
      {/if}
      {#key variant}<CalendarGrid {today} theme={THEMES[variant]} />{/key}
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
