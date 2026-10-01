<script lang="ts" module>
	export type MapTool = 'select' | 'place' | 'route';
	export type MapSelection = { kind: 'place' | 'route' | 'layer'; id: string } | null;
</script>

<script lang="ts">
	import {
		isPlaceWaypoint,
		mapName,
		placeImageBox,
		routePoints,
		visibleLabels,
		waypointPoint,
		type MapPlace,
		type Point,
		type Waypoint,
		type WorldMap
	} from '@scutoons/content';
	import { MapRoutes, MapViewport, mapGestures, type DrawnRoute } from '@scutoons/reader';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import { freeWaypointId, nearestSegment } from '$lib/editor/map.ts';

	/**
	 * The builder's canvas (agents/MAP.md §4.2): the world at any zoom (25 %–800 %), the tools – select (click, drag places
	 * and waypoints, click a route), + place, draw route (places snap within 12 px, the terrain gives free points); panning with the middle mouse button, Space + drag or a
	 * drag on the empty terrain (no hand tool, DESIGN.md "Icons") –
	 * the toon filter (others dimmed), the reader view (hidden things greyed) and the phone frame (a 375 × 812 reader
	 * screen at the reader's zoom, labels that would overlap there outlined red). Edits go out through callbacks.
	 */
	let {
		map,
		view,
		tool = $bindable('select'),
		selection,
		toonFilter,
		readerView,
		phoneFrame,
		startView = null,
		hiddenLayers,
		sea,
		onselect,
		onaddplace,
		onmoveplace,
		onmovewaypoint,
		oninsertwaypoint,
		onaddroute
	}: {
		map: WorldMap;
		view: MapViewport;
		tool?: MapTool;
		selection: MapSelection;
		toonFilter: string;
		readerView: boolean;
		phoneFrame: boolean;
		/** the filtered toon's start view (the phone frame shows the reader's first screen there) */
		startView?: { center: Point; zoom: number } | null;
		hiddenLayers: Set<string>;
		/** the sea's colour (the reader view: the filtered toon's colour) */
		sea: string;
		onselect: (selection: MapSelection) => void;
		onaddplace: (at: Point) => void;
		/** `done` = the drag ended (one undo step per drag) */
		onmoveplace: (id: string, at: Point) => void;
		onmovewaypoint: (route: string, index: number, at: Point) => void;
		oninsertwaypoint: (route: string, after: number, at: Point) => void;
		onaddroute: (waypoints: Waypoint[]) => void;
	} = $props();

	const locale = $derived(getLocale());
	const inFilter = (toons: string[] | undefined) => !toonFilter || !!toons?.includes(toonFilter);
	const placeById = $derived(new Map(map.locations.map((p) => [p.id, p])));

	// ── panning: the middle button, Space held, or a drag on the empty terrain with Select (touch: drag / two fingers) ──
	let space = $state(false);
	function canPan(e: PointerEvent) {
		if (space || e.button === 1) return true;
		if (tool === 'select') return !(e.target as HTMLElement | null)?.closest('[data-handle]');
		return false;
	}
	function onTap(e: PointerEvent, at: Point) {
		if (space) return;
		const world = view.toWorld(at);
		if (tool === 'place') {
			if ((e.target as HTMLElement | null)?.closest('[data-handle]')) return;
			onaddplace(world);
			return;
		}
		if (tool === 'route') {
			addDraftPoint(at, world);
			return;
		}
		if (!(e.target as HTMLElement | null)?.closest('[data-handle], [data-route]')) onselect(null);
	}

	// ── drawing a route ──────────────────────────────────────────────────────────────────────────────────────
	let draft = $state<Waypoint[]>([]);
	let hover = $state<Point | null>(null);
	/** the place within 12 screen px of a screen point */
	function snapPlace(at: Point): MapPlace | null {
		let best: MapPlace | null = null;
		let distance = 12;
		for (const p of map.locations) {
			const s = view.toScreen(p);
			const d = Math.hypot(s.x - at.x, s.y - at.y);
			if (d <= distance) {
				distance = d;
				best = p;
			}
		}
		return best;
	}
	function addDraftPoint(at: Point, world: Point) {
		const place = snapPlace(at);
		if (place) {
			const last = draft.at(-1);
			if (last && isPlaceWaypoint(last) && last.location === place.id) return;
			draft = [...draft, { location: place.id }];
		} else {
			draft = [...draft, { id: freeWaypointId(draft, map.locations.map((p) => p.id)), x: Math.round(world.x), y: Math.round(world.y) }];
		}
	}
	export function finishRoute() {
		if (draft.length >= 2) onaddroute(draft);
		draft = [];
	}
	export function cancelRoute() {
		draft = [];
	}
	export function undoDraftPoint(): boolean {
		if (!draft.length) return false;
		draft = draft.slice(0, -1);
		return true;
	}
	export const drafting = () => draft.length > 0;
	$effect(() => {
		if (tool !== 'route') draft = [];
	});
	const draftPoints = $derived([...routePoints(map.locations, draft), ...(hover && tool === 'route' && draft.length ? [view.toWorld(hover)] : [])]);

	function onDoubleClick(): boolean {
		if (tool !== 'route') return false;
		// the double click's two taps set the same free point twice: drop the second
		const last = draft.at(-1);
		const before = draft.at(-2);
		if (last && before && !isPlaceWaypoint(last) && !isPlaceWaypoint(before) && last.x === before.x && last.y === before.y) draft = draft.slice(0, -1);
		finishRoute();
		return true;
	}

	// ── dragging places and waypoints ─────────────────────────────────────────────────────────────────────────
	let dragging = $state<{ kind: 'place'; id: string } | { kind: 'waypoint'; route: string; index: number } | null>(null);
	let dragMoved = false;
	function startDrag(e: PointerEvent, target: NonNullable<typeof dragging>) {
		if (tool !== 'select' || space || e.button !== 0) return;
		e.stopPropagation();
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
		dragging = target;
		dragMoved = false;
	}
	function local(e: PointerEvent): Point {
		const r = viewportEl!.getBoundingClientRect();
		return { x: e.clientX - r.left, y: e.clientY - r.top };
	}
	function dragMove(e: PointerEvent) {
		if (!dragging) return;
		const world = view.toWorld(local(e));
		const at = { x: Math.round(Math.max(0, Math.min(map.width, world.x))), y: Math.round(Math.max(0, Math.min(map.height, world.y))) };
		dragMoved = true;
		if (dragging.kind === 'place') onmoveplace(dragging.id, at);
		else onmovewaypoint(dragging.route, dragging.index, at);
	}
	function dragEnd(e: PointerEvent, click: () => void) {
		if (!dragging) return;
		const moved = dragMoved;
		dragging = null;
		dragMoved = false;
		if (!moved) click();
		e.stopPropagation();
	}

	// ── what is drawn ─────────────────────────────────────────────────────────────────────────────────────────
	const selectedRoute = $derived(selection?.kind === 'route' ? map.routes.find((r) => r.id === selection.id) ?? null : null);
	const routes = $derived<DrawnRoute[]>(
		map.routes.map((r) => ({
			id: r.id,
			waypoints: r.waypoints,
			style: r.style,
			dimmed: !inFilter(r.toons) || (readerView && !!r.hidden),
			selected: selection?.kind === 'route' && selection.id === r.id
		}))
	);
	const layerVisible = (id: string) => !hiddenLayers.has(id);

	/** the phone frame: a 375 × 812 reader screen at the reader's zoom (the toon's start zoom, 1) around the canvas centre */
	/** the reader's zoom in the frame: the toon's start zoom, else 1 */
	const frameZoom = $derived(startView?.zoom ?? 1);
	const frame = $derived.by(() => {
		const at = startView ? view.toScreen(startView.center) : { x: view.width / 2, y: view.height / 2 };
		return { w: (375 * view.zoom) / frameZoom, h: (812 * view.zoom) / frameZoom, x: at.x, y: at.y };
	});
	/** labels that would overlap at zoom 1 on a phone (only with the frame on): outlined red */
	const collisions = $derived.by(() => {
		if (!phoneFrame) return new Set<string>();
		const items = map.locations.filter((p) => inFilter(p.toons) && !(readerView && p.hidden));
		const name = (p: MapPlace) => mapName(p.name, locale) || p.id;
		const shown = visibleLabels(
			// world units at the reader's zoom (phone: 14 px root, 0.75rem labels = 10.5 px, 1 rem markers)
			items.map((p) => ({ id: p.id, x: p.x, y: p.y - 8.4 / frameZoom, width: (name(p).length * 6.3 + 6) / frameZoom, height: 14.5 / frameZoom })),
			items.map((p) => ({ x: p.x, y: p.y, size: 14 / frameZoom }))
		);
		return new Set(items.filter((p) => !shown.has(p.id)).map((p) => p.id));
	});

	/** labels at the current zoom without overlaps (the selected place and the filtered toon's first; zoom in for the rest) */
	const labelled = $derived.by(() => {
		const name = (p: MapPlace) => mapName(p.name, locale) || p.id;
		const items = map.locations
			.filter((p) => view.onScreen(p, 100))
			.map((p) => ({ p, s: view.toScreen(p), rank: selection?.kind === 'place' && selection.id === p.id ? 0 : inFilter(p.toons) ? 1 : 2 }))
			.sort((a, b) => a.rank - b.rank);
		return visibleLabels(
			items.map(({ p, s }) => ({ id: p.id, x: s.x, y: s.y - 12, width: name(p).length * 6.6 + 6, height: 13 })),
			items.map(({ s }) => ({ x: s.x, y: s.y, size: 16 }))
		);
	});

	let viewportEl = $state<HTMLDivElement>();
	function onRouteClick(id: string, e: MouseEvent) {
		if (tool !== 'select') return;
		e.stopPropagation();
		if (selection?.kind === 'route' && selection.id === id) {
			// a click on the selected route's line inserts a free point there
			const r = viewportEl!.getBoundingClientRect();
			const at = view.toWorld({ x: e.clientX - r.left, y: e.clientY - r.top });
			const route = map.routes.find((x) => x.id === id);
			if (!route) return;
			const indexed = route.waypoints.map((w, i) => ({ i, p: waypointPoint(map.locations, w) })).filter((x) => x.p) as { i: number; p: Point }[];
			const segment = nearestSegment(indexed.map((x) => x.p), at);
			oninsertwaypoint(id, indexed[segment]?.i ?? 0, at);
		} else onselect({ kind: 'route', id });
	}

	function onKey(e: KeyboardEvent, down: boolean) {
		if (e.code !== 'Space') return;
		const target = e.target as HTMLElement | null;
		if (target?.closest('input, textarea, select, [contenteditable]')) return;
		if (down) e.preventDefault();
		space = down;
	}
</script>

<svelte:window onkeydown={(e) => onKey(e, true)} onkeyup={(e) => onKey(e, false)} />

<div
	class="canvas tool-{tool}"
	style:background={sea}
	class:space
	class:moving={view.moving}
	bind:this={viewportEl}
	{@attach mapGestures(view, { canPan, onTap, onDoubleClick, onHover: (_, at) => (hover = at) })}
	onpointerleave={() => (hover = null)}
	role="application"
	aria-label={m['creator.map']()}
	data-testid="map-canvas"
>
	<div class="world" style:transform={view.transform} style:width="{map.width}px" style:height="{map.height}px">
		{#each map.layers ?? [] as layer (layer.id)}
			{#if layerVisible(layer.id) && layer.image}
				<img class="layer" src={asset(layer.image)} alt="" draggable="false" />
			{/if}
		{/each}
	</div>

	<MapRoutes {routes} places={map.locations} zoom={view.zoom} x={view.x} y={view.y} onroute={onRouteClick} />

	{#if draftPoints.length >= 2}
		<svg class="draft" aria-hidden="true">
			<polyline points={draftPoints.map((p) => { const s = view.toScreen(p); return `${s.x},${s.y}`; }).join(' ')} />
		</svg>
	{/if}

	<div class="places">
		{#each map.locations as p (p.id)}
			{@const s = view.toScreen(p)}
			{#if view.onScreen(p, 200)}
				{@const dim = !inFilter(p.toons) || (readerView && !!p.hidden)}
				{@const active = selection?.kind === 'place' && selection.id === p.id}
				<div class="place" class:has-image={!!p.image?.src} class:dim class:active class:collides={collisions.has(p.id)} class:hidden-place={!!p.hidden} style:transform="translate({s.x}px, {s.y}px)">
					{#if p.image?.src}
						{@const box = placeImageBox(p.image, view.zoom)}
						<img class="place-image" src={asset(p.image.src)} alt="" draggable="false" style:width="{box.width}px" style:left="{box.left}px" style:top="{box.top}px" />
					{/if}
					<button
						class="marker"
						data-handle
						aria-label={mapName(p.name, locale) || p.id}
						onpointerdown={(e) => startDrag(e, { kind: 'place', id: p.id })}
						onpointermove={dragMove}
						onpointerup={(e) => dragEnd(e, () => (tool === 'select' ? onselect({ kind: 'place', id: p.id }) : null))}
						onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') onselect({ kind: 'place', id: p.id }); }}
						data-testid="builder-place-{p.id}"
					></button>
					{#if labelled.has(p.id)}<span class="label">{mapName(p.name, locale) || p.id}</span>{/if}
				</div>
			{/if}
		{/each}
	</div>

	{#if selectedRoute}
		<div class="handles">
			{#each selectedRoute.waypoints as w, i (i)}
				{@const pt = waypointPoint(map.locations, w)}
				{#if pt}
					{@const s = view.toScreen(pt)}
					<button
						class="handle"
						class:free={!isPlaceWaypoint(w)}
						data-handle
						style:transform="translate({s.x}px, {s.y}px)"
						aria-label={String(i + 1)}
						title="{i + 1}: {isPlaceWaypoint(w) ? w.location : w.id}"
						onpointerdown={(e) => startDrag(e, { kind: 'waypoint', route: selectedRoute.id, index: i })}
						onpointermove={dragMove}
						onpointerup={(e) => dragEnd(e, () => {})}
						data-testid="builder-waypoint-{i}"
					>{i + 1}</button>
				{/if}
			{/each}
		</div>
	{/if}

	{#if phoneFrame}
		<div class="phone" style:width="{frame.w}px" style:height="{frame.h}px" style:left="{frame.x}px" style:top="{frame.y}px" aria-hidden="true" data-testid="phone-frame-box"><span>375 × 812 · {Math.round(frameZoom * 100)} %{startView ? ' · start' : ''}</span></div>
	{/if}

	{#if tool === 'route' || tool === 'place'}
		<p class="hint">{tool === 'route' ? m['creator.map_route_hint']() : m['creator.map_place_hint']()}</p>
	{/if}
</div>

<style>
	.canvas {
		position: relative;
		width: 100%;
		height: 100%;
		overflow: hidden;
		background: var(--primary-cyan, #00d1ff);
		touch-action: none;
		user-select: none;
		cursor: default;
	}
	.space {
		cursor: grab;
	}
	.moving {
		cursor: grabbing;
	}
	.tool-place,
	.tool-route {
		cursor: crosshair;
	}
	.world {
		position: absolute;
		top: 0;
		left: 0;
		transform-origin: 0 0;
	}
	.moving .world {
		will-change: transform;
	}
	.layer {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
	}
	.places,
	.handles {
		position: absolute;
		inset: 0;
		pointer-events: none;
	}
	.place {
		position: absolute;
		top: 0;
		left: 0;
	}
	.place.dim {
		opacity: 0.3;
	}
	.place-image {
		position: absolute;
		pointer-events: none;
		height: auto;
	}
	.marker {
		position: absolute;
		left: -8px;
		top: -8px;
		width: 16px;
		height: 16px;
		padding: 0;
		border-radius: 50%;
		border: 2px solid var(--ink);
		background: var(--surface-tint-cyan, #f0fcff);
		pointer-events: auto;
		cursor: pointer;
	}
	.hidden-place .marker {
		border-style: dashed;
	}
	.active .marker {
		background: var(--ink);
		box-shadow: 0 0 0 4px var(--primary-cyan, #00d1ff);
	}
	.collides .marker {
		outline: 2px solid var(--error, #e00);
		outline-offset: 2px;
	}
	/* with an image the name goes under the point (the image stands above it) */
	.has-image .label {
		top: 12px;
		transform: translate(-50%, 0);
	}
	.tool-select .marker {
		cursor: move;
	}
	.label {
		position: absolute;
		left: 0;
		top: -12px;
		transform: translate(-50%, -100%);
		white-space: nowrap;
		font-size: var(--type-caption-size);
		line-height: 1;
		pointer-events: none;
		text-shadow:
			-1px -1px 0 #fff,
			1px -1px 0 #fff,
			1px 1px 0 #fff,
			-1px 1px 0 #fff,
			-2px 0 0 #fff,
			2px 0 0 #fff,
			0 -2px 0 #fff,
			0 2px 0 #fff;
	}
	.handle {
		position: absolute;
		left: -10px;
		top: -10px;
		width: 20px;
		height: 20px;
		padding: 0;
		border-radius: 50%;
		border: 2px solid var(--ink);
		background: var(--primary-cyan, #00d1ff);
		color: var(--ink);
		font: 700 9px/1 var(--font-mono);
		pointer-events: auto;
		cursor: move;
	}
	.handle.free {
		background: var(--paper);
		border-radius: 0;
	}
	.draft {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
	}
	.draft polyline {
		fill: none;
		stroke: var(--ink);
		stroke-width: 3px;
		stroke-dasharray: 6 4;
	}
	.phone {
		position: absolute;
		left: 50%;
		top: 50%;
		transform: translate(-50%, -50%);
		border: 2px dashed var(--ink);
		pointer-events: none;
		box-sizing: border-box;
	}
	.phone span {
		position: absolute;
		top: -1.4rem;
		left: 0;
		font-size: var(--type-caption-size);
		background: var(--paper);
		padding: 0 0.3rem;
	}
	.hint {
		position: absolute;
		left: 50%;
		bottom: 0.75rem;
		transform: translateX(-50%);
		margin: 0;
		background: var(--paper);
		border: 2px solid var(--ink);
		padding: 0.25rem 0.5rem;
		font-size: var(--type-caption-size);
		pointer-events: none;
		max-width: 90%;
	}
</style>
