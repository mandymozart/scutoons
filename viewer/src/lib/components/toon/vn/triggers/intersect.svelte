<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import type { VNSliceEvent, VNEvent } from '@scutoons/content';
	import { EventProcessor } from '$lib/visual-novel/event-processor.ts';
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import { getGameContext } from '$lib/contexts/game.svelte';

	/** the slice events sharing this trigger: one run per click / hover / intersect, in their order */
	let { events }: { events: VNSliceEvent[] } = $props();

	const vn = getVisualNovelContext();
	const game = getGameContext();

	let triggerElement: HTMLDivElement | null = $state(null);
	let triggered = $state(false);
	/** a run that still waits (a message, choices): the trigger doesn't start another one meanwhile (owner 2026-10-03: the
	 * stroll end's choices were added four times while the last slice scrolled in and out) */
	let running = false;

	const once = $derived(events[0].trigger?.once ?? true);
	const rect = $derived(events[0].trigger?.rect);

	async function executeEvent() {
		if (once && triggered) return;
		if (running) return;

		const processor = new EventProcessor(events.map(({ trigger, ...vnEvent }) => vnEvent as VNEvent), game, vn);
		running = true;
		// marked before the run (it can wait for the reader a long time)
		if (once) triggered = true;
		const result = await processor.run().finally(() => (running = false));

		if (result) {
			console.log('🎯 Intersect trigger navigation:', result);
			goto(result);
		}

	}

	onMount(() => {
		if (!triggerElement) return;

		const target = rect ? triggerElement : triggerElement.parentElement;
		if (!target) return;

		const observer = new IntersectionObserver(
			async (entries) => {
				for (const entry of entries) {
					if (entry.isIntersecting) {
						await executeEvent();
					}
				}
			},
			{ threshold: 0.5 }
		);

		observer.observe(target);
		return () => observer.disconnect();
	});
</script>

{#if rect}
	<div
		bind:this={triggerElement}
		class="vn-trigger intersect"
		class:triggered
		style:left="{rect.x}%"
		style:top="{rect.y}%"
		style:width="{rect.width}%"
		style:height="{rect.height}%"
	></div>
{:else}
	<div bind:this={triggerElement} style="display: none;"></div>
{/if}

<style>
	.vn-trigger {
		position: absolute;
		pointer-events: none;
		opacity: 0.5;
		transition: opacity 0.3s ease;
		z-index: 2;
	}

	.vn-trigger.intersect {
		border: 2px dashed rgba(255, 152, 0, 0.8);
		background: rgba(255, 152, 0, 0.1);
		box-shadow: inset 0 0 20px rgba(255, 152, 0, 0.3);
	}

	.vn-trigger:hover {
		opacity: 0.8;
	}

	.vn-trigger.triggered {
		opacity: 0.2;
		border-style: solid;
	}
</style>
