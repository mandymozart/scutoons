<script lang="ts">
	import { CloseIcon } from '@scutoons/ui';
	import { page } from '$app/state';
	import {
		isItemPublic,
		isPlaceNew,
		isPlaceVisible,
		placeImageBox,
		routeDrawnTo,
		visibleLabels,
		type LoadedSequenceItem,
		type ReaderMap,
		type ReaderPlace,
		type Toon
	} from '@scutoons/content';
	import { MapRoutes, MapViewport, mapGestures, type DrawnRoute } from '@scutoons/reader';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import { getWorldContext } from '$lib/contexts/world.svelte.ts';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';

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
		onclose
	}: { data: ReaderMap | null; failed: boolean; open: boolean; toon: string; onclose: () => void } = $props();

	const world = getWorldContext();
	const membership = getMembershipContext();
	const view = new MapViewport();
	const mapState = $derived(world?.map ?? { places: {}, routes: {}, positions: {} });

	const places = $derived((data?.locations ?? []).filter((p) => isPlaceVisible(p, mapState)));
	const visibleIds = $derived(new Set(places.map((p) => p.id)));
	const own = (p: ReaderPlace) => p.toons?.includes(toon);
	const routes = $derived<DrawnRoute[]>(
		(data?.routes ?? [])
			.map((r) => ({ id: r.id, waypoints: r.waypoints, style: r.style, to: routeDrawnTo(r, mapState, (id) => visibleIds.has(id)) }))
			.filter((r) => r.to > 0)
	);

	// ── the camera ───────────────────────────────────────────────────────────────────────────────────────────
	let placed = '';
	$effect(() => {
		if (!data || !view.width) return;
		view.worldWidth = data.width;
		view.worldHeight = data.height;
		const key = `${toon}:${data.view?.center.x}:${data.view?.center.y}`;
		if (placed === key) return;
		placed = key;
		recenter();
	});
	function recenter() {
		if (!data) return;
		// the toon's start view; a phone sees less of the world: never further out than the whole width
		const center = data.view?.center ?? { x: data.width / 2, y: data.height / 2 };
		view.minZoom = Math.min(1, Math.max(0.15, view.width / data.width));
		view.centerOn(center, data.view?.zoom ?? 1);
		// the start view is set per toon in the creator (owner 2026-10-01); only when it would show none of the places this
		// reader discovered in this toon, the map fits those instead (never zoomed in further than 3×)
		const mine = places.filter(own);
		if (mine.length && !mine.some((p) => view.onScreen(p, -24))) view.fit(mine, 56, 3);
	}

	// ── labels: priority new → this toon's → others; hidden where they'd overlap ───────────────────────────────────
	/** the root font size (0.75rem labels, 1rem markers: 14 / 16 / 24 px by the viewport, DESIGN.md "Type") */
	let rem = $state(16);
	$effect(() => {
		const read = () => (rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16);
		read();
		window.addEventListener('resize', read);
		return () => window.removeEventListener('resize', read);
	});
	const labelled = $derived.by(() => {
		const onScreen = places.map((p) => ({ p, s: view.toScreen(p), isNew: isPlaceNew(p.id, mapState) })).filter(({ s }) => s.x > -100 && s.y > -40 && s.x < view.width + 100 && s.y < view.height + 40);
		const items = onScreen.filter(({ p }) => own(p)).sort((a, b) => Number(b.isNew) - Number(a.isNew));
		const char = 0.75 * rem * 0.6;
		return visibleLabels(
			items.map(({ p, s }) => ({ id: p.id, x: s.x, y: s.y - 0.6 * rem, width: p.name.length * char + 6, height: 0.75 * rem + 4 })),
			onScreen.map(({ s }) => ({ x: s.x, y: s.y, size: rem }))
		);
	});

	// ── seen: new places that were on screen while the map was open lose their halo (and the badge goes) ─────────
	$effect(() => {
		if (!open || !world) return;
		const onScreen = places.filter((p) => isPlaceNew(p.id, mapState) && view.onScreen(p)).map((p) => p.id);
		if (!onScreen.length) return;
		const timer = setTimeout(() => world.seen(onScreen), 1500);
		return () => clearTimeout(timer);
	});

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
		if (!item || !isItemPublic(item, import.meta.env.DEV)) return null;
		if (!item.isPreview && !membership?.isValid) return null;
		return link.number;
	});
	const here = $derived.by(() => {
		const p = mapState.positions[toon];
		return !!card && !!p && 'location' in p && p.location === card.id;
	});

	function select(p: ReaderPlace) {
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
		if (e.key === 'Escape') {
			e.stopPropagation();
			if (selected) selected = null;
			else if (listOpen) listOpen = false;
			else onclose();
			return;
		}
		const target = e.target as HTMLElement | null;
		if (target?.closest('input, textarea, select')) return;
		if (e.key === '+' || e.key === '=') view.zoomAt(1.5);
		else if (e.key === '-') view.zoomAt(1 / 1.5);
		else if (e.key.startsWith('Arrow') && !target?.closest('button, a')) {
			const step = 80;
			view.panBy(e.key === 'ArrowLeft' ? step : e.key === 'ArrowRight' ? -step : 0, e.key === 'ArrowUp' ? step : e.key === 'ArrowDown' ? -step : 0);
		}
	}

	const layerVisible = (l: { minZoom?: number; maxZoom?: number }) => (l.minZoom === undefined || view.zoom >= l.minZoom) && (l.maxZoom === undefined || view.zoom <= l.maxZoom);
	const imageBox = (p: ReaderPlace) => placeImageBox(p.image!, Math.min(view.zoom, 3));
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
		{@attach mapGestures(view, { onTap: (e) => { if (!(e.target as HTMLElement)?.closest('button, a')) selected = null; } })}
		data-testid="map-viewport"
	>
		{#if data}
			<div class="world" style:transform={view.transform} style:width="{data.width}px" style:height="{data.height}px" data-testid="map-world">
				{#each data.layers as layer (layer.id)}
					{#if layerVisible(layer)}
						<img class="layer" src={asset(layer.image)} alt="" width={data.width} height={data.height} draggable="false" />
					{/if}
				{/each}
			</div>
			<MapRoutes {routes} places={data.locations} zoom={view.zoom} x={view.x} y={view.y} visible={(id) => visibleIds.has(id)} />
			<div class="places">
				{#each places as p (p.id)}
					{@const s = view.toScreen(p)}
					{#if view.onScreen(p, 160)}
						{@const isNew = isPlaceNew(p.id, mapState)}
						<div class="place" class:own={own(p)} class:new={isNew} class:active={selected === p.id} style:transform="translate({s.x}px, {s.y}px)">
							{#if p.image && own(p)}
								{@const box = imageBox(p)}
								<img class="place-image" src={asset(p.image.src)} alt="" loading="lazy" draggable="false" style:width="{box.width}px" style:left="{box.left}px" style:top="{box.top}px" />
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
					{/if}
				{/each}
			</div>
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
		<button class="tool round" onclick={() => view.zoomAt(1.5)} aria-label={m['map.zoom_in']()} data-testid="map-zoom-in"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="M8 3v10M3 8h10" /></svg></button>
		<button class="tool round" onclick={() => view.zoomAt(1 / 1.5)} aria-label={m['map.zoom_out']()} data-testid="map-zoom-out"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8h10" /></svg></button>
		<button class="tool round" onclick={recenter} aria-label={m['map.recenter']()} title={m['map.recenter']()}>◎</button>
		<button class="tool round" onclick={onclose} aria-label={m['map.close']()} data-testid="map-close"><CloseIcon size="1rem" /></button>
	</div>

	{#if listOpen}
		<section class="list" aria-label={m['map.places_title']()} data-testid="map-list">
			<h2>{m['map.places_title']()}</h2>
			{#if listed.length}
				<ul>
					{#each listed as p (p.id)}
						<li>
							<button onclick={() => { select(p); listOpen = false; view.centerOn(p, Math.max(view.zoom, 2)); }}>
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
			{#if card.image}<img class="card-image" src={asset(card.image.src)} alt="" />{/if}
			<h2>{card.name}</h2>
			{#if !own(card)}<p class="muted">{m['map.other_toon']()}</p>{/if}
			{#if here}<p>{m['map.you_are_here']({ name: toonTitle })}</p>{/if}
			{#if cardChapter !== null}
				<a class="read sc-control" href={localizeHref(`/${toon}/chapter/${cardChapter}`)} onclick={onclose} data-testid="map-card-chapter">
					{cardChapter === 0 ? m['map.read_prologue']() : m['map.read_chapter']({ number: cardChapter })}
				</a>
			{/if}
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
	.read {
		display: inline-flex;
		align-items: center;
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
