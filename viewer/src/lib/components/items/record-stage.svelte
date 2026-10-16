<script lang="ts">
	import { untrack } from 'svelte';
	import { mountRecordStage, type RecordStageHandle } from './record-stage.js';

	/**
	 * The record player card's 3D stage (FEATURES 10.102): the LP's model – its sleeve with the record inside – and a
	 * turntable; on play the record comes out onto the platter and turns (record-stage.js). Mounted only while the card
	 * is open (one renderer, freed on close); a model that fails calls `onfail` and the card shows its 2D deck.
	 */
	let {
		url,
		playing,
		side,
		arm,
		lifted,
		label,
		onfail
	}: { url: string; playing: boolean; side: 'a' | 'b'; arm: number; lifted: boolean; label: string; onfail: () => void } = $props();

	let canvas = $state<HTMLCanvasElement>();
	let stage: RecordStageHandle | null = null;
	let ready = $state(false);

	$effect(() => {
		const el = canvas;
		if (!el || !url) return;
		ready = false;
		const handle = mountRecordStage(el, url, { onload: () => (ready = true), onerror: () => onfail() });
		untrack(() => handle.update({ playing, side, arm, lifted }));
		stage = handle;
		return () => {
			handle.dispose();
			stage = null;
		};
	});

	$effect(() => {
		const state = { playing, side, arm, lifted };
		stage?.update(state);
	});
</script>

<canvas bind:this={canvas} class:ready aria-hidden="true" title={label} data-testid="record-stage"></canvas>

<style>
	canvas {
		display: block;
		width: min(24rem, 100%);
		aspect-ratio: 4 / 3;
		opacity: 0;
		transition: opacity 0.3s ease;
	}

	canvas.ready {
		opacity: 1;
	}
</style>
