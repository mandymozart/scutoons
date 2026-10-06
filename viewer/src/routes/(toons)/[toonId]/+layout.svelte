<script lang="ts">
	import ToonShell from '$lib/components/toon/toon-shell.svelte';

	let { children, data, params } = $props();
</script>

<!-- one shell per toon: SvelteKit keeps this layout across /catpope/… → /toptoy/… – the shell (VN, audio, sequence, cast) must not -->
{#key params.toonId}
	<ToonShell {data} {params}>{@render children()}</ToonShell>
{/key}
