<script lang="ts">
	import '@scutoons/tokens/index.css';
	import { getLocale, localizeHref, locales } from '$lib/paraglide/runtime.js';
	import { page, navigating } from '$app/state';
	import { goto } from '$app/navigation';
	import type { LayoutData } from './$types.js';
	import { onMount, type Snippet } from 'svelte';
	import Header from '$lib/components/header.svelte';
	import { draftPreview } from '$lib/draft-preview.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { invalidateAll } from '$app/navigation';
	import Loader from '$lib/components/loader.svelte';
	import ItemsHost from '$lib/components/items/items-host.svelte';
	import { setMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import { setSessionContext } from '$lib/contexts/session.svelte.ts';
	import { setInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { setItemsUiContext } from '$lib/contexts/items-ui.svelte.ts';
	import { setWorldContext } from '$lib/contexts/world.svelte.ts';
	import { initAnimatedFavicon } from '$lib/utils/favicon.ts';

	let { data, children }: { data: LayoutData; children: Snippet } = $props();

	let toons = $derived(data.toons);
	const session = setSessionContext();
	const membership = setMembershipContext(session);
	const inventory = setInventoryContext(session);
	const itemsUi = setItemsUiContext();
	// the map discoveries of the whole universe (agents/MAP.md §2.4), synced like progress for members
	setWorldContext(session);

	// the static build serves one fallback index.html (`<html lang="de">`): the document language follows the locale of
	// the URL (`/en/…` prefix, else cookie / browser language), like the dev server's `%paraglide.lang%`
	$effect(() => {
		void page.url.pathname;
		document.documentElement.lang = getLocale();
	});

	onMount(async () => {
		initAnimatedFavicon();

		// device access from a scanned code (+ silent legacy migration, periodic re-check)
		membership.init();
		inventory.init();

		const params = page.url.searchParams;
		// `/account?code=` = "Add to my account" from the QR landing page: sign in, then claim
		const addToAccount = /\/account\/?$/.test(page.url.pathname);
		// magic link from the login mail: `<origin>/?login=<token>[&c=<code>]`
		const magicToken = params.get('login') || params.get('token') || null;
		const code = params.get('code') || (magicToken ? params.get('c') : null) || null;
		// `?code=…&claim=1` = "This is mine? File a claim" from the QR landing page (ITEMS.md "Ownership, claims, gifting")
		const dispute = params.get('claim') === '1';

		if (code || magicToken) {
			const url = new URL(window.location.href);
			url.searchParams.delete('code');
			url.searchParams.delete('claim');
			if (magicToken) url.searchParams.delete('c');
			// already consumed by the toon inventory (child onMount runs first)
			url.searchParams.delete('inventory');
			url.searchParams.delete('login');
			url.searchParams.delete('token');
			goto(url.pathname + url.search, { replaceState: true });
		}

		// magic link from the login mail, else the single sign-on session (any SCUTOONS app) or a stored token
		await session.init(magicToken);

		if (code) {
			// the sheet grants device access via /access/ right away and offers "Add to my account"
			itemsUi.openClaim(code, addToAccount, dispute);
		}
	});
</script>

<Loader visible={!!navigating.to} oncancel={() => goto(page.url.href, { noScroll: true })} />

{#if draftPreview.on}
	<!-- a creator's draft preview is on (account page): drafts and coming-soon chapters are readable here -->
	<p class="draft-preview" data-testid="draft-preview-bar">
		{m['account.draft_preview']()}
		<button type="button" onclick={() => { draftPreview.set(false); invalidateAll(); }}>{m['account.draft_preview_off']()}</button>
	</p>
{/if}
<Header {toons} />

<div class="content">
	{@render children()}
</div>

<ItemsHost />

<div style="display:none">
	{#each locales as locale}
		<a href={localizeHref(page.url.pathname, { locale })}>{locale}</a>
	{/each}
</div>

<style>
	/* a small label bottom left: clear of the fixed header buttons, below the menus that slide in (owner 2026-10-03:
	   the bar at the top looked broken on mobile) */
	.draft-preview {
		position: fixed;
		left: 0.5rem;
		bottom: calc(0.5rem + env(safe-area-inset-bottom, 0px));
		z-index: 900;
		margin: 0;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.25rem 0.25rem 0.25rem 0.5rem;
		background: var(--ink);
		color: var(--paper);
		font-size: 0.6667rem;
		line-height: 1;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		white-space: nowrap;
		button {
			font: inherit;
			color: inherit;
			background: none;
			border: 1px solid currentColor;
			box-sizing: border-box;
			height: var(--control-height);
			padding: 0 0.5rem;
			cursor: pointer;
		}
	}
</style>
