<script lang="ts">
	import type {  Sample } from '$lib/models/audio.ts';
	import SampleTrigger from './sample-trigger.svelte';

	let { samples }: { samples?: Sample[] } = $props();
	$inspect(samples);
</script>

{#if samples}
{#each samples as sample}
	<SampleTrigger trigger={sample.trigger} />
{/each}
{/if}
