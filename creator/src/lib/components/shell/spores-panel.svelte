<script lang="ts">
	import { onMount } from 'svelte';
	import { sporeScope } from '$lib/spores/scope.svelte.ts';
	import { SporesView, type SporeGesture } from '$lib/spores/view.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The Spores tab (#763): the front item's weave of the spore network, drawn by the Spores widget (deck-spores in
	 * wasm) – the same canvas the deck editor's node view is. What the item offers (lib/spores/scope.svelte.ts) is drawn;
	 * an item without a projector yet offers nothing.
	 */
	let host = $state<HTMLDivElement>();
	let canvas = $state<HTMLCanvasElement>();
	let view: SporesView | null = null;
	let failed = $state('');

	const weave = $derived(sporeScope.offer?.weave() ?? null);

	function ongesture(_g: SporeGesture) {
		// read-only for now: the story's documents are edited in the chapter editor, a rig's wiring in the deck editor
	}

	onMount(() => {
		if (!host || !canvas) return;
		const v = new SporesView(host, canvas, ongesture);
		view = v;
		const night = document.documentElement.dataset.theme !== 'day';
		v.theme(night);
		v.start().catch((e) => (failed = String(e)));
		return () => v.stop();
	});

	$effect(() => {
		view?.show(weave);
	});
</script>

<div class="spores" bind:this={host} data-testid="spores">
	<canvas bind:this={canvas} class:hidden={!weave}></canvas>
	{#if failed}
		<p class="sc-empty">{m['shell.spores_failed']()}</p>
	{:else if !weave}
		<p class="sc-empty">{m['shell.spores_none']()}</p>
	{/if}
</div>

<style>
	.spores {
		position: relative;
		width: 100%;
		height: 100%;
		min-height: 12rem;
		overflow: hidden;
	}
	canvas {
		display: block;
		outline: none;
	}
	.hidden {
		visibility: hidden;
	}
	.sc-empty {
		position: absolute;
		inset: 1rem auto auto 1rem;
		margin: 0;
	}
</style>
