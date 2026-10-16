<script lang="ts">
	// Account page "App" (agents/research/PWA.md "Install prompt"): "Install app" whenever the browser can install
	// (Android / Chromium, not already installed) – also while the bar is snoozed. Nothing otherwise (iOS: a later how-to).
	import { install } from '$lib/install.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
</script>

{#if install.available}
	<fieldset class="install-app" data-testid="install-app-setting">
		<legend>{m['pwa.app_title']()}</legend>
		<p>
			<button type="button" class="btn-secondary" onclick={() => install.install()}>{m['pwa.install_app']()}</button>
			<small>{m['pwa.install_hint']()}</small>
		</p>
	</fieldset>
{/if}

<style>
	.install-app {
		border: 0;
		margin: 0 0 1rem;
		padding: 0;
		min-width: 0;
	}
	/* like the account page's section headings (fx-setting.svelte) */
	legend {
		padding: 0;
		margin-bottom: 0.75rem;
		font-size: 1rem;
		font-weight: 700;
	}
	p {
		margin: 0;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem 0.75rem;
	}
	button {
		height: var(--control-height);
		padding: 0 1rem;
	}
	small {
		font-size: 0.75rem;
		line-height: 1.3;
		color: #6b6b6b;
	}
</style>
