<script module lang="ts">
	/**
	 * The first stage of a page load shows the way out of the sleeve even when the record already plays (owner 2026-10-16:
	 * "beim ersten Mal öffnen … pro Browser Reload"); after that a record that is on just lies on the platter.
	 */
	let introduced = false;
</script>

<script lang="ts">
	import { untrack } from 'svelte';
	import type { ModelSource } from '@scutoons/content';
	import { modelLoader } from './model-source.ts';
	import { mountRecordStage, type RecordStageHandle } from './record-stage.js';

	/**
	 * The record player card's 3D stage (FEATURES 10.102): the LP's model – its sleeve with the record inside – and a
	 * turntable; on play the record comes out onto the platter and turns (record-stage.js). Mounted only while the card
	 * is open (one renderer, freed on close); a model that fails calls `onfail` and the card shows its 2D deck.
	 * The canvas fills its parent (the card's top, as its background – owner 2026-10-16); `lift` = the lower part of it
	 * that the controls cover (the scene moves above them).
	 */
	let {
		model,
		playing,
		side,
		arm,
		lifted,
		label,
		lift = 0,
		resetKey = 0,
		placed = false,
		onfail
	}: {
		/** the item's model: a .glb or the shared record model's look (model-source.ts) */
		model: ModelSource;
		playing: boolean;
		side: 'a' | 'b';
		arm: number;
		lifted: boolean;
		label: string;
		lift?: number;
		/** a new value eases the reader's pivot / zoom back (the card bumps it on a pick in the song list) */
		resetKey?: number;
		/** the record is already on when the stage opens: it lies on the platter, the way out runs only when it is put on */
		placed?: boolean;
		onfail: () => void;
	} = $props();

	let canvas = $state<HTMLCanvasElement>();
	let stage: RecordStageHandle | null = null;
	let ready = $state(false);

	$effect(() => {
		const el = canvas;
		if (!el || !model) return;
		ready = false;
		const onAlready = introduced && untrack(() => placed);
		introduced = true;
		const handle = mountRecordStage(el, modelLoader(model), { lift: untrack(() => lift), placed: onAlready, onload: () => (ready = true), onerror: () => onfail() });
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

	$effect(() => {
		void resetKey;
		untrack(() => stage?.resetView());
	});
</script>

<canvas bind:this={canvas} class:ready aria-hidden="true" title={label} data-testid="record-stage"></canvas>

<style>
	canvas {
		position: absolute;
		inset: 0;
		display: block;
		width: 100%;
		height: 100%;
		opacity: 0;
		transition: opacity 0.3s ease;
		/* drag pivots, pinch zooms (the sheet scrolls from the song and controls below) */
		touch-action: none;
		cursor: grab;
	}

	canvas:active {
		cursor: grabbing;
	}

	canvas.ready {
		opacity: 1;
	}
</style>
