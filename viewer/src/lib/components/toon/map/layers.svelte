<script lang="ts">
	import Layer from './layer.svelte';

	let { layers, width, height } = $props();
	const keys = $derived(Object.keys(layers));
</script>

{#if keys.length > 0}
	{#each keys as key}
		<Layer {key} image={layers[key]} {width} {height} />
	{/each}
{/if}
