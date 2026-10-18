<script lang="ts" module>
	/** the gestures on every page's overlay (a drag's moves share their number: one undo step) */
	let drags = 0;
</script>

<script lang="ts">
	import type { CollagePiece } from '@scutoons/content';
	import { drawablePiece, pieceBox } from '@scutoons/reader';
	import { anchorScorePiece, pageFrame, placeScorePiece, type PageGeometry } from '@scutoons/reader/score-anchors';
	import { cornerWidth, pieceHandle, type ScorePlaceAt } from '$lib/editor/score-collage.ts';

	/**
	 * The stage's handles on one score page (research/COLLAGE.md §4.4), drawn as the score view's `decor` over the
	 * music: a box per piece whose bar is on this page, with the same box CSS as the piece itself (`pieceBox`, on the
	 * page's frame). Only the boxes take the pointer – empty paper still swipes to the next page. A drag re-anchors the
	 * piece on every move (`anchorScorePiece`: the bar and staff under it), the corner handle scales it (never stretched);
	 * `drag` numbers the gesture, so the stage makes it one undo step. Pointer positions are % of this layer's box, so
	 * the screen's scale cancels out. The layer clips to the page (a box over its edge never makes the leaf scroll: a
	 * scroll bar would narrow the leaf, and with it the pieces' frame, unlike a reader's); the handle stays on the page.
	 */
	let {
		pieces,
		geometry,
		selected,
		onselect,
		onmove,
		onscale,
		ongeometry,
		label,
		scaleLabel
	}: {
		pieces: CollagePiece[];
		geometry: PageGeometry;
		selected: string | null;
		onselect: (id: string) => void;
		onmove: (id: string, place: ScorePlaceAt, drag: number) => void;
		onscale: (id: string, width: number, drag: number) => void;
		/** this page's geometry, on every new layout (the stage places a new piece and switches Bar / Page by it) */
		ongeometry?: (geometry: PageGeometry) => void;
		/** a box's accessible name */
		label: (piece: CollagePiece) => string;
		scaleLabel: string;
	} = $props();

	const frame = $derived(pageFrame(geometry));
	$effect(() => ongeometry?.(geometry));
	const placed = $derived(
		pieces.flatMap((piece) => {
			if (!drawablePiece(piece)) return [];
			const place = placeScorePiece(piece, geometry);
			return place ? [{ piece, place, box: pieceBox(piece, place.left, place.top, place.width) }] : [];
		})
	);

	const handle = $derived.by(() => {
		const sel = placed.find((p) => p.piece.id === selected);
		return sel ? { ...sel, ...pieceHandle(sel.piece, sel.place, geometry) } : null;
	});

	let layer = $state<HTMLElement>();
	/** a pointer travels this far (% of the page's width) before a press becomes a move – a click never re-anchors */
	const SLOP = 0.6;
	type Point = { x: number; y: number };
	let gesture: { kind: 'move' | 'scale'; id: string; pointer: number; n: number; start: Point; grab: Point; off: Point; moved: boolean } | null = null;

	/** the pointer in % of the page (this layer is the page's frame) */
	function pointOf(e: PointerEvent): { x: number; y: number } {
		const r = layer!.getBoundingClientRect();
		return { x: ((e.clientX - r.left) / r.width) * 100, y: ((e.clientY - r.top) / r.height) * 100 };
	}

	function down(e: PointerEvent, piece: CollagePiece, place: { left: number; top: number }, kind: 'move' | 'scale') {
		if (e.button !== 0 || !layer) return;
		e.preventDefault();
		e.stopPropagation();
		onselect(piece.id);
		// the focus goes to the box (pressed without it, the focus stayed where it was): the stage's keys act on it
		layer.querySelector<HTMLElement>(`.box[data-piece="${CSS.escape(piece.id)}"]`)?.focus({ preventScroll: true });
		const at = pointOf(e);
		// a move keeps the grab point under the pointer; a scale works from the piece's centre, the handle's offset from
		// the box's corner kept (on the page's edge the handle is held on the page)
		const h = handle?.piece.id === piece.id ? handle : null;
		const grab = kind === 'move' ? { x: at.x - place.left, y: at.y - place.top } : (h?.centre ?? { x: place.left, y: place.top });
		const off = kind === 'scale' && h ? { x: at.x - h.corner.x, y: at.y - h.corner.y } : { x: 0, y: 0 };
		gesture = { kind, id: piece.id, pointer: e.pointerId, n: ++drags, start: at, grab, off, moved: false };
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
	}

	function move(e: PointerEvent) {
		const g = gesture;
		if (!g || e.pointerId !== g.pointer || !layer) return;
		const at = pointOf(e);
		if (!g.moved && Math.hypot(at.x - g.start.x, ((at.y - g.start.y) * geometry.height) / geometry.width) < SLOP) return;
		g.moved = true;
		const piece = pieces.find((p) => p.id === g.id);
		if (!piece) return;
		if (g.kind === 'scale') return onscale(g.id, cornerWidth(piece, g.grab, { x: at.x - g.off.x, y: at.y - g.off.y }, geometry), g.n);
		const place = anchorScorePiece({ x: at.x - g.grab.x, y: at.y - g.grab.y }, geometry, !!piece.at?.page);
		if (place) onmove(g.id, place, g.n);
	}

	function up(e: PointerEvent) {
		if (gesture?.pointer === e.pointerId) gesture = null;
	}
</script>

<div class="score-overlay" bind:this={layer} style:width={frame.width} style:aspect-ratio={frame['aspect-ratio']} style:margin-left={frame['margin-left']} style:margin-top={frame['margin-top']} data-testid="score-overlay">
	{#each placed as { piece, place, box } (piece.id)}
		<div
			class="box"
			class:selected={piece.id === selected}
			class:behind={piece.behind}
			style:left={box.left}
			style:top={box.top}
			style:width={box.width}
			style:aspect-ratio={box['aspect-ratio']}
			style:translate={box.translate}
			style:rotate={box.rotate}
			role="button"
			tabindex="-1"
			aria-label={label(piece)}
			aria-pressed={piece.id === selected}
			onpointerdown={(e) => down(e, piece, place, 'move')}
			onpointermove={move}
			onpointerup={up}
			onpointercancel={up}
			data-piece={piece.id}
			data-bar={piece.at?.bar}
			data-testid="stage-piece-{piece.id}"
		></div>
	{/each}
	{#if handle}
		<span class="corner" role="presentation" title={scaleLabel} style:--x={handle.corner.x} style:--y={handle.corner.y} onpointerdown={(e) => down(e, handle.piece, handle.place, 'scale')} onpointermove={move} onpointerup={up} onpointercancel={up} data-testid="stage-piece-scale"></span>
	{/if}
</div>

<style>
	/* over the music and the pieces (z 2): the stage's own layer; only the boxes take the pointer */
	.score-overlay {
		position: absolute;
		left: 0;
		top: 0;
		z-index: 3;
		overflow: clip;
		pointer-events: none;
	}
	/* the piece's box; lines and handles keep their on-screen size in the scaled screen (score-frame.svelte) */
	.box {
		position: absolute;
		box-sizing: border-box;
		outline: calc(var(--border-width-hairline) / var(--stage-scale, 1)) dashed var(--ink);
		outline-offset: calc(-1 * var(--border-width-hairline) / var(--stage-scale, 1));
		pointer-events: auto;
		touch-action: none;
		cursor: move;
	}
	.box.behind {
		outline-style: dotted;
	}
	.box.selected {
		outline: calc(var(--border-width-regular) / var(--stage-scale, 1)) solid var(--primary-cyan);
	}
	/* the round scale handle on the box's bottom-right corner (--x / --y: % of the page), held on the page at its edges */
	.corner {
		--half: calc(0.5rem / var(--stage-scale, 1));
		position: absolute;
		left: clamp(var(--half), calc(var(--x) * 1%), calc(100% - var(--half)));
		top: clamp(var(--half), calc(var(--y) * 1%), calc(100% - var(--half)));
		width: calc(2 * var(--half));
		height: calc(2 * var(--half));
		translate: -50% -50%;
		box-sizing: border-box;
		border: calc(var(--border-width-regular) / var(--stage-scale, 1)) solid var(--ink);
		border-radius: 50%;
		background: var(--paper);
		cursor: nwse-resize;
		touch-action: none;
		pointer-events: auto;
	}
</style>
