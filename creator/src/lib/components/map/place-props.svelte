<script lang="ts">
	import { confirmDelete } from '$lib/editor/ask.ts';
	import { loadPlaceUnlocks } from '$lib/editor/toon-facts.ts';
	import { placeBrush } from '$lib/editor/place-brush.svelte.ts';
	import { Icon } from '@scutoons/ui';
	import { BRUSH_MAX, BRUSH_MIN, MAP_MAX_ZOOM, MAP_MIN_ZOOM, MAP_SOURCE_LANGUAGE, PLACE_SIZE_MAX, PLACE_SIZE_MIN, PLACE_SIZE_STEP, PLACE_ZOOM, zoomPercent, zoomFromPercent, placeSize, placeView, mapName, slugId, type LocaleText, type MapPlace, type WorldMap } from '@scutoons/content';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { autoId, editPlace, numberOrUnset, removePlace, renamePlace, routesThrough } from '$lib/editor/map.ts';
	import LocaleMapField from '$lib/components/fields/locale-map-field.svelte';
	import MapToonsChips from '$lib/components/map/map-toons-chips.svelte';
	import PlaceImage from '$lib/components/map/place-image.svelte';
	import type { MapSelection } from '$lib/components/map/map-canvas.svelte';
	import type { MapViewport } from '@scutoons/reader';

	/**
	 * The selected place (agents/MAP.md §4.3–4.5): id (editable until the first publish), names, position, toons, hidden
	 * (an unlockLocation event unlocks it – owner 2026-10-11), the
	 * card's chapter link, the reader map's view of it (zoom ≥ 200 % + a centre shift; "Use the current view" takes the
	 * canvas's), the image (PlaceImage), where it is used, delete.
	 */
	let {
		doc,
		map,
		place,
		toons,
		languages,
		locked,
		onselect,
		view,
		bust = 0
	}: {
		doc: ContentDoc;
		map: WorldMap;
		place: MapPlace;
		toons: { id: string; name: string }[];
		languages: string[];
		/** the id is in the published map (fixed: events reference it) */
		locked: boolean;
		onselect: (selection: MapSelection) => void;
		/** the builder's canvas view ("Use the current view", "Show") */
		view: MapViewport;
		/** set after an import: the image's preview loads again */
		bust?: number;
	} = $props();

	const locale = $derived(getLocale());

	function changeId(value: string) {
		onselect({ kind: 'place', id: renamePlace(doc, place.id, value) });
	}
	function setName(name: LocaleText) {
		const before = place.name?.[MAP_SOURCE_LANGUAGE];
		const id = place.id;
		editPlace(doc, id, (p) => (p.name = name), 'place-name');
		const next = name[MAP_SOURCE_LANGUAGE]?.trim();
		if (!locked && next && autoId(id, before, 'ort') && slugId(next) && slugId(next) !== id) onselect({ kind: 'place', id: renamePlace(doc, id, next, 'place-name') });
	}

	// ── the reader map's view of the place (owner 2026-10-10/11): zoom MAP_MIN_ZOOM..MAP_MAX_ZOOM (empty = PLACE_ZOOM), offset in world units ──
	// in the percentages people see: 100 % = the place zoom (`zoomPercent`)
	const MIN_PCT = zoomPercent(MAP_MIN_ZOOM);
	const MAX_PCT = zoomPercent(MAP_MAX_ZOOM);
	/** edit the place's view; an empty one leaves the place */
	function editView(change: (v: NonNullable<MapPlace['view']>) => void) {
		editPlace(doc, place.id, (p) => {
			const v = { ...(p.view ?? {}) };
			change(v);
			if (v.offset && !v.offset.x && !v.offset.y) delete v.offset;
			if (v.zoom === undefined && !v.offset) delete p.view;
			else p.view = v;
		});
	}
	function setViewZoom(value: string) {
		const pct = numberOrUnset(value);
		editView((v) => {
			if (pct === undefined || !Number.isFinite(pct)) delete v.zoom;
			else v.zoom = Math.min(MAP_MAX_ZOOM, Math.max(MAP_MIN_ZOOM, zoomFromPercent(pct)));
		});
	}
	function setViewOffset(axis: 'x' | 'y', value: string) {
		const n = numberOrUnset(value);
		editView((v) => {
			v.offset = { x: v.offset?.x ?? 0, y: v.offset?.y ?? 0, [axis]: n === undefined || !Number.isFinite(n) ? 0 : Math.round(n) };
		});
	}
	function useCurrentView() {
		const c = view.toWorld({ x: view.width / 2, y: view.height / 2 });
		const zoom = Math.round(Math.min(MAP_MAX_ZOOM, Math.max(MAP_MIN_ZOOM, view.zoom)) * 100) / 100;
		editView((v) => {
			v.zoom = zoom;
			v.offset = { x: Math.round(c.x - place.x), y: Math.round(c.y - place.y) };
		});
		showView();
	}
	function showView() {
		const { center, zoom } = placeView(place);
		view.centerOn(center, zoom);
	}

	const usedBy = $derived(routesThrough(map, place.id));
	async function remove() {
		if (usedBy.length) return;
		// events that unlock the place do nothing once it is gone (owner 2026-10-11) – said before it goes
		const events = (await loadPlaceUnlocks())?.[place.id]?.length ?? 0;
		const name = mapName(place.name, locale) || place.id;
		if (!(await confirmDelete(events ? m['creator.map_delete_confirm_events']({ name, count: events }) : m['creator.map_delete_confirm']({ name })))) return;
		removePlace(doc, place.id);
		onselect(null);
	}
</script>

<h2>{mapName(place.name, locale) || place.id}</h2>
<div class="tab" data-tab="place">
<label class="field">
	<span>{m['creator.map_id']()}</span>
	<input type="text" value={place.id} disabled={locked} title={locked ? m['creator.map_id_locked']() : undefined} onchange={(e) => changeId(e.currentTarget.value)} data-testid="place-id" />
</label>
<LocaleMapField value={place.name} {languages} label={m['creator.map_name']()} required testid="place-name" onchange={setName} />
<div class="row">
	<label class="field"><span>x</span><input type="number" value={place.x} onchange={(e) => editPlace(doc, place.id, (p) => (p.x = Math.round(Number(e.currentTarget.value))))} data-testid="place-x" /></label>
	<label class="field"><span>y</span><input type="number" value={place.y} onchange={(e) => editPlace(doc, place.id, (p) => (p.y = Math.round(Number(e.currentTarget.value))))} data-testid="place-y" /></label>
</div>

<MapToonsChips {toons} value={place.toons} onchange={(next) => editPlace(doc, place.id, (p) => (p.toons = next))} />
<!-- its size on the map (owner 2026-10-11, "wie in SimCity"): the diorama's width in world units – how many hex cells it takes -->
<label class="field"><span>{m['creator.map_footprint']()} · {placeSize(place)}</span>
	<input type="range" min={PLACE_SIZE_MIN} max={PLACE_SIZE_MAX} step={PLACE_SIZE_STEP} value={placeSize(place)} oninput={(e) => editPlace(doc, place.id, (p) => (p.size = Number(e.currentTarget.value)), `size:${place.id}`)} data-testid="place-size" />
</label>
<!-- the ground: painted cells (owner 2026-10-12: "a brush that adds or removes hexagons to the place's base") – the brush on the
     map, add / remove, its size; refused where another place stands; "computed" goes back to the size above -->
<fieldset class="ground" data-testid="place-ground">
	<legend>{m['creator.map_cells']()}</legend>
	<p class="hint">{place.cells?.length ? m['creator.map_cells_painted']({ count: place.cells.length }) : m['creator.map_cells_computed']()}</p>
	<div class="row">
		<button type="button" class:secondary={placeBrush.place !== place.id} aria-pressed={placeBrush.place === place.id} onclick={() => placeBrush.toggle(place.id)} data-testid="place-cells-paint"><Icon name="hexagon" /> {m['creator.map_cells_paint']()}</button>
		{#if placeBrush.place === place.id}
			<span class="sc-seg" role="group" aria-label={m['creator.map_cells']()}>
				<button type="button" class:on={placeBrush.mode === 'add'} aria-pressed={placeBrush.mode === 'add'} onclick={() => (placeBrush.mode = 'add')} data-testid="place-cells-add">{m['creator.map_cells_add']()}</button>
				<button type="button" class:on={placeBrush.mode === 'remove'} aria-pressed={placeBrush.mode === 'remove'} onclick={() => (placeBrush.mode = 'remove')} data-testid="place-cells-remove">{m['creator.map_cells_remove']()}</button>
			</span>
		{/if}
	</div>
	{#if placeBrush.place === place.id}
		<label class="field"><span>{m['creator.map_brush_size']()} · {placeBrush.size}</span>
			<input type="range" min={BRUSH_MIN} max={BRUSH_MAX} step="1" bind:value={placeBrush.size} data-testid="place-brush-size" />
		</label>
	{/if}
	{#if place.cells?.length}
		<button type="button" class="ghost" onclick={() => editPlace(doc, place.id, (p) => void delete p.cells)} data-testid="place-cells-reset">{m['creator.map_cells_reset']()}</button>
	{/if}
</fieldset>
</div>

<div class="tab" data-tab="place">
<label class="check"><input type="checkbox" checked={!!place.hidden} onchange={(e) => editPlace(doc, place.id, (p) => { if (e.currentTarget.checked) p.hidden = true; else delete p.hidden; })} data-testid="place-hidden" /> {m['creator.map_hidden']()}</label>
{#if place.hidden}<p class="hint">{m['creator.map_hidden_by_event']()}</p>{/if}
</div>

<div class="tab" data-tab="place">
<fieldset>
	<legend>{m['creator.map_chapter_link']()}</legend>
	<div class="row">
		<select value={place.chapter?.toon ?? ''} onchange={(e) => editPlace(doc, place.id, (p) => { const t = e.currentTarget.value; if (!t) delete p.chapter; else p.chapter = { toon: t, number: p.chapter?.number ?? 1 }; })} aria-label={m['creator.map_chapter_link']()}>
			<option value="">{m['creator.map_chapter_none']()}</option>
			{#each toons as t (t.id)}<option value={t.id}>{t.name}</option>{/each}
		</select>
		{#if place.chapter}
			<label class="field small"><span>{m['creator.map_chapter']()}</span><input type="number" min="0" value={place.chapter.number} onchange={(e) => editPlace(doc, place.id, (p) => p.chapter && (p.chapter.number = Math.max(0, Math.round(Number(e.currentTarget.value)))))} /></label>
		{/if}
	</div>
</fieldset>
</div>

<div class="tab" data-tab="place">
<!-- where routes run into and out of the diorama (BACKLOG 10.66 (4), owner 2026-10-12: "the computed ones first, then
     adjustable"): computed toward the neighbouring waypoints; dragging a gate's handle on the canvas sets it -->
<fieldset data-testid="place-gates">
	<legend>{m['creator.map_gates']()}</legend>
	<p class="hint">{m['creator.map_gates_hint']({ entry: place.gates?.entry ? m['creator.map_gate_set']() : m['creator.map_gate_auto'](), exit: place.gates?.exit ? m['creator.map_gate_set']() : m['creator.map_gate_auto']() })}</p>
	{#if place.gates}
		<button type="button" class="ghost" onclick={() => editPlace(doc, place.id, (p) => void delete p.gates)} data-testid="place-gates-reset">{m['creator.map_gates_reset']()}</button>
	{/if}
</fieldset>
</div>

<div class="tab" data-tab="view">
<fieldset data-testid="place-view">
	<legend>{m['creator.map_place_view']()}</legend>
	<p class="hint">{m['creator.map_place_view_hint']()}</p>
	<div class="row">
		<label class="field small"><span>{m['creator.map_place_view_zoom']()}</span><input type="number" min={MIN_PCT} max={MAX_PCT} step="5" placeholder={String(zoomPercent(PLACE_ZOOM))} value={place.view?.zoom !== undefined ? zoomPercent(place.view.zoom) : ''} onchange={(e) => setViewZoom(e.currentTarget.value)} data-testid="place-view-zoom" /></label>
		<label class="field small"><span>{m['creator.map_place_view_dx']()}</span><input type="number" placeholder="0" value={place.view?.offset?.x ?? ''} onchange={(e) => setViewOffset('x', e.currentTarget.value)} data-testid="place-view-dx" /></label>
		<label class="field small"><span>{m['creator.map_place_view_dy']()}</span><input type="number" placeholder="0" value={place.view?.offset?.y ?? ''} onchange={(e) => setViewOffset('y', e.currentTarget.value)} data-testid="place-view-dy" /></label>
	</div>
	<div class="row">
		<button type="button" class="ghost" onclick={useCurrentView} title={m['creator.map_place_view_use_hint']()} data-testid="place-view-use">{m['creator.map_place_view_use']()}</button>
		<button type="button" class="ghost" onclick={showView} data-testid="place-view-go">{m['creator.map_place_view_go']()}</button>
		{#if place.view}<button type="button" class="ghost" onclick={() => editView((v) => { delete v.zoom; delete v.offset; })} data-testid="place-view-clear">{m['creator.map_place_view_clear']()}</button>{/if}
	</div>
</fieldset>
</div>

<div class="tab" data-tab="image">
<PlaceImage {doc} {place} {bust} />
</div>

<div class="tab" data-tab="place">
<fieldset>
	<legend>{m['creator.map_used_in']()}</legend>
	{#if usedBy.length}<p>{m['creator.map_used_in_routes']({ routes: usedBy.join(', ') })}</p>{/if}
	<p class="hint">{m['creator.map_used_in_events']()}</p>
</fieldset>
<button type="button" class="danger" disabled={usedBy.length > 0} title={usedBy.length ? m['creator.map_delete_blocked']({ routes: usedBy.join(', ') }) : undefined} onclick={remove} data-testid="place-delete">{m['creator.map_delete_place']()}</button>
{#if usedBy.length}<p class="hint">{m['creator.map_delete_blocked']({ routes: usedBy.join(', ') })}</p>{/if}
</div>

<style>
	/* the phone sheet's tabs (map-builder.svelte, UI-CONCEPTS E.10) show one group; elsewhere the wrappers are invisible */
	.tab {
		display: contents;
	}
</style>
