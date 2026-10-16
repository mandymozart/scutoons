<script lang="ts">
	// "Install SCUTOONS as an app – Install / Not now" (agents/research/PWA.md "Install prompt"): Android / Chromium only,
	// at the moment lib/install-rules.ts picks. The update prompt's look; a waiting update goes first. Its text links to
	// the account page's group for the details (Offline-Inhalte → App, ACCOUNT-PAGE.md §0.7).
	import { install } from '$lib/install.svelte.ts';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { pwa } from '$lib/pwa.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
</script>

{#if install.offer && !pwa.updateReady}
	<div class="install" role="status" data-testid="install-prompt">
		<p><a href={localizeHref('/account/offline')} data-testid="install-details">{m['pwa.install_offer']()}</a></p>
		<button class="btn-secondary" onclick={() => install.notNow()} data-testid="install-not-now">{m['pwa.not_now']()}</button>
		<button onclick={() => install.install()} data-testid="install-app">{m['pwa.install']()}</button>
	</div>
{/if}

<style>
	.install {
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
	.install p {
		margin: 0;
		flex: 1 1 12rem;
	}
	.install a {
		color: inherit;
	}
	.install button {
		padding: 0 1rem;
	}
</style>
