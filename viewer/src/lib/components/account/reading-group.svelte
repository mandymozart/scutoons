<script lang="ts">
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import MediaQualitySetting from './media-quality-setting.svelte';
	import AccountToggle from './account-toggle.svelte';

	/**
	 * Account page "Lesen": media quality, and for members "Show hints" (off by default, BACKLOG M.1) and "Hide
	 * sensitive content" (on by default) – both on the account (`PUT /me/`). Guests always hide sensitive content and see
	 * no hints: one line says these come with an account (owner 2026-10-16, ACCOUNT-PAGE.md Q2).
	 */
	const session = getSessionContext();
</script>

<MediaQualitySetting />

{#if session.isLoggedIn && session.user}
	<AccountToggle
		checked={session.user.show_hints === true}
		label={m['account.show_hints']()}
		hint={m['account.show_hints_hint']()}
		save={(v) => session.setShowHints(v)}
		testid="show-hints"
	/>
	<AccountToggle
		checked={session.user.hide_sensitive !== false}
		label={m['account.hide_sensitive']()}
		hint={m['account.hide_sensitive_hint']()}
		save={(v) => session.setHideSensitive(v)}
		testid="hide-sensitive"
	/>
{:else}
	<p class="guest">{m['account.reading_guest']()}</p>
{/if}

<style>
	.guest {
		margin: 0;
		font-size: 0.75rem;
		line-height: 1.3;
		color: var(--text-muted);
	}
</style>
