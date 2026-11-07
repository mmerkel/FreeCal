<!--
  The toolbar over the grid: Today, previous and next,
  the shown period, the view switch, the status indicator and the main menu (Sync now, Import…, Settings…,
  About FreeCal and Quit). The Today button is a
  toggle whose pressed state is tracking today. In day and week view the month names in the shown
  period open their month.
-->
<script lang="ts">
  import ChevronLeft from '@lucide/svelte/icons/chevron-left';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import CircleAlert from '@lucide/svelte/icons/circle-alert';
  import CloudCheck from '@lucide/svelte/icons/cloud-check';
  import CloudOff from '@lucide/svelte/icons/cloud-off';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
  import EllipsisVertical from '@lucide/svelte/icons/ellipsis-vertical';
  import ImportIcon from '@lucide/svelte/icons/import';
  import Info from '@lucide/svelte/icons/info';
  import Power from '@lucide/svelte/icons/power';
  import SettingsIcon from '@lucide/svelte/icons/settings';
  import type { OverallState, OverallStatus, View } from '../core/types';
  import { t, type MessageKey } from '../i18n';
  import Menu from './Menu.svelte';
  import { monthTitle, type TitlePart } from './navigation';
  import { shortcutHint, shortcutKeys } from './shortcuts';
  import { lastSyncText } from './syncDisplay';

  interface Props {
    view: View;
    tracking: boolean;
    /** The shown period in words. */
    title: TitlePart[];
    ontoday: () => void;
    onstep: (by: -1 | 1) => void;
    onview: (view: View) => void;
    /** A month name in the title was clicked. */
    onmonth: (month: Temporal.PlainYearMonth) => void;
    /** The overall status as shown, once read. */
    sync?: OverallStatus;
    today: Temporal.PlainDate;
    onsyncnow: () => void;
    onimport: () => void;
    onsettings: () => void;
    onabout: () => void;
    onquit: () => void;
  }

  let {
    view,
    tracking,
    title,
    ontoday,
    onstep,
    onview,
    onmonth,
    sync,
    today,
    onsyncnow,
    onimport,
    onsettings,
    onabout,
    onquit,
  }: Props = $props();

  let menuOpen = $state(false);
  let menuButton = $state<HTMLButtonElement>();

  const views: { view: View; label: MessageKey }[] = [
    { view: 'day', label: 'view.day' },
    { view: 'week', label: 'view.week' },
    { view: 'month', label: 'view.month' },
  ];

  /** The names of the previous and next buttons in each view. */
  const steps: Record<View, { previous: MessageKey; next: MessageKey }> = {
    day: { previous: 'nav.previousDay', next: 'nav.nextDay' },
    week: { previous: 'nav.previousWeek', next: 'nav.nextWeek' },
    month: { previous: 'nav.previousMonth', next: 'nav.nextMonth' },
  };

  const previous = $derived(t(steps[view].previous));
  const next = $derived(t(steps[view].next));

  /** Each overall state's words and icon in the status indicator. */
  const syncStates: Record<OverallState['kind'], { label: MessageKey; icon: typeof CloudCheck }> = {
    conflict: { label: 'sync.conflict', icon: TriangleAlert },
    syncError: { label: 'sync.errorShort', icon: CircleAlert },
    offline: { label: 'sync.offline', icon: CloudOff },
    syncing: { label: 'sync.syncing', icon: RefreshCw },
    synced: { label: 'sync.synced', icon: CloudCheck },
  };

  const SyncIcon = $derived(syncStates[sync?.state.kind ?? 'synced'].icon);

  /** What the status indicator says: the overall state, with when the last sync finished. */
  const syncText = $derived.by(() => {
    if (!sync) return '';
    const state = t(syncStates[sync.state.kind].label);
    if (!sync.lastSync) return state;
    const time = lastSyncText(sync.lastSync, today);
    return sync.state.kind === 'synced'
      ? t('sync.syncedAt', { time })
      : t('sync.lastSync', { state, time });
  });

  /** Runs a menu item's action: the menu closes first, so focus returns to its button. */
  function choose(action: () => void) {
    menuOpen = false;
    action();
  }
</script>

<div class="toolbar" role="toolbar" aria-label={t('nav.toolbar')}>
  <button
    type="button"
    class="button outline today"
    aria-pressed={tracking}
    title={shortcutHint(t('nav.today'), 'today')}
    onclick={ontoday}
  >
    {t('nav.today')}
  </button>
  <button
    type="button"
    class="icon-button"
    aria-label={previous}
    title={shortcutHint(previous, 'previous')}
    onclick={() => onstep(-1)}
  >
    <ChevronLeft size={18} />
  </button>
  <button
    type="button"
    class="icon-button"
    aria-label={next}
    title={shortcutHint(next, 'next')}
    onclick={() => onstep(1)}
  >
    <ChevronRight size={18} />
  </button>
  <h1 class="period" aria-live="polite">
    {#each title as part, index (index)}
      {#if part.month}
        {@const month = part.month}
        {@const label = t('nav.showMonth', { month: monthTitle(month) })}
        <button
          type="button"
          class="month"
          aria-label={label}
          title={label}
          onclick={() => onmonth(month)}>{part.text}</button
        >
      {:else}
        {part.text}
      {/if}
    {/each}
  </h1>
  <div class="views" role="group" aria-label={t('nav.views')}>
    {#each views as option (option.view)}
      <button
        type="button"
        aria-pressed={option.view === view}
        title={shortcutHint(t(option.label), option.view)}
        onclick={() => onview(option.view)}
      >
        {t(option.label)}
      </button>
    {/each}
  </div>
  {#if sync}
    <!-- Not a live region: it would announce every routine sync. -->
    <div class="sync-status marker" role="group" aria-label={t('sync.indicator')}>
      <SyncIcon size={16} aria-hidden="true" />
      <span>{syncText}</span>
    </div>
  {/if}
  <div class="main-menu">
    <button
      type="button"
      class="icon-button"
      aria-label={t('menu.main')}
      title={t('menu.main')}
      aria-haspopup="menu"
      aria-expanded={menuOpen}
      bind:this={menuButton}
      onclick={() => (menuOpen = !menuOpen)}
    >
      <EllipsisVertical size={18} aria-hidden="true" />
    </button>
    {#if menuOpen && menuButton}
      <Menu label={t('menu.main')} anchor={menuButton} onclose={() => (menuOpen = false)}>
        <button type="button" class="menu-item" role="menuitem" onclick={() => choose(onsyncnow)}>
          <RefreshCw size={14} aria-hidden="true" />
          {t('menu.syncNow')}
          <kbd aria-hidden="true">{shortcutKeys('sync')}</kbd>
        </button>
        <button type="button" class="menu-item" role="menuitem" onclick={() => choose(onimport)}>
          <ImportIcon size={14} aria-hidden="true" />
          {t('menu.import')}
        </button>
        <button type="button" class="menu-item" role="menuitem" onclick={() => choose(onsettings)}>
          <SettingsIcon size={14} aria-hidden="true" />
          {t('menu.settings')}
          <kbd aria-hidden="true">{shortcutKeys('settings')}</kbd>
        </button>
        <button type="button" class="menu-item" role="menuitem" onclick={() => choose(onabout)}>
          <Info size={14} aria-hidden="true" />
          {t('menu.about')}
        </button>
        <hr class="menu-separator" />
        <button type="button" class="menu-item" role="menuitem" onclick={() => choose(onquit)}>
          <Power size={14} aria-hidden="true" />
          {t('menu.quit')}
        </button>
      </Menu>
    {/if}
  </div>
</div>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-6);
  }

  .today {
    margin-right: var(--space-4);
  }

  /* Pressed while tracking today. Neutral, as Calendar colours are the only
     strong colours. */
  .today[aria-pressed='true'] {
    border-color: var(--border-strong);
    background: var(--hover-strong);
  }

  .toolbar .icon-button {
    width: var(--control-height);
    height: var(--control-height);
  }

  .period {
    flex: 1;
    min-width: 0;
    margin: 0 0 0 var(--space-4);
    overflow: hidden;
    font-size: var(--text-lg);
    font-weight: 600;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .month {
    padding: 0;
    border: 0;
    border-radius: var(--radius);
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }

  .month:hover {
    text-decoration: underline;
  }

  .main-menu {
    margin-left: var(--space-2);
  }

  .sync-status {
    gap: var(--space-2);
    margin-left: var(--space-4);
    font-size: var(--text-sm);
    white-space: nowrap;
  }

  kbd {
    margin-left: auto;
    padding-left: var(--space-6);
    color: var(--foreground-muted);
    font: inherit;
    font-size: var(--text-xs);
  }

  .views {
    display: flex;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .views button {
    min-height: var(--control-height);
    padding: 0 var(--space-7);
    border: 0;
    background: transparent;
    color: var(--foreground-muted);
    font: inherit;
    cursor: pointer;
  }

  .views button + button {
    border-left: 1px solid var(--border);
  }

  .views button:hover {
    background: var(--hover);
    color: var(--foreground);
  }

  .views button[aria-pressed='true'] {
    background: var(--hover-strong);
    color: var(--foreground);
    font-weight: 500;
  }
</style>
