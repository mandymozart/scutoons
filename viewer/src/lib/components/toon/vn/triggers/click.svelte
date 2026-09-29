<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import type { VNSliceEvent, VNEvent } from '@scutoons/content';
	import { EventProcessor } from '$lib/visual-novel/event-processor.ts';
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import { getGameContext } from '$lib/contexts/game.svelte';

	let { event }: { event: VNSliceEvent } = $props();

	const vn = getVisualNovelContext();
	const game = getGameContext();

	let triggerElement: HTMLDivElement | null = $state(null);
	let triggered = $state(false);

	const once = event.trigger?.once ?? true;
	const rect = event.trigger?.rect;

	function isWithinRect(e: MouseEvent, element: HTMLElement): boolean {
		if (!rect) return true;

		const bounds = element.getBoundingClientRect();
		const x = ((e.clientX - bounds.left) / bounds.width) * 100;
		const y = ((e.clientY - bounds.top) / bounds.height) * 100;

		return x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height;
	}

	async function handleClick(e: MouseEvent) {
		if (once && triggered) return;
		if (!triggerElement?.parentElement) return;
		if (!isWithinRect(e, triggerElement.parentElement)) return;

		const { trigger, ...vnEvent } = event;
		const processor = new EventProcessor([vnEvent as VNEvent], game, vn);
		const result = await processor.run();

		if (result) {
			console.log('🎯 Click trigger navigation:', result);
			goto(result);
		}

		if (once) triggered = true;
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
