<script lang="ts">
	import type { VNSliceEvent } from '@scutoons/content';
	import { Trigger } from '@scutoons/content';
	import ClickTrigger from './triggers/click.svelte';
	import HoverTrigger from './triggers/hover.svelte';
	import IntersectTrigger from './triggers/intersect.svelte';

	let { events }: { events?: VNSliceEvent[] } = $props();
</script>

{#if events}
	{#each events as event}
		{#if event.trigger?.type === Trigger.CLICK}
			<ClickTrigger {event} />
		{:else if event.trigger?.type === Trigger.HOVER}
			<HoverTrigger {event} />
		{:else if event.trigger?.type === Trigger.INTERSECT}
			<IntersectTrigger {event} />
		{/if}
	{/each}
{/if}
