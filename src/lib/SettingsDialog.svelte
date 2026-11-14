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
    /** Whether the desktop is still being asked to change start at login. */
    startAtLoginPending?: boolean;
    /** One setting changed; `update` gives only that one. */
    onchange: (update: SettingsUpdate) => void;
    onclose: () => void;
  }

  let { settings, failure, startAtLoginPending = false, onchange, onclose }: Props = $props();

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
    <div class="segmented">
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
  <fieldset class="setting">
    <legend>{t('settings.background')}</legend>
    <div class="switches">
      <label class="check">
        <input
          type="checkbox"
          role="switch"
          checked={settings.runInBackground}
          onchange={(event) => onchange({ runInBackground: event.currentTarget.checked })}
        />
        {t('settings.runInBackground')}
      </label>
      <!-- Disabled while the desktop is asked, which may show its own dialog. -->
      <label class="check">
        <input
          type="checkbox"
          role="switch"
          checked={settings.startAtLogin}
          disabled={startAtLoginPending}
          onchange={(event) => onchange({ startAtLogin: event.currentTarget.checked })}
        />
        {t('settings.startAtLogin')}
      </label>
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

  .setting + .setting {
    margin-top: var(--space-5);
  }

  /* A segmented control, like the toolbar's view switch. */
  .segmented {
    display: flex;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .segmented label {
    display: flex;
    flex: 1;
    align-items: center;
    justify-content: center;
    min-height: var(--control-height);
    padding: 0 var(--space-4);
    color: var(--foreground-muted);
    cursor: pointer;
  }

  .segmented label + label {
    border-left: 1px solid var(--border);
  }

  .segmented label:hover {
    background: var(--hover);
    color: var(--foreground);
  }

  .segmented label:has(input:checked) {
    background: var(--hover-strong);
    color: var(--foreground);
    font-weight: 500;
  }

  .segmented label:has(input:focus-visible) {
    outline: 2px solid var(--focus-ring);
    outline-offset: -2px;
  }

  /* The radio itself is hidden but stays focusable and keyboard-operable. */
  .segmented input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .switches {
    display: grid;
    gap: var(--space-4);
  }

  .switches input {
    margin: 0;
  }

  .failure {
    margin-top: var(--space-4);
    color: var(--danger);
  }
</style>
