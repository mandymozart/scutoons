<script lang="ts">
	import { draftPreview } from '$lib/draft-preview.svelte.ts';
	import { invalidateAll } from '$app/navigation';
	import { confirmDialog } from '@scutoons/ui';
	import { getWorldContext } from '$lib/contexts/world.svelte.ts';
	import { CREATOR_URL } from '$lib/config.ts';
	import { m } from '$lib/paraglide/messages.js';
	import AccountToggle from './account-toggle.svelte';

	/**
	 * Account page "Creator" (creators / admins): the link to the creator, the draft preview (read draft and coming-soon
	 * chapters here, this device only) and, while it is on, "Reset the map" for checking (owner 2026-10-11: only the
	 * map, the rest of the progress stays).
	 */
	const world = getWorldContext();
	/** the map reset happened in this visit (the hint says so) */
	let mapReset = $state(false);

	async function resetMap() {
		if (!world) return;
		const ok = await confirmDialog({ title: m['account.reset_map_confirm_title'](), message: m['account.reset_map_confirm'](), confirmLabel: m['account.reset_map'](), cancelLabel: m['account.reset_map_cancel']() });
		if (!ok) return;
		world.resetMap();
		mapReset = true;
	}
</script>

<p class="line">
	<!-- another app: a new window without an opener – the installed Creator opens if there is one (research/PWA-APPS.md §3a) -->
	<a class="btn btn-secondary" href={CREATOR_URL} target="_blank" rel="noopener">{m['account.creator_link']()}</a>
	<small>{m['account.creator_hint']()}</small>
</p>
<AccountToggle
	checked={draftPreview.on}
	label={m['account.draft_preview']()}
	hint={m['account.draft_preview_hint']()}
	save={(v) => {
		draftPreview.set(v);
		void invalidateAll();
	}}
	testid="draft-preview"
/>
{#if draftPreview.on && world}
	<p class="line">
		<button type="button" class="btn btn-secondary" onclick={resetMap} data-testid="reset-map">{m['account.reset_map']()}</button>
		<small>{mapReset ? m['account.reset_map_done']() : m['account.reset_map_hint']()}</small>
	</p>
{/if}

<style>
	.line {
		display: flex;
		align-items: center;
		gap: 0.5rem 1rem;
		flex-wrap: wrap;
		margin: 0;
	}
	.line .btn {
		padding: 0 1rem;
		text-decoration: none;
	}
	small {
		flex: 1 1 12rem;
		font-size: 0.75rem;
		line-height: 1.3;
		color: var(--text-muted);
	}
</style>
