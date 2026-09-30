<script lang="ts">
	import '@scutoons/tokens/index.css';
	import { getLocale, localizeHref, locales } from '$lib/paraglide/runtime.js';
	import { page, navigating } from '$app/state';
	import { goto } from '$app/navigation';
	import type { LayoutData } from './$types.js';
	import { onMount, type Snippet } from 'svelte';
	import Header from '$lib/components/header.svelte';
	import Loader from '$lib/components/loader.svelte';
	import ItemsHost from '$lib/components/items/items-host.svelte';
	import { setMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import { setSessionContext } from '$lib/contexts/session.svelte.ts';
	import { setInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { setItemsUiContext } from '$lib/contexts/items-ui.svelte.ts';
	import { initAnimatedFavicon } from '$lib/utils/favicon.ts';

	let { data, children }: { data: LayoutData; children: Snippet } = $props();

	let toons = $derived(data.toons);
	const session = setSessionContext();
	const membership = setMembershipContext(session);
	const inventory = setInventoryContext(session);
	const itemsUi = setItemsUiContext();

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

		if (code || magicToken) {
			const url = new URL(window.location.href);
			url.searchParams.delete('code');
			if (magicToken) url.searchParams.delete('c');
			// already consumed by the toon inventory (child onMount runs first)
			url.searchParams.delete('inventory');
			url.searchParams.delete('login');
			url.searchParams.delete('token');
			goto(url.pathname + url.search, { replaceState: true });
		}

		// magic link from the login mail
		if (magicToken && !session.token) {
			try {
				await session.verifyToken(magicToken);
			} catch (e) {
				console.warn('Magic link login failed', e);
			}
		}
		await session.init();

		if (code) {
			// the sheet grants device access via /access/ right away and offers "Add to my account"
			itemsUi.openClaim(code, addToAccount);
		}
	});
</script>

<Loader visible={!!navigating.to} oncancel={() => goto(page.url.href, { noScroll: true })} />

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
</style>
