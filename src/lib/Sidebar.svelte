<script lang="ts">
  import type { Account } from '../core/types';
  import { t } from '../i18n';

  interface Props {
    accounts: Account[];
  }

  let { accounts }: Props = $props();

  function accountName(account: Account): string {
    switch (account.provider) {
      case 'local':
        return t('account.local');
    }
  }
</script>

<nav class="sidebar" aria-label={t('sidebar.label')}>
  <ul>
    {#each accounts as account (account.id)}
      <li>{accountName(account)}</li>
    {/each}
  </ul>
</nav>

<style>
  .sidebar {
    width: 14rem;
    padding: 0.75rem;
    overflow-y: auto;
    border-right: 1px solid #ddd;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
</style>
