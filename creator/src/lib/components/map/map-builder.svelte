<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { page } from '$app/state';
	import { replaceState } from '$app/navigation';
	import {
		MAP_SOURCE_LANGUAGE,
		checkWorldMap,
		entities,
		isPlaceWaypoint,
		mapName,
		toonLanguages,
		toonTileColor,
		type CharacterRegistry,
		type ToonMapView,
		type Point,
		type Problem,
		type Waypoint,
		type WorldMap
	} from '@scutoons/content';
	import { MapViewport } from '@scutoons/reader';
	import { Icon } from '@scutoons/ui';
	import { getPublished, library } from '$lib/api.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { getCreatorNav, usePageNav } from '$lib/nav.svelte.ts';
	import { addPlace, addRoute, cdnHas, editMap, editPlace, editRoute, ensureMap, insertWaypoint, mapDoc, mapOf } from '$lib/editor/map.ts';
	import { toonIndexDoc } from '$lib/editor/languages.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import MetaPage from '../meta-page.svelte';
	import MapCanvas, { type MapSelection, type MapTool } from './map-canvas.svelte';
	import MapProps from './map-props.svelte';

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
		entryDoc.structural((entry) => {
			entry.map = { center: { x: Math.round(c.x), y: Math.round(c.y) }, ...(zoom !== 1 ? { zoom } : {}) };
		});
		if (!touchedEntries.includes(toonFilter)) touchedEntries = [...touchedEntries, toonFilter];
		phoneFrame = true;
	}
	function gotoStartView() {
		if (startView) view.centerOn(startView.center, startView.zoom);
	}
	const pageDocs = $derived<ContentDoc[]>([doc, ...touchedEntries.map((t) => entryDocs[t]).filter((d): d is ContentDoc => !!d)]);

	onMount(async () => {
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
			view.maxZoom = 8;
			const focus = selectionPoint();
			if (focus) view.centerOn(focus, 2);
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
		fill: true,
		extra: () => (selection ? [{ href: page.url.pathname + page.url.search, label: selectionName }] : [])
	});

	// ── edits from the canvas ────────────────────────────────────────────────────────────────────────────────
	const filterToons = () => (toonFilter ? [toonFilter] : []);
	function onAddPlace(at: Point) {
		const id = addPlace(doc, at, filterToons());
		tool = 'select';
		selection = { kind: 'place', id };
		// the name first (the id follows from it until the first publish)
		setTimeout(() => document.querySelector<HTMLInputElement>(`[data-testid="place-name-${MAP_SOURCE_LANGUAGE}"]`)?.focus(), 50);
	}
	function onMovePlace(id: string, at: Point) {
		editPlace(doc, id, (p) => {
			p.x = at.x;
			p.y = at.y;
		}, `drag:${id}`);
	}
	let sharedConfirm = { key: '', at: 0, ok: false };
	function onMoveWaypoint(route: string, index: number, at: Point) {
		const r = map?.routes.find((x) => x.id === route);
		const w = r?.waypoints[index];
		if (!w) return;
		if (isPlaceWaypoint(w)) {
			// a place waypoint moves the place (shared with other routes): ask once per drag
			const key = `${route}:${index}`;
			const now = Date.now();
			if (sharedConfirm.key !== key || now - sharedConfirm.at > 1500) {
				const place = map?.locations.find((p) => p.id === w.location);
				sharedConfirm = { key, at: now, ok: !!place && confirm(m['creator.map_move_shared_place']({ name: mapName(place.name, locale) || w.location })) };
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
		if (e.key === 'Escape' && selection) return void (selection = null);
		const tools: Record<string, MapTool> = { v: 'select', p: 'place', r: 'route' };
		if (tools[e.key.toLowerCase()] && !e.shiftKey) return void (tool = tools[e.key.toLowerCase()]);
		if (e.key.startsWith('Arrow') && selection?.kind === 'place') {
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
	const zoomPercent = $derived(Math.round(view.zoom * 100));
</script>

{#snippet mark(key: string)}
	{@const level = marks.get(key)}
	{#if level === 'error'}<span class="mark bad" title={problems.filter((p) => { const t = problemTarget(p); return t && `${t.kind}:${t.id}` === key; }).map((p) => p.message).join('\n')}>!</span>{:else if level === 'warning'}<span class="mark warn">!</span>{/if}
{/snippet}

{#snippet mapNav()}
	<div class="map-nav" data-testid="map-nav">
		<p class="head"><Icon name="layers" size="1.1em" /> {m['creator.map_layers']()}</p>
		<ul>
			{#each map?.layers ?? [] as l (l.id)}
				<li class="item-row">
					<input type="checkbox" checked={!hiddenLayers.has(l.id)} onchange={() => toggleLayer(l.id)} aria-label={m['creator.map_layer_show']()} title={m['creator.map_layer_show']()} />
					<button type="button" class="item" aria-current={selection?.kind === 'layer' && selection.id === l.id ? 'page' : undefined} onclick={() => select({ kind: 'layer', id: l.id })}>{l.id}{@render mark(`layer:${l.id}`)}</button>
				</li>
			{/each}
			<li><button type="button" class="item add" onclick={addLayer}>{m['creator.map_add_layer']()}</button></li>
		</ul>
		<p class="head">{m['creator.map_places']()} ({map?.locations.length ?? 0})</p>
		<ul data-testid="map-nav-places">
			{#each sortedPlaces as p (p.id)}
				<li>
					<button type="button" class="item" class:dim={!!toonFilter && !p.toons?.includes(toonFilter)} aria-current={selection?.kind === 'place' && selection.id === p.id ? 'page' : undefined} onclick={() => select({ kind: 'place', id: p.id }, true)} data-testid="nav-place-{p.id}">
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
					<button type="button" class="item" class:dim={!!toonFilter && !r.toons?.includes(toonFilter)} aria-current={selection?.kind === 'route' && selection.id === r.id ? 'page' : undefined} onclick={() => select({ kind: 'route', id: r.id }, true)} data-testid="nav-route-{r.id}">
						<span class="name">{mapName(r.name, locale) || r.id}</span>{@render mark(`route:${r.id}`)}
					</button>
				</li>
			{/each}
			<li><button type="button" class="item add" onclick={() => (tool = 'route')}>{m['creator.map_tool_route']()}</button></li>
		</ul>
	</div>
{/snippet}

<svelte:window onkeydown={onKeydown} />

<MetaPage toon="" title={m['creator.map']()} docs={pageDocs} validate={() => problems} wide fill help="map">
	{#if map}
		<div class="builder" data-testid="map-builder">
			<div class="toolbar" role="toolbar" aria-label={m['creator.map']()}>
				<div class="group" role="radiogroup">
					<!-- icons for the tools (DESIGN.md "Icons": Feather, label + shortcut in the tooltip) -->
					{#each [['select', m['creator.map_tool_select'](), 'V', 'mouse-pointer'], ['place', m['creator.map_tool_place'](), 'P', 'map-pin'], ['route', m['creator.map_tool_route'](), 'R', 'share-2']] as [id, label, key, icon] (id)}
						<button type="button" class="tool icon" class:on={tool === id} role="radio" aria-checked={tool === id} aria-label="{label} ({key})" title="{label} ({key})" onclick={() => (tool = id as MapTool)} data-testid="tool-{id}"><Icon name={icon} /></button>
					{/each}
				</div>
				<div class="group chips">
					<button type="button" class="chip" class:on={!toonFilter} onclick={() => (toonFilter = '')} data-testid="filter-all">{m['creator.map_all_toons']()}</button>
					{#each toons as t (t.id)}
						<button type="button" class="chip" class:on={toonFilter === t.id} onclick={() => (toonFilter = toonFilter === t.id ? '' : t.id)} data-testid="filter-{t.id}">{t.name}</button>
					{/each}
				</div>
				<div class="group">
					<label class="check"><input type="checkbox" bind:checked={readerView} data-testid="reader-view" /> {m['creator.map_reader_view']()}</label>
					<label class="check"><input type="checkbox" bind:checked={phoneFrame} data-testid="phone-frame" /> {m['creator.map_phone_frame']()}</label>
				</div>
				{#if toonFilter}
					<div class="group start" title={m['creator.map_start_view_hint']()} data-testid="start-view">
						<span class="hint">
							{startView
								? m['creator.map_start_view_value']({ toon: nav.toonName(toonFilter), x: Math.round(startView.center.x), y: Math.round(startView.center.y), zoom: Math.round(startView.zoom * 100) })
								: m['creator.map_start_view_none']({ toon: nav.toonName(toonFilter) })}
						</span>
						<button type="button" class="ghost" onclick={setStartView} disabled={!entryDoc} data-testid="start-view-set">{m['creator.map_start_view_set']()}</button>
						{#if startView}<button type="button" class="ghost" onclick={gotoStartView} data-testid="start-view-go">{m['creator.map_start_view_go']()}</button>{/if}
					</div>
				{/if}
				<div class="group zoom">
					<button type="button" class="ghost" onclick={() => view.zoomAt(1 / 1.25)} aria-label={m['creator.map_zoom_out']()} title={m['creator.map_zoom_out']()}><Icon name="zoom-out" /></button>
					<span class="pct" data-testid="zoom-pct">{zoomPercent} %</span>
					<button type="button" class="ghost" onclick={() => view.zoomAt(1.25)} aria-label={m['creator.map_zoom_in']()} title={m['creator.map_zoom_in']()}><Icon name="zoom-in" /></button>
					<button type="button" class="ghost" onclick={() => map && (map.locations.length ? view.fit(map.locations, 60, 3) : view.centerOn({ x: map.width / 2, y: map.height / 2 }, 0.5))} aria-label={m['creator.map_fit']()} title={m['creator.map_fit']()}><Icon name="maximize" /></button>
				</div>
			</div>
			<div class="stage">
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
					{hiddenLayers}
					{sea}
					onselect={(s) => select(s)}
					onaddplace={onAddPlace}
					onmoveplace={onMovePlace}
					onmovewaypoint={onMoveWaypoint}
					oninsertwaypoint={(route, after, at) => insertWaypoint(doc, route, after, at)}
					onaddroute={onAddRoute}
				/>
			</div>
			<aside class="side">
				{#if selection}
					<MapProps {doc} {map} {selection} {toons} {languages} {publishedIds} onselect={(s) => select(s, true)} />
				{:else}
					<MapProps {doc} {map} selection={null} {toons} {languages} {publishedIds} onselect={(s) => select(s, true)} />
					<section class="validation" data-testid="map-validation">
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
						{:else}
							<p class="hint">{m['creator.map_no_problems']()}</p>
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
		grid-template-columns: minmax(0, 1fr) 340px;
		grid-template-rows: auto minmax(0, 1fr);
		height: 100%;
		min-height: 24rem;
	}
	.toolbar {
		grid-column: 1 / -1;
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem 1rem;
		align-items: center;
		padding: 0.4rem 0.75rem;
		border-bottom: 1px solid var(--border-subtle);
	}
	.group {
		display: flex;
		gap: 0.25rem;
		align-items: center;
		flex-wrap: wrap;
	}
	.tool,
	.chip {
		font: inherit;
		box-sizing: border-box;
		min-height: var(--control-height);
		padding: 0 0.55rem;
		border: var(--border-width-regular) solid var(--border-muted);
		background: var(--paper);
		color: var(--ink);
		cursor: pointer;
	}
	.tool.icon {
		display: inline-flex;
		align-items: center;
		padding: 0 0.4rem;
	}
	.tool.on,
	.chip.on {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
	.check {
		display: flex;
		gap: 0.3rem;
		align-items: center;
	}
	.zoom .pct {
		min-width: 3.5rem;
		text-align: center;
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
			grid-template-rows: auto minmax(22rem, 1fr) auto;
		}
		.stage {
			border-right: 0;
			border-bottom: 2px solid var(--ink);
		}
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
	.map-nav .item-row {
		display: flex;
		align-items: center;
		padding-left: 0.6rem;
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
	.map-nav .item:hover {
		background: var(--surface-hover);
	}
	.map-nav .item[aria-current='page'] {
		border-left-color: var(--ink);
		background: var(--surface-hover);
		font-weight: var(--font-weight-regular);
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
