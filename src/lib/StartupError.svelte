<!--
  The startup error screen: why FreeCal couldn't start, in place of the whole
  window. It is drawn with the frontend's default settings, because the core
  couldn't say the stored ones. Closing the window also quits FreeCal.
-->
<script lang="ts">
  import { t } from '../i18n';

  interface Props {
    /** Why FreeCal couldn't start, in words. */
    reason: string;
    onquit: () => void;
  }

  let { reason, onquit }: Props = $props();
</script>

<main class="startup-error">
  <h1>{t('startup.title')}</h1>
  <p role="alert">{reason}</p>
  <button type="button" class="button solid" onclick={onquit}>{t('startup.quit')}</button>
</main>

<style>
  .startup-error {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-6);
    max-width: 36rem;
    padding: var(--space-12);
  }

  h1 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 600;
  }

  p {
    margin: 0;
  }
</style>
