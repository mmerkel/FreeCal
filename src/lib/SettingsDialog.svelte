<!--
  The Settings dialog. Every change takes effect and is saved at once, so it
  has no buttons but the ✕. Its sections come in the order Date & time,
  Appearance, Background; a section with no settings yet isn't shown.
-->
<script lang="ts">
  import type { Appearance, Settings, SettingsUpdate } from '../core/types';
  import { t, type MessageKey } from '../i18n';
  import Dialog from './Dialog.svelte';

  interface Props {
    settings: Settings;
    /** Why the last change couldn't be saved, if it couldn't. */
    failure?: string;
    /** One setting changed; `update` gives only that one. */
    onchange: (update: SettingsUpdate) => void;
    onclose: () => void;
  }

  let { settings, failure, onchange, onclose }: Props = $props();

  const id = $props.id();

  const appearances: { value: Appearance; label: MessageKey }[] = [
    { value: 'system', label: 'appearance.system' },
    { value: 'light', label: 'appearance.light' },
    { value: 'dark', label: 'appearance.dark' },
  ];
</script>

<Dialog title={t('settings.title')} {onclose}>
  <fieldset class="setting">
    <legend>{t('settings.appearance')}</legend>
    <div class="choices">
      {#each appearances as appearance (appearance.value)}
        <label>
          <input
            type="radio"
            name="{id}-appearance"
            value={appearance.value}
            checked={settings.appearance === appearance.value}
            data-initial-focus={settings.appearance === appearance.value || undefined}
            onchange={() => onchange({ appearance: appearance.value })}
          />
          {t(appearance.label)}
        </label>
      {/each}
    </div>
  </fieldset>
  {#if failure}
    <p class="failure" role="alert">{failure}</p>
  {/if}
</Dialog>

<style>
  .setting {
    margin: 0;
    padding: 0;
    border: none;
  }

  legend {
    margin-bottom: var(--space-3);
    padding: 0;
    color: var(--foreground);
    font-weight: 600;
  }

  /* A segmented control, like the toolbar's view switch. */
  .choices {
    display: flex;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  label {
    display: flex;
    flex: 1;
    align-items: center;
    justify-content: center;
    min-height: var(--control-height);
    padding: 0 var(--space-4);
    color: var(--foreground-muted);
    cursor: pointer;
  }

  label + label {
    border-left: 1px solid var(--border);
  }

  label:hover {
    background: var(--hover);
    color: var(--foreground);
  }

  label:has(input:checked) {
    background: var(--hover-strong);
    color: var(--foreground);
    font-weight: 500;
  }

  label:has(input:focus-visible) {
    outline: 2px solid var(--focus-ring);
    outline-offset: -2px;
  }

  /* The radio itself is hidden but stays focusable and keyboard-operable. */
  input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .failure {
    margin-top: var(--space-4);
    color: var(--danger);
  }
</style>
