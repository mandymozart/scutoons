<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { page } from '$app/state';
	import { replaceState } from '$app/navigation';
	import {
		MAP_MAX_ZOOM,
		MAP_SOURCE_LANGUAGE,
		PLACE_ZOOM,
		zoomPercent,
		characterImage,
		checkWorldMap,
		entities,
		isPlaceWaypoint,
		mapName,
		placeImageFile,
		placeImagePath,
		parsePlaceImageName,
		PLACE_IMAGE_WIDTHS,
		placeView,
		snapPlace,
		placeSize,
		toonLanguages,
		toonTileColor,
		type CharacterRegistry,
		type MediaQuality,
		type ToonMapView,
		type Point,
		type Problem,
		type Waypoint,
		type WorldMap
	} from '@scutoons/content';
	import { MapViewport } from '@scutoons/reader';
	import { BarTabs, Icon, clickOutside, confirmDialog, liveRegion, region, type PhoneBarItem } from '@scutoons/ui';
	import { confirmDelete } from '$lib/editor/ask.ts';
	import { loadPlaceUnlocks } from '$lib/editor/toon-facts.ts';
	import { isKnownMissing } from '$lib/editor/characters.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import scutoonsAvatar from '$lib/icons/scutoons-avatar.svg';
	import { getPublished, library } from '$lib/api.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { getCreatorNav, usePageNav } from '$lib/nav.svelte.ts';
	import { addPlace, addRoute, cdnHas, editMap, editPlace, editRoute, ensureMap, insertWaypoint, mapDoc, mapOf, removePlace, removeRoute, routesThrough } from '$lib/editor/map.ts';
	import { toonIndexDoc } from '$lib/editor/languages.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { toWebp, uploadOrReplace } from '@scutoons/media';
	import MetaPage from '$lib/components/page/meta-page.svelte';
	import MapCanvas, { type MapSelection, type MapTool } from '$lib/components/map/map-canvas.svelte';
	import MapProps from '$lib/components/map/map-props.svelte';

	/**
	 * Universe → Karte (agents/MAP.md §4): the builder of the one world map (entity `map`). Sidebar = layers, places
	 * (A–Z, hidden ones marked), routes, each with its problems; canvas with the tools; properties of the selection (or the
	 * validation table when nothing is selected). Selection in the URL: `/map?location=cathedral`, `?route=season1`,
	 * `?layer=terrain`, `&toon=catpope` (the toon filter). Saved / published through the bottom bar like every entity.
	 */
	const doc = mapDoc();
	const nav = getCreatorNav();
	const view = new MapViewport();
	const locale = $derived(getLocale());
	const map = $derived(mapOf(doc));

	// ── import: place illustrations by file name (owner 2026-10-03) ──────────────────────────────────────────────
	let importInput = $state<HTMLInputElement>();
	let importing = $state<{ done: number; total: number } | null>(null);
	let importSummary = $state('');
	let importProblems = $state<string[]>([]);
	/** set after an import: previews of files with new bytes at the same path load again */
	let bust = $state(0);
	/**
	 * "Import images …": each file named after a place id becomes that place's image, read like the place files are named
	 * (`parsePlaceImageName`, BACKLOG 10.65): `old-church.png` the painted 1x, `old-church.2x.png` / `.3x` its 2x / 3x,
	 * `old-church-plaster.png` (+ `.2x` / `.3x`) the plaster model – case and extension don't matter. Each one WebP at its
	 * level's width (256 / 512 / 1024, PLACE_IMAGE_WIDTHS), at the place's current WebP path for that file or the
	 * convention path next to its painted 1x (`<toon>/map/places/<id>[-plaster][.2x].webp`); a file already there gets the
	 * new bytes (a new version, the CDN purged). One undo step; names that are no place, and a 2x / 3x / plaster model of
	 * a place without a painted 1x, are reported.
	 */
	async function importImages(event: Event) {
		const files = [...((event.currentTarget as HTMLInputElement).files ?? [])];
		if (importInput) importInput.value = '';
		if (!files.length || !map) return;
		importProblems = [];
		importSummary = '';
		const counts = { uploaded: 0, replaced: 0, unchanged: 0 };
		const placed: { id: string; plaster: boolean; level: MediaQuality; src: string }[] = [];
		// the painted 1x first: the other files sit next to it, and a place needs it before anything else
		const named = files
			.map((file) => ({ file, name: parsePlaceImageName(file.name) }))
			.sort((a, b) => Number(!!b.name && !b.name.plaster && b.name.level === 'low') - Number(!!a.name && !a.name.plaster && a.name.level === 'low'));
		const painted = new Map(map.locations.filter((p) => p.image?.src).map((p) => [p.id, p.image!.src]));
		importing = { done: 0, total: files.length };
		for (const { file, name } of named) {
			const place = name ? map.locations.find((p) => p.id.toLowerCase() === name.id) : undefined;
			try {
				if (!name || !place) throw new Error(m['creator.map_import_no_place']({ name: name?.id ?? file.name }));
				const first = name.level === 'low' && !name.plaster;
				const base = painted.get(place.id);
				if (!first && !base) throw new Error(m['creator.map_import_needs_painted']({ name: place.id }));
				const own = name.plaster ? (name.level === 'low' ? place.image?.plaster : place.image?.plasterVariants?.[name.level]) : first ? place.image?.src : place.image?.variants?.[name.level];
				const home = base?.endsWith('.webp') ? base : placeImagePath(place);
				const src = own?.endsWith('.webp') ? own : first ? home : placeImageFile(home, name.plaster, name.level);
				const bitmap = await createImageBitmap(file);
				let blob: Blob;
				try {
					const scale = Math.min(1, PLACE_IMAGE_WIDTHS[name.level] / Math.max(bitmap.width, bitmap.height));
					blob = (await toWebp(bitmap, Math.round(bitmap.width * scale))).blob;
				} finally {
					bitmap.close();
				}
				counts[await uploadOrReplace(blob, src, 'map import')]++;
				if (first) painted.set(place.id, src);
				placed.push({ id: place.id, plaster: name.plaster, level: name.level, src });
			} catch (e) {
				importProblems = [...importProblems, `${file.name}: ${e instanceof Error ? e.message : String(e)}`];
			}
			importing = { done: importing.done + 1, total: files.length };
		}
		if (placed.length)
			editMap(doc, (next) => {
				for (const { id, plaster, level, src } of placed) {
					const place = next.locations.find((p) => p.id === id);
					if (!place) continue;
					const image = (place.image ??= { src });
					if (!plaster && level === 'low') image.src = src;
					else if (!plaster) image.variants = { ...(image.variants ?? {}), low: image.src, [level]: src };
					else if (level === 'low') image.plaster = src;
					else image.plasterVariants = { ...(image.plasterVariants ?? {}), ...(image.plaster ? { low: image.plaster } : {}), [level]: src };
				}
			});
		importing = null;
		importSummary = m['creator.characters_import_summary'](counts);
		bust = Date.now();
	}

	let tool = $state<MapTool>('select');
	let selection = $state<MapSelection>(null);
	let toonFilter = $state('');
	let readerView = $state(false);
	let phoneFrame = $state(false);
	let hiddenLayers = $state(new Set<string>());
	let canvas = $state<ReturnType<typeof MapCanvas>>();
	let published = $state<WorldMap | null>(null);

	const toons = $derived(nav.toons.map((t) => ({ id: t.id, name: nav.toonName(t.id) })));
	const languages = $derived([...new Set([MAP_SOURCE_LANGUAGE, 'en', ...nav.toons.flatMap((t) => toonLanguages(t).languages as string[])])]);
	const publishedIds = $derived({
		places: new Set((published?.locations ?? []).map((p) => p.id)),
		routes: new Set((published?.routes ?? []).map((r) => r.id))
	});

	/** the reader view's sea = the filtered toon's colour (owner 2026-10-01: tile colour, else its star's); else the default neon green */
	let registry = $state<CharacterRegistry | null>(null);
	const NEUTRAL_SEA = '#00ff00';
	const sea = $derived.by(() => {
		if (!readerView || !toonFilter) return NEUTRAL_SEA;
		const entry = nav.toons.find((t) => t.id === toonFilter);
		return toonTileColor(registry, toonFilter, entry?.tile) ?? NEUTRAL_SEA;
	});

	/** the toon chip's mini avatar (owner 2026-10-11): the toon's star (the registry character whose own toon it is) */
	function toonAvatar(toon: string): { src: string; color: string } | null {
		const star = registry?.characters?.find((c) => c?.own_toon === toon);
		if (!star) return null;
		const src = characterImage(star, {}, 'avatar', { isMissing: isKnownMissing });
		return { src: src ? asset(src) : '', color: star.color ?? 'transparent' };
	}
	let toonMenu = $state(false);

	// ── the toon's start view (owner 2026-10-01): set here per toon, stored in the toon entry `toon:<toon>` (`map: { center,
	//    zoom }`, language-neutral); the entry is saved / published with the map once set here ─────────────────────────
	let entryDocs = $state<Record<string, ContentDoc>>({});
	let touchedEntries = $state<string[]>([]);
	$effect(() => {
		const t = toonFilter;
		if (!t || entryDocs[t]) return;
		const d = toonIndexDoc(t);
		d.load().then(() => (entryDocs = { ...entryDocs, [t]: d }));
	});
	const entryDoc = $derived(toonFilter ? entryDocs[toonFilter] : undefined);
	const startView = $derived.by(() => {
		const v = entryDoc?.data['_']?.map as ToonMapView | undefined;
		return v?.center ? { center: v.center, zoom: v.zoom ?? 1 } : null;
	});
	function setStartView() {
		if (!entryDoc?.present.length || !toonFilter) return;
		const c = view.toWorld({ x: view.width / 2, y: view.height / 2 });
		const zoom = Math.round(view.zoom * 100) / 100;
		entryDoc.edit((entry) => {
			entry.map = { center: { x: Math.round(c.x), y: Math.round(c.y) }, ...(zoom !== 1 ? { zoom } : {}) };
		});
		if (!touchedEntries.includes(toonFilter)) touchedEntries = [...touchedEntries, toonFilter];
		phoneFrame = true;
	}
	/** the selected place's reader view (zoom ≥ 200 %, centre + shift): the phone frame shows it */
	const placeFrame = $derived.by(() => {
		if (selection?.kind !== 'place' || !map) return null;
		const id = selection.id;
		const p = map.locations.find((l) => l.id === id);
		return p ? placeView(p) : null;
	});
	function gotoStartView() {
		if (startView) view.centerOn(startView.center, startView.zoom);
	}
	const pageDocs = $derived<ContentDoc[]>([doc, ...touchedEntries.map((t) => entryDocs[t]).filter((d): d is ContentDoc => !!d)]);

	/** place id → the documents whose unlockLocation events unlock it (all toons; null until / unless loaded) */
	let placeUnlocks = $state<Record<string, string[]> | null>(null);
	onMount(async () => {
		loadPlaceUnlocks().then((u) => (placeUnlocks = u));
		library.fetchRegistry({ draft: true }).then((r) => (registry = r)).catch(() => {});
		await doc.load();
		ensureMap(doc);
	});
	// the published map's ids are fixed (events reference them): reloaded after every publish
	$effect(() => {
		void doc.published['_'];
		getPublished<WorldMap>(entities.map)
			.then((r) => (published = r.content))
			.catch(() => (published = null));
	});

	// the first view: every place (the world when there are none)
	let framed = false;
	$effect(() => {
		const w = map;
		if (!w || !view.width || framed) return;
		framed = true;
		untrack(() => {
			view.worldWidth = w.width;
			view.worldHeight = w.height;
			view.minZoom = 0.25;
			view.maxZoom = MAP_MAX_ZOOM;
			const focus = selectionPoint();
			if (focus) view.centerOn(focus, PLACE_ZOOM);
			else if (w.locations.length) view.fit(w.locations, 80, 3);
			else view.centerOn({ x: w.width / 2, y: w.height / 2 }, Math.min(1, view.width / w.width));
		});
	});
	$effect(() => {
		if (map) {
			view.worldWidth = map.width;
			view.worldHeight = map.height;
		}
	});

	// ── selection ↔ URL ──────────────────────────────────────────────────────────────────────────────────────
	$effect(() => {
		const params = page.url.searchParams;
		const next: MapSelection = params.get('location')
			? { kind: 'place', id: params.get('location')! }
			: params.get('route')
				? { kind: 'route', id: params.get('route')! }
				: params.get('layer')
					? { kind: 'layer', id: params.get('layer')! }
					: null;
		const toon = params.get('toon') ?? '';
		untrack(() => {
			if (JSON.stringify(next) !== JSON.stringify(selection)) selection = next;
			if (toon !== toonFilter) toonFilter = toon;
		});
	});
	$effect(() => {
		const s = selection;
		const t = toonFilter;
		try {
			// the address bar, not page.url: replaceState (shallow routing) leaves page.url at the last navigation
			const url = new URL(location.href);
			for (const key of ['location', 'route', 'layer']) url.searchParams.delete(key);
			if (s) url.searchParams.set(s.kind === 'place' ? 'location' : s.kind, s.id);
			if (t) url.searchParams.set('toon', t);
			else url.searchParams.delete('toon');
			if (url.href !== location.href) replaceState(url, {});
		} catch {
			/* before the router is ready */
		}
	});
	function select(next: MapSelection, focus = false) {
		selection = next;
		if (focus) {
			const p = selectionPoint();
			if (p && !view.onScreen(p, -60)) view.centerOn(p);
		}
	}
	function selectionPoint(): Point | null {
		const s = selection;
		if (!s || !map) return null;
		if (s.kind === 'place') return map.locations.find((p) => p.id === s.id) ?? null;
		if (s.kind === 'route') {
			const r = map.routes.find((x) => x.id === s.id);
			const w = r?.waypoints.find((w) => !isPlaceWaypoint(w) || map.locations.some((p) => p.id === w.location));
			if (!w) return null;
			return isPlaceWaypoint(w) ? map.locations.find((p) => p.id === w.location)! : w;
		}
		return null;
	}
	const selectionName = $derived.by(() => {
		const s = selection;
		if (!s || !map) return '';
		if (s.kind === 'place') return mapName(map.locations.find((p) => p.id === s.id)?.name, locale) || s.id;
		if (s.kind === 'route') return mapName(map.routes.find((r) => r.id === s.id)?.name, locale) || s.id;
		return s.id;
	});
	usePageNav({
		sidebar: mapNav,
		crumbEnd: toonChip,
		fill: true,
		// the selection's own address (page.url lags behind: the selection is written with replaceState); the crumbs are the
		// way up – "Map" clears the selection (owner 2026-10-11, BACKLOG 10.68)
		extra: () => (selection ? [{ href: `${page.url.pathname}?${selection.kind === 'place' ? 'location' : selection.kind}=${encodeURIComponent(selection.id)}${toonFilter ? `&toon=${encodeURIComponent(toonFilter)}` : ''}`, label: selectionName }] : [])
	});

	// ── edits from the canvas ────────────────────────────────────────────────────────────────────────────────
	const filterToons = () => (toonFilter ? [toonFilter] : []);
	// places sit on the hex grid, one per cell (owner 2026-10-11, "wie in SimCity"): placing and dragging snap to the
	// nearest spot where the place's whole footprint is free
	function onAddPlace(at: Point) {
		const id = addPlace(doc, map ? snapPlace(map, at) : at, filterToons());
		tool = 'select';
		selection = { kind: 'place', id };
		// the name first (the id follows from it until the first publish)
		setTimeout(() => document.querySelector<HTMLInputElement>(`[data-testid="place-name-${MAP_SOURCE_LANGUAGE}"]`)?.focus(), 50);
	}
	function onMovePlace(id: string, at: Point) {
		const place = map?.locations.find((p) => p.id === id);
		const spot = map && place ? snapPlace(map, at, placeSize(place), id) : at;
		editPlace(doc, id, (p) => {
			p.x = spot.x;
			p.y = spot.y;
		}, `drag:${id}`);
	}
	let sharedConfirm = { key: '', at: 0, ok: false, asking: false, last: null as Point | null };
	function onMoveWaypoint(route: string, index: number, at: Point) {
		const r = map?.routes.find((x) => x.id === route);
		const w = r?.waypoints[index];
		if (!w) return;
		if (isPlaceWaypoint(w)) {
			// a place waypoint moves the place (shared with other routes): ask once per drag – in-page (owner 2026-10-03),
			// so the drag pauses while the question is open; "Move" puts the place where it was dragged to, and the same
			// waypoint then moves without asking for a moment (1.5 s since the last move)
			const key = `${route}:${index}`;
			const now = Date.now();
			sharedConfirm.last = at;
			if (sharedConfirm.asking) return;
			if (sharedConfirm.key !== key || now - sharedConfirm.at > 1500) {
				const place = map?.locations.find((p) => p.id === w.location);
				if (!place) return;
				sharedConfirm = { key, at: now, ok: false, asking: true, last: at };
				confirmDialog({ title: m['creator.move_place_title'](), message: m['creator.map_move_shared_place']({ name: mapName(place.name, locale) || w.location }), confirmLabel: m['creator.move_place_label']() }).then((ok) => {
					sharedConfirm = { ...sharedConfirm, ok, asking: false, at: Date.now() };
					if (ok && sharedConfirm.last) onMovePlace(w.location, sharedConfirm.last);
				});
				return;
			}
			sharedConfirm.at = now;
			if (sharedConfirm.ok) onMovePlace(w.location, at);
			return;
		}
		editRoute(doc, route, (r) => {
			const x = r.waypoints[index];
			if (x && !isPlaceWaypoint(x)) {
				x.x = at.x;
				x.y = at.y;
			}
		}, `drag:${route}:${index}`);
	}
	function onAddRoute(waypoints: Waypoint[]) {
		const id = addRoute(doc, waypoints, filterToons());
		tool = 'select';
		selection = { kind: 'route', id };
		setTimeout(() => document.querySelector<HTMLInputElement>(`[data-testid="route-name-${MAP_SOURCE_LANGUAGE}"]`)?.focus(), 50);
	}
	function addLayer() {
		let id = '';
		editMap(doc, (w) => {
			const taken = new Set(w.layers.map((l) => l.id));
			for (let n = 1; !id; n++) if (!taken.has(n === 1 ? 'layer' : `layer-${n}`)) id = n === 1 ? 'layer' : `layer-${n}`;
			w.layers.push({ id, image: '' });
		});
		selection = { kind: 'layer', id };
	}
	function toggleLayer(id: string) {
		const next = new Set(hiddenLayers);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		hiddenLayers = next;
	}

	// ── keyboard: tools, nudging, the route being drawn ──────────────────────────────────────────────────────
	function onKeydown(e: KeyboardEvent) {
		const target = e.target as HTMLElement | null;
		if (target?.closest('input, textarea, select, [contenteditable]') || e.ctrlKey || e.metaKey || e.altKey) return;
		if (tool === 'route' && canvas) {
			if (e.key === 'Enter') return void (e.preventDefault(), canvas.finishRoute());
			if (e.key === 'Escape') return void (e.preventDefault(), canvas.cancelRoute(), (tool = 'select'));
			if (e.key === 'Backspace' && canvas.undoDraftPoint()) return void e.preventDefault();
		}
		// Esc / arrows act on the selection only while the map region is live (DESIGN.md "Selection")
		if (e.key === 'Escape' && selection && liveRegion.is('map')) return void (selection = null);
		const tools: Record<string, MapTool> = { v: 'select', p: 'place', r: 'route' };
		if (tools[e.key.toLowerCase()] && !e.shiftKey) return void (tool = tools[e.key.toLowerCase()]);
		if (e.key.startsWith('Arrow') && selection?.kind === 'place' && liveRegion.is('map')) {
			e.preventDefault();
			const step = e.shiftKey ? 10 : 1;
			const dx = e.key === 'ArrowLeft' ? -step : e.key === 'ArrowRight' ? step : 0;
			const dy = e.key === 'ArrowUp' ? -step : e.key === 'ArrowDown' ? step : 0;
			const id = selection.id;
			editPlace(doc, id, (p) => {
				p.x = Math.max(0, Math.min(map?.width ?? Infinity, p.x + dx));
				p.y = Math.max(0, Math.min(map?.height ?? Infinity, p.y + dy));
			}, `nudge:${id}`);
		}
	}

	// ── validation (agents/MAP.md §4.7; the checks against story events come with M3/M4) ───────────────────────
	let missingImages = $state<Set<string>>(new Set());
	$effect(() => {
		const paths = [...new Set((map?.locations ?? []).map((p) => p.image?.src).filter((s): s is string => !!s))];
		Promise.all(paths.map(async (p) => ((await cdnHas(p)) ? null : p))).then((list) => (missingImages = new Set(list.filter((x): x is string => !!x))));
	});
	const problems = $derived.by<Problem[]>(() => {
		if (!map) return [];
		const out: Problem[] = checkWorldMap(map, { toons: nav.toons.length ? nav.toons.map((t) => t.id) : undefined, languages: ['de', 'en'] }).map((p) => ({
			...p,
			file: entities.map,
			locale: null
		}));
		map.locations.forEach((p, i) => {
			// a hidden place no unlockLocation event unlocks stays hidden for every reader (owner 2026-10-11)
			if (p.hidden && placeUnlocks && !placeUnlocks[p.id]?.length)
				out.push({ level: 'warning', file: entities.map, locale: null, path: ['locations', i, 'hidden'], message: m['creator.map_hidden_never_unlocked']({ name: mapName(p.name, locale) || p.id }) });
			if (p.image?.src && missingImages.has(p.image.src)) out.push({ level: 'warning', file: entities.map, locale: null, path: ['locations', i, 'image', 'src'], message: `place "${p.id}": the image is not on the CDN (${p.image.src})` });
		});
		const order = { error: 0, warning: 1, info: 2 };
		return out.sort((a, b) => order[a.level] - order[b.level]);
	});
	/** problems by the item they belong to (sidebar marks, table links) */
	function problemTarget(p: Problem): MapSelection {
		const [kind, index] = p.path as [string, number];
		if (!map || typeof index !== 'number') return null;
		if (kind === 'locations' && map.locations[index]) return { kind: 'place', id: map.locations[index].id };
		if (kind === 'routes' && map.routes[index]) return { kind: 'route', id: map.routes[index].id };
		if (kind === 'layers' && map.layers[index]) return { kind: 'layer', id: map.layers[index].id };
		return null;
	}
	const marks = $derived.by(() => {
		const out = new Map<string, 'error' | 'warning' | 'info'>();
		for (const p of problems) {
			const t = problemTarget(p);
			if (!t) continue;
			const key = `${t.kind}:${t.id}`;
			if (!out.has(key) || p.level === 'error') out.set(key, p.level);
		}
		return out;
	});
	const sortedPlaces = $derived([...(map?.locations ?? [])].sort((a, b) => (mapName(a.name, locale) || a.id).localeCompare(mapName(b.name, locale) || b.id)));
	const fitAll = () => map && (map.locations.length ? view.fit(map.locations, 60, 3) : view.centerOn({ x: map.width / 2, y: map.height / 2 }, 0.5));
	/** the crosshair on phones: the selected place's reader view, else the toon's start view */
	function gotoView() {
		if (placeFrame) view.centerOn(placeFrame.center, placeFrame.zoom);
		else gotoStartView();
	}

	// ── the phone sheet (UI-CONCEPTS E.10, owner 2026-10-11, variant C): the properties slide up from below, one tab at a
	//    time; closed it is its tab row only – a tap on a tab opens it there, on the open tab closes it; it closes
	//    while the map is being dragged ─────────────────────────────────────────────────────────────────────────
	type SheetTab = 'place' | 'image' | 'view' | 'map' | 'start';
	let sheetOpen = $state(false);
	let sheetTab = $state<SheetTab>('map');
	let mapDragging = $state(false);
	const sheetTabs = $derived.by<{ id: SheetTab; icon: string; label: string }[]>(() => {
		if (selection?.kind === 'place')
			return [
				{ id: 'place', icon: 'info', label: m['creator.map_sheet_place']() },
				{ id: 'image', icon: 'image', label: m['creator.map_image']() },
				{ id: 'view', icon: 'crosshair', label: m['creator.map_place_view']() }
			];
		if (selection) return [];
		return [
			{ id: 'map', icon: 'map', label: m['creator.map']() },
			// Problems: once, in the bar's More (the page's Problems panel) – not a sheet tab too (owner 2026-10-11)
			...(toonFilter ? [{ id: 'start' as const, icon: 'flag', label: m['creator.map_sheet_start']() }] : [])
		];
	});
	$effect(() => {
		const tabs = sheetTabs;
		untrack(() => {
			if (tabs.length && !tabs.some((t) => t.id === sheetTab)) sheetTab = tabs[0].id;
		});
	});
	function pickTab(id: SheetTab) {
		if (swallowTap) return;
		if (sheetTab === id && sheetOpen) sheetOpen = false;
		else {
			sheetTab = id;
			sheetOpen = true;
		}
	}
	function openSheet(id: SheetTab) {
		sheetTab = id;
		sheetOpen = true;
	}
	/** phones (< 768px): the sheet's Map tab lists the layers, places and routes (the drawer's lists) */
	let phone = $state(false);
	$effect(() => {
		const query = window.matchMedia('(max-width: 767px)');
		phone = query.matches;
		const change = (e: MediaQueryListEvent) => (phone = e.matches);
		query.addEventListener('change', change);
		return () => query.removeEventListener('change', change);
	});
	/**
	 * the tab row drags the sheet (owner 2026-10-11: "warum ist das Sheet nicht mehr klein ziehbar"): up = taller / open,
	 * down = lower; let go low = closed. A drag never counts as a tap on the tab under it.
	 */
	let sheetRem = $state<number | null>(null);
	let sheetEl = $state<HTMLElement>();
	let drag: { id: number; y: number; start: number; moved: boolean } | null = null;
	let swallowTap = false;
	let dragging = $state(false);
	const rootRem = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
	function headDown(e: PointerEvent) {
		drag = { id: e.pointerId, y: e.clientY, start: sheetOpen && sheetEl ? sheetEl.getBoundingClientRect().height / rootRem() : 2.75, moved: false };
	}
	function headMove(e: PointerEvent) {
		if (!drag || drag.id !== e.pointerId) return;
		const dy = e.clientY - drag.y;
		if (!drag.moved && Math.abs(dy) < 8) return;
		if (!drag.moved) try {
			(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
		} catch {
			/* a pointer that is gone */
		}
		drag.moved = true;
		dragging = true;
		sheetOpen = true;
		sheetRem = Math.max(2.75, Math.min((window.innerHeight * 0.8) / rootRem(), drag.start - dy / rootRem()));
	}
	function headUp(e: PointerEvent) {
		if (!drag || drag.id !== e.pointerId) return;
		const moved = drag.moved;
		drag = null;
		dragging = false;
		if (!moved) return;
		swallowTap = true;
		setTimeout(() => (swallowTap = false), 50);
		if ((sheetRem ?? 0) < 6) {
			sheetOpen = false;
			sheetRem = null;
		}
	}

	// ── the phone bar (UI-CONCEPTS E.10): More holds Import; a selection brings its own actions (like CapCut) ──
	const phoneMore = $derived<PhoneBarItem[]>([
		{ id: 'import', icon: 'upload', label: m['creator.characters_import'](), disabled: !!importing, run: () => importInput?.click() }
	]);
	/** the selected place's reader view = the canvas's view now (as the place form's "Use the current view") */
	function useViewFor(id: string) {
		const place = map?.locations.find((p) => p.id === id);
		if (!place) return;
		const c = view.toWorld({ x: view.width / 2, y: view.height / 2 });
		const zoom = Math.round(Math.min(view.maxZoom, Math.max(view.minZoom, view.zoom)) * 100) / 100;
		editPlace(doc, id, (p) => {
			const offset = { x: Math.round(c.x - place.x), y: Math.round(c.y - place.y) };
			p.view = { zoom, ...(offset.x || offset.y ? { offset } : {}) };
		});
	}
	async function deletePlace(id: string) {
		const place = map?.locations.find((p) => p.id === id);
		if (!place || routesThrough(map, id).length) return;
		const events = placeUnlocks?.[id]?.length ?? 0;
		const name = mapName(place.name, locale) || id;
		if (!(await confirmDelete(events ? m['creator.map_delete_confirm_events']({ name, count: events }) : m['creator.map_delete_confirm']({ name })))) return;
		removePlace(doc, id);
		selection = null;
	}
	async function deleteRoute(id: string) {
		const route = map?.routes.find((r) => r.id === id);
		if (!route || !(await confirmDelete(m['creator.map_delete_confirm']({ name: mapName(route.name, locale) || id })))) return;
		removeRoute(doc, id);
		selection = null;
	}
	const phoneSelection = $derived.by(() => {
		const s = selection;
		if (!s || !map) return null;
		const onback = () => (selection = null);
		const edit = (tab: SheetTab): PhoneBarItem => ({ id: 'edit', icon: 'edit-2', label: m['creator.bar_edit'](), active: sheetOpen, run: () => openSheet(tab) });
		if (s.kind === 'place') {
			const place = map.locations.find((p) => p.id === s.id);
			if (!place) return null;
			const used = routesThrough(map, s.id);
			return {
				label: selectionName,
				onback,
				items: [
					edit('place'),
					{ id: 'image', icon: 'image', label: m['creator.map_image'](), run: () => openSheet('image') },
					{ id: 'use-view', icon: 'crosshair', label: m['creator.bar_view'](), run: () => useViewFor(s.id), testid: 'phone-place-use-view' },
					{ id: 'show', icon: 'eye', label: m['creator.bar_show'](), run: () => placeFrame && view.centerOn(placeFrame.center, placeFrame.zoom) },
					{ id: 'hidden', icon: place.hidden ? 'eye-off' : 'eye', label: m['creator.bar_hidden'](), active: !!place.hidden, run: () => editPlace(doc, s.id, (p) => { if (p.hidden) delete p.hidden; else p.hidden = true; }) },
					{ id: 'delete', icon: 'trash-2', label: m['creator.bar_delete'](), disabled: used.length > 0, run: () => deletePlace(s.id) }
				]
			};
		}
		if (s.kind === 'route')
			return {
				label: selectionName,
				onback,
				items: [edit('place'), { id: 'delete', icon: 'trash-2', label: m['creator.bar_delete'](), run: () => deleteRoute(s.id) }]
			};
		return {
			label: selectionName,
			onback,
			items: [edit('place')]
		};
	});
</script>

{#snippet mark(key: string)}
	{@const level = marks.get(key)}
	{#if level === 'error'}<span class="mark bad" title={problems.filter((p) => { const t = problemTarget(p); return t && `${t.kind}:${t.id}` === key; }).map((p) => p.message).join('\n')}>!</span>{:else if level === 'warning'}<span class="mark warn">!</span>{/if}
{/snippet}

{#snippet mapNav()}
	<!-- the nav lists + the map = ONE selection region ('map', DESIGN.md "Selection"): the selected layer / place / route
	     is sc-selected here, the marker's halo on the map -->
	<div class="map-nav" data-testid="map-nav" use:region={'map'}>
		<p class="head"><Icon name="layers" size="1.1em" /> {m['creator.map_layers']()}</p>
		<ul>
			{#each map?.layers ?? [] as l (l.id)}
				<li class="item-row">
					<button type="button" class="item" class:sc-selected={selection?.kind === 'layer' && selection.id === l.id} onclick={() => select({ kind: 'layer', id: l.id })}>{l.id}{@render mark(`layer:${l.id}`)}</button>
					<!-- shown on the canvas: an eye, on / off (owner 2026-10-11), right of the name – only here, not in the layer's own view -->
					<button type="button" class="eye" aria-pressed={!hiddenLayers.has(l.id)} onclick={() => toggleLayer(l.id)} aria-label={m['creator.map_layer_show']()} title={m['creator.map_layer_show']()} data-testid="layer-eye-{l.id}"><Icon name={hiddenLayers.has(l.id) ? 'eye-off' : 'eye'} size="1.1em" /></button>
				</li>
			{/each}
			<li><button type="button" class="item add" onclick={addLayer}>{m['creator.map_add_layer']()}</button></li>
		</ul>
		<p class="head">{m['creator.map_places']()} ({map?.locations.length ?? 0})</p>
		<ul data-testid="map-nav-places">
			{#each sortedPlaces as p (p.id)}
				<li>
					<button type="button" class="item" class:dim={!!toonFilter && !p.toons?.includes(toonFilter)} class:sc-selected={selection?.kind === 'place' && selection.id === p.id} onclick={() => select({ kind: 'place', id: p.id }, true)} data-testid="nav-place-{p.id}">
						<span class="dot" class:hidden-dot={!!p.hidden} title={p.hidden ? m['creator.map_hidden']() : undefined}></span>
						<span class="name">{mapName(p.name, locale) || p.id}</span>{@render mark(`place:${p.id}`)}
					</button>
				</li>
			{/each}
			<li><button type="button" class="item add" onclick={() => (tool = 'place')} data-testid="nav-add-place">{m['creator.map_add_place']()}</button></li>
		</ul>
		<p class="head">{m['creator.map_routes']()} ({map?.routes.length ?? 0})</p>
		<ul data-testid="map-nav-routes">
			{#each map?.routes ?? [] as r (r.id)}
				<li>
					<button type="button" class="item" class:dim={!!toonFilter && !r.toons?.includes(toonFilter)} class:sc-selected={selection?.kind === 'route' && selection.id === r.id} onclick={() => select({ kind: 'route', id: r.id }, true)} data-testid="nav-route-{r.id}">
						<span class="name">{mapName(r.name, locale) || r.id}</span>{@render mark(`route:${r.id}`)}
					</button>
				</li>
			{/each}
			<li><button type="button" class="item add" onclick={() => (tool = 'route')}>{m['creator.map_tool_route']()}</button></li>
		</ul>
		<p class="nav-import">
			<button type="button" class="item add" disabled={!!importing} onclick={() => importInput?.click()} title={m['creator.map_import_hint']()}>
				<Icon name="upload" size="1.1em" />
				{importing ? m['creator.import_slices_progress']({ done: importing.done, total: importing.total }) : m['creator.characters_import']()}
			</button>
		</p>
	</div>
{/snippet}

{#snippet avatar(toon: string)}
	{@const a = toon ? toonAvatar(toon) : null}
	<!-- all toons = the SCUTOONS avatar (the viewer's account face; owner 2026-10-11: no globe – it reads as the language switch) -->
	<span class="mini" style:background={a?.color}>{#if a?.src}<img src={a.src} alt="" />{:else if !toon}<img src={scutoonsAvatar} alt="" />{/if}</span>
{/snippet}

{#snippet toonChip()}
	<!-- UI-CONCEPTS E.10 (owner 2026-10-11; desktop too since then): the toon filter at the end of the crumbs – a chip with the
	     toon's mini avatar (its star), a tap lists the toons -->
	<span class="toon-chip" {@attach clickOutside(() => (toonMenu = false))}>
		<button type="button" class="chip-button" aria-haspopup="listbox" aria-expanded={toonMenu} aria-label="{m['creator.map_toon_filter']()}: {toonFilter ? nav.toonName(toonFilter) : m['creator.map_all_toons']()}" title={toonFilter ? nav.toonName(toonFilter) : m['creator.map_all_toons']()} onclick={() => (toonMenu = !toonMenu)} data-testid="toon-filter">
			<!-- the pill: the avatar only (owner 2026-10-11); the name is in the list and the label -->
			{@render avatar(toonFilter)}<Icon name="chevron-down" size="1rem" />
		</button>
		{#if toonMenu}
			<ul class="toon-menu" role="listbox" aria-label={m['creator.map_toon_filter']()}>
				{#each [{ id: '', name: m['creator.map_all_toons']() }, ...toons] as t (t.id)}
					<li>
						<button type="button" role="option" aria-selected={toonFilter === t.id} class:on={toonFilter === t.id} onclick={() => ((toonFilter = t.id), (toonMenu = false))} data-testid="toon-{t.id || 'all'}">
							{@render avatar(t.id)}<span>{t.name}</span>
						</button>
					</li>
				{/each}
			</ul>
		{/if}
	</span>
{/snippet}

<svelte:window onkeydown={onKeydown} onpointerup={() => (mapDragging = false)} onpointercancel={() => (mapDragging = false)} />

<MetaPage toon="" title={m['creator.map']()} docs={pageDocs} validate={() => problems} wide fill help="map" {phoneMore} {phoneSelection}>
	{#if map}
		<div class="builder" data-testid="map-builder" use:region={{ name: 'map', initial: true }}>
			<!-- the tools are strips on the canvas at every width (owner 2026-10-11: "in Desktop platzeffizienter – Toons und
			     Tools wie auf Mobile"); the toon filter is the pill in the header, Import sits under the lists -->
			<input bind:this={importInput} type="file" accept="image/png,image/jpeg,image/webp" multiple hidden onchange={importImages} />
			{#if importSummary}<p class="sc-caption import-summary" role="status">{importSummary}</p>{/if}
			{#each importProblems as problem}<p class="error" role="alert">{problem}</p>{/each}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div class="stage" onpointerdown={(e) => (mapDragging = !(e.target as HTMLElement).closest('.strip'))}>
				<MapCanvas
					bind:this={canvas}
					{map}
					{view}
					bind:tool
					{selection}
					{toonFilter}
					{readerView}
					{phoneFrame}
					{startView}
					{placeFrame}
					{hiddenLayers}
					{sea}
					onselect={(s) => select(s)}
					onaddplace={onAddPlace}
					onmoveplace={onMovePlace}
					onmovewaypoint={onMoveWaypoint}
					onmovegate={(place, which, offset) => editPlace(doc, place, (p) => (p.gates = { ...(p.gates ?? {}), [which]: offset }), `gate:${place}:${which}`)}
					oninsertwaypoint={(route, after, at) => insertWaypoint(doc, route, after, at)}
					onaddroute={onAddRoute}
				/>
				<!-- UI-CONCEPTS E.10, on every width since 2026-10-11: tools left, fit + go to the view right (zoom: wheel / two fingers) -->
				<div class="strip tools" class:sheet-up={sheetOpen} role="toolbar" aria-label={m['creator.map']()}>
					{#each [['select', m['creator.map_tool_select'](), 'mouse-pointer', 'V'], ['place', m['creator.map_tool_place'](), 'map-pin', 'P'], ['route', m['creator.map_tool_route'](), 'share-2', 'R']] as [id, label, icon, key] (id)}
						<button type="button" class:on={tool === id} aria-pressed={tool === id} aria-label="{label} ({key})" title="{label} ({key})" onclick={() => (tool = id as MapTool)} data-testid="tool-{id}"><Icon name={icon} /></button>
					{/each}
					<span class="sep"></span>
					<button type="button" class:on={readerView} aria-pressed={readerView} aria-label={m['creator.map_reader_view']()} title={m['creator.map_reader_view']()} onclick={() => (readerView = !readerView)} data-testid="reader-view"><Icon name="eye" /></button>
					<button type="button" class:on={phoneFrame} aria-pressed={phoneFrame} aria-label={m['creator.map_phone_frame']()} title={m['creator.map_phone_frame']()} onclick={() => (phoneFrame = !phoneFrame)} data-testid="phone-frame"><Icon name="smartphone" /></button>
				</div>
				<div class="strip zooms" class:sheet-up={sheetOpen} role="group" aria-label={m['creator.field_map_zoom']()}>
					<button type="button" onclick={fitAll} aria-label={m['creator.map_fit']()} title={m['creator.map_fit']()}><Icon name="maximize" /></button>
					<button type="button" onclick={gotoView} disabled={!placeFrame && !startView} aria-label={m['creator.map_go_view']()} title={m['creator.map_go_view']()}><Icon name="crosshair" /></button>
				</div>
			</div>
			<aside class="side" class:open={sheetOpen && !mapDragging} class:dragging bind:this={sheetEl} style:--sheet-max={sheetRem !== null ? `${sheetRem}rem` : null} data-sheet-tab={sheetTabs.length ? sheetTab : undefined} data-testid="map-sheet">
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="phone-only sheet-head" onpointerdown={headDown} onpointermove={headMove} onpointerup={headUp} onpointercancel={() => ((drag = null), (dragging = false))}>
					{#if sheetTabs.length}
						<BarTabs label={m['creator.map']()} value={sheetOpen ? sheetTab : ''} tabs={sheetTabs} onchange={(id) => pickTab(id as SheetTab)} />
					{:else}
						<button type="button" class="sheet-toggle" aria-expanded={sheetOpen} onclick={() => !swallowTap && (sheetOpen = !sheetOpen)} data-testid="sheet-toggle"><Icon name={sheetOpen ? 'chevron-down' : 'chevron-up'} /> {selectionName}</button>
					{/if}
				</div>
				{#if selection}
					<MapProps {doc} {map} {selection} {view} {bust} {toons} {languages} {publishedIds} onselect={(s) => select(s, true)} />
				{:else}
					<div class="tab" data-tab="map">
						<MapProps {doc} {map} selection={null} {view} {toons} {languages} {publishedIds} onselect={(s) => select(s, true)} />
						<!-- phones: the Map tab = the lists (layers, places, routes) – a pick selects on the map -->
						{#if phone}<div class="sheet-lists">{@render mapNav()}</div>{/if}
					</div>
					{#if toonFilter}
						<section class="tab start-tab" data-tab="start" data-testid="start-view">
							<h3>{m['creator.map_sheet_start']()}</h3>
							<p class="hint">
								{startView
									? m['creator.map_start_view_value']({ toon: nav.toonName(toonFilter), x: Math.round(startView.center.x), y: Math.round(startView.center.y), zoom: zoomPercent(startView.zoom) })
									: m['creator.map_start_view_none']({ toon: nav.toonName(toonFilter) })}
							</p>
							<div class="start-row">
								<button type="button" class="secondary" onclick={setStartView} disabled={!entryDoc} data-testid="start-view-set">{m['creator.map_start_view_set']()}</button>
								{#if startView}<button type="button" class="ghost" onclick={gotoStartView} data-testid="start-view-go">{m['creator.map_start_view_go']()}</button>{/if}
							</div>
						</section>
					{/if}
					<section class="validation tab" data-tab="problems" data-testid="map-validation">
						<h3>{m['creator.map_validation']()} ({problems.length})</h3>
						{#if problems.length}
							<table>
								<thead><tr><th>{m['creator.map_level']()}</th><th>{m['creator.map_where']()}</th><th>{m['creator.map_message']()}</th></tr></thead>
								<tbody>
									{#each problems as p, i (i)}
										{@const t = problemTarget(p)}
										<tr class={p.level}>
											<td>{p.level}</td>
											<td>{#if t}<button type="button" class="link" onclick={() => select(t, true)}>{t.id}</button>{:else}{p.path.join('.')}{/if}</td>
											<td>{p.message}</td>
										</tr>
									{/each}
								</tbody>
							</table>
						{/if}
					</section>
				{/if}
			</aside>
		</div>
	{/if}
</MetaPage>

<style>
	.builder {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 16rem;
		grid-template-rows: minmax(0, 1fr);
		height: 100%;
		min-height: 24rem;
	}
	.stage {
		position: relative;
		min-height: 0;
		border-right: 2px solid var(--ink);
	}
	.side {
		min-height: 0;
		overflow: auto;
	}
	.validation {
		padding: 0 0.75rem 1rem;
	}
	.validation h3 {
		margin: 0.5rem 0;
	}
	table {
		width: 100%;
		border-collapse: collapse;
		font-size: var(--type-caption-size);
	}
	th,
	td {
		text-align: left;
		vertical-align: top;
		padding: 0.2rem 0.25rem;
		border-bottom: 1px solid var(--border-subtle);
	}
	tr.error td:first-child {
		color: var(--error);
		font-weight: var(--font-weight-regular);
	}
	tr.warning td:first-child {
		color: var(--warn);
	}
	.link {
		all: unset;
		cursor: pointer;
		text-decoration: underline;
	}
	.hint {
		font-size: var(--type-caption-size);
		color: var(--muted);
	}
	@media (max-width: 1100px) {
		.builder {
			grid-template-columns: minmax(0, 1fr);
			grid-template-rows: minmax(22rem, 1fr) auto;
		}
		.stage {
			border-right: 0;
			border-bottom: 2px solid var(--ink);
		}
	}

	/* the canvas strips (every width): square, grouped controls (DESIGN.md "Round vs square") */
	.strip {
		position: absolute;
		z-index: 2;
		flex-direction: column;
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
	}
	.strip.tools {
		top: 0.6rem;
		left: 0.6rem;
	}
	.strip.zooms {
		right: 0.6rem;
		bottom: 0.6rem;
	}
	.strip button {
		all: unset;
		box-sizing: border-box;
		display: flex;
		align-items: center;
		justify-content: center;
		width: var(--control-height);
		height: var(--control-height);
		color: var(--ink);
		cursor: pointer;
	}
	.strip button.on {
		background: var(--ink);
		color: var(--paper);
	}
	.strip button:disabled {
		opacity: 0.35;
		cursor: default;
	}
	.strip .sep {
		height: var(--border-width-regular);
		background: var(--ink);
	}
	.start-tab {
		padding: 0 0.75rem 1rem;
	}
	.start-row {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
	}
	.nav-import {
		margin: 0.5rem 0 0;
	}
	.import-summary {
		grid-column: 1 / -1;
		margin: 0;
		padding: 0.25rem 0.75rem;
	}
	/* ── phones (UI-CONCEPTS E.10): the canvas gets the screen, the properties are a sheet from below ── */
	.phone-only,
	.tab.phone-only {
		display: none;
	}
	.tab {
		display: contents;
	}
	@media (max-width: 767px) {
		.builder {
			grid-template-rows: minmax(0, 1fr) auto;
			min-height: 0;
		}
		.phone-only {
			display: flex;
		}
		.tab.phone-only {
			display: block;
		}
		.stage {
			border-bottom: 0;
		}
		.side {
			max-height: 2.75rem;
			overflow: hidden;
			border-top: 2px solid var(--ink);
			background: var(--paper);
			transition: max-height 0.2s ease;
		}
		.side.open {
			max-height: var(--sheet-max, 60dvh);
			overflow: auto;
		}
		.side.dragging {
			transition: none;
		}
		/* the floating strips step aside while the sheet is open */
		.strip.sheet-up {
			display: none;
		}
		.sheet-head {
			touch-action: none;
		}
		.sheet-head {
			position: sticky;
			top: 0;
			z-index: 1;
			flex-direction: column;
			background: var(--paper);
			border-bottom: 1px solid var(--border-subtle);
		}
		.sheet-toggle {
			all: unset;
			box-sizing: border-box;
			display: flex;
			align-items: center;
			gap: 0.4rem;
			width: 100%;
			height: 2.75rem;
			padding: 0 0.75rem;
			cursor: pointer;
		}
		/* closed: only the tab row shows */
		.side:not(.open) > :not(.sheet-head) {
			visibility: hidden;
		}
		/* one tab at a time: the groups carry data-tab (place-props.svelte; the map / start view / problems here) */
		.side[data-sheet-tab='place'] :global([data-tab]:not([data-tab='place'])),
		.side[data-sheet-tab='image'] :global([data-tab]:not([data-tab='image'])),
		.side[data-sheet-tab='view'] :global([data-tab]:not([data-tab='view'])),
		.side[data-sheet-tab='map'] :global([data-tab]:not([data-tab='map'])),
		.side[data-sheet-tab='start'] :global([data-tab]:not([data-tab='start'])) {
			display: none;
		}
	}

	/* the toon filter in the crumb row (phones; a single thing = round, DESIGN.md "Round vs square") */
	.toon-chip {
		position: relative;
		display: inline-flex;
	}
	.chip-button {
		all: unset;
		box-sizing: border-box;
		display: inline-flex;
		align-items: center;
		gap: 0.2rem;
		min-height: 2.25rem;
		padding: 0 0.4rem 0 0.15rem;
		border: var(--border-width-regular) solid var(--ink);
		border-radius: 999rem;
		background: var(--paper);
		color: var(--ink);
		cursor: pointer;
	}
	.mini {
		flex: none;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 1.75rem;
		height: 1.75rem;
		border-radius: 50%;
		overflow: hidden;
	}
	.mini img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.toon-menu {
		position: absolute;
		z-index: 50;
		top: calc(100% + 0.3rem);
		right: 0;
		min-width: 12rem;
		margin: 0;
		padding: 0;
		list-style: none;
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
	}
	.toon-menu button {
		all: unset;
		box-sizing: border-box;
		display: flex;
		align-items: center;
		gap: 0.6rem;
		width: 100%;
		min-height: 2.75rem;
		padding: 0 0.75rem;
		cursor: pointer;
	}
	.toon-menu button.on {
		background: var(--ink);
		color: var(--paper);
	}

	/* the sidebar (rendered into the shell's left column) */
	.map-nav .head {
		margin: 0;
		padding: 1rem 1rem 0.25rem; /* the sidebar's 1rem line (owner 2026-10-03) */
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		color: var(--muted);
	}
	.map-nav ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.map-nav .eye {
		all: unset;
		box-sizing: border-box;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: var(--control-height);
		height: var(--control-height);
		flex: none;
		cursor: pointer;
	}
	.map-nav .eye[aria-pressed='false'] {
		color: var(--muted);
	}
	.map-nav .eye:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-focus));
	}
	.map-nav .item-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding-right: 0.25rem;
	}
	.map-nav .item {
		all: unset;
		box-sizing: border-box;
		display: flex;
		align-items: center;
		gap: 0.4rem;
		width: 100%;
		min-height: var(--control-height);
		padding: 0 0.75rem;
		border-left: 3px solid transparent;
		cursor: pointer;
	}
	/* hover frames (never inverted); the selected one: tools.css .sc-selected */
	.map-nav .item:hover {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
	.map-nav .item.dim {
		opacity: 0.45;
	}
	.map-nav .add {
		color: var(--muted);
	}
	.map-nav .name {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.dot {
		flex: none;
		width: 8px;
		height: 8px;
		border-radius: 50%;
		border: 1.5px solid var(--ink);
		background: var(--surface-tint-cyan, #f0fcff);
	}
	.dot.hidden-dot {
		border-style: dashed;
		background: transparent;
	}
	.mark {
		margin-left: auto;
		font-weight: var(--font-weight-regular);
	}
	.mark.bad {
		color: var(--error);
	}
	.mark.warn {
		color: var(--warn);
	}
</style>
