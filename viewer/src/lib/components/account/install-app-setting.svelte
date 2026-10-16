<script lang="ts">
	// Account page "App" (agents/research/PWA.md "Install prompt"): "Install app" whenever the browser can install
	// (Android / Chromium, not already installed) – also while the bar is snoozed. iPhone / iPad Safari (no install
	// event): how to add it by hand – Share, then "Add to Home Screen" (PWA phase 4). Nothing otherwise.
	import { install } from '$lib/install.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
</script>

{#if install.available || install.iosHowTo}
	<fieldset class="install-app" data-testid="install-app-setting">
		<legend>{m['pwa.app_title']()}</legend>
		{#if install.available}
			<p>
				<button type="button" class="btn-secondary" onclick={() => install.install()}>{m['pwa.install_app']()}</button>
				<small>{m['pwa.install_hint']()}</small>
			</p>
		{:else}
			<p class="ios" data-testid="install-ios-howto">
				<span>
					{m['pwa.ios_add_before']()}
					<!-- Safari's share icon (a square with an arrow up), as the reader sees it in the toolbar -->
					<svg class="share" viewBox="0 0 16 16" width="16" height="16" role="img" aria-label={m['pwa.ios_share']()}>
						<path d="M8 1v9M4.5 4.5 8 1l3.5 3.5M5 7H3v8h10V7h-2" fill="none" stroke="currentColor" stroke-width="1.5" />
					</svg>
					{m['pwa.ios_add_after']()}
				</span>
				<small>{m['pwa.ios_hint']()}</small>
			</p>
		{/if}
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
	.ios {
		flex-direction: column;
		align-items: flex-start;
		gap: 0.25rem;
	}
	.share {
		width: 1rem;
		height: 1rem;
		vertical-align: -0.125rem;
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
