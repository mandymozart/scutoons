<script lang="ts">
	// "A new version is ready – Reload" (agents/research/PWA.md "Update flow"): the service worker never swaps the app
	// under the reader; the new version waits for this tap (or the next start). The banner look of items-host's notice.
	import { pwa } from '$lib/pwa.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
</script>

{#if pwa.updateReady}
	<div class="update" role="status" data-testid="update-prompt">
		<p>{m['pwa.update_ready']()}</p>
		<button class="btn-secondary" onclick={() => pwa.dismiss()}>{m['pwa.later']()}</button>
		<button onclick={() => pwa.apply()} data-testid="update-reload">{m['pwa.reload']()}</button>
	</div>
{/if}

<style>
	.update {
		position: fixed;
		z-index: 2040;
		left: 0.5rem;
		right: 0.5rem;
		bottom: 0.5rem;
		max-width: 31.25rem;
		margin: 0 auto;
		box-sizing: border-box;
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular, 2px) solid var(--ink);
		padding: 0.75rem;
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 0.5rem;
	}
	.update p {
		margin: 0;
		flex: 1 1 12rem;
	}
	.update button {
		padding: 0 1rem;
	}
</style>
