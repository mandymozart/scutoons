<script lang="ts">
	import {
		DEFAULT_PLACE_ANCHOR,
		DEFAULT_PLACE_IMAGE_SIZE,
		MAP_SOURCE_LANGUAGE,
		slugId,
		chapterSlices,
		placeImagePath,
		sliceUnlockPoint,
		type ChapterSliceRef,
		type LocaleText,
		isPlaceWaypoint,
		mapName,
		type MapLayer,
		type MapPath,
		type MapPlace,
		type WorldMap
	} from '@scutoons/content';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import { library } from '$lib/api.ts';
	import { Icon } from '@scutoons/ui';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import {
		cdnFolder,
		cdnHas,
		cdnSize,
		editMap,
		editPlace,
		editRoute,
		removePlace,
		removeRoute,
		renamePlace,
		renameRoute,
		routesThrough
	} from '$lib/editor/map.ts';
	import LocaleMapField from '../credits/locale-map-field.svelte';
	import type { MapSelection } from './map-canvas.svelte';

	/**
	 * The builder's properties (agents/MAP.md §4.3–4.5): the selected place (id, names, position, image with size, ground
	 * point and previews, toons, hidden + its unlock point, the card's chapter link, used in, delete), route (id, names,
	 * style, toons, hidden, waypoints: reorder, rename free points, remove, reverse, delete) or layer.
	 */
	let {
		doc,
		map,
		selection,
		toons,
		languages,
		publishedIds,
		onselect
	}: {
		doc: ContentDoc;
		map: WorldMap;
		selection: MapSelection;
		toons: { id: string; name: string }[];
		languages: string[];
		/** ids in the published map (fixed: events reference them) */
		publishedIds: { places: Set<string>; routes: Set<string> };
		onselect: (selection: MapSelection) => void;
	} = $props();

	const locale = $derived(getLocale());
	const place = $derived(selection?.kind === 'place' ? map.locations.find((p) => p.id === selection.id) ?? null : null);
	const route = $derived(selection?.kind === 'route' ? map.routes.find((r) => r.id === selection.id) ?? null : null);
	const layerIndex = $derived(selection?.kind === 'layer' ? (map.layers ?? []).findIndex((l) => l.id === selection.id) : -1);
	const layer = $derived(layerIndex >= 0 ? map.layers[layerIndex] : null);
	const toonName = (id: string) => toons.find((t) => t.id === id)?.name ?? id;
	const num = (v: string) => (v.trim() === '' ? undefined : Number(v));

	// ── ids (editable until the first publish) ───────────────────────────────────────────────────────────────
	function changePlaceId(value: string) {
		if (!place) return;
		const id = renamePlace(doc, place.id, value);
		onselect({ kind: 'place', id });
	}
	function changeRouteId(value: string) {
		if (!route) return;
		const id = renameRoute(doc, route.id, value);
		onselect({ kind: 'route', id });
	}

	/**
	 * The id follows the source-language name until the first publish (agents/MAP.md §4.3), as long as it was never typed
	 * by hand (it still is the default `ort` / `weg` or the slug of the previous name)
	 */
	const autoId = (id: string, previous: string | undefined, fallback: string) =>
		[fallback, previous ? slugId(previous) : ''].some((base) => !!base && (id === base || (id.startsWith(`${base}-`) && /^[0-9]+$/.test(id.slice(base.length + 1)))));
	function setPlaceName(name: LocaleText, locked: boolean) {
		if (!place) return;
		const before = place.name?.[MAP_SOURCE_LANGUAGE];
		const id = place.id;
		editPlace(doc, id, (p) => (p.name = name), 'place-name');
		const next = name[MAP_SOURCE_LANGUAGE]?.trim();
		if (!locked && next && autoId(id, before, 'ort') && slugId(next) && slugId(next) !== id) onselect({ kind: 'place', id: renamePlace(doc, id, next, 'place-name') });
	}
	function setRouteName(name: LocaleText, locked: boolean) {
		if (!route) return;
		const before = route.name?.[MAP_SOURCE_LANGUAGE];
		const id = route.id;
		editRoute(doc, id, (r) => (r.name = name), 'route-name');
		const next = name[MAP_SOURCE_LANGUAGE]?.trim();
		if (!locked && next && autoId(id, before, 'weg') && slugId(next) && slugId(next) !== id) onselect({ kind: 'route', id: renameRoute(doc, id, next, 'route-name') });
	}

	/** "not unlockable yet" (owner 2026-10-01, the airport): `unlock: null`, a valid hidden state */
	const PENDING = '__pending';
	function setUnlockToon(value: string) {
		if (!place) return;
		editPlace(doc, place.id, (p) => {
			if (!value) delete p.unlock;
			else if (value === PENDING) p.unlock = null;
			else p.unlock = { ...(p.unlock ?? { chapter: 1 }), toon: value };
		});
	}

	// ── the unlocking slice (the same storage the chapter editor's slice edits) ───────────────────────────────────────
	let unlockSlices = $state<ChapterSliceRef[]>([]);
	let unlockGroups = $state<{ label?: string }[]>([]);
	let slicesLoaded = $state(false);
	const unlockKey = $derived(place?.unlock ? `${place.unlock.toon}:${place.unlock.chapter}` : '');
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
		if (!place?.unlock) return;
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

	// ── toons chips ──────────────────────────────────────────────────────────────────────────────────────────
	function toggleToon(list: string[] | undefined, id: string, set: (next: string[]) => void) {
		const current = list ?? [];
		set(current.includes(id) ? current.filter((t) => t !== id) : [...current, id]);
	}

	// ── image ────────────────────────────────────────────────────────────────────────────────────────────────
	/** the place illustrations' folder (owner 2026-10-01): `<toon>/images/map/places/` (`scutoons/…` for a shared place) */
	let folder = $state('');
	$effect(() => {
		if (place) folder = placeImagePath(place).replace(/[^/]+$/, '');
	});
	let folderFiles = $state<string[] | null>(null);
	let browsing = $state(false);
	async function browse() {
		browsing = !browsing;
		if (browsing) folderFiles = await cdnFolder(folder);
	}
	let natural = $state<{ w: number; h: number } | null>(null);
	let imageState = $state<{ missing: boolean; kb: number | null }>({ missing: false, kb: null });
	$effect(() => {
		const src = place?.image?.src;
		natural = null;
		imageState = { missing: false, kb: null };
		if (!src) return;
		cdnHas(src).then((ok) => {
			if (place?.image?.src === src) imageState = { ...imageState, missing: !ok };
		});
		cdnSize(src).then((bytes) => {
			if (place?.image?.src === src) imageState = { ...imageState, kb: bytes === null ? null : Math.round(bytes / 1024) };
		});
	});
	function setImage(src: string) {
		if (!place) return;
		editPlace(doc, place.id, (p) => {
			if (!src) delete p.image;
			else p.image = { ...(p.image ?? {}), src };
		});
	}
	function setImageField(field: 'size', value: number | undefined) {
		if (!place?.image) return;
		editPlace(doc, place.id, (p) => {
			if (!p.image) return;
			if (value === undefined || !Number.isFinite(value)) delete p.image[field];
			else p.image[field] = value;
		}, `image-${field}`);
	}
	let anchorBox = $state<HTMLDivElement>();
	let anchorDrag = false;
	function setAnchor(e: PointerEvent) {
		if (!place?.image || !anchorBox) return;
		const r = anchorBox.getBoundingClientRect();
		const x = Math.round(Math.max(0, Math.min(100, ((e.clientX - r.left) / r.width) * 100)));
		const y = Math.round(Math.max(0, Math.min(100, ((e.clientY - r.top) / r.height) * 100)));
		editPlace(doc, place.id, (p) => {
			if (!p.image) return;
			if (x === DEFAULT_PLACE_ANCHOR.x && y === DEFAULT_PLACE_ANCHOR.y) delete p.image.anchor;
			else p.image.anchor = { x, y };
		}, 'image-anchor');
	}
	const anchor = $derived(place?.image?.anchor ?? DEFAULT_PLACE_ANCHOR);
	const size = $derived(place?.image?.size ?? DEFAULT_PLACE_IMAGE_SIZE);
	/** blurry at 200 %: the file is narrower than twice its drawn width there */
	const blurry = $derived(!!natural && natural.w < size * 2);

	// ── delete ───────────────────────────────────────────────────────────────────────────────────────────────
	const usedBy = $derived(place ? routesThrough(map, place.id) : []);
	function deletePlace() {
		if (!place || usedBy.length) return;
		if (!confirm(m['creator.map_delete_confirm']({ name: mapName(place.name, locale) || place.id }))) return;
		removePlace(doc, place.id);
		onselect(null);
	}
	function deleteRoute() {
		if (!route) return;
		if (!confirm(m['creator.map_delete_confirm']({ name: mapName(route.name, locale) || route.id }))) return;
		removeRoute(doc, route.id);
		onselect(null);
	}
	function moveWaypoint(i: number, by: number) {
		if (!route) return;
		editRoute(doc, route.id, (r) => {
			const j = i + by;
			if (j < 0 || j >= r.waypoints.length) return;
			[r.waypoints[i], r.waypoints[j]] = [r.waypoints[j], r.waypoints[i]];
		});
	}
	function removeWaypoint(i: number) {
		if (!route) return;
		editRoute(doc, route.id, (r) => r.waypoints.splice(i, 1));
	}

	// ── layers ───────────────────────────────────────────────────────────────────────────────────────────────
	function editLayer(change: (l: MapLayer) => void, key?: string) {
		const i = layerIndex;
		editMap(doc, (map) => {
			if (map.layers[i]) change(map.layers[i]);
		}, key);
	}
	function deleteLayer() {
		if (!layer || !confirm(m['creator.map_delete_confirm']({ name: layer.id }))) return;
		const i = layerIndex;
		editMap(doc, (map) => map.layers.splice(i, 1));
		onselect(null);
	}
</script>

<div class="props" data-testid="map-props">
	{#if place}
		{@const locked = publishedIds.places.has(place.id)}
		<h2>{mapName(place.name, locale) || place.id}</h2>
		<label class="field">
			<span>{m['creator.map_id']()}</span>
			<input type="text" value={place.id} disabled={locked} title={locked ? m['creator.map_id_locked']() : undefined} onchange={(e) => changePlaceId(e.currentTarget.value)} data-testid="place-id" />
		</label>
		<LocaleMapField value={place.name} {languages} label={m['creator.map_name']()} required testid="place-name" onchange={(v) => setPlaceName(v, locked)} />
		<div class="row">
			<label class="field"><span>x</span><input type="number" value={place.x} onchange={(e) => editPlace(doc, place.id, (p) => (p.x = Math.round(Number(e.currentTarget.value))))} data-testid="place-x" /></label>
			<label class="field"><span>y</span><input type="number" value={place.y} onchange={(e) => editPlace(doc, place.id, (p) => (p.y = Math.round(Number(e.currentTarget.value))))} data-testid="place-y" /></label>
		</div>

		<fieldset>
			<legend>{m['creator.map_toons']()}</legend>
			<div class="chips">
				{#each toons as t (t.id)}
					<button type="button" class="chip" class:on={place.toons?.includes(t.id)} aria-pressed={place.toons?.includes(t.id)} onclick={() => toggleToon(place.toons, t.id, (next) => editPlace(doc, place.id, (p) => (p.toons = next)))}>{t.name}</button>
				{/each}
			</div>
		</fieldset>

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
						<label class="field small"><span>{m['creator.map_unlock_scene']()}</span><input type="number" min="0" value={place.unlock.scene ?? ''} onchange={(e) => editPlace(doc, place.id, (p) => { if (!p.unlock) return; const v = num(e.currentTarget.value); if (v === undefined) delete p.unlock.scene; else p.unlock.scene = Math.round(v); })} /></label>
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

		<fieldset data-testid="place-image">
			<legend>{m['creator.map_image']()}</legend>
			<input type="text" value={place.image?.src ?? ''} placeholder={placeImagePath(place)} onchange={(e) => setImage(e.currentTarget.value.trim())} aria-label={m['creator.map_image']()} data-testid="place-image-path" />
			<div class="row">
				<label class="field"><span>{m['creator.map_image_folder']()}</span><input type="text" bind:value={folder} /></label>
				<button type="button" class="ghost" onclick={browse} aria-expanded={browsing}>{folderFiles ? m['creator.map_image_browse']({ count: folderFiles.length }) : '…'}</button>
			</div>
			{#if browsing}
				{#if folderFiles?.length}
					<ul class="grid">
						{#each folderFiles as f (f)}
							<li><button type="button" class="thumb" data-control-exempt="image thumbnail card" class:current={f === place.image?.src} title={f} onclick={() => setImage(f)}><img src={asset(f)} alt="" loading="lazy" /><span>{f.split('/').pop()}</span></button></li>
						{/each}
					</ul>
				{:else}
					<p class="hint">{m['creator.map_image_none']()}</p>
				{/if}
			{/if}
			{#if place.image?.src}
				<p class="hint">
					{#if imageState.missing}<span class="bad">{m['creator.map_image_missing']()}</span>{/if}
					{#if natural}{natural.w} × {natural.h} px{/if}
					{#if imageState.kb !== null} · {imageState.kb} KB{/if}
					{#if imageState.kb !== null && imageState.kb > 60}<span class="warn"> · {m['creator.map_image_large']({ kb: imageState.kb })}</span>{/if}
					{#if blurry}<span class="warn"> · {m['creator.map_image_blurry']({ px: size * 2 })}</span>{/if}
				</p>
				<label class="field small"><span>{m['creator.map_image_size']()}</span><input type="number" min="8" value={place.image.size ?? ''} placeholder={String(DEFAULT_PLACE_IMAGE_SIZE)} onchange={(e) => setImageField('size', num(e.currentTarget.value))} data-testid="place-image-size" /></label>
				<span class="lbl">{m['creator.map_image_anchor']()} ({anchor.x} / {anchor.y} %)</span>
				<div
					class="anchor checker"
					bind:this={anchorBox}
					role="slider"
					tabindex="0"
					aria-label={m['creator.map_image_anchor']()}
					aria-valuenow={anchor.y}
					title={m['creator.map_image_anchor_hint']()}
					onpointerdown={(e) => { anchorDrag = true; (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId); setAnchor(e); }}
					onpointermove={(e) => anchorDrag && setAnchor(e)}
					onpointerup={() => (anchorDrag = false)}
					data-testid="place-anchor"
				>
					<img src={asset(place.image.src)} alt="" draggable="false" onload={(e) => { const img = e.currentTarget as HTMLImageElement; natural = { w: img.naturalWidth, h: img.naturalHeight }; }} />
					<span class="cross" style:left="{anchor.x}%" style:top="{anchor.y}%"></span>
				</div>
				<span class="lbl">{m['creator.map_image_previews']()}</span>
				<div class="previews">
					{#each [0.5, 1, 2] as z (z)}
						<div class="preview" style:height="{size * z * 1.1 + 12}px">
							<img src={asset(place.image.src)} alt="" style:width="{size * z}px" style:left="calc(50% - {size * z * (anchor.x / 100)}px)" style:bottom="{6 + size * z * ((100 - anchor.y) / 100) * (natural ? natural.h / natural.w : 1)}px" />
							<span class="dot"></span>
							<span class="z">{z * 100} %</span>
						</div>
					{/each}
				</div>
				<button type="button" class="ghost" onclick={() => setImage('')}>{m['creator.map_image_remove']()}</button>
			{/if}
		</fieldset>

		<fieldset>
			<legend>{m['creator.map_used_in']()}</legend>
			{#if usedBy.length}<p>{m['creator.map_used_in_routes']({ routes: usedBy.join(', ') })}</p>{/if}
			<p class="hint">{m['creator.map_used_in_events']()}</p>
		</fieldset>
		<button type="button" class="danger" disabled={usedBy.length > 0} title={usedBy.length ? m['creator.map_delete_blocked']({ routes: usedBy.join(', ') }) : undefined} onclick={deletePlace} data-testid="place-delete">{m['creator.map_delete_place']()}</button>
		{#if usedBy.length}<p class="hint">{m['creator.map_delete_blocked']({ routes: usedBy.join(', ') })}</p>{/if}
	{:else if route}
		{@const locked = publishedIds.routes.has(route.id)}
		<h2>{mapName(route.name, locale) || route.id}</h2>
		<label class="field">
			<span>{m['creator.map_id']()}</span>
			<input type="text" value={route.id} disabled={locked} title={locked ? m['creator.map_id_locked']() : undefined} onchange={(e) => changeRouteId(e.currentTarget.value)} data-testid="route-id" />
		</label>
		<LocaleMapField value={route.name} {languages} label={m['creator.map_name']()} required testid="route-name" onchange={(v) => setRouteName(v, locked)} />
		<fieldset>
			<legend>{m['creator.map_toons']()}</legend>
			<div class="chips">
				{#each toons as t (t.id)}
					<button type="button" class="chip" class:on={route.toons?.includes(t.id)} aria-pressed={route.toons?.includes(t.id)} onclick={() => toggleToon(route.toons, t.id, (next) => editRoute(doc, route.id, (r) => (r.toons = next)))}>{t.name}</button>
				{/each}
			</div>
		</fieldset>
		<label class="check"><input type="checkbox" checked={!!route.hidden} onchange={(e) => editRoute(doc, route.id, (r) => { if (e.currentTarget.checked) r.hidden = true; else delete r.hidden; })} /> {m['creator.map_hidden']()}</label>
		<fieldset>
			<legend>{m['creator.map_waypoints']()} ({route.waypoints.length})</legend>
			<p class="hint">{m['creator.map_insert_hint']()}</p>
			<ol class="waypoints" data-testid="route-waypoints">
				{#each route.waypoints as w, i (i)}
					<li>
						{#if isPlaceWaypoint(w)}
							{@const target = map.locations.find((p) => p.id === w.location)}
							<button type="button" class="link" class:bad={!target} onclick={() => target && onselect({ kind: 'place', id: target.id })}>{target ? mapName(target.name, locale) || target.id : `${w.location} · ${m['creator.map_waypoint_unknown']()}`}</button>
						{:else}
							<span class="free">
								<input type="text" value={w.id} aria-label={m['creator.map_id']()} onchange={(e) => editRoute(doc, route.id, (r) => { const x = r.waypoints[i]; if (!isPlaceWaypoint(x)) x.id = e.currentTarget.value.trim() || x.id; })} />
								<input type="text" value={mapName(w.name, locale)} placeholder={m['creator.map_free_name']()} aria-label={m['creator.map_free_name']()} onchange={(e) => editRoute(doc, route.id, (r) => { const x = r.waypoints[i]; if (isPlaceWaypoint(x)) return; const v = e.currentTarget.value.trim(); if (v) x.name = { ...(x.name ?? {}), [locale]: v }; else delete x.name; })} />
							</span>
						{/if}
						<span class="tools">
							<button type="button" class="ghost" disabled={i === 0} aria-label={m['creator.map_move_up']()} title={m['creator.map_move_up']()} onclick={() => moveWaypoint(i, -1)}><Icon name="arrow-up" /></button>
							<button type="button" class="ghost" disabled={i === route.waypoints.length - 1} aria-label={m['creator.map_move_down']()} title={m['creator.map_move_down']()} onclick={() => moveWaypoint(i, 1)}><Icon name="arrow-down" /></button>
							<button type="button" class="ghost" aria-label={m['creator.map_remove_waypoint']()} title={m['creator.map_remove_waypoint']()} onclick={() => removeWaypoint(i)}><Icon name="x" /></button>
						</span>
					</li>
				{/each}
			</ol>
			<button type="button" class="ghost" onclick={() => editRoute(doc, route.id, (r) => r.waypoints.reverse())}>{m['creator.map_reverse']()}</button>
		</fieldset>
		<button type="button" class="danger" onclick={deleteRoute} data-testid="route-delete">{m['creator.map_delete_route']()}</button>
	{:else if layer}
		<h2>{layer.id}</h2>
		<label class="field"><span>{m['creator.map_layer_id']()}</span><input type="text" value={layer.id} onchange={(e) => { const id = e.currentTarget.value.trim(); if (!id) return; editLayer((l) => (l.id = id)); onselect({ kind: 'layer', id }); }} /></label>
		<label class="field"><span>{m['creator.map_layer_image']()}</span><input type="text" value={layer.image} onchange={(e) => editLayer((l) => (l.image = e.currentTarget.value.trim()))} data-testid="layer-image" /></label>
		{#if layer.image}<img class="layer-preview" src={asset(layer.image)} alt="" />{/if}
		<span class="lbl">{m['creator.map_layer_zoom']()}</span>
		<div class="row">
			<input type="number" step="0.25" min="0" value={layer.minZoom ?? ''} aria-label="minZoom" onchange={(e) => editLayer((l) => { const v = num(e.currentTarget.value); if (v === undefined) delete l.minZoom; else l.minZoom = v; })} />
			<input type="number" step="0.25" min="0" value={layer.maxZoom ?? ''} aria-label="maxZoom" onchange={(e) => editLayer((l) => { const v = num(e.currentTarget.value); if (v === undefined) delete l.maxZoom; else l.maxZoom = v; })} />
		</div>
		<span class="lbl">{m['creator.map_world_size']()}</span>
		<div class="row">
			<input type="number" min="1" value={map.width} aria-label="width" onchange={(e) => editMap(doc, (w) => (w.width = Math.max(1, Math.round(Number(e.currentTarget.value)))))} />
			<input type="number" min="1" value={map.height} aria-label="height" onchange={(e) => editMap(doc, (w) => (w.height = Math.max(1, Math.round(Number(e.currentTarget.value)))))} />
		</div>
		<button type="button" class="danger" onclick={deleteLayer}>{m['creator.map_delete_layer']()}</button>
	{:else}
		<p class="hint">{m['creator.map_none_selected']()}</p>
	{/if}
</div>

<style>
	.props {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		padding: 0.75rem;
	}
	h2 {
		margin: 0;
		word-break: break-word;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		min-width: 0;
		flex: 1;
	}
	.field.small {
		flex: 0 1 5.5rem;
	}
	.field span,
	.lbl {
		font-size: var(--type-caption-size);
		color: var(--muted);
	}
	.row {
		display: flex;
		gap: 0.4rem;
		align-items: flex-end;
		flex-wrap: wrap;
	}
	.row > input,
	.row > select {
		flex: 1;
		min-width: 0;
	}
	fieldset {
		border: 1px solid var(--border-subtle);
		margin: 0;
		padding: 0.4rem 0.5rem 0.5rem;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		min-width: 0;
	}
	legend {
		font-size: var(--type-caption-size);
		font-weight: var(--font-weight-regular);
	}
	.check {
		display: flex;
		gap: 0.35rem;
		align-items: center;
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
	}
	.chip {
		font: inherit;
		font-size: var(--type-caption-size);
		box-sizing: border-box;
		min-height: var(--control-height);
		padding: 0 0.45rem;
		border: var(--border-width-regular) solid var(--border-muted);
		background: var(--paper);
		color: var(--ink);
		cursor: pointer;
	}
	.chip.on {
		border-color: var(--ink);
		background: var(--ink);
		color: var(--paper);
	}
	.hint {
		font-size: var(--type-caption-size);
		color: var(--muted);
		margin: 0;
	}
	.bad {
		color: var(--error);
	}
	.warn {
		color: var(--warn);
	}
	.danger {
		align-self: flex-start;
	}
	.grid {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(4.5rem, 1fr));
		gap: 0.25rem;
		max-height: 14rem;
		overflow: auto;
	}
	.thumb {
		all: unset;
		box-sizing: border-box;
		display: flex;
		flex-direction: column;
		align-items: center;
		width: 100%;
		padding: 0.2rem;
		border: 1px solid var(--border-subtle);
		cursor: pointer;
		font-size: var(--type-caption-size);
	}
	.thumb.current {
		border: 2px solid var(--ink);
	}
	.thumb img {
		width: 100%;
		height: 3rem;
		object-fit: contain;
	}
	.thumb span {
		max-width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.checker {
		background-color: #fff;
		background-image: linear-gradient(45deg, #ddd 25%, transparent 25%), linear-gradient(-45deg, #ddd 25%, transparent 25%),
			linear-gradient(45deg, transparent 75%, #ddd 75%), linear-gradient(-45deg, transparent 75%, #ddd 75%);
		background-size: 16px 16px;
		background-position: 0 0, 0 8px, 8px -8px, -8px 0;
	}
	.anchor {
		position: relative;
		width: 160px;
		border: 1px solid var(--ink);
		cursor: crosshair;
		touch-action: none;
	}
	.anchor img {
		display: block;
		width: 100%;
		height: auto;
		pointer-events: none;
	}
	.cross {
		position: absolute;
		width: 18px;
		height: 18px;
		transform: translate(-50%, -50%);
		pointer-events: none;
		background:
			linear-gradient(var(--error, #e00), var(--error, #e00)) center / 2px 100% no-repeat,
			linear-gradient(var(--error, #e00), var(--error, #e00)) center / 100% 2px no-repeat;
	}
	.previews {
		display: flex;
		gap: 0.4rem;
		align-items: flex-end;
	}
	.preview {
		position: relative;
		flex: 1;
		min-width: 3rem;
		background: var(--primary-cyan, #00d1ff);
		overflow: hidden;
	}
	.preview img {
		position: absolute;
		height: auto;
	}
	.preview .dot {
		position: absolute;
		left: calc(50% - 3px);
		bottom: 3px;
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--ink);
	}
	.preview .z {
		position: absolute;
		top: 2px;
		left: 3px;
		font-size: var(--type-caption-size);
	}
	.waypoints {
		margin: 0;
		padding-left: 1.4rem;
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}
	.waypoints li {
		display: flex;
		gap: 0.3rem;
		align-items: center;
		justify-content: space-between;
	}
	.waypoints .tools {
		display: flex;
		flex: none;
	}
	.waypoints .tools button {
		padding: 0 0.3rem;
	}
	.free {
		display: flex;
		gap: 0.2rem;
		min-width: 0;
	}
	.free input {
		width: 5rem;
		min-width: 0;
	}
	.link {
		all: unset;
		cursor: pointer;
		text-decoration: underline;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.layer-preview {
		width: 100%;
		max-height: 8rem;
		object-fit: contain;
		background: var(--primary-cyan, #00d1ff);
	}
</style>
