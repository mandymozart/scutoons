<script lang="ts" module>
	import type { HexCell } from '@scutoons/content';

	/** what a brush paints on: any set of map cells (a place's ground, later a region's cover) */
	export interface HexBrushTarget {
		/** the cells now (absolute map cells) */
		cells: readonly HexCell[];
		/** add touched cells or take them out */
		mode: 'add' | 'remove';
		/** the brush's size in cells, 1..256 (BRUSH_MIN..BRUSH_MAX) */
		size: number;
		/** refuses cells when adding (another place's, outside the map …) */
		allowed?: (cell: HexCell) => boolean;
		/** one patch (a place's ground – owner 2026-10-12): no add starts an island, no remove splits it or takes `anchor` */
		connected?: boolean;
		/** the cell that must stay (a place's home cell) */
		anchor?: HexCell;
		/** once per stroke (one undo step): the cells after it */
		onchange: (cells: HexCell[]) => void;
		/**
		 * a big cover (a map region – agents/MAP.md §8.10, hundreds of thousands of cells): the caller draws the cover and
		 * paints it (`paintRuns` on its row runs); the brush shows the stroke only and hands over the cells it touched,
		 * once on release (`onstroke`) – no cell-by-cell painting per pointer move
		 */
		bulk?: boolean;
		onstroke?: (touched: HexCell[]) => void;
	}
</script>

<script lang="ts">
	import { HEX_CELL, brushArea, cellsOutline, hexCellAt, hexCorners, paintCells, type Point } from '@scutoons/content';
	import type { MapViewport } from '@scutoons/reader';

	/**
	 * The hex brush (owner 2026-10-12: "a brush that adds or removes hexagons to the place's base"; the map regions paint
	 * their covers with it too): an overlay over the map canvas – a press or a drag over the cells adds / removes the brush's
	 * cells (`brushArea`, 1..256), the stroke is collected and handed over ONCE on release (`onchange`, one undo step).
	 * Drawn light: the cells as one path, the cover's outer edge as one darker path (thousands of cells stay cheap), the
	 * brush under the pointer in the highlight. Panning stays with the middle mouse button / two fingers (the canvas).
	 */
	let { view, target }: { view: MapViewport; target: HexBrushTarget } = $props();

	/** the cells while a stroke is under way (null: none) */
	let stroke = $state<HexCell[] | null>(null);
	let hover = $state<HexCell | null>(null);
	const shown = $derived(target.bulk ? [] : (stroke ?? target.cells));
	/**
	 * bulk: the cells the stroke touched so far (handed over on release) – NOT drawn cell by cell: with a 256 brush a long
	 * stroke sweeps tens of thousands of cells and one polygon each per pointer move stalled the browser (owner 2026-10-12,
	 * Firefox: "this page is slowing down"); the stroke shows as the pointer's trail at the brush's width instead
	 */
	let swept = new Map<string, HexCell>();
	let trail = $state<Point[]>([]);
	/** the brush's width in world units: the rings `brushArea` fills for its size (3r(r+1)+1 cells) */
	const brushWidth = $derived((2 * Math.ceil((-3 + Math.sqrt(9 + 12 * Math.max(0, target.size - 1))) / 6) + 1) * HEX_CELL);
	const touched = $derived(hover ? brushArea(hover, target.size) : []);

	let box = $state<HTMLElement>();
	/** the last point of the stroke (world units): a fast drag paints every cell between two events */
	let last: Point | null = null;
	function worldAt(e: PointerEvent): Point {
		const r = box!.getBoundingClientRect();
		return view.toWorld({ x: e.clientX - r.left, y: e.clientY - r.top });
	}
	function paint(at: HexCell) {
		if (target.bulk) {
			for (const c of brushArea(at, target.size)) {
				const key = `${c.q},${c.r}`;
				if (!swept.has(key)) swept.set(key, c);
			}
			return;
		}
		stroke = paintCells(stroke ?? target.cells, brushArea(at, target.size), target.mode, { allowed: target.allowed, connected: target.connected, anchor: target.anchor });
	}
	/** paints from the last point to `to`, a step of half a cell */
	function paintTo(to: Point) {
		const from = last ?? to;
		// a big bulk brush steps by a quarter of its width (its areas overlap), a small one by half a cell
		const step = target.bulk ? Math.max(HEX_CELL / 2, brushWidth / 4) : HEX_CELL / 2;
		const steps = Math.max(1, Math.ceil(Math.hypot(to.x - from.x, to.y - from.y) / step));
		if (target.bulk) {
			const end = trail[trail.length - 1];
			if (!end || Math.hypot(to.x - end.x, to.y - end.y) >= HEX_CELL / 2) trail = [...trail, to];
		}
		let prev = '';
		for (let i = last ? 1 : 0; i <= steps; i++) {
			const c = hexCellAt({ x: from.x + ((to.x - from.x) * i) / steps, y: from.y + ((to.y - from.y) * i) / steps });
			const key = `${c.q},${c.r}`;
			if (key !== prev) paint(c);
			prev = key;
		}
		last = to;
	}
	function down(e: PointerEvent) {
		if (e.button !== 0) return;
		e.stopPropagation();
		box!.setPointerCapture(e.pointerId);
		const at = worldAt(e);
		hover = hexCellAt(at);
		stroke = target.bulk ? [] : [...target.cells];
		swept = new Map();
		trail = [];
		last = null;
		paintTo(at);
	}
	function move(e: PointerEvent) {
		const at = worldAt(e);
		const cell = hexCellAt(at);
		if (!stroke && hover && cell.q === hover.q && cell.r === hover.r) return;
		hover = cell;
		if (stroke) paintTo(at);
	}
	function up(e: PointerEvent) {
		if (!stroke) return;
		e.stopPropagation();
		paintTo(worldAt(e));
		const done = stroke!;
		stroke = null;
		last = null;
		if (target.bulk) {
			const touched = [...swept.values()];
			swept = new Map();
			trail = [];
			if (touched.length) target.onstroke?.(touched);
			return;
		}
		if (done.length !== target.cells.length || done.some((c, i) => c.q !== target.cells[i]?.q || c.r !== target.cells[i]?.r)) target.onchange(done);
	}

	const toScreen = (p: Point) => view.toScreen(p);
	const cellsD = (cells: readonly HexCell[]) =>
		cells
			.map((c) => 'M' + hexCorners(c).map((k) => { const s = toScreen(k); return `${s.x.toFixed(1)},${s.y.toFixed(1)}`; }).join('L') + 'Z')
			.join('');
	const edgeD = (cells: readonly HexCell[]) =>
		cellsOutline(cells)
			.map(([a, b]) => { const s = toScreen(a), t = toScreen(b); return `M${s.x.toFixed(1)},${s.y.toFixed(1)}L${t.x.toFixed(1)},${t.y.toFixed(1)}`; })
			.join('');
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="hex-brush"
	class:remove={target.mode === 'remove'}
	data-handle
	bind:this={box}
	onpointerdown={down}
	onpointermove={move}
	onpointerup={up}
	onpointercancel={up}
	onpointerleave={() => !stroke && (hover = null)}
	data-testid="hex-brush"
	data-cells={shown.length}
>
	<svg aria-hidden="true">
		<path class="cells" d={cellsD(shown)} />
		<path class="edge" d={edgeD(shown)} />
		{#if trail.length}
			<!-- bulk: the stroke so far as the pointer's trail at the brush's width (world units, the camera's transform) -->
			<g transform="translate({view.x} {view.y}) scale({view.zoom})">
				<polyline class="trail" points={(trail.length === 1 ? [trail[0], trail[0]] : trail).map((p) => `${p.x.toFixed(2)},${p.y.toFixed(2)}`).join(' ')} stroke-width={brushWidth} />
			</g>
		{/if}
		{#if touched.length}<path class="brush" d={cellsD(touched)} />{/if}
	</svg>
</div>

<style>
	.hex-brush {
		position: absolute;
		z-index: 3;
		inset: 0;
		cursor: crosshair;
		touch-action: none;
	}
	svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		overflow: visible;
		pointer-events: none;
	}
	/* the same look as a place's footprint (owner 2026-10-12): light cells, faint inner lines, a darker outer edge */
	.cells {
		fill: var(--primary-yellow, #ffe600);
		fill-opacity: 0.22;
		stroke: var(--ink);
		stroke-opacity: 0.2;
		stroke-width: 1;
	}
	.edge {
		fill: none;
		stroke: var(--ink);
		stroke-opacity: 0.85;
		stroke-width: 1.5;
		stroke-linecap: round;
	}
	/* the brush under the pointer: the highlight; removing: dashed */
	.brush {
		fill: var(--primary-cyan, #00d1ff);
		fill-opacity: 0.35;
		stroke: var(--primary-cyan, #00d1ff);
		stroke-width: 1.5;
	}
	.trail {
		fill: none;
		stroke: var(--primary-cyan, #00d1ff);
		stroke-opacity: 0.35;
		stroke-linecap: round;
		stroke-linejoin: round;
	}
	.remove .trail {
		stroke: var(--ink);
		stroke-opacity: 0.25;
	}
	.remove .brush {
		fill-opacity: 0.15;
		stroke-dasharray: 3 2;
	}
</style>
