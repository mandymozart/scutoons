<script lang="ts">
	import { onMount, type Component } from 'svelte';
	import { page } from '$app/state';
	import ToonShell from '$lib/components/toon/toon-shell.svelte';

	let { children, data, params } = $props();

	// #518 / #528: a reader in a session (`?live=<code>`): linked-reader.svelte joins it, asks first and shows the linked
	// sign (reader-session.ts tells it the story events it fires once the person said yes); normal reading loads none of it
	// and sends nothing
	let Linked = $state<Component | null>(null);
	onMount(() => {
		if (!page.url.searchParams.has('live')) return;
		void import('$lib/components/toon/linked-reader.svelte').then((c) => (Linked = c.default));
	});
</script>

<!-- one shell per toon: SvelteKit keeps this layout across /catpope/… → /toptoy/… – the shell (VN, audio, sequence, cast) must not -->
{#key params.toonId}
	<ToonShell {data} {params}>{@render children()}</ToonShell>
{/key}
{#if Linked}<Linked />{/if}
