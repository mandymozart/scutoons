<script lang="ts">
	import type { VNSliceEvent } from '@scutoons/content';
	import { Trigger } from '@scutoons/content';
	import ClickTrigger from './triggers/click.svelte';
	import HoverTrigger from './triggers/hover.svelte';
	import IntersectTrigger from './triggers/intersect.svelte';

	let { events }: { events?: VNSliceEvent[] } = $props();

	/**
	 * Events with the same trigger (type, rect, once) run as ONE script, in their order: a click on a cucumber is
	 * message + addChoice + addChoice + showChoices – one run per event drops the choices a run adds but doesn't show
	 * (runner.ts), so the offer never appeared.
	 */
	const groups = $derived.by(() => {
		const byTrigger = new Map<string, VNSliceEvent[]>();
		for (const event of events ?? []) {
			if (!event.trigger) continue;
			const key = JSON.stringify(event.trigger);
			byTrigger.set(key, [...(byTrigger.get(key) ?? []), event]);
		}
		return [...byTrigger.values()];
	});
</script>

{#each groups as group (JSON.stringify(group[0].trigger))}
	{#if group[0].trigger?.type === Trigger.CLICK}
		<ClickTrigger events={group} />
	{:else if group[0].trigger?.type === Trigger.HOVER}
		<HoverTrigger events={group} />
	{:else if group[0].trigger?.type === Trigger.INTERSECT}
		<IntersectTrigger events={group} />
	{/if}
{/each}
