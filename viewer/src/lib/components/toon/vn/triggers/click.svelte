<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import type { VNSliceEvent, VNEvent } from '@scutoons/content';
	import { EventProcessor } from '$lib/visual-novel/event-processor.ts';
	import { shortHash } from '$lib/expressions/resolver.svelte.ts';
	import { getGroupScope } from '$lib/visual-novel/group-scope.ts';
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import { getGameContext } from '$lib/contexts/game.svelte';

	/** the slice events sharing this trigger: one run per click / hover / intersect, in their order */
	let { events }: { events: VNSliceEvent[] } = $props();

	const vn = getVisualNovelContext();
	const game = getGameContext();
	// the slice's group: a divert goes into its script, its text boxes (linked groups share a page)
	const group = getGroupScope();

	let triggerElement: HTMLDivElement | null = $state(null);
	let triggered = $state(false);
	/** a run that still waits (a message, choices): the trigger doesn't start another one meanwhile (owner 2026-10-03: the
	 * stroll end's choices were added four times while the last slice scrolled in and out) */
	let running = false;

	const once = $derived(events[0].trigger?.once ?? true);
	const rect = $derived(events[0].trigger?.rect);

	function isWithinRect(e: MouseEvent, element: HTMLElement): boolean {
		if (!rect) return true;

		const bounds = element.getBoundingClientRect();
		const x = ((e.clientX - bounds.left) / bounds.width) * 100;
		const y = ((e.clientY - bounds.top) / bounds.height) * 100;

		return x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height;
	}

	async function handleClick(e: MouseEvent) {
		if (once && triggered) return;
		if (running) return;
		if (!triggerElement?.parentElement) return;
		if (!isWithinRect(e, triggerElement.parentElement)) return;

		const processor = new EventProcessor(events.map(({ trigger, ...vnEvent }) => vnEvent as VNEvent), game, vn, group, `on/${shortHash(JSON.stringify(events[0].trigger))}`);
		running = true;
		// marked before the run (it can wait for the reader a long time)
		if (once) triggered = true;
		const result = await processor.run().finally(() => (running = false));

		if (result) {
			console.log('🎯 Click trigger navigation:', result);
			goto(result);
		}

	}

	onMount(() => {
		const parent = triggerElement?.parentElement;
		if (!parent) return;

		parent.addEventListener('click', handleClick);
		return () => parent.removeEventListener('click', handleClick);
	});
</script>

{#if rect}
	<div
		bind:this={triggerElement}
		class="event-trigger event-trigger--click"
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
	.event-trigger {
		position: absolute;
		pointer-events: none;
		opacity: 0.5;
		transition: opacity 0.3s ease;
		z-index: 2;
	}

	.event-trigger--click {
		border: 2px dashed rgba(76, 175, 80, 0.8);
		background: rgba(76, 175, 80, 0.1);
		box-shadow: inset 0 0 20px rgba(76, 175, 80, 0.3);
		cursor: pointer;
	}

	.event-trigger:hover {
		opacity: 0.8;
	}

	.event-trigger.triggered {
		opacity: 0.2;
		border-style: solid;
	}
</style>
