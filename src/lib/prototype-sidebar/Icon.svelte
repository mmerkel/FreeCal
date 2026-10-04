<!-- PROTOTYPE. Lucide-style line icons (ISC), drawn inline. -->
<script lang="ts">
  let { name, size = 16 }: { name: 'plus' | 'more' | 'pencil' | 'trash' | 'check' | 'chevron' | 'eye'; size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  {#if name === 'plus'}<path d="M5 12h14" /><path d="M12 5v14" />
  {:else if name === 'more'}<circle cx="12" cy="5" r="1" /><circle cx="12" cy="12" r="1" /><circle cx="12" cy="19" r="1" />
  {:else if name === 'pencil'}<path d="M21.17 6.81a1 1 0 0 0-3.99-3.99L3.84 16.17a2 2 0 0 0-.5.83l-1.32 4.35a.5.5 0 0 0 .62.62l4.35-1.32a2 2 0 0 0 .83-.5z" /><path d="m15 5 4 4" />
  {:else if name === 'trash'}<path d="M3 6h18" /><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" /><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
  {:else if name === 'check'}<path d="M20 6 9 17l-5-5" />
  {:else if name === 'chevron'}<path d="m6 9 6 6 6-6" />
  {:else if name === 'eye'}<path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z" /><circle cx="12" cy="12" r="3" />
  {/if}
</svg>
