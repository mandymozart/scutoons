<script lang="ts">
	import { isAreaTrigger, type Sample } from '@scutoons/content';
	import SampleTrigger from './sample-trigger.svelte';

	/** `areas: false` = without the click / hover areas (they are in the slice's layer stack – BACKLOG 10.54) */
	let { samples, areas = true }: { samples?: Sample[]; areas?: boolean } = $props();
</script>

{#if samples}
	{#each samples as sample}
		{#if areas || !isAreaTrigger(sample.trigger)}
			<SampleTrigger trigger={sample.trigger} />
		{/if}
	{/each}
{/if}
