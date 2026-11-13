<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { LIVE_QUIC, LIVE_URL } from '$lib/config.ts';
	import ToonShell from '$lib/components/toon/toon-shell.svelte';

	let { children, data, params } = $props();

	// #518: a reader in a session (`?live=<code>`, reader-session.ts) tells it the story events it fires; normal reading
	// loads none of it and sends nothing
	onMount(() => {
		if (!page.url.searchParams.has('live')) return;
		void import('$lib/reader-session.ts').then(({ sessionOf, joinSession }) => {
			const relay = sessionOf(page.url.searchParams, LIVE_URL, LIVE_QUIC);
			if (relay) joinSession(relay);
		});
	});
</script>

<!-- one shell per toon: SvelteKit keeps this layout across /catpope/… → /toptoy/… – the shell (VN, audio, sequence, cast) must not -->
{#key params.toonId}
	<ToonShell {data} {params}>{@render children()}</ToonShell>
{/key}
