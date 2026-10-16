<script module lang="ts">
	/**
	 * The first stage of a page load shows the way out of the sleeve even when the record already plays (owner 2026-10-16:
	 * "beim ersten Mal öffnen … pro Browser Reload"); after that a record that is on just lies on the platter.
	 */
	let introduced = false;

	/** the table's surface: a map texture (owner 2026-10-16: "Texture drauf, aber ich nehme Jungle als Preset"); the lab
	 *  compares (`?table=plain|desert|jungle`). scutoons/3d/table-desert.webp, scutoons/3d/table-jungle.webp (painted map
	 *  tiles; AssetUsages CODE_REFERENCES) */
	export type StageTable = 'plain' | 'desert' | 'jungle';
	export const TABLE: StageTable = 'jungle';
</script>

<script lang="ts">
	import { untrack } from 'svelte';
	import type { ModelSource } from '@scutoons/content';
	import { modelLoader } from './model-source.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { mountRecordStage, type RecordStageHandle } from './record-stage.js';

	/**
	 * The record player card's 3D stage (FEATURES 10.102): the reader's records – each its sleeve with the record inside –
	 * as a carousel beside a turntable; on play the current one comes out onto the platter and turns, a swipe over the
	 * carousel selects another (`onselect`), another `current` swaps them (record-stage.js). Mounted only while the card
	 * is open (one renderer, freed on close); a model that fails calls `onfail` and the card shows its 2D deck.
	 * The canvas fills its parent (the card's top, as its background – owner 2026-10-16); `lift` = the lower part of it
	 * that the controls cover (the scene moves above them).
	 */
	let {
		records,
		current,
		onselect,
		onplay,
		table = TABLE,
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
		/** the reader's records: a key and the model (a .glb or the shared record model's look – model-source.ts) */
		records: { key: string; model: ModelSource }[];
		/** the key of the record that plays */
		current: string | null;
		/** the carousel's front record changed */
		onselect?: (key: string) => void;
		/** a record was carried onto the turntable: put it on */
		onplay?: (key: string) => void;
		/** the table's surface (TABLE) */
		table?: StageTable;
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

	// the stage mounts again only when the records themselves change (not on every new array of the same ones)
	const signature = $derived(JSON.stringify(records.map((r) => [r.key, r.model])));
	$effect(() => {
		const el = canvas;
		void signature;
		void table;
		const list = untrack(() => records.map((r) => ({ key: r.key, source: modelLoader(r.model) })));
		if (!el || !list.length) return;
		ready = false;
		const onAlready = introduced && untrack(() => placed);
		introduced = true;
		const handle = mountRecordStage(el, list, {
			lift: untrack(() => lift),
			placed: onAlready,
			onload: () => (ready = true),
			onerror: () => onfail(),
			onselect: (key) => onselect?.(key),
			onplay: (key) => onplay?.(key),
			table: table === 'plain' ? null : asset(`scutoons/3d/table-${table}.webp`)
		});
		untrack(() => handle.update({ playing, side, arm, lifted, current }));
		stage = handle;
		return () => {
			handle.dispose();
			stage = null;
		};
	});

	$effect(() => {
		const state = { playing, side, arm, lifted, current };
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
		/* the stage's own gestures (record-gestures.js); the sheet scrolls from the song and controls below */
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
