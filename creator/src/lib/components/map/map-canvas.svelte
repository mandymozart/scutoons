<script lang="ts" module>
	export type MapTool = 'select' | 'place' | 'route';
	export type MapSelection = { kind: 'place' | 'route' | 'layer'; id: string } | null;
</script>

<script lang="ts">
	import {
		isPlaceWaypoint,
		mapName,
		placeImageBox,
		placeSize,
		hexCellAt,
		hexCenter,
		hexCorners,
		growRect,
		pointIndex,
		PLACE_SIZE_MAX,
		zoomPercent,
		placeCells,
		snapPlace as snapToGrid,
		DEFAULT_PLACE_IMAGE_SIZE,
		HEX_CELL,
		routePoints,
		routeStroke,
		bezierPath,
		visibleLabels,
		waypointPoint,
		type MapPlace,
		type Point,
		type Waypoint,
		type WorldMap
	} from '@scutoons/content';
	import { MapRoutes, MapTerrain, MapViewport, mapGestures, type DrawnRoute } from '@scutoons/reader';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import { freeWaypointId, nearestSegment } from '$lib/editor/map.ts';
	import { page } from '$app/state';

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
		placeFrame = null,
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
		/** the selected place's reader view (`placeView`): the phone frame shows it instead of the start view */
		placeFrame?: { center: Point; zoom: number } | null;
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

	// ── the hex grid (owner 2026-10-11): shown only while a place is placed or dragged – the cells on screen, one path ──
	const showGrid = $derived(tool === 'place' || dragging?.kind === 'place');
	const gridPath = $derived.by(() => {
		if (!showGrid || view.zoom * HEX_CELL < 6) return ''; // cells under 6 px are noise
		const a = view.toWorld({ x: 0, y: 0 });
		const b = view.toWorld({ x: view.width, y: view.height });
		const r = HEX_CELL / Math.sqrt(3);
		const first = hexCellAt({ x: Math.max(0, a.x), y: Math.max(0, a.y) });
		const last = hexCellAt({ x: Math.min(map.width, b.x), y: Math.min(map.height, b.y) });
		if ((last.r - first.r) * ((b.x - a.x) / HEX_CELL) > 6000) return ''; // too far out to draw every cell
		let d = '';
		for (let row = first.r - 1; row <= last.r + 1; row++)
			for (let col = Math.floor(a.x / HEX_CELL - row / 2) - 1; col <= Math.ceil(b.x / HEX_CELL - row / 2) + 1; col++) {
				const c = hexCenter({ q: col, r: row });
				if (c.x < a.x - HEX_CELL || c.x > b.x + HEX_CELL || c.y < a.y - HEX_CELL || c.y > b.y + HEX_CELL) continue;
				const pts = [0, 1, 2, 3, 4, 5].map((i) => {
					const ang = (Math.PI / 180) * (60 * i - 30);
					const w = view.toScreen({ x: c.x + r * Math.cos(ang), y: c.y + r * Math.sin(ang) });
					return `${w.x.toFixed(1)},${w.y.toFixed(1)}`;
				});
				d += `M${pts.join('L')}Z`;
			}
		return d;
	});

	// ── a place's footprint (owner 2026-10-11): the cells it takes – shown while it is dragged, while it is selected and,
	//    with the place tool, where a new one would land; the same cells are its hit area (outside them a click goes through)
	const cellsPath = (cells: ReturnType<typeof placeCells>, origin: Point) =>
		cells
			.map((c) => 'M' + hexCorners(c).map((k) => `${((k.x - origin.x) * view.zoom).toFixed(1)},${((k.y - origin.y) * view.zoom).toFixed(1)}`).join('L') + 'Z')
			.join('');
	const footprint = $derived.by(() => {
		const id = dragging?.kind === 'place' ? dragging.id : tool === 'select' && selection?.kind === 'place' ? selection.id : null;
		const place = id ? map.locations.find((p) => p.id === id) : null;
		if (place) return { cells: placeCells(place), preview: false };
		if (tool === 'place' && hover && !space) {
			const at = snapToGrid(map, view.toWorld(hover), DEFAULT_PLACE_IMAGE_SIZE);
			return { cells: placeCells({ ...at, size: DEFAULT_PLACE_IMAGE_SIZE }), preview: true };
		}
		return null;
	});

	// culling (BACKLOG 10.63): the places near the screen through a spatial index; the margin covers the biggest diorama
	const placeIndex = $derived(pointIndex(map.locations));
	const shownPlaces = $derived(placeIndex.query(growRect(view.worldRect(200), PLACE_SIZE_MAX / 2)));

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

	// ── DRAFT (BACKLOG 10.66 (4), for the owner – `?routeDraft=a|b|c`): routes going behind a place's diorama with an entry
	// and an exit point. a = gates: the route stops at the diorama's edge where it comes in (a gate mark) and goes on from
	// where it leaves – nothing drawn across the place; b = tunnel: the route under the diorama as today, the hidden part
	// through it shown dashed and faint on top; c = behind: the route bends round the back of the diorama (its far half),
	// under the picture, so it disappears behind the building and comes out on the other side. Entry / exit = where the
	// line to the previous / next waypoint crosses the place's footprint (later: points set per place by the author).
	const routeDraft = $derived(page.url.searchParams.get('routeDraft'));
	interface DraftRoute {
		id: string;
		stroke: ReturnType<typeof routeStroke>;
		segments: Point[][];
		gates: Point[];
		inner: Point[][];
	}
	const draftRoutes = $derived.by<DraftRoute[]>(() => {
		if (!routeDraft) return [];
		return map.routes.map((r) => {
			const pts = r.waypoints
				.map((w) => {
					const p = waypointPoint(map.locations, w);
					const place = isPlaceWaypoint(w) ? map.locations.find((x) => x.id === w.location) : undefined;
					return p ? { p, place } : null;
				})
				.filter((x): x is { p: Point; place: MapPlace | undefined } => !!x);
			const segments: Point[][] = [[]];
			const gates: Point[] = [];
			const inner: Point[][] = [];
			// the diorama's ground: an ellipse (an isometric patch is flatter than wide) on the place's point
			const FLAT = 0.6;
			const toward = (from: Point, to: Point, r: number) => {
				const a = Math.atan2((to.y - from.y) / FLAT, to.x - from.x);
				return { x: from.x + Math.cos(a) * r, y: from.y + Math.sin(a) * r * FLAT };
			};
			pts.forEach(({ p, place }, i) => {
				const current = segments[segments.length - 1];
				if (!place) return void current.push(p);
				const r = (placeSize(place) / 2) * 0.5;
				const prev = pts[i - 1]?.p;
				const next = pts[i + 1]?.p;
				const entry = prev ? toward(p, prev, r) : null;
				const exit = next ? toward(p, next, r) : null;
				if (routeDraft === 'a') {
					if (entry) {
						current.push(entry);
						gates.push(entry);
					}
					if (exit) {
						segments.push([exit]);
						gates.push(exit);
					}
				} else if (routeDraft === 'b') {
					current.push(p);
					inner.push([entry ?? p, p, exit ?? p]);
				} else {
					if (!entry || !exit) return void current.push(entry ?? exit ?? p);
					// the far half: from the entry's angle to the exit's, the way round that passes the top (−90°)
					const a1 = Math.atan2((entry.y - p.y) / FLAT, entry.x - p.x);
					const a2 = Math.atan2((exit.y - p.y) / FLAT, exit.x - p.x);
					const up = -Math.PI / 2;
					const passes = (from: number, delta: number) => {
						const t = (((up - from) % (2 * Math.PI)) + 2 * Math.PI) % (2 * Math.PI);
						return delta >= 0 ? t <= delta : 2 * Math.PI - t <= -delta;
					};
					let delta = a2 - a1;
					if (delta > Math.PI) delta -= 2 * Math.PI;
					if (delta < -Math.PI) delta += 2 * Math.PI;
					if (!passes(a1, delta)) delta = delta > 0 ? delta - 2 * Math.PI : delta + 2 * Math.PI;
					for (let k = 0; k <= 8; k++) {
						const a = a1 + (delta * k) / 8;
						current.push({ x: p.x + Math.cos(a) * r, y: p.y + Math.sin(a) * r * FLAT });
					}
				}
			});
			return { id: r.id, stroke: routeStroke(r.style), segments: segments.filter((x) => x.length >= 2), gates, inner };
		});
	});

	/** the phone frame: a 375 × 812 reader screen at the reader's zoom (the toon's start zoom, 1) around the canvas centre */
	/** what the frame shows: the selected place's reader view, else the toon's start view */
	const framed = $derived(placeFrame ?? startView);
	/** the reader's zoom in the frame: the place's / the toon's start zoom, else 1 */
	const frameZoom = $derived(framed?.zoom ?? 1);
	const frame = $derived.by(() => {
		const at = framed ? view.toScreen(framed.center) : { x: view.width / 2, y: view.height / 2 };
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
	<MapTerrain {view} width={map.width} height={map.height} layers={map.layers ?? []} url={asset} visible={layerVisible} zoomRange={false} />
	{#if gridPath}
		<svg class="hex-grid" aria-hidden="true"><path d={gridPath} /></svg>
	{/if}
	{#if footprint}
		{@const o = view.toScreen({ x: 0, y: 0 })}
		<svg class="footprint" class:preview={footprint.preview} aria-hidden="true" data-testid="place-footprint"><path transform="translate({o.x} {o.y})" d={cellsPath(footprint.cells, { x: 0, y: 0 })} /></svg>
	{/if}

	{#if routeDraft}
		<svg class="route-draft" aria-hidden="true">
			<g transform="translate({view.x} {view.y}) scale({view.zoom})">
				{#each draftRoutes as r (r.id)}
					{#each r.segments as seg, si (si)}
						{#if r.stroke.outline}<path d={bezierPath(seg, r.stroke.tension, r.stroke.bow)} stroke={r.stroke.outline} style:stroke-width="{r.stroke.width + 3}px" />{/if}
						<path d={bezierPath(seg, r.stroke.tension, r.stroke.bow)} stroke={r.stroke.color} stroke-dasharray={r.stroke.dash || undefined} style:stroke-width="{r.stroke.width}px" />
					{/each}
				{/each}
			</g>
		</svg>
	{:else}
		<MapRoutes {routes} places={map.locations} zoom={view.zoom} x={view.x} y={view.y} onroute={onRouteClick} />
	{/if}

	{#if draftPoints.length >= 2}
		<svg class="draft" aria-hidden="true">
			<polyline points={draftPoints.map((p) => { const s = view.toScreen(p); return `${s.x},${s.y}`; }).join(' ')} />
		</svg>
	{/if}

	<div class="places">
		{#each shownPlaces as p (p.id)}
			{@const s = view.toScreen(p)}
			{@const dim = !inFilter(p.toons) || (readerView && !!p.hidden)}
			{@const active = selection?.kind === 'place' && selection.id === p.id}
			<div class="place" class:has-image={!!p.image?.src} class:dim class:active class:collides={collisions.has(p.id)} class:hidden-place={!!p.hidden} style:transform="translate({s.x}px, {s.y}px)">
				{#if p.image?.src}
					{@const box = placeImageBox(p.image, view.zoom, 1, p.size)}
					<!-- the diorama selects its place too (owner 2026-10-11), with the select tool -->
					<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
					<img class="place-image" src={asset(p.image.src)} alt="" draggable="false" style:width="{box.width}px" style:left="{box.left}px" style:top="{box.top}px" />
				{:else}
					<!-- no diorama yet: a dashed outline of one at the place's size, centred on its cells (owner 2026-10-11) -->
					{@const w = placeSize(p) * view.zoom}
					<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
					<svg class="place-outline" viewBox="0 0 100 70" style:width="{w}px" style:height="{w * 0.7}px" style:left="{-w / 2}px" style:top="{-w * 0.7 / 2}px">
						<path d="M50 6 L94 30 L94 42 L50 66 L6 42 L6 30 Z M6 30 L50 54 L94 30 M50 54 L50 66" />
					</svg>
				{/if}
				<!-- its hit area: the cells it takes, not the picture's box – a picture reaching over a neighbour's cells lets
				     clicks there through (owner 2026-10-11); dragging it moves the place like the marker -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<svg class="place-hit" aria-hidden="true" data-handle>
					<path
						d={cellsPath(placeCells(p), p)}
						onpointerdown={(e) => startDrag(e, { kind: 'place', id: p.id })}
						onpointermove={dragMove}
						onpointerup={(e) => dragEnd(e, () => (tool === 'select' ? onselect({ kind: 'place', id: p.id }) : null))}
					/>
				</svg>
				<button
					class="marker"
					data-control-exempt="map marker on the canvas"
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
		{/each}
	</div>

	{#if routeDraft === 'a' || routeDraft === 'b'}
		<!-- DRAFT 10.66 (4): over the dioramas – the gates (a) / the hidden path through the place (b) -->
		<svg class="route-draft" aria-hidden="true">
			<g transform="translate({view.x} {view.y}) scale({view.zoom})">
				{#each draftRoutes as r (r.id)}
					{#each r.inner as seg, si (si)}
						<path class="inner" d={bezierPath(seg, 0.3, 0)} stroke={r.stroke.color} style:stroke-width="{r.stroke.width}px" />
					{/each}
					{#each r.gates as g, gi (gi)}
						<circle class="gate" cx={g.x} cy={g.y} r={4 / view.zoom} stroke={r.stroke.color} />
					{/each}
				{/each}
			</g>
		</svg>
	{/if}

	{#if selectedRoute}
		<div class="handles">
			{#each selectedRoute.waypoints as w, i (i)}
				{@const pt = waypointPoint(map.locations, w)}
				{#if pt}
					{@const s = view.toScreen(pt)}
					<button
						class="handle"
						data-control-exempt="canvas resize handle"
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
		<div class="phone" style:width="{frame.w}px" style:height="{frame.h}px" style:left="{frame.x}px" style:top="{frame.y}px" aria-hidden="true" data-testid="phone-frame-box"><span>375 × 812 · {zoomPercent(frameZoom)} %{placeFrame ? ' · place' : startView ? ' · start' : ''}</span></div>
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
	.hex-grid {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
	}
	.hex-grid path {
		fill: none;
		stroke: var(--ink);
		stroke-opacity: 0.25;
		stroke-width: 1;
	}
	.footprint {
		position: absolute;
		z-index: 1; /* over the dioramas: the size must show (owner 2026-10-11) */
		inset: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
	}
	.footprint path {
		fill: var(--primary-yellow, #ffe600);
		fill-opacity: 0.45;
		stroke: var(--ink);
		stroke-width: 1.5;
	}
	.footprint.preview path {
		fill-opacity: 0.3;
		stroke-dasharray: 4 3;
	}
	.place-hit {
		position: absolute;
		left: 0;
		top: 0;
		width: 1px;
		height: 1px;
		overflow: visible;
		pointer-events: none;
	}
	.place-hit path {
		fill: transparent;
		pointer-events: fill;
		cursor: pointer;
	}
	.place-outline {
		position: absolute;
		overflow: visible;
		pointer-events: none;
	}
	.place-outline path {
		fill: rgb(255 255 255 / 0.35);
		stroke: var(--ink);
		stroke-width: 2;
		stroke-dasharray: 5 4;
		vector-effect: non-scaling-stroke;
	}
	/* DRAFT 10.66 (4) */
	.route-draft {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		overflow: visible;
		pointer-events: none;
	}
	.route-draft path {
		fill: none;
		vector-effect: non-scaling-stroke;
		stroke-linecap: round;
		stroke-linejoin: round;
	}
	.route-draft .inner {
		stroke-dasharray: 4 7;
		opacity: 0.8;
	}
	.route-draft circle.gate {
		fill: var(--paper);
		stroke-width: 2.5px;
		vector-effect: non-scaling-stroke;
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
		/* the creator's global button rule sets min-height: var(--control-height) (36–52px): without this reset the round
		   marker became a tall ellipse (owner screenshot 2026-10-01) */
		min-width: 0;
		min-height: 0;
		box-sizing: border-box;
		padding: 0;
		border-radius: 50%;
		border: 2px solid var(--ink);
		background: var(--surface-tint-cyan, #f0fcff);
		pointer-events: auto;
		cursor: pointer;
	}
	/* touch screens (E.9): a 2.75rem target around the 16px point – the point itself stays small (the map's density) */
	@media (pointer: coarse) {
		.marker::before {
			content: '';
			position: absolute;
			left: 50%;
			top: 50%;
			width: 2.75rem;
			height: 2.75rem;
			transform: translate(-50%, -50%);
		}
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
		min-width: 0;
		min-height: 0;
		box-sizing: border-box;
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
