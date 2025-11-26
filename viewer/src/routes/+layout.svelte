<script lang="ts">
	import { localizeHref, locales } from '$lib/paraglide/runtime.js';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import type { LayoutData } from './$types.js';
	import { onMount, type Snippet } from 'svelte';
	import Header from '$lib/components/header.svelte';
	import { setMembershipContext } from '$lib/contexts/membership.svelte.ts';

	let { data, children }: { data: LayoutData; children: Snippet } = $props();

	let toons = data.toons;
	const membership = setMembershipContext();

	onMount(async () => {
		// initAnimatedFavicon();

		membership.initFromStorage();
		const code = page.url.searchParams.get('code') || null;

		if (!membership.isValid && code) {
			await membership.validate(code);

			const url = new URL(window.location.href);
			url.searchParams.delete('code');
			goto(url.pathname + url.search, { replaceState: true });
		}
	});
</script>

<Header {toons} />

<div class="content">
	{@render children()}
</div>

<div style="display:none">
	{#each locales as locale}
		<a href={localizeHref(page.url.pathname, { locale })}>{locale}</a>
	{/each}
</div>

<style>
</style>
