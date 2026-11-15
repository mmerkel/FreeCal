<!--
  A list of Journal Event Modifications: what each did to which Event, in
  which Calendar, and whether it is undone. In an Event's History each also
  says when and from where, and has Undo; in an expanded Journal Transaction
  of the Change Journal list, each has Undo only where its Journal
  Transaction is undone one Event at a time (ADR 0009).
-->
<script lang="ts">
  import type { Calendar, ListedJournalEventModification } from '../core/types';
  import { t } from '../i18n';
  import { journalTimeText, modificationUndoing, type Undoing } from './undo';

  interface Props {
    modifications: ListedJournalEventModification[];
    /** Every Calendar, to name those of the Events. */
    calendars: Calendar[];
    /** Whether each says when it was made and from where, as an Event's History does. */
    history: boolean;
    /** Whether each not undone has Undo. */
    undoEach: boolean;
    /** Name of the list, for screen readers. */
    label: string;
    onundo: (undoing: Undoing) => void;
  }

  let { modifications, calendars, history, undoEach, label, onundo }: Props = $props();

  const calendarName = (id: Calendar['id']) =>
    calendars.find((calendar) => calendar.id === id)?.name ?? t('journal.calendarGone');
</script>

<ul class="journal-list" aria-label={label}>
  {#each modifications as modification, index (index)}
    {@const undoing = undoEach ? modificationUndoing(modification) : null}
    <li class="journal-row">
      <div class="journal-row-text">
        <span>
          {t(`journal.event.${modification.kind}`, {
            title: modification.title || t('event.untitled'),
          })}
        </span>
        <span class="note">
          {#if history}
            {journalTimeText(modification.madeAt)} ·
            {t(`journal.origin.${modification.origin.kind}`)} ·
          {/if}
          <bdi>{calendarName(modification.calendarId)}</bdi>
          {#if modification.undone}
            · {t('journal.undone')}
          {/if}
        </span>
      </div>
      {#if undoing}
        <button type="button" class="button outline" onclick={() => onundo(undoing)}>
          {t('common.undo')}
        </button>
      {/if}
    </li>
  {/each}
</ul>
