<script lang="ts">
	// Account page → Offline-Inhalte → "App" (agents/research/PWA.md §2a, ACCOUNT-PAGE.md §0): what this browser can do
	// (lib/install-rules.ts `installWay`), always said – owner 2026-10-17: "no prompt, nothing" on Firefox / Android:
	// - available (Chromium): "Install app" → the browser's dialog (also while the install bar is snoozed);
	// - ios (iPhone / iPad Safari): Share, then "Add to Home Screen";
	// - android-firefox: ⋮ menu, then "Add to Home screen" (Firefox sends no install event);
	// - installed: "You're using the app";
	// - unsupported: no install here (Chrome on Android, Safari on iPhone; already installed → open it from the home screen).
	// Nothing only in the creator's Reader frame.
	import { install } from '$lib/install.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
</script>

{#if install.way !== 'hidden'}
	<fieldset class="install-app" data-testid="install-app-setting" data-way={install.way}>
		<legend>{m['pwa.app_title']()}</legend>
		{#if install.way === 'available'}
			<p class="action">
				<button type="button" class="btn-secondary" onclick={() => install.install()} data-testid="install-app">{m['pwa.install_app']()}</button>
				<small>{m['pwa.install_hint']()}</small>
			</p>
		{:else if install.way === 'ios'}
			<p class="how" data-testid="install-ios-howto">
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
		{:else if install.way === 'android-firefox'}
			<p class="how" data-testid="install-firefox-howto">
				<span>
					{m['pwa.firefox_add_before']()}
					<!-- Firefox's menu button (three dots), as the reader sees it in the toolbar -->
					<svg class="share" viewBox="0 0 16 16" width="16" height="16" role="img" aria-label={m['pwa.firefox_menu']()}>
						<circle cx="8" cy="3" r="1.5" fill="currentColor" /><circle cx="8" cy="8" r="1.5" fill="currentColor" /><circle cx="8" cy="13" r="1.5" fill="currentColor" />
					</svg>
					{m['pwa.firefox_add_after']()}
				</span>
				<small>{m['pwa.install_hint']()}</small>
			</p>
		{:else if install.way === 'installed'}
			<p class="how" data-testid="install-installed"><span>{m['pwa.installed_now']()}</span></p>
		{:else}
			<p class="how" data-testid="install-unsupported">
				<span>{m['pwa.cannot_install']()}</span>
				<small>{m['pwa.cannot_install_hint']()}</small>
			</p>
		{/if}
	</fieldset>
{/if}

<style>
	.install-app {
		border: 0;
		margin: 0;
		padding: 0;
		min-width: 0;
	}
	legend {
		padding: 0;
		margin-bottom: 0.75rem;
		font-size: 1rem;
		font-weight: 700;
	}
	p {
		margin: 0;
	}
	/* an action: the button, its hint beside it (wraps under it on a phone) */
	.action {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem 0.75rem;
	}
	.action button {
		height: var(--control-height);
		padding: 0 1rem;
	}
	.how {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	.share {
		width: 1rem;
		height: 1rem;
		vertical-align: -0.125rem;
	}
	small {
		font-size: 0.75rem;
		line-height: 1.3;
		color: var(--text-muted);
	}
</style>
