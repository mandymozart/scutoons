<script lang="ts">
	import { confirmDelete } from '$lib/editor/ask.ts';
	import { MAP_MAX_ZOOM, MAP_MIN_ZOOM, MAP_SOURCE_LANGUAGE, PLACE_ZOOM, chapterSlices, placeView, mapName, sliceUnlockPoint, slugId, type ChapterSliceRef, type LocaleText, type MapPlace, type WorldMap } from '@scutoons/content';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { library } from '$lib/api.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { autoId, editPlace, numberOrUnset, removePlace, renamePlace, routesThrough } from '$lib/editor/map.ts';
	import LocaleMapField from '$lib/components/fields/locale-map-field.svelte';
	import MapToonsChips from '$lib/components/map/map-toons-chips.svelte';
	import PlaceImage from '$lib/components/map/place-image.svelte';
	import type { MapSelection } from '$lib/components/map/map-canvas.svelte';
	import type { MapViewport } from '@scutoons/reader';

	/**
	 * The selected place (agents/MAP.md §4.3–4.5): id (editable until the first publish), names, position, toons, hidden +
	 * its unlock point (toon, chapter, the unlocking slice – the same storage the chapter editor's slice edits), the
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

	/** "not unlockable yet" (owner 2026-10-01, the airport): `unlock: null`, a valid hidden state */
	const PENDING = '__pending';
	function setUnlockToon(value: string) {
		editPlace(doc, place.id, (p) => {
			if (!value) delete p.unlock;
			else if (value === PENDING) p.unlock = null;
			else p.unlock = { ...(p.unlock ?? { chapter: 1 }), toon: value };
		});
	}

	// ── the unlocking slice (the same storage the chapter editor's slice edits) ──
	let unlockSlices = $state<ChapterSliceRef[]>([]);
	let unlockGroups = $state<{ label?: string }[]>([]);
	let slicesLoaded = $state(false);
	const unlockKey = $derived(place.unlock ? `${place.unlock.toon}:${place.unlock.chapter}` : '');
	$effect(() => {
		const key = unlockKey;
		unlockSlices = [];
		slicesLoaded = false;
		if (!key) return;
		const [toon, chapter] = [key.slice(0, key.lastIndexOf(':')), Number(key.slice(key.lastIndexOf(':') + 1))];
		library
			.fetchChapter(toon, chapter, undefined, { draft: true })
			.then((c) => {
				if (unlockKey !== key) return;
				unlockGroups = (c.groups ?? []) as { label?: string }[];
				unlockSlices = chapterSlices(c.groups as never);
				slicesLoaded = true;
			})
			.catch(() => {
				if (unlockKey === key) slicesLoaded = true;
			});
	});
	const sliceLabel = (s: ChapterSliceRef) => `${s.overall + 1}. ${unlockGroups[s.group]?.label ?? `#${s.group + 1}`} · ${s.index + 1} · ${s.image.split('/').pop() || '–'}`;
	/** the chapter editor's address of a slice (`?group=<label>&slice=<n>`) */
	const sliceHref = (toon: string, chapter: number, s: ChapterSliceRef) =>
		`/${toon}/chapter/${chapter}?group=${encodeURIComponent(unlockGroups[s.group]?.label ?? String(s.group + 1))}&slice=${s.index + 1}`;
	function pickSlice(image: string) {
		if (!place.unlock) return;
		const at = image ? sliceUnlockPoint(unlockSlices, image) : null;
		editPlace(doc, place.id, (p) => {
			if (!p.unlock) return;
			if (!image) delete p.unlock.slice;
			else {
				p.unlock.slice = image;
				if (at !== null) p.unlock.at = at;
			}
		});
	}

	// ── the reader map's view of the place (owner 2026-10-10/11): zoom MAP_MIN_ZOOM..MAP_MAX_ZOOM (empty = PLACE_ZOOM), offset in world units ──
	const MIN_PCT = MAP_MIN_ZOOM * 100;
	const MAX_PCT = MAP_MAX_ZOOM * 100;
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
			else v.zoom = Math.round(Math.min(MAX_PCT, Math.max(MIN_PCT, pct))) / 100;
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
		if (!(await confirmDelete(m['creator.map_delete_confirm']({ name: mapName(place.name, locale) || place.id })))) return;
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
</div>

<div class="tab" data-tab="unlock">
<label class="check"><input type="checkbox" checked={!!place.hidden} onchange={(e) => editPlace(doc, place.id, (p) => { if (e.currentTarget.checked) p.hidden = true; else delete p.hidden; })} data-testid="place-hidden" /> {m['creator.map_hidden']()}</label>
{#if place.hidden}
	<fieldset>
		<legend>{m['creator.map_unlock']()}</legend>
		<p class="hint">{m['creator.map_unlock_hint']()}</p>
		<div class="row">
			<select value={place.unlock === null ? PENDING : (place.unlock?.toon ?? '')} onchange={(e) => setUnlockToon(e.currentTarget.value)} aria-label={m['creator.map_unlock']()} data-testid="place-unlock-toon">
				<option value="">{m['creator.map_unlock_none']()}</option>
				<option value={PENDING}>{m['creator.map_unlock_pending']()}</option>
				{#each toons as t (t.id)}<option value={t.id}>{t.name}</option>{/each}
			</select>
			{#if place.unlock}
				<label class="field small"><span>{m['creator.map_chapter']()}</span><input type="number" min="0" value={place.unlock.chapter} onchange={(e) => editPlace(doc, place.id, (p) => p.unlock && (p.unlock.chapter = Math.max(0, Math.round(Number(e.currentTarget.value)))))} /></label>
				<label class="field small"><span>{m['creator.map_unlock_at']()}</span><input type="number" min="0" max="100" value={Math.round((place.unlock.at ?? 0) * 100)} onchange={(e) => editPlace(doc, place.id, (p) => { if (!p.unlock) return; const v = Math.max(0, Math.min(100, Number(e.currentTarget.value))) / 100; if (v) p.unlock.at = v; else delete p.unlock.at; })} /></label>
				<label class="field small"><span>{m['creator.map_unlock_scene']()}</span><input type="number" min="0" value={place.unlock.scene ?? ''} onchange={(e) => editPlace(doc, place.id, (p) => { if (!p.unlock) return; const v = numberOrUnset(e.currentTarget.value); if (v === undefined) delete p.unlock.scene; else p.unlock.scene = Math.round(v); })} /></label>
			{/if}
		</div>
		{#if place.unlock}
			{@const unlock = place.unlock}
			<label class="field">
				<span>{m['creator.map_unlock_slice']()}</span>
				<select value={unlock.slice ?? ''} onchange={(e) => pickSlice(e.currentTarget.value)} data-testid="place-unlock-slice">
					<option value="">{m['creator.map_unlock_no_slice']()}</option>
					{#if unlock.slice && !unlockSlices.some((s) => s.image === unlock.slice)}<option value={unlock.slice}>{unlock.slice}</option>{/if}
					{#each unlockSlices as s (s.overall)}<option value={s.image}>{sliceLabel(s)}</option>{/each}
				</select>
			</label>
			{#if unlock.slice}
				{@const ref = unlockSlices.find((s) => s.image === unlock.slice)}
				{#if ref}
					<p class="hint" data-testid="place-unlock-where">
						{m['creator.map_unlock_in']({ chapter: unlock.chapter, slice: ref.overall + 1 })} · <a href={sliceHref(unlock.toon, unlock.chapter, ref)} data-testid="place-unlock-open">{m['creator.map_open_slice']()}</a>
					</p>
					{@const point = sliceUnlockPoint(unlockSlices, unlock.slice)}
					{#if point !== null && point !== (unlock.at ?? 0)}
						<p class="hint warn">{m['creator.map_unlock_stale']({ now: Math.round(point * 100), at: Math.round((unlock.at ?? 0) * 100) })} <button type="button" class="ghost" onclick={() => pickSlice(unlock.slice ?? '')}>{m['creator.map_unlock_update']()}</button></p>
					{/if}
				{:else if slicesLoaded}
					<p class="hint bad">{m['creator.map_unlock_missing']()}</p>
				{/if}
			{/if}
		{/if}
	</fieldset>
{/if}
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

<div class="tab" data-tab="view">
<fieldset data-testid="place-view">
	<legend>{m['creator.map_place_view']()}</legend>
	<p class="hint">{m['creator.map_place_view_hint']()}</p>
	<div class="row">
		<label class="field small"><span>{m['creator.map_place_view_zoom']()}</span><input type="number" min={MIN_PCT} max={MAX_PCT} step="10" placeholder={String(PLACE_ZOOM * 100)} value={place.view?.zoom !== undefined ? Math.round(place.view.zoom * 100) : ''} onchange={(e) => setViewZoom(e.currentTarget.value)} data-testid="place-view-zoom" /></label>
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
