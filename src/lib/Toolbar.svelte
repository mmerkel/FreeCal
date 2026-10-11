<!--
  The toolbar over the grid: Today, previous and next, the shown period and
  the view switch. The Today button is a toggle whose pressed state is
  tracking today.
-->
<script lang="ts">
  import ChevronLeft from '@lucide/svelte/icons/chevron-left';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import type { View } from '../core/types';
  import { t, type MessageKey } from '../i18n';

  interface Props {
    view: View;
    tracking: boolean;
    /** The shown period in words. */
    title: string;
    ontoday: () => void;
    onstep: (by: -1 | 1) => void;
    onview: (view: View) => void;
  }

  let { view, tracking, title, ontoday, onstep, onview }: Props = $props();

  const views: { view: View; label: MessageKey; keys: string }[] = [
    { view: 'day', label: 'view.day', keys: 'D' },
    { view: 'week', label: 'view.week', keys: 'W' },
    { view: 'month', label: 'view.month', keys: 'M' },
  ];

  const previous = $derived(
    t(({ day: 'nav.previousDay', week: 'nav.previousWeek', month: 'nav.previousMonth' } as const)[view]),
  );
  const next = $derived(
    t(({ day: 'nav.nextDay', week: 'nav.nextWeek', month: 'nav.nextMonth' } as const)[view]),
  );

  function hint(action: string, keys: string) {
    return t('shortcut.hint', { action, keys });
  }
</script>

<div class="toolbar" role="toolbar" aria-label={t('nav.toolbar')}>
  <button
    type="button"
    class="button outline today"
    aria-pressed={tracking}
    title={hint(t('nav.today'), 'T')}
    onclick={ontoday}
  >
    {t('nav.today')}
  </button>
  <button
    type="button"
    class="icon-button"
    aria-label={previous}
    title={hint(previous, 'K')}
    onclick={() => onstep(-1)}
  >
    <ChevronLeft size={18} />
  </button>
  <button
    type="button"
    class="icon-button"
    aria-label={next}
    title={hint(next, 'J')}
    onclick={() => onstep(1)}
  >
    <ChevronRight size={18} />
  </button>
  <h1 class="period" aria-live="polite">{title}</h1>
  <div class="views" role="group" aria-label={t('nav.views')}>
    {#each views as option (option.view)}
      <button
        type="button"
        aria-pressed={option.view === view}
        title={hint(t(option.label), option.keys)}
        onclick={() => onview(option.view)}
      >
        {t(option.label)}
      </button>
    {/each}
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
