<script>
	// The variants board's tool row (agents/research/VARIANTS-CANVAS.md §3.2, §3.5; the mockup
	// research/studio-layout/variants-canvas.html): the editor switch (V), the selection's count, the keys as one line (the
	// whole list in its tooltip), "Compare · n" – a square toggle: the selection side by side right of the bands (C; §3.4 –
	// shown once a card is selected or Compare is open) –, "Show rejected · n" – a square toggle that unfolds every rejected card in every band
	// (hidden at 0, not remembered) –, the zoom: one square group − · % · + (the % back to 100 %; 0 fits the widest
	// band, Ctrl / ⌘ + wheel zooms about the pointer – Board, board-view.svelte.js) – and + (round): images or videos for
	// the current shot, the keys' way to what a drop on a band does (the mockup's "Add images to the current shot").
	import { Icon, IconButton } from '@scutoons/ui';
	import { board } from '../board.svelte.js';
	import { DROP_TYPES } from '../board-sources.svelte.js';
	import { percent, zoomStep, ZOOM_MAX, ZOOM_MIN } from '../board-zoom.js';
	import { studio } from '../studio.svelte.js';
	import EditorSwitch from './EditorSwitch.svelte';

	/** @type {{ rejected: number, selected: number, oncompare: () => void, onzoom: (zoom: number) => void, onfiles: (files: File[]) => void }} */
	let { rejected, selected, oncompare, onzoom, onfiles } = $props();
	let picker = $state();
	const HOW = 'Click: the current card (its shot current) · Ctrl + click: select (A–D) · C: Compare – the selected side by side (open: a click fills an empty side, else replaces the last, A stays; X empties a side; Space plays them on one clock) · ← → ↑ ↓ move (strips and folded bands skipped) · Enter chosen · R review · X reject – it folds to a strip (again: back) · I improve on this – fills Generate (nothing is sent) · a click on a strip: a look into it (Esc closes it) · wheel: down through the bands, Shift + wheel: sideways, a drag on the background: pan · Ctrl + wheel / + / −: zoom · 0: fit · Esc: Compare closes, then the selection · V: the Viewport';
	const SHORT = 'click current · Ctrl + click select · C compare · arrows · Enter ✓ · R ? · X ✕ · I ⚡ · + − 0 zoom';
</script>

<div class="tools">
	<EditorSwitch />
	{#if selected || board.compare}
		<div class="sc-seg">
			<button type="button" aria-pressed={board.compare} title={board.compare ? 'Close Compare (C, Esc)' : `Compare the ${selected} selected side by side (C) – fewer than two: the shot's chosen as A; Ctrl + click takes one off`} onclick={oncompare} data-testid="compare-toggle"><Icon name="columns" />Compare · {selected}</button>
		</div>
	{/if}
	<span class="hint" title={HOW}><span>{SHORT}</span></span>
	{#if rejected}
		<div class="sc-seg">
			<button type="button" aria-pressed={board.showRejected} title="Show rejected – unfold the {rejected} rejected card{rejected === 1 ? '' : 's'} in every band (off: each folds to a strip in its place)" onclick={() => (board.showRejected = !board.showRejected)} data-testid="show-rejected">Show rejected · {rejected}</button>
		</div>
	{/if}
	<div class="sc-seg zoom" role="group" aria-label="Zoom – one for all bands">
		<button type="button" class="is-square" aria-label="Zoom out (−)" title="Zoom out (−)" disabled={board.zoom <= ZOOM_MIN} onclick={() => onzoom(zoomStep(board.zoom, -1))}><Icon name="minus" /></button>
		<button type="button" class="pct" aria-label="Zoom {percent(board.zoom)} – back to 100 %" title="Zoom {percent(board.zoom)} – click: 100 % · 0: fit the widest band · Ctrl + wheel: about the pointer" onclick={() => onzoom(1)} data-testid="board-zoom">{percent(board.zoom)}</button>
		<button type="button" class="is-square" aria-label="Zoom in (+)" title="Zoom in (+)" disabled={board.zoom >= ZOOM_MAX} onclick={() => onzoom(zoomStep(board.zoom, 1))}><Icon name="plus" /></button>
	</div>
	{#if studio.sel}
		<IconButton label="Add images or videos to {studio.sel.id} – or drop them onto a band (its shot's)" class="sc-round" onclick={() => picker.click()} data-testid="board-add"><Icon name="upload" /></IconButton>
		<input bind:this={picker} type="file" accept={DROP_TYPES.join(',')} multiple hidden onchange={(e) => { onfiles([...e.currentTarget.files]); e.currentTarget.value = ''; }} />
	{/if}
</div>

<style>
	.tools {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		padding: var(--space-025) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	/* as the Viewport's: one line in the room the tools leave, the whole list in its tooltip; too narrow, it goes whole */
	.hint {
		flex: 1 1 0;
		min-width: 0;
		container-type: inline-size;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		font-size: var(--type-caption-size);
		opacity: 0.7;
	}
	@container (max-width: 22rem) {
		.hint > span {
			display: none;
		}
	}
	.sc-seg > button {
		white-space: nowrap;
	}
	/* the % keeps its width while it changes (80 % … 200 %) */
	.pct {
		min-width: 4.5rem;
		font-variant-numeric: tabular-nums;
	}
</style>
