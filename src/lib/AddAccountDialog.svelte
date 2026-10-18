<!--
  The "Add account…" dialog: the user enters a CalDAV server's address, a
  username and a password, and the core checks them against the server. A
  failure is shown in the dialog, which keeps what was typed. It can't be
  cancelled while the server is asked, since the core may still add the
  Account then.
-->
<script lang="ts">
  import type { CoreApi } from '../core/CoreApi';
  import type { AddedAccount } from '../core/types';
  import { errorText, t } from '../i18n';
  import Dialog from './Dialog.svelte';

  interface Props {
    core: CoreApi;
    /** Called once the Account is added, with the Calendars its server offers. */
    onadded: (added: AddedAccount) => void;
    oncancel: () => void;
  }

  let { core, onadded, oncancel }: Props = $props();

  const id = $props.id();

  let url = $state('');
  let username = $state('');
  let password = $state('');
  let connecting = $state(false);
  let failure = $state<string>();

  async function add() {
    connecting = true;
    failure = undefined;
    try {
      onadded(await core.addCalDavAccount(url.trim(), username, password));
    } catch (error) {
      failure = t('addAccount.failed', { error: errorText(error) });
    } finally {
      connecting = false;
    }
  }

  function cancel() {
    if (!connecting) oncancel();
  }
</script>

<Dialog title={t('addAccount.title')} onclose={cancel}>
  <form
    id="{id}-form"
    class="form"
    onsubmit={(submitted) => {
      submitted.preventDefault();
      add();
    }}
  >
    <label class="field">
      <span class="field-label">{t('addAccount.url')}</span>
      <input
        class="input"
        inputmode="url"
        spellcheck="false"
        required
        autocomplete="url"
        placeholder="https://"
        bind:value={url}
        data-initial-focus
      />
    </label>
    <label class="field">
      <span class="field-label">{t('addAccount.username')}</span>
      <input class="input" required autocomplete="username" bind:value={username} />
    </label>
    <label class="field">
      <span class="field-label">{t('addAccount.password')}</span>
      <input
        class="input"
        type="password"
        required
        autocomplete="current-password"
        bind:value={password}
      />
    </label>
    <div aria-live="polite">
      {#if connecting}
        <p class="note">{t('addAccount.connecting')}</p>
      {:else if failure}
        <p class="problem" role="alert">{failure}</p>
      {/if}
    </div>
  </form>

  {#snippet actions()}
    <button type="button" class="button outline" disabled={connecting} onclick={cancel}>
      {t('common.cancel')}
    </button>
    <button type="submit" form="{id}-form" class="button solid" disabled={connecting}>
      {t('addAccount.add')}
    </button>
  {/snippet}
</Dialog>
