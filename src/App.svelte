<script lang="ts">
  import type { CoreApi } from './core/CoreApi';
  import type { Account } from './core/types';
  import { t } from './i18n';
  import CalendarGrid from './lib/CalendarGrid.svelte';
  import Sidebar from './lib/Sidebar.svelte';

  interface Props {
    core: CoreApi;
  }

  let { core }: Props = $props();

  interface AppState {
    today: string;
    accounts: Account[];
  }

  async function start(): Promise<AppState> {
    const [today, accounts] = await Promise.all([core.today(), core.listAccounts()]);
    return { today, accounts };
  }

  const started = start();
</script>

{#await started}
  <p>{t('app.loading')}</p>
{:then state}
  <div class="app">
    <Sidebar accounts={state.accounts} />
    <main class="grid">
      <CalendarGrid today={state.today} />
    </main>
  </div>
{:catch error}
  <p role="alert">{t('app.loadFailed', { error: String(error) })}</p>
{/await}

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
