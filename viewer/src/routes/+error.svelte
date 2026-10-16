<script lang="ts">
	import { page } from '$app/state';
	import { m } from '$lib/paraglide/messages.js';
	// offline (the service worker served the app shell, the API can't be reached): say so instead of an error
	// (agents/research/PWA.md "Offline page")
	const offline = typeof navigator !== 'undefined' && !navigator.onLine;
</script>

<div class="view">
	{#if offline}
		<p data-testid="offline">{m['pwa.offline']()}</p>
		<button onclick={() => location.reload()}>{m['pwa.retry']()}</button>
	{:else}
		<h1>Upsi! Mea culpa!</h1>
		<p>{page.error?.message}</p>
	{/if}
</div>

<style>
	.view {
		margin-top: 4rem;
	}
</style>
