<script lang="ts" module>
	export type MapTool = 'select' | 'place' | 'route';
	export type MapSelection = { kind: 'place' | 'route' | 'layer' | 'region'; id: string } | null;
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
		routeGeometry,
		snapToPlace,
		cellsOutline,
		ROUTE_HANDLE,
		type PlaceGate,
		visibleLabels,
		waypointPoint,
		type MapPlace,
		type Point,
		type Waypoint,
		type WorldMap
	} from '@scutoons/content';
	import { MapRoutes, MapTerrain, MapViewport, mapGestures, type DrawnRoute } from '@scutoons/reader';
	import { Icon } from '@scutoons/ui';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import { freeWaypointId, nearestSegment } from '$lib/editor/map.ts';
	import HexBrush, { type HexBrushTarget } from '$lib/components/map/hex-brush.svelte';

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
		onmovegate,
		onmovewaypointhandle,
		oninsertwaypoint,
		onaddroute,
		onendpoint,
		brush = null,
		regions = []
	}: {
		/** the hex brush over the canvas (a place's ground – owner 2026-10-12): painting replaces selecting while it is on */
		brush?: HexBrushTarget | null;
		/**
		 * the regions' covers to draw (agents/MAP.md §8.10): SVG paths in world units (`runsPaths`, computed once per change
		 * by the builder) under the camera's transform – a light fill each, the selected one with its outline
		 */
		regions?: { id: string; colour: string; fill: string; edge: string; selected: boolean }[];
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
		/**
		 * a route gate of a place dragged (BACKLOG 10.66 (4)) or its Bézier handle (owner 2026-10-12): the gate's offset from
		 * the place's point with its handle, world units; null = back to computed
		 */
		onmovegate: (place: string, which: 'entry' | 'exit', gate: PlaceGate | null) => void;
		/** a free waypoint's Bézier handle dragged (an offset from the point; null = smooth again) */
		onmovewaypointhandle: (route: string, index: number, handle: Point | null) => void;
		oninsertwaypoint: (route: string, after: number, at: Point) => void;
		onaddroute: (waypoints: Waypoint[]) => void;
		/**
		 * a route starts and ends at a place (owner 2026-10-12: "if each endpoint is a place, dropping a pin should prompt
		 * select location or create one"): asked for a start / end point on the open terrain – the place id, or null
		 */
		onendpoint: (at: Point, which: 'start' | 'end') => Promise<string | null>;
	} = $props();

	const locale = $derived(getLocale());
	const inFilter = (toons: string[] | undefined) => !toonFilter || !!toons?.includes(toonFilter);
	const placeById = $derived(new Map(map.locations.map((p) => [p.id, p])));

	// ── panning: the middle button, Space held, a drag on the empty terrain with Select, any drag with the route / place
	//    tools (a tap there sets the point; touch: drag / two fingers) ──
	let space = $state(false);
	// a safeguard (owner 2026-10-12: "while scrolling around I often accidentally move things … items, Bézier and route
	// entries – it is a mess if it just goes"): anything draggable moves only once it is active – a place when it is
	// selected, a gate / curve handle / route point when it was tapped (`activeHandle`). The first tap activates; a drag
	// on something not active pans the map.
	const isActivePlace = (id: string) => selection?.kind === 'place' && selection.id === id;
	let activeHandle = $state<string | null>(null);
	$effect(() => {
		void selection;
		activeHandle = null;
	});
	const placeOf = (e: PointerEvent) => (e.target as HTMLElement | null)?.closest<HTMLElement | SVGElement>('[data-place]')?.dataset.place ?? null;
	const handleOf = (e: PointerEvent) => (e.target as HTMLElement | null)?.closest<HTMLElement>('[data-drag]')?.dataset.drag ?? null;
	function canPan(e: PointerEvent) {
		if (space || e.button === 1) return true;
		// the hex brush paints with the primary button / one finger (two fingers still pan – the gesture's pinch)
		if (brush && (e.target as HTMLElement | null)?.closest('[data-testid="hex-brush"]')) return false;
		// drawing a route / adding a place: a drag pans, a tap sets the point (owner 2026-10-12, phone: "I can not drag
		// the map in route mode")
		if (tool !== 'select') return true;
		if (!(e.target as HTMLElement | null)?.closest('[data-handle]')) return true;
		const place = placeOf(e);
		if (place) return !isActivePlace(place);
		const handle = handleOf(e);
		return !!handle && handle !== activeHandle;
	}
	function onTap(e: PointerEvent, at: Point) {
		if (space) return;
		const world = view.toWorld(at);
		if (tool === 'place') {
			if ((e.target as HTMLElement | null)?.closest('[data-handle]')) return;
			// the tool turns to select with the new place selected: the click that follows this tap must not select a
			// route whose line is under it (check-map 2026-10-12 – the new place lost its selection)
			placedAt = e.timeStamp;
			onaddplace(world);
			return;
		}
		if (tool === 'route') {
			addDraftPoint(at, world);
			return;
		}
		// something not active yet: the tap activates it (it moves from the next drag on)
		const place = placeOf(e);
		if (place) return onselect({ kind: 'place', id: place });
		const handle = handleOf(e);
		if (handle) {
			activeHandle = handle;
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
	let asking = false;
	async function addDraftPoint(at: Point, world: Point) {
		const place = snapPlace(at);
		// the start is a place: on the open terrain ask which (or a new one there)
		if (!place && !draft.length) {
			if (asking) return;
			asking = true;
			const id = await onendpoint({ x: Math.round(world.x), y: Math.round(world.y) }, 'start').finally(() => (asking = false));
			if (id) draft = [{ location: id }];
			return;
		}
		if (place) {
			const last = draft.at(-1);
			if (last && isPlaceWaypoint(last) && last.location === place.id) return;
			draft = [...draft, { location: place.id }];
		} else {
			draft = [...draft, { id: freeWaypointId(draft, map.locations.map((p) => p.id)), x: Math.round(world.x), y: Math.round(world.y) }];
		}
	}
	export async function finishRoute() {
		if (draft.length < 2 || asking) return;
		const last = draft.at(-1)!;
		if (!isPlaceWaypoint(last)) {
			// the end is a place: the last free point becomes one (picked or new there)
			asking = true;
			const id = await onendpoint({ x: Math.round(last.x), y: Math.round(last.y) }, 'end').finally(() => (asking = false));
			if (!id) return; // cancelled: keep drawing
			const before = draft.at(-2);
			draft = before && isPlaceWaypoint(before) && before.location === id ? draft.slice(0, -1) : [...draft.slice(0, -1), { location: id }];
			if (draft.length < 2) return;
		}
		onaddroute(draft);
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
	let dragging = $state<
		| { kind: 'place'; id: string }
		| { kind: 'waypoint'; route: string; index: number }
		| { kind: 'gate'; place: string; which: 'entry' | 'exit' }
		| { kind: 'gate-handle'; place: string; which: 'entry' | 'exit' }
		| { kind: 'waypoint-handle'; route: string; index: number }
		| null
	>(null);

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
	// ── the selected route's places (owner 2026-10-12, B + C of three drafts: "the circle handle in the centre is really
	// ugly … would only be needed if the zoom level makes the hexagon too small … the number label is needed"): the
	// place's patch edge in the route's highlight + a small numbered chip at its entry gate (its exit for the first);
	// the round centre handle only while the patch is under 2rem on screen; a free point is a small dot
	const rootPx = typeof document === 'undefined' ? 16 : parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
	const tinyPlace = (place: MapPlace) => placeSize(place) * view.zoom < 2 * rootPx;
	/** where a place waypoint's chip sits: its entry gate (its exit for the first) */
	function chipAt(route: { waypoints: readonly Waypoint[] }, i: number, fallback: Point): Point {
		const w = route.waypoints[i];
		if (!w || !isPlaceWaypoint(w)) return fallback;
		const g = routeGeometry(map.locations, route.waypoints).gates.find((x) => x.place === w.location && x.which === (i === 0 ? 'exit' : 'entry'));
		return g?.at ?? fallback;
	}

	/** the patch's outer edge (owner 2026-10-12: "the peripheral ones darker than the others") – one path of segments */
	const outlinePath = (cells: ReturnType<typeof placeCells>, origin: Point) =>
		cellsOutline(cells)
			.map(([a, b]) => `M${((a.x - origin.x) * view.zoom).toFixed(1)},${((a.y - origin.y) * view.zoom).toFixed(1)}L${((b.x - origin.x) * view.zoom).toFixed(1)},${((b.y - origin.y) * view.zoom).toFixed(1)}`)
			.join('');
	/** while a gate is dragged: the hexagon it snaps to lights up (owner 2026-10-12) */
	let gateCell = $state<{ place: string; cell: ReturnType<typeof hexCellAt> } | null>(null);
	const footprint = $derived.by(() => {
		const id = dragging?.kind === 'place' ? dragging.id : tool === 'select' && selection?.kind === 'place' ? selection.id : null;
		const place = id ? map.locations.find((p) => p.id === id) : null;
		if (brush && place) return null; // the brush draws the cells it paints
		if (place) return { cells: placeCells(place), preview: false };
		if (tool === 'place' && hover && !space) {
			const at = snapToGrid(map, view.toWorld(hover), DEFAULT_PLACE_IMAGE_SIZE);
			return { cells: placeCells({ ...at, size: DEFAULT_PLACE_IMAGE_SIZE }), preview: true };
		}
		return null;
	});

	// the selected place's route gates (BACKLOG 10.66 (4), owner 2026-10-12: "the computed ones first, then adjustable"):
	// one handle per gate the routes through it use – dragging one sets its offset (filled = set, hollow = computed)
	const placeGates = $derived.by(() => {
		if (tool !== 'select' || selection?.kind !== 'place') return [];
		const id = selection.id;
		const place = map.locations.find((p) => p.id === id);
		if (!place) return [];
		const seen = new Set<string>();
		const out: { which: 'entry' | 'exit'; at: Point; handle: Point; set: boolean; handleSet: boolean }[] = [];
		for (const r of map.routes)
			for (const g of routeGeometry(map.locations, r.waypoints).gates)
				if (g.place === id && !seen.has(g.which)) {
					seen.add(g.which);
					out.push({ which: g.which, at: g.at, handle: g.handle, set: g.set, handleSet: g.handleSet });
				}
		return out;
	});
	// the selected route's free points: their Bézier handles (owner 2026-10-12) – their own, else where the smooth curve's
	// control point lies (hollow), so a drag starts from the shape that is drawn
	const waypointHandles = $derived.by(() => {
		if (tool !== 'select' || selection?.kind !== 'route') return [];
		const route = map.routes.find((r) => r.id === (selection as { id: string }).id);
		if (!route) return [];
		const out: { index: number; at: Point; handle: Point; set: boolean }[] = [];
		route.waypoints.forEach((w, i) => {
			if (isPlaceWaypoint(w)) return;
			const at = { x: w.x, y: w.y };
			if (w.handle) return void out.push({ index: i, at, handle: { x: w.x + w.handle.x, y: w.y + w.handle.y }, set: true });
			const prev = route.waypoints[i - 1] ? waypointPoint(map.locations, route.waypoints[i - 1]) : null;
			const next = route.waypoints[i + 1] ? waypointPoint(map.locations, route.waypoints[i + 1]) : null;
			const a = prev ?? at, b = next ?? at;
			const t = { x: b.x - a.x, y: b.y - a.y };
			const l = Math.hypot(t.x, t.y) || 1;
			const len = Math.max(ROUTE_HANDLE, (l / 2) * 0.3);
			out.push({ index: i, at, handle: { x: at.x + (t.x / l) * len, y: at.y + (t.y / l) * len }, set: false });
		});
		return out;
	});
	/** a gate as stored: its offset (the drawn position when it was computed) and its handle */
	function gateOf(placeId: string, which: 'entry' | 'exit'): { place: MapPlace; gate: PlaceGate } | null {
		const place = map.locations.find((p) => p.id === placeId);
		const drawn = placeGates.find((g) => g.which === which);
		if (!place || !drawn) return null;
		const own = place.gates?.[which];
		const r = (n: number) => Math.round(n * 100) / 100;
		return { place, gate: { x: r(drawn.at.x - place.x), y: r(drawn.at.y - place.y), ...(own?.handle ? { handle: own.handle } : {}) } };
	}

	// culling (BACKLOG 10.63): the places near the screen through a spatial index; the margin covers the biggest diorama
	const placeIndex = $derived(pointIndex(map.locations));
	const shownPlaces = $derived(placeIndex.query(growRect(view.worldRect(200), PLACE_SIZE_MAX / 2)));

	let dragMoved = false;
	/** a handle's key for the safeguard (`data-drag`) */
	function dragKey(t: NonNullable<typeof dragging>): string {
		if (t.kind === 'place') return `place:${t.id}`;
		if (t.kind === 'gate' || t.kind === 'gate-handle') return `${t.kind}:${t.place}:${t.which}`;
		return `${t.kind}:${t.route}:${t.index}`;
	}
	function startDrag(e: PointerEvent, target: NonNullable<typeof dragging>) {
		if (tool !== 'select' || space || e.button !== 0) return;
		// not active yet: no drag – the map's gestures pan, a tap activates (canPan / onTap)
		if (target.kind === 'place' ? !isActivePlace(target.id) : dragKey(target) !== activeHandle) return;
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
		else if (dragging.kind === 'gate') {
			// a gate snaps to the place's own hexagons – never outside them (owner 2026-10-12); its handle stays
			const found = gateOf(dragging.place, dragging.which);
			if (!found) return;
			const c = snapToPlace(found.place, world);
			gateCell = { place: found.place.id, cell: hexCellAt(c) };
			onmovegate(found.place.id, dragging.which, { ...found.gate, x: Math.round((c.x - found.place.x) * 100) / 100, y: Math.round((c.y - found.place.y) * 100) / 100 });
		} else if (dragging.kind === 'gate-handle') {
			const found = gateOf(dragging.place, dragging.which);
			if (!found) return;
			const gx = found.place.x + found.gate.x, gy = found.place.y + found.gate.y;
			onmovegate(found.place.id, dragging.which, { ...found.gate, handle: { x: Math.round((world.x - gx) * 10) / 10, y: Math.round((world.y - gy) * 10) / 10 } });
		} else if (dragging.kind === 'waypoint-handle') {
			const route = map.routes.find((r) => r.id === (dragging as { route: string }).route);
			const w = route?.waypoints[dragging.index];
			if (w && !isPlaceWaypoint(w)) onmovewaypointhandle(route!.id, dragging.index, { x: Math.round((world.x - w.x) * 10) / 10, y: Math.round((world.y - w.y) * 10) / 10 });
		} else onmovewaypoint(dragging.route, dragging.index, at);
	}
	function dragEnd(e: PointerEvent, click: () => void) {
		gateCell = null;
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
	/** when the last tap added a place (its trailing click is not a route click) */
	let placedAt = -Infinity;
	function onRouteClick(id: string, e: MouseEvent) {
		if (tool !== 'select' || e.timeStamp - placedAt < 500) return;
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
	{@attach mapGestures(view, { canPan, onTap, onDoubleClick, onHover: (e, at) => (hover = e.pointerType === 'mouse' ? at : null) })}
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
		<svg class="footprint" class:preview={footprint.preview} aria-hidden="true" data-testid="place-footprint"><path transform="translate({o.x} {o.y})" d={cellsPath(footprint.cells, { x: 0, y: 0 })} /><path class="edge" transform="translate({o.x} {o.y})" d={outlinePath(footprint.cells, { x: 0, y: 0 })} />{#if gateCell}<path class="snap" transform="translate({o.x} {o.y})" d={cellsPath([gateCell.cell], { x: 0, y: 0 })} />{/if}</svg>
	{/if}

	{#if regions.length}
		<!-- one SVG (its own layer) per region: after a stroke the browser repaints the changed region only -->
		<div class="regions-layers" data-testid="map-regions">
			{#each regions as r (r.id)}
				<svg class="regions" aria-hidden="true">
					<g transform="translate({view.x} {view.y}) scale({view.zoom})">
						<path class="region-fill" class:selected={r.selected} d={r.fill} fill={r.colour} data-region={r.id} />
						{#if r.selected}<path class="region-edge under" d={r.edge} /><path class="region-edge" d={r.edge} stroke={r.colour} />{/if}
					</g>
				</svg>
			{/each}
		</div>
	{/if}
	<MapRoutes {routes} places={map.locations} zoom={view.zoom} x={view.x} y={view.y} onroute={onRouteClick} />

	{#if tool === 'route' && draft.length}
		<!-- drawing a route: the last point back, done (on a phone there is no Enter, a double tap is unreliable) -->
		<div class="draw-bar sc-seg" role="group" data-testid="route-draw-bar">
			<button type="button" onclick={() => undoDraftPoint()} title={m['creator.map_route_undo_point']()} aria-label={m['creator.map_route_undo_point']()}><Icon name="corner-up-left" /></button>
			<button type="button" class="on" onclick={() => void finishRoute()} disabled={draft.length < 2} data-testid="route-done"><Icon name="check" /> {m['creator.map_route_done']()}</button>
		</div>
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
				{/if}
				<!-- its hit area: the cells it takes, not the picture's box – a picture reaching over a neighbour's cells lets
				     clicks there through (owner 2026-10-11); dragging it moves the place like the marker -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<!-- without a diorama the cells themselves show, dashed (owner 2026-10-12: "the hexagons are enough") -->
				<svg class="place-hit" class:bare={!p.image?.src} aria-hidden="true" data-handle data-place={p.id}>
					<path
						d={cellsPath(placeCells(p), p)}
						onpointerdown={(e) => startDrag(e, { kind: 'place', id: p.id })}
						onpointermove={dragMove}
						onpointerup={(e) => dragEnd(e, () => (tool === 'select' ? onselect({ kind: 'place', id: p.id }) : null))}
					/>
					{#if !p.image?.src}<path class="edge" d={outlinePath(placeCells(p), p)} />{/if}
				</svg>
				<button
					class="marker"
					data-control-exempt="map marker on the canvas"
					data-handle
					data-place={p.id}
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

	<!-- the routes' gates over the dioramas (BACKLOG 10.66 (4), owner 2026-10-12: variant A – computed, adjustable) -->
	<MapRoutes {routes} places={map.locations} zoom={view.zoom} x={view.x} y={view.y} layer="gates" />

	{#if placeGates.length || waypointHandles.length}
		<!-- the Bézier handles' arms (owner 2026-10-12): gate or free point → its control point -->
		<svg class="arms" aria-hidden="true">
			{#each [...placeGates.map((g) => ({ key: g.which, at: g.at, handle: g.handle, set: g.handleSet })), ...waypointHandles.map((h) => ({ key: `wp${h.index}`, at: h.at, handle: h.handle, set: h.set }))] as arm (arm.key)}
				{@const a = view.toScreen(arm.at)}
				{@const b = view.toScreen(arm.handle)}
				<line x1={a.x} y1={a.y} x2={b.x} y2={b.y} class:set={arm.set} />
			{/each}
		</svg>
	{/if}
	{#if placeGates.length}
		<div class="handles">
			{#each placeGates as g (g.which)}
				{@const h = view.toScreen(g.handle)}
				<button
					class="handle curve"
					class:set={g.handleSet}
					class:active={activeHandle === `gate-handle:${(selection as { id: string }).id}:${g.which}`}
					data-control-exempt="canvas resize handle"
					data-handle
					data-drag="gate-handle:{(selection as { id: string }).id}:{g.which}"
					style:transform="translate({h.x}px, {h.y}px)"
					aria-label={m['creator.map_curve_handle']()}
					title="{m['creator.map_curve_handle']()} · {g.handleSet ? m['creator.map_gate_set']() : m['creator.map_gate_auto']()} – {m['creator.map_curve_reset_hint']()}"
					onpointerdown={(e) => startDrag(e, { kind: 'gate-handle', place: (selection as { id: string }).id, which: g.which })}
					onpointermove={dragMove}
					onpointerup={(e) => dragEnd(e, () => {})}
					ondblclick={() => { const found = gateOf((selection as { id: string }).id, g.which); if (found) { const { handle: _, ...rest } = found.gate; onmovegate(found.place.id, g.which, g.set ? rest : null); } }}
					data-testid="builder-gate-handle-{g.which}"
				></button>
			{/each}
			{#each placeGates as g (g.which)}
				{@const s = view.toScreen(g.at)}
				<button
					class="handle gate"
					class:set={g.set}
					class:active={activeHandle === `gate:${(selection as { id: string }).id}:${g.which}`}
					data-control-exempt="canvas resize handle"
					data-handle
					data-drag="gate:{(selection as { id: string }).id}:{g.which}"
					style:transform="translate({s.x}px, {s.y}px)"
					aria-label={g.which === 'entry' ? m['creator.map_gate_entry']() : m['creator.map_gate_exit']()}
					title="{g.which === 'entry' ? m['creator.map_gate_entry']() : m['creator.map_gate_exit']()} · {g.set ? m['creator.map_gate_set']() : m['creator.map_gate_auto']()}"
					onpointerenter={() => !dragging && (gateCell = { place: (selection as { id: string }).id, cell: hexCellAt(g.at) })}
					onpointerleave={() => !dragging && (gateCell = null)}
					onpointerdown={(e) => startDrag(e, { kind: 'gate', place: (selection as { id: string }).id, which: g.which })}
					onpointermove={dragMove}
					onpointerup={(e) => dragEnd(e, () => {})}
					ondblclick={() => onmovegate((selection as { id: string }).id, g.which, null)}
					data-testid="builder-gate-{g.which}"
				>{g.which === 'entry' ? m['creator.map_gate_entry_short']() : m['creator.map_gate_exit_short']()}</button>
			{/each}
		</div>
	{/if}

	{#if selectedRoute && waypointHandles.length}
		<div class="handles">
			{#each waypointHandles as h (h.index)}
				{@const s = view.toScreen(h.handle)}
				<button
					class="handle curve"
					class:set={h.set}
					class:active={activeHandle === `waypoint-handle:${selectedRoute.id}:${h.index}`}
					data-control-exempt="canvas resize handle"
					data-handle
					data-drag="waypoint-handle:{selectedRoute.id}:{h.index}"
					style:transform="translate({s.x}px, {s.y}px)"
					aria-label={m['creator.map_curve_handle']()}
					title="{m['creator.map_curve_handle']()} · {h.set ? m['creator.map_gate_set']() : m['creator.map_gate_auto']()} – {m['creator.map_curve_reset_hint']()}"
					onpointerdown={(e) => startDrag(e, { kind: 'waypoint-handle', route: selectedRoute.id, index: h.index })}
					onpointermove={dragMove}
					onpointerup={(e) => dragEnd(e, () => {})}
					ondblclick={() => onmovewaypointhandle(selectedRoute.id, h.index, null)}
					data-testid="builder-waypoint-handle-{h.index}"
				></button>
			{/each}
		</div>
	{/if}
	{#if selectedRoute}
		<!-- the route's places as their patch's edge (B) -->
		<svg class="route-patches" aria-hidden="true">
			{#each selectedRoute.waypoints as w, i (i)}
				{#if isPlaceWaypoint(w)}
					{@const place = map.locations.find((p) => p.id === w.location)}
					{#if place && !tinyPlace(place)}
						{@const o = view.toScreen(place)}
						<path transform="translate({o.x} {o.y})" d={outlinePath(placeCells(place), place)} />
					{/if}
				{/if}
			{/each}
		</svg>
		<div class="handles">
			{#each selectedRoute.waypoints as w, i (i)}
				{@const pt = waypointPoint(map.locations, w)}
				{@const place = isPlaceWaypoint(w) ? map.locations.find((p) => p.id === w.location) : undefined}
				{#if pt}
					{@const tiny = !!place && tinyPlace(place)}
					{@const s = view.toScreen(place && !tiny ? chipAt(selectedRoute, i, pt) : pt)}
					<button
						class="handle"
						data-control-exempt="canvas resize handle"
						class:free={!isPlaceWaypoint(w)}
						class:chip={!!place && !tiny}
						class:active={activeHandle === `waypoint:${selectedRoute.id}:${i}`}
						data-handle
						data-drag="waypoint:{selectedRoute.id}:{i}"
						style:transform="translate({s.x}px, {s.y}px)"
						aria-label={String(i + 1)}
						title="{i + 1}: {isPlaceWaypoint(w) ? w.location : w.id}"
						onpointerdown={(e) => startDrag(e, { kind: 'waypoint', route: selectedRoute.id, index: i })}
						onpointermove={dragMove}
						onpointerup={(e) => dragEnd(e, () => {})}
						data-testid="builder-waypoint-{i}"
					>{isPlaceWaypoint(w) ? i + 1 : ''}</button>
				{/if}
			{/each}
		</div>
	{/if}

	{#if brush}<HexBrush {view} target={brush} />{/if}

	{#if phoneFrame}
		<div class="phone" style:width="{frame.w}px" style:height="{frame.h}px" style:left="{frame.x}px" style:top="{frame.y}px" aria-hidden="true" data-testid="phone-frame-box"><span>375 × 812 · {zoomPercent(frameZoom)} %{placeFrame ? ' · place' : startView ? ' · start' : ''}</span></div>
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
	/* the cells (owner 2026-10-12: "more transparent, less dark the outlines … the peripheral ones darker"): a light fill,
	   faint inner lines, the patch's outer edge in ink */
	.footprint path {
		fill: var(--primary-yellow, #ffe600);
		fill-opacity: 0.18;
		stroke: var(--ink);
		stroke-opacity: 0.2;
		stroke-width: 1;
	}
	.footprint path.edge,
	.place-hit path.edge {
		fill: none;
		stroke: var(--ink);
		stroke-opacity: 0.85;
		stroke-width: 1.5;
		stroke-linecap: round;
		pointer-events: none;
	}
	.footprint.preview path {
		fill-opacity: 0.12;
	}
	.footprint.preview path.edge {
		stroke-dasharray: 4 3;
	}
	/* the hexagon a dragged gate snaps to lights up */
	.footprint path.snap {
		fill: var(--primary-cyan, #00d1ff);
		fill-opacity: 0.45;
		stroke: var(--primary-cyan, #00d1ff);
		stroke-opacity: 1;
		stroke-width: 1.5;
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
	/* a place without a diorama: its cells, dashed (owner 2026-10-12) */
	.place-hit.bare path {
		fill: rgb(255 255 255 / 0.2);
		stroke: var(--ink);
		stroke-opacity: 0.2;
		stroke-width: 1;
	}
	.place-hit.bare path.edge {
		stroke-dasharray: 3 3;
	}
	/* the Bézier handles (owner 2026-10-12): a small square on a thin arm – hollow while computed, filled once set */
	.arms {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
	}
	.arms line {
		stroke: var(--ink);
		stroke-width: 1;
		stroke-dasharray: 3 2;
	}
	.arms line.set {
		stroke-dasharray: none;
	}
	.handle.curve {
		left: -0.4rem;
		top: -0.4rem;
		width: 0.8rem;
		height: 0.8rem;
		border-radius: 0;
		border-width: 0.125rem;
		background: var(--paper);
	}
	.handle.curve.set {
		background: var(--ink);
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
	/* the active handle – the only one a drag moves (the safeguard: a tap activates) */
	.handle.active {
		outline: var(--border-width-focus) solid var(--primary-cyan);
		outline-offset: var(--focus-offset);
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
	/* the selected route's places (owner 2026-10-12): the patch edge in the route's highlight */
	.route-patches {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
	}
	.route-patches path {
		fill: none;
		stroke: var(--primary-cyan, #00d1ff);
		stroke-width: 3;
		stroke-linecap: round;
	}
	/* a dark line under the region's colour: the outline reads on any terrain */
	.region-edge.under {
		stroke: var(--ink, #000);
		stroke-width: 5;
	}
	/* a place waypoint's number: a small ink chip at its entry gate */
	.handle.chip {
		left: -0.45rem;
		top: -0.45rem;
		width: 0.9rem;
		height: 0.9rem;
		border-radius: 0;
		border-width: 0.0625rem;
		background: var(--ink);
		color: var(--paper);
		font-size: 0.55rem;
	}
	/* a free point: a small dot */
	.handle.free {
		left: -0.3rem;
		top: -0.3rem;
		width: 0.6rem;
		height: 0.6rem;
		border-width: 0.125rem;
		background: var(--paper);
	}
	/* a route gate (BACKLOG 10.66 (4)): hollow while computed, filled once the author set it */
	.handle.gate {
		background: var(--paper);
	}
	.handle.gate.set {
		background: var(--ink);
		color: var(--paper);
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
	/* drawing a route: undo / done, bottom centre over the map */
	.draw-bar {
		position: absolute;
		left: 50%;
		bottom: 1rem;
		transform: translateX(-50%);
		z-index: 3;
		background: var(--paper);
	}
	/* the regions' covers (agents/MAP.md §8.10): a light wash each, the selected one stronger with its outline */
	.regions-layers {
		position: absolute;
		inset: 0;
		pointer-events: none;
	}
	.regions {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		overflow: visible;
		pointer-events: none;
		will-change: transform;
	}
	.region-fill {
		fill-opacity: 0.18;
		stroke: none;
	}
	.region-fill.selected {
		fill-opacity: 0.45;
	}
	.region-edge {
		fill: none;
		stroke-width: 2.5;
		vector-effect: non-scaling-stroke;
		stroke-linecap: round;
	}
</style>
