<script lang="ts">
	import type { Sample } from '@scutoons/content';
	import SampleTrigger from './sample-trigger.svelte';

	let { samples }: { samples?: Sample[] } = $props();
</script>

{#if samples}
	{#each samples as sample}
		<SampleTrigger trigger={sample.trigger} />
	{/each}
{/if}
