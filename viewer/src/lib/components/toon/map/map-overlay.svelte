<script lang="ts">
	import { CloseIcon, Icon } from '@scutoons/ui';
	import { page } from '$app/state';
	import { MAP_MAX_ZOOM, PLACE_SIZE_MAX, PLACE_ZOOM, fogPlaces, fogRoutePoints, growRect, pointIndex, hexCorners, placeCells, isPlaceNew, isPlaceVisible, placeImageBox, placeImageSrcset, placeView, routeDrawnTo, visibleLabels, type LoadedSequenceItem, type ReaderMap, type ReaderPlace, type Toon } from '@scutoons/content';
	import { MapRoutes, MapTerrain, MapViewport, mapGestures, type DrawnRoute } from '@scutoons/reader';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import { getWorldContext } from '$lib/contexts/world.svelte.ts';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import { getHomesContext } from '$lib/contexts/homes.svelte.ts';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { MAP_NETWORKS, RIDE_MS, RIDE_PRICES, WALK_MS, randomCarrier, ridePath, walkPath, walkPoints, type Point, type RideKind } from '@scutoons/content';
	import MapAvatar from './map-avatar.svelte';
	import RideActions from './ride-actions.svelte';
	import HomeActions from './home-actions.svelte';
	import { isReadable } from '$lib/draft-preview.svelte.ts';
	import { untrack } from 'svelte';
	import { MAP_FOCUS_EVENT, flightAt, loadMapView, openingView, readerHere, saveMapView, type StoredMapView } from './map-view.ts';

	/**
	 * The map overlay (agents/MAP.md M1, DESIGN.md): the SCUTOONS world with pan / pinch / wheel zoom (one transform for
	 * the terrain, routes and places stay crisp in screen space), labels hidden where they would overlap (zoom in to read
	 * them all – the phone overlap), a place card on click, and "Orte" – the discovered places as a list (the text
	 * alternative of the map; Tab also walks the places and pans to the focused one). Visible = not hidden, or unlocked
	 * (the world state); places of other toons are plain markers without a label (one universe).
	 */
	let {
		data,
		failed,
		open,
		toon,
		onclose,
		reveal = null,
		revealStays = false,
		onrevealed = () => {},
		onreplay
	}: {
		data: ReaderMap | null;
		failed: boolean;
		open: boolean;
		toon: string;
		onclose: () => void;
		/** a place just unlocked by the story (map-reveal.ts): fly to it, show it, then `onrevealed` (agents/MAP.md §8.5a) */
		reveal?: string | null;
		/** the map stays open after this reveal (it was open – a new place tapped in it): the camera stays at the place */
		revealStays?: boolean;
		/** `kept`: the reader touched the map during the reveal – it stays open, they close it themselves */
		onrevealed?: (kept: boolean) => void;
		/** a new (unseen) place tapped: play its reveal (owner 2026-10-11 – "der kleine Punkt") */
		onreplay?: (id: string) => void;
	} = $props();

	const world = getWorldContext();
	const membership = getMembershipContext();
	const view = new MapViewport();
	const mapState = $derived(world?.map ?? { places: {}, routes: {}, positions: {} });

	// ── the reader's own avatar: its home, where it stands, walking (research/CAVE.md D1, D2) ──────────────────
	const homes = getHomesContext();
	const inventory = getInventoryContext();
	$effect(() => {
		if (open) void homes?.load();
	});
	const homePlace = $derived(homes?.homeOf(inventory?.currentBody?.character)?.place);
	/** where the avatar stands: its last walk, else its home */
	const avatarAt = $derived(mapState.avatar && 'location' in mapState.avatar ? mapState.avatar.location : homePlace);
	/** the avatar walks between unlocked places and its home only – no spoilers */
	const walkable = (id: string) => !!mapState.places[id] || id === homePlace;
	let walking = $state<{ points: Point[]; ms: number; to: string; carrier?: string | null } | null>(null);
	const hasVr = $derived(!!inventory?.entries.some((e) => inventory.item(e.item_id)?.skills?.some((s) => s.type === 'enter_home')));

	// visible = not hidden, or unlocked – and the avatar's home is always there for it
	const places = $derived((data?.locations ?? []).filter((p) => isPlaceVisible(p, mapState) || p.id === homePlace));
	const visibleIds = $derived(new Set(places.map((p) => p.id)));
	const own = (p: ReaderPlace) => p.toons?.includes(toon);
	const routes = $derived<DrawnRoute[]>(
		(data?.routes ?? [])
			.map((r) => ({ id: r.id, waypoints: r.waypoints, style: r.style, network: r.network, to: routeDrawnTo(r, mapState, (id) => visibleIds.has(id)) }))
			.filter((r) => r.to > 0)
	);

	// ── the camera ───────────────────────────────────────────────────────────────────────────────────────────
	// Every opening decides where the map opens (map-view.ts, owner 2026-10-10): the first time centred on where the reader
	// is at the zoom level, then the view the reader left it at – unless a focus was sent or a new place was reached since.
	/** the toon whose opening view is applied (cleared on close: the next opening decides again) */
	let placed = '';
	$effect(() => {
		if (!data || !view.width) return;
		view.worldWidth = data.width;
		view.worldHeight = data.height;
		view.bounded = !!data.bounded;
		// a phone sees less of the world: never further out than the whole width
		view.minZoom = Math.min(1, Math.max(0.15, view.width / data.width));
		if (!open) {
			placed = '';
			return;
		}
		if (placed === toon) return;
		placed = toon;
		untrack(applyOpening);
	});
	const whereNow = () => readerHere(places, toon, mapState);
	function applyOpening() {
		if (!data) return;
		if (reveal) {
			// the reveal flies from where the reader's map was (their last view, else the start view); nothing is stored
			revealFrom = loadMapView(toon);
			if (revealFrom) view.centerOn({ x: revealFrom.x, y: revealFrom.y }, revealFrom.zoom);
			else startView();
			return;
		}
		const decision = openingView(loadMapView(toon), whereNow());
		if (decision.kind === 'start') startView();
		else {
			view.centerOn(decision.center, decision.zoom);
			saveMapView(toon, decision.store);
		}
	}
	function startView() {
		if (!data) return;
		// the toon's start view (set in the creator, owner 2026-10-01); only when it would show none of the places this reader
		// discovered in this toon, the map fits those instead (never zoomed in further than a place's zoom)
		view.centerOn(data.view?.center ?? { x: data.width / 2, y: data.height / 2 }, data.view?.zoom ?? 1);
		const mine = places.filter(own);
		if (mine.length && !mine.some((p) => view.onScreen(p, -24))) view.fit(mine, 56, PLACE_ZOOM);
	}
	/** ◎: back to where the reader is (the place's view: point + offset at its zoom, default 2), else the start view */
	function recenter() {
		const at = whereNow();
		if (at && data) view.centerOn(at, at.zoom);
		else startView();
	}
	/** "Orte": centre on a place (its view), zoomed in to at least its zoom */
	function showPlace(p: ReaderPlace) {
		const { center, zoom } = placeView(p);
		view.centerOn(center, Math.max(view.zoom, zoom));
	}
	// remember the reader's last view (centre + zoom) while the map is open – saved shortly after a move, and on close
	let unsaved: { toon: string; x: number; y: number; zoom: number } | null = null;
	let saveTimer: ReturnType<typeof setTimeout> | undefined;
	function flush() {
		clearTimeout(saveTimer);
		if (!unsaved) return;
		const { toon: forToon, ...at } = unsaved;
		unsaved = null;
		saveMapView(forToon, { ...at, here: forToon === toon ? (whereNow()?.id ?? null) : (loadMapView(forToon)?.here ?? null) });
	}
	$effect(() => {
		if (!open || !data || !view.width || placed !== toon || reveal || revealing) return;
		unsaved = { toon, ...view.toWorld({ x: view.width / 2, y: view.height / 2 }), zoom: view.zoom };
		clearTimeout(saveTimer);
		saveTimer = setTimeout(() => untrack(flush), 300);
	});
	$effect(() => {
		if (!open) untrack(flush);
	});
	$effect(() => () => untrack(flush));
	// an explicit focus (map-view.ts `focusMap`) while the map is open: show it now
	$effect(() => {
		const onFocus = (e: Event) => {
			if ((e as CustomEvent<{ toon: string }>).detail?.toon === toon && open && view.width) applyOpening();
		};
		window.addEventListener(MAP_FOCUS_EVENT, onFocus);
		return () => window.removeEventListener(MAP_FOCUS_EVENT, onFocus);
	});

	// ── the unlock reveal (agents/MAP.md §8.5a, owner 2026-10-11): fly from the reader's view to the new place's view,
	//    the image fades in with a slight wobble over a halo in the toon's colour, a short hold, close; the reader's
	//    previous view stays the stored one (with the new place as its `here`, so the next opening doesn't jump there).
	//    Esc skips to the end; reduced motion: no flight, no wobble. The reader touching the map (tap, drag, pinch, wheel)
	//    stops it where it is and the map stays open – they close it themselves (owner 2026-10-12: "dann wird die Karte
	//    nicht wieder weggefahren") ─────────────────────────────────────────────────────────────────────────────────
	let revealFrom: StoredMapView | null = null;
	let revealing = $state<string | null>(null);
	let revealShow = $state(false);
	let skip = false;
	/** the reader took the map over during the reveal */
	let kept = false;
	function takeOver() {
		if (revealing) kept = true;
	}
	const reducedMotion = () => globalThis.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
	const pause = (ms: number) =>
		new Promise<void>((resolve) => {
			const end = performance.now() + ms;
			const tick = () => (skip || kept || performance.now() >= end ? resolve() : requestAnimationFrame(tick));
			requestAnimationFrame(tick);
		});
	function fly(to: { x: number; y: number; zoom: number }, ms: number): Promise<void> {
		const c = view.toWorld({ x: view.width / 2, y: view.height / 2 });
		const from = { x: c.x, y: c.y, zoom: view.zoom };
		const start = performance.now();
		return new Promise((resolve) => {
			const step = () => {
				// taken over: the camera stays where the reader has it
				if (kept) return resolve();
				const t = skip ? 1 : (performance.now() - start) / ms;
				const at = flightAt(from, to, t);
				view.stir(250);
				view.centerOn({ x: at.x, y: at.y }, at.zoom);
				if (t >= 1) resolve();
				else requestAnimationFrame(step);
			};
			requestAnimationFrame(step);
		});
	}
	$effect(() => {
		if (!reveal || !open || !data || !view.width || placed !== toon || revealing) return;
		const id = reveal;
		untrack(() => runReveal(id));
	});
	async function runReveal(id: string) {
		const place = data?.locations.find((p) => p.id === id);
		revealing = id;
		skip = false;
		kept = false;
		if (place) {
			const target = placeView(place);
			const still = reducedMotion();
			// the panel slides in first
			await pause(still ? 150 : 450);
			if (still || skip) view.centerOn(target.center, target.zoom);
			else await fly({ ...target.center, zoom: target.zoom }, 1200);
			revealShow = true;
			await pause(still ? 1400 : 2000);
			world?.seen([id]);
		}
		// the map stays where it flew: the revealed place is the remembered view, the next opening starts there (owner
		// 2026-10-11: "not cool that it jumps back to the previous position – better it moves and stays at the new one")
		if (kept) {
			// the reader's own view is saved as they move it (the effect above, once the reveal is over)
		} else if (place) {
			const at = placeView(place);
			saveMapView(toon, { x: at.center.x, y: at.center.y, zoom: at.zoom, here: whereNow()?.id ?? id });
		} else if (revealFrom) saveMapView(toon, { x: revealFrom.x, y: revealFrom.y, zoom: revealFrom.zoom, here: whereNow()?.id ?? id });
		revealFrom = null;
		revealShow = false;
		revealing = null;
		// a map that stays open keeps its camera at the place; one that closes opens fresh next time
		if (!revealStays && !kept) placed = '';
		onrevealed(kept);
	}
	// closed by the reader meanwhile (✕): the reveal ends at once – the story never waits for a map that is gone
	$effect(() => {
		if (!open && revealing) skip = true;
	});
	const revealColor = $derived(data?.view?.color ?? 'var(--primary-cyan, #00d1ff)');

	// ── labels: priority new → this toon's → others; hidden where they'd overlap ───────────────────────────────────
	/** the root font size (0.75rem labels, 1rem markers: 14 / 16 / 24 px by the viewport, DESIGN.md "Type") */
	let rem = $state(16);
	$effect(() => {
		const read = () => (rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16);
		read();
		window.addEventListener('resize', read);
		return () => window.removeEventListener('resize', read);
	});
	// which labels show: worked out at rest only – during a pan / pinch / flight the set stays, only the positions move
	// (the overlap test is O(n·m); agents/MAP.md §8.5c)
	let labelled = $state<Set<string>>(new Set());
	$effect(() => {
		if (view.moving) return;
		labelled = new Set(labelsNow());
	});
	const labelsNow = () => {
		const onScreen = places.map((p) => ({ p, s: view.toScreen(p), isNew: isPlaceNew(p.id, mapState) })).filter(({ s }) => s.x > -100 && s.y > -40 && s.x < view.width + 100 && s.y < view.height + 40);
		const items = onScreen.filter(({ p }) => own(p)).sort((a, b) => Number(b.isNew) - Number(a.isNew));
		const char = 0.75 * rem * 0.6;
		return visibleLabels(
			items.map(({ p, s }) => ({ id: p.id, x: s.x, y: s.y - 0.6 * rem, width: p.name.length * char + 6, height: 0.75 * rem + 4 })),
			onScreen.map(({ s }) => ({ x: s.x, y: s.y, size: rem }))
		);
	};

	// ── seen: new places that were on screen while the map was open – a new place stands as its plaster model until then;
	//    it gets painted right there (wobble, pop – owner 2026-10-11), then it is seen (its halo and the badge go) ─────
	/** new places being painted in place (no flight) */
	let painting = $state<Set<string>>(new Set());
	const PAINT_MS = 2300;
	$effect(() => {
		if (!open || !world || revealing) return;
		const onScreen = places.filter((p) => isPlaceNew(p.id, mapState) && !painting.has(p.id) && view.onScreen(p)).map((p) => p.id);
		if (!onScreen.length) return;
		const timer = setTimeout(() => {
			painting = new Set([...painting, ...onScreen]);
			setTimeout(() => {
				world.seen(onScreen);
				painting = new Set([...painting].filter((id) => !onScreen.includes(id)));
			}, PAINT_MS);
		}, 1500);
		return () => clearTimeout(timer);
	});

	// ── locked places (owner 2026-10-11: "die Gips-Varianten schon vorher auf der Karte"): every place of this toon that is
	//    not unlocked yet stands as its plaster model – no name, no marker, no tap; unlocking paints it ──────────────────
	// ── fog of war (owner 2026-10-12, agents/MAP.md §8.9): a fog layer (the painted terrain) shows only around the places the
	//    reader unlocked – a new place's surroundings get their colour with its reveal or its painting in place ─────────
	//    and along a route between two such places in a row (owner 2026-10-12, `fogRoutePoints`) ─────────────────────
	const fogShownPlaces = $derived(
		fogPlaces(data?.locations ?? [], mapState)
			.filter((p) => !isPlaceNew(p.id, mapState) || (revealing === p.id && revealShow) || painting.has(p.id))
			.map((p) => ({ id: p.id, x: p.x, y: p.y }))
	);
	const fogPoints = $derived.by(() => {
		const shown = new Set(fogShownPlaces.map((p) => p.id));
		return [...fogShownPlaces, ...fogRoutePoints(data?.locations ?? [], data?.routes ?? [], (id) => shown.has(id))];
	});

	const lockedPlaces = $derived((data?.locations ?? []).filter((p) => !isPlaceVisible(p, mapState) && own(p) && !!p.image?.plaster));
	const lockedIndex = $derived(pointIndex(lockedPlaces));
	const shownLocked = $derived(lockedIndex.query(growRect(view.worldRect(160), PLACE_SIZE_MAX / 2)));

	// ── place card ───────────────────────────────────────────────────────────────────────────────────────────
	let selected = $state<string | null>(null);
	const card = $derived(places.find((p) => p.id === selected) ?? null);
	const sequence = $derived((page.data?.sequence ?? []) as LoadedSequenceItem[]);
	const toonTitle = $derived((page.data?.toon as Toon | undefined)?.title ?? toon);
	/** the chapter the card links: this toon's, listed and readable for this reader (never coming soon / locked) */
	const cardChapter = $derived.by(() => {
		const link = card?.chapter;
		if (!link || link.toon !== toon) return null;
		const item = sequence.find((s) => Number(s.number) === link.number);
		if (!item || !isReadable(item)) return null;
		if (!item.isPreview && !membership?.isValid) return null;
		return link.number;
	});
	const here = $derived.by(() => {
		const p = mapState.positions[toon];
		return !!card && !!p && 'location' in p && p.location === card.id;
	});

	const cardPath = $derived(card && avatarAt && data ? walkPath(data.routes ?? [], mapState, avatarAt, card.id, walkable) : null);
	function walkTo() {
		if (!card || !data || !cardPath?.length) return;
		walking = { points: walkPoints(data.locations, data.routes ?? [], cardPath), ms: WALK_MS * cardPath.length, to: card.id };
	}
	// rides (research/CAVE.md D8): the networks that reach the card's place from the avatar's stop
	// the creature airlift: a taxi from anywhere to any open place (unlocked or home) – no lines
	const rideOptions = $derived.by(() => {
		const out: { network: RideKind; steps: NonNullable<ReturnType<typeof ridePath>> | null }[] = [];
		if (!card || !avatarAt || !data || card.id === avatarAt) return out;
		for (const network of MAP_NETWORKS) {
			const steps = ridePath(data.routes ?? [], mapState, network, avatarAt, card.id, walkable);
			if (steps) out.push({ network, steps });
		}
		if (walkable(card.id)) out.push({ network: 'airlift', steps: null });
		return out;
	});
	/** the flight: an arc through the sky from where the avatar stands to the place */
	function flightPoints(from: Point, to: Point): Point[] {
		const lift = Math.hypot(to.x - from.x, to.y - from.y) * 0.35;
		return Array.from({ length: 25 }, (_, i) => {
			const t = i / 24;
			return { x: from.x + (to.x - from.x) * t, y: from.y + (to.y - from.y) * t - Math.sin(Math.PI * t) * lift };
		});
	}
	let rideNote = $state('');
	async function ride(network: RideKind) {
		const option = rideOptions.find((o) => o.network === network);
		if (!card || !data || !option || !inventory) return;
		const paid = await inventory.payFare(network, RIDE_PRICES[network]);
		if (!paid.ok) {
			rideNote = paid.error === 'cannot_afford' ? m['avatar_home.ride_cannot_afford']({ price: RIDE_PRICES[network] }) : m['avatar_home.err_failed']();
			return;
		}
		rideNote = '';
		if (!option.steps) {
			const from = data.locations.find((p) => p.id === avatarAt);
			if (!from) return;
			const carrier = randomCarrier(inventory.catalog?.airlift_carriers);
			walking = { points: flightPoints(from, card), ms: 2500, to: card.id, carrier: carrier?.image ? asset(carrier.image) : null };
			return;
		}
		walking = { points: walkPoints(data.locations, data.routes ?? [], option.steps), ms: RIDE_MS * option.steps.length, to: card.id };
	}
	function walked() {
		if (walking) world?.avatarTo({ location: walking.to });
		walking = null;
	}
	function enterHome(how: 'there' | 'vr') {
		onclose();
		void homes?.openOwn(how);
	}

	function select(p: ReaderPlace) {
		// a new place (its dot / halo): its reveal first – the card comes with the next tap
		if (onreplay && !revealing && isPlaceNew(p.id, mapState) && own(p)) return onreplay(p.id);
		selected = p.id;
		if (!view.onScreen(p, -40)) view.centerOn(p);
	}
	function focusPlace(p: ReaderPlace) {
		if (!view.onScreen(p, -24)) view.centerOn(p);
	}

	// ── the list (text alternative) ────────────────────────────────────────────────────────────────────────────
	let listOpen = $state(false);
	const listed = $derived([...places].sort((a, b) => a.name.localeCompare(b.name)));

	function onKeydown(e: KeyboardEvent) {
		if (!open) return;
		if (revealing) {
			if (e.key === 'Escape') (e.stopPropagation(), (skip = true));
			return;
		}
		if (e.key === 'Escape') {
			e.stopPropagation();
			if (selected) selected = null;
			else if (listOpen) listOpen = false;
			else onclose();
			return;
		}
		const target = e.target as HTMLElement | null;
		if (target?.closest('input, textarea, select')) return;
		if (e.key === '+' || e.key === '=') (view.stir(), view.zoomAt(1.5));
		else if (e.key === '-') (view.stir(), view.zoomAt(1 / 1.5));
		else if (e.key.startsWith('Arrow') && !target?.closest('button, a')) {
			const step = 80;
			view.panBy(e.key === 'ArrowLeft' ? step : e.key === 'ArrowRight' ? -step : 0, e.key === 'ArrowUp' ? step : e.key === 'ArrowDown' ? -step : 0);
		}
	}

	// a place image has a size in world units and scales with the map at every zoom (owner 2026-10-11); laid out once at the
	// largest zoom and scaled down by a transform – zooming then never lays out or resamples it per frame
	const IMAGE_ZOOM = MAP_MAX_ZOOM;
	const imageBox = (p: ReaderPlace) => placeImageBox(p.image!, IMAGE_ZOOM, 1, p.size);
	const imageScale = $derived(view.zoom / IMAGE_ZOOM);
	// culling (BACKLOG 10.63): only the places near the screen are in the DOM – a spatial index instead of a test per
	// place; the margin covers the biggest diorama
	const placeIndex = $derived(pointIndex(places));
	const shownPlaces = $derived(placeIndex.query(growRect(view.worldRect(160), PLACE_SIZE_MAX / 2)));
	/** a diorama's hit area: the hex cells its place takes (at IMAGE_ZOOM, around its point) – outside them a tap goes
	 *  through to what lies behind (owner 2026-10-11) */
	const hitPath = (p: ReaderPlace) =>
		placeCells(p)
			.map((c) => 'M' + hexCorners(c).map((k) => `${((k.x - p.x) * IMAGE_ZOOM).toFixed(1)},${((k.y - p.y) * IMAGE_ZOOM).toFixed(1)}`).join('L') + 'Z')
			.join('');
	/** the zoom the files are picked for: updated at rest only (no new downloads mid-gesture) */
	let restZoom = $state(1);
	$effect(() => {
		if (!view.moving) restZoom = view.zoom;
	});
	/** `sizes`: the width the image is drawn at – the browser picks 1x / 2x / 3x from it and the screen's density */
	const drawnSize = (p: ReaderPlace) => `${Math.ceil(placeImageBox(p.image!, restZoom, 1, p.size).width)}px`;
</script>

<svelte:window onkeydown={onKeydown} />

<div class="map {open ? 'visible' : 'hidden'}" style:--map-sea={data?.view?.color ?? null} role="dialog" aria-modal="true" aria-label={m['map.title']()} aria-hidden={!open} inert={!open} data-testid="map-overlay">
	<img class="frame frame--1" src={asset('catpope/vectors/vector-demon-saw-1.svg')} alt="" />
	<img class="frame frame--2" src={asset('catpope/vectors/vector-demon-saw-1.svg')} alt="" />
	<img class="frame frame--3" src={asset('catpope/vectors/vector-demon-saw-1.svg')} alt="" />
	<img class="frame frame--4" src={asset('catpope/vectors/vector-demon-saw-1.svg')} alt="" />

	<div
		class="viewport"
		class:moving={view.moving}
		{@attach mapGestures(view, { onTap: (e) => { if (!revealing && !(e.target as HTMLElement)?.closest('button, a')) selected = null; } })}
		onpointerdowncapture={takeOver}
		onwheelcapture={takeOver}
		data-testid="map-viewport"
	>
		{#if data}
			<MapTerrain {view} width={data.width} height={data.height} layers={data.layers} url={asset} fog={fogPoints} testid="map-world" />
			<MapRoutes {routes} places={data.locations} zoom={view.zoom} x={view.x} y={view.y} visible={(id) => visibleIds.has(id)} />
			<div class="places locked" aria-hidden="true">
				{#each shownLocked as p (p.id)}
					{@const s = view.toScreen(p)}
					{@const box = imageBox(p)}
					<div class="place" style:transform="translate({s.x}px, {s.y}px)">
						<span class="image-scale" style:transform="scale({imageScale})">
							<img class="place-image plaster still" src={asset(p.image!.plaster!)} srcset={placeImageSrcset(p.image!.plasterVariants, asset)} sizes={drawnSize(p)} alt="" loading="lazy" draggable="false" style:width="{box.width}px" style:left="{box.left}px" style:top="{box.top}px" />
						</span>
					</div>
				{/each}
			</div>
			<div class="places">
				{#each shownPlaces as p (p.id)}
					{@const s = view.toScreen(p)}
					{@const isNew = isPlaceNew(p.id, mapState)}
					{@const showing = (revealShow && revealing === p.id) || painting.has(p.id)}
					<!-- a new place: plaster until its reveal (the flight) or its painting in place -->
					{@const plasterOnly = isNew && !!p.image?.plaster && revealing !== p.id && !painting.has(p.id)}
					<div class="place" class:own={own(p)} class:new={isNew} class:active={selected === p.id} class:revealed={showing} style:--reveal-color={revealColor} style:transform="translate({s.x}px, {s.y}px)">
						{#if showing}<span class="halo" aria-hidden="true"></span>{/if}
						{#if p.image && own(p)}
							{@const box = imageBox(p)}
							<span class="image-scale" style:transform="scale({imageScale})">
							<!-- the reveal (§8.5a): the plaster model first, the painted diorama fades in over it -->
							{#if p.image.plaster && (revealing === p.id || painting.has(p.id) || plasterOnly)}
								<img class="place-image plaster" class:still={plasterOnly} src={asset(p.image.plaster)} srcset={placeImageSrcset(p.image.plasterVariants, asset)} sizes={drawnSize(p)} alt="" draggable="false" style:width="{box.width}px" style:left="{box.left}px" style:top="{box.top}px" />
							{/if}
							<img class="place-image" class:waiting={(revealing === p.id && !revealShow) || plasterOnly} src={asset(p.image.src)} srcset={placeImageSrcset(p.image.variants, asset)} sizes={drawnSize(p)} alt="" loading="lazy" draggable="false" style:width="{box.width}px" style:left="{box.left}px" style:top="{box.top}px" />
							<!-- the diorama opens its place's card too (owner 2026-10-11) – on the cells it takes, not its picture's box -->
							<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
							<svg class="place-hit" aria-hidden="true"><path d={hitPath(p)} onclick={() => select(p)} /></svg>
							</span>
						{/if}
						<button
							class="marker"
							data-control-exempt="map marker on the picture"
							aria-label={own(p) ? p.name : `${p.name} · ${m['map.other_toon']()}`}
							onclick={() => select(p)}
							onfocus={() => focusPlace(p)}
							data-testid="map-place-{p.id}"
						></button>
					</div>
				{/each}
			</div>
			{#if inventory?.currentBody}
				<MapAvatar {view} at={data.locations.find((p) => p.id === avatarAt) ?? null} walk={walking} image={asset(inventory.currentAvatar)} ondone={walked} />
			{/if}
			<!-- the routes' gates over the dioramas (BACKLOG 10.66 (4), owner 2026-10-12): where a route runs in and out -->
			<MapRoutes {routes} places={data.locations} zoom={view.zoom} x={view.x} y={view.y} visible={(id) => visibleIds.has(id)} layer="gates" />
			<div class="labels" aria-hidden="true">
				{#each places as p (p.id)}
					{#if labelled.has(p.id)}
						{@const s = view.toScreen(p)}
						<span class="label" style:transform="translate({s.x}px, {s.y}px)" data-testid="map-label-{p.id}">{p.name}</span>
					{/if}
				{/each}
			</div>
		{:else}
			<p class="status">{failed ? m['map.unavailable']() : m['map.loading']()}</p>
		{/if}
	</div>

	<div class="tools">
		<button class="tool" onclick={() => (listOpen = !listOpen)} aria-expanded={listOpen} data-testid="map-list-toggle">{m['map.places']({ count: places.length })}</button>
		<button class="tool round" onclick={recenter} aria-label={m['map.recenter']()} title={m['map.recenter']()} data-testid="map-recenter"><svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="8" cy="8" r="5.5" /><circle class="dot" cx="8" cy="8" r="1.75" /></svg></button>
		<button class="tool round" onclick={onclose} aria-label={m['map.close']()} data-testid="map-close"><CloseIcon size="1rem" /></button>
	</div>

	{#if listOpen}
		<section class="list" aria-label={m['map.places_title']()} data-testid="map-list">
			<h2>{m['map.places_title']()}</h2>
			{#if listed.length}
				<ul>
					{#each listed as p (p.id)}
						<li>
							<button onclick={() => { select(p); listOpen = false; showPlace(p); }}>
								{p.name}{#if isPlaceNew(p.id, mapState)}<span class="new-tag">{m['map.new_place']()}</span>{/if}
							</button>
						</li>
					{/each}
				</ul>
			{:else}
				<p>{m['map.places_empty']()}</p>
			{/if}
		</section>
	{/if}

	{#if card}
		<section class="card" aria-label={card.name} data-testid="map-card">
			<button class="card-close" onclick={() => (selected = null)} aria-label={m['map.close_card']()}><CloseIcon size="1rem" /></button>
			{#if card.image}<img class="card-image" src={asset(card.image.src)} srcset={placeImageSrcset(card.image.variants, asset)} sizes="6rem" alt="" />{/if}
			<h2>{card.name}</h2>
			{#if !own(card)}<p class="muted">{m['map.other_toon']()}</p>{/if}
			{#if here}<p>{m['map.you_are_here']({ name: toonTitle })}</p>{/if}
			{#if cardChapter !== null}
				<a class="read sc-control" href={localizeHref(`/${toon}/chapter/${cardChapter}`)} onclick={onclose} data-testid="map-card-chapter">
					{cardChapter === 0 ? m['map.read_prologue']() : m['map.read_chapter']({ number: cardChapter })}
					<Icon name="chevron-right" size="1rem" />
				</a>
			{/if}
			<RideActions networks={rideOptions.map((o) => o.network)} busy={!!walking} note={rideNote} onride={ride} />
			<HomeActions place={card.id} at={avatarAt} home={homePlace} reachable={!!cardPath?.length} walking={!!walking} vr={hasVr} onwalk={walkTo} onenter={enterHome} />
		</section>
	{/if}
</div>

<style>
	.map {
		position: fixed;
		inset: 0;
		z-index: 1000;
		/* the sea = the toon's colour (owner 2026-10-01: tile colour, else its star's; /library/map view.color); default cyan */
		background: var(--map-sea, #00ff00);
		opacity: 0;
		transform: translate(0, 100%) scale(1.1);
		transition:
			opacity 0.2s ease,
			transform 0.4s linear(0, 0.03 1.1%, 0.125 2.4%, 0.906 9.8%, 1.046 12.3%, 1.11 15%, 1.116 16.3%, 1.11 17.8%, 1.014 25.8%, 0.987 31.2%, 1.001 47.2%, 1);
		overflow: hidden;
	}
	.map.visible {
		opacity: 1;
		transform: translate(0, 0) scale(1);
	}
	/* hidden, its scaled box still reaches into the viewport: let clicks through */
	.map.hidden,
	.map.hidden * {
		pointer-events: none;
	}
	.frame {
		position: absolute;
		pointer-events: none;
		z-index: 2;
		filter: drop-shadow(0px 0px 13px var(--color-acid-purple));
		transition: transform 2s linear(0, 0.03 1.1%, 0.125 2.4%, 0.906 9.8%, 1.046 12.3%, 1.11 15%, 1.116 16.3%, 1.11 17.8%, 1.014 25.8%, 0.987 31.2%, 1.001 47.2%, 1);
	}
	.frame--1 {
		top: 0;
		right: 0;
		height: 105vw;
		transform: rotate(90deg);
		transform-origin: top;
	}
	.frame--2 {
		top: 0;
		right: -37px;
		height: 100vh;
		transform: rotateY(180deg);
	}
	.frame--3 {
		bottom: 0;
		right: 0;
		height: 105vw;
		transform: rotate(-90deg);
		transform-origin: bottom;
	}
	.frame--4 {
		top: 0;
		left: -33px;
		height: 100vh;
	}
	.visible .frame--1 {
		transform: rotate(90deg) translate(3px, -6px);
	}
	.visible .frame--2 {
		transform: translate(-13px, -8px);
	}
	.visible .frame--3 {
		transform: rotate(-90deg) translate(1px, 25px);
	}
	.visible .frame--4 {
		transform: translate(13px, 8px);
	}

	.viewport {
		position: absolute;
		inset: 0;
		overflow: hidden;
		touch-action: none;
		cursor: grab;
		user-select: none;
		-webkit-user-select: none;
	}
	.viewport.moving {
		cursor: grabbing;
	}
	.places {
		position: absolute;
		inset: 0;
		pointer-events: none;
	}
	.place {
		position: absolute;
		top: 0;
		left: 0;
		width: 0;
		height: 0;
	}
	.place-image {
		position: absolute;
		pointer-events: none;
		height: auto;
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
	/* scales the place's images around its point (the .place origin) */
	.image-scale {
		position: absolute;
		left: 0;
		top: 0;
		transform-origin: 0 0;
	}
	.marker {
		position: absolute;
		left: -0.5rem;
		top: -0.5rem;
		box-sizing: border-box;
		width: 1rem;
		height: 1rem;
		/* a dot on the map, not a control: the global button min-height (one control height) would stretch it into an
		   ellipse (owner 2026-10-02) control-ok */
		min-width: 0;
		min-height: 0;
		padding: 0;
		border-radius: 50%;
		border: 2px solid #000;
		background: var(--surface-tint-cyan, #f0fcff);
		pointer-events: auto;
		cursor: pointer;
		touch-action: manipulation;
	}
	/* other toons' places: small, no label */
	.place:not(.own) .marker {
		left: -0.3rem;
		top: -0.3rem;
		width: 0.6rem;
		height: 0.6rem;
		border-width: 1px;
	}
	.marker:hover,
	.marker:focus-visible,
	.active .marker {
		background: var(--primary-cyan, #00d1ff);
	}
	.marker:focus-visible {
		outline: 2px solid #000;
		outline-offset: 3px;
	}
	/* the unlock reveal (§8.5a): a halo in the toon's colour pulses behind; the image fades in with a slight wobble */
	.halo {
		position: absolute;
		left: -4rem;
		top: -4rem;
		width: 8rem;
		height: 8rem;
		border-radius: 50%;
		background: radial-gradient(circle, var(--reveal-color) 0%, color-mix(in srgb, var(--reveal-color) 45%, transparent) 40%, transparent 70%);
		pointer-events: none;
		animation: reveal-halo 1s ease-out 2;
	}
	.revealed .place-image:not(.plaster) {
		transform-origin: 50% 90%;
		animation: reveal-image 1.4s ease-out both;
	}
	/* during the flight the new place shows its plaster model (or nothing) – the painted diorama comes with the reveal */
	.place-image.waiting {
		opacity: 0;
	}
	/* with a plaster model: it stands there first, the painted one fades in a little later and covers it */
	.revealed .plaster ~ .place-image {
		animation-duration: 1.8s;
		animation-delay: 0.35s;
	}
	.revealed .marker {
		box-shadow:
			0 0 0 3px var(--reveal-color),
			0 0 0 5px #000;
	}
	@keyframes reveal-halo {
		from {
			transform: scale(0.3);
			opacity: 0.95;
		}
		to {
			transform: scale(1.6);
			opacity: 0;
		}
	}
	/* the paint pops: up past full size, a squash, then a wobble that settles (owner 2026-10-11: "wackeln und aufpoppen") */
	@keyframes reveal-image {
		0% {
			opacity: 0;
			transform: scale(0.7) rotate(0deg);
		}
		22% {
			opacity: 1;
			transform: scale(1.22, 1.14) rotate(-6deg);
		}
		36% {
			transform: scale(0.94, 1.04) rotate(5deg);
		}
		50% {
			transform: scale(1.06) rotate(-3.5deg);
		}
		64% {
			transform: scale(0.99) rotate(2deg);
		}
		80% {
			transform: scale(1.01) rotate(-1deg);
		}
		100% {
			opacity: 1;
			transform: scale(1) rotate(0deg);
		}
	}
	/* the plaster model shakes a little before the paint comes */
	.revealed .plaster {
		transform-origin: 50% 90%;
		animation: plaster-shake 0.45s ease-in-out both;
	}
	@keyframes plaster-shake {
		0%,
		100% {
			transform: rotate(0deg);
		}
		25% {
			transform: rotate(-3deg) scale(1.02);
		}
		50% {
			transform: rotate(3deg) scale(1.03);
		}
		75% {
			transform: rotate(-2deg) scale(1.02);
		}
	}
	/* a plaster model that just stands (locked, or new and not revealed yet): no animation, no taps */
	.plaster.still {
		animation: none;
	}
	.places.locked,
	.places.locked * {
		pointer-events: none;
	}
	@media (prefers-reduced-motion: reduce) {
		.halo {
			animation: none;
			opacity: 0.6;
		}
		.revealed .place-image:not(.plaster),
		.revealed .plaster {
			animation: none;
		}
	}
	/* new: the cyan halo until the place was on screen */
	.new .marker {
		/* cyan with an ink ring: readable on every toon colour (the sea) */
		box-shadow:
			0 0 0 3px var(--primary-cyan, #00d1ff),
			0 0 0 5px #000;
	}
	.labels {
		position: absolute;
		inset: 0;
		pointer-events: none;
	}
	.label {
		position: absolute;
		left: 0;
		top: 0;
		translate: -50% calc(-100% - 0.6rem);
		white-space: nowrap;
		font-size: 0.75rem;
		line-height: 1;
		pointer-events: none;
		/* a 2 px paper halo: ink text stays readable over land and over every toon colour (the sea) */
		text-shadow:
			-1px -1px 0 white,
			0 -1px 0 white,
			1px -1px 0 white,
			1px 0 0 white,
			1px 1px 0 white,
			0 1px 0 white,
			-1px 1px 0 white,
			-1px 0 0 white,
			-2px 0 0 white,
			2px 0 0 white,
			0 -2px 0 white,
			0 2px 0 white;
	}
	.status {
		position: absolute;
		top: 50%;
		left: 0;
		right: 0;
		text-align: center;
	}

	.tools {
		position: absolute;
		right: 0.75rem;
		bottom: 1rem;
		z-index: 3;
		display: flex;
		gap: 0.4rem;
		align-items: center;
	}
	.tool {
		font: inherit;
		font-size: 0.75rem;
		box-sizing: border-box;
		height: var(--control-height);
		padding: 0 0.75rem;
		background: #fff;
		color: #000;
		border: 2px solid #000;
		cursor: pointer;
		touch-action: manipulation;
	}
	.tool.round {
		display: inline-grid;
		place-items: center;
		width: var(--control-height);
		padding: 0;
		flex: none;
		border-radius: 50%;
		font-size: 1rem;
		line-height: 1;
	}
	/* drawn icons: exactly centred (the font's + and − sat off-centre) */
	.tool svg {
		display: block;
		width: 1rem;
		height: 1rem;
		fill: none;
		stroke: currentColor;
		stroke-width: 2;
		stroke-linecap: square;
	}
	/* ◎ recenter: drawn too (owner 2026-10-10: the font's ◎ shrank to a tiny dot on phones) */
	.tool svg .dot {
		fill: currentColor;
		stroke: none;
	}
	.tool:hover {
		background: var(--primary-cyan, #00d1ff);
	}

	.list,
	.card {
		color: #000;
		position: absolute;
		z-index: 4;
		background: #fff;
		border: 2px solid #000;
		box-sizing: border-box;
	}
	.list {
		right: 0.75rem;
		bottom: 3.75rem;
		width: min(18rem, calc(100vw - 1.5rem));
		max-height: 60vh;
		overflow: auto;
		padding: 0.75rem;
	}
	.list h2,
	.card h2 {
		font-size: 1rem;
		margin: 0 0 0.5rem;
	}
	.list ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.list li button {
		all: unset;
		box-sizing: border-box;
		color: #000;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		justify-content: space-between;
		width: 100%;
		min-height: var(--control-height);
		padding: 0 0.25rem;
		cursor: pointer;
		border-bottom: 1px solid var(--border-subtle, #ddd);
	}
	.list li button:hover,
	.list li button:focus-visible {
		background: var(--surface-hover, #f0f0f0);
	}
	.new-tag {
		font-size: 0.625rem;
		border: 1px solid #000;
		padding: 0 0.25rem;
		background: var(--primary-cyan, #00d1ff);
	}
	.card {
		top: 3rem;
		left: 50%;
		transform: translateX(-50%);
		width: min(20rem, calc(100vw - 1.5rem));
		padding: 0.75rem 2.5rem 0.75rem 0.75rem;
	}
	.card h2 {
		line-height: 1.2;
	}
	/* top right corner, level with the title (owner 2026-10-02): the control's centre on the title line's centre
	   (card padding 0.75rem + half the 1.2 line); the cross is the one CloseIcon, as high as the title's capitals */
	.card-close {
		color: #000;
		position: absolute;
		top: calc(0.75rem + 0.6rem - var(--control-height) / 2);
		right: 0;
		display: grid;
		place-items: center;
		box-sizing: border-box;
		width: var(--control-height);
		height: var(--control-height);
		padding: 0;
		border: 0;
		background: transparent;
		cursor: pointer;
		font: inherit;
	}
	.card-image {
		display: block;
		width: 6rem;
		height: auto;
		margin-bottom: 0.5rem;
	}
	.card p {
		margin: 0 0 0.5rem;
	}
	.muted {
		color: var(--text-muted, #6b6b6b);
		font-size: 0.75rem;
	}
	/* forward: the label, then the chevron (DESIGN.md "Direction arrows") */
	.read {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		box-sizing: border-box;
		min-height: var(--control-height);
		background: #000;
		color: #fff;
		padding: 0 1rem;
		text-decoration: none;
	}
	@media (prefers-reduced-motion: reduce) {
		.map,
		.frame {
			transition: opacity 0.2s ease;
		}
	}
</style>
