<script lang="ts">
	import { confirmDelete } from '$lib/editor/ask.ts';
	import { MAP_SOURCE_LANGUAGE, chapterSlices, mapName, sliceUnlockPoint, slugId, type ChapterSliceRef, type LocaleText, type MapPlace, type WorldMap } from '@scutoons/content';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { library } from '$lib/api.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { autoId, editPlace, numberOrUnset, removePlace, renamePlace, routesThrough } from '$lib/editor/map.ts';
	import LocaleMapField from '$lib/components/fields/locale-map-field.svelte';
	import MapToonsChips from '$lib/components/map/map-toons-chips.svelte';
	import PlaceImage from '$lib/components/map/place-image.svelte';
	import type { MapSelection } from '$lib/components/map/map-canvas.svelte';

	/**
	 * The selected place (agents/MAP.md §4.3–4.5): id (editable until the first publish), names, position, toons, hidden +
	 * its unlock point (toon, chapter, the unlocking slice – the same storage the chapter editor's slice edits), the
	 * card's chapter link, the image (PlaceImage), where it is used, delete.
	 */
	let {
		doc,
		map,
		place,
		toons,
		languages,
		locked,
		onselect,
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

	const usedBy = $derived(routesThrough(map, place.id));
	async function remove() {
		if (usedBy.length) return;
		if (!(await confirmDelete(m['creator.map_delete_confirm']({ name: mapName(place.name, locale) || place.id })))) return;
		removePlace(doc, place.id);
		onselect(null);
	}
</script>

<h2>{mapName(place.name, locale) || place.id}</h2>
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

<PlaceImage {doc} {place} {bust} />

<fieldset>
	<legend>{m['creator.map_used_in']()}</legend>
	{#if usedBy.length}<p>{m['creator.map_used_in_routes']({ routes: usedBy.join(', ') })}</p>{/if}
	<p class="hint">{m['creator.map_used_in_events']()}</p>
</fieldset>
<button type="button" class="danger" disabled={usedBy.length > 0} title={usedBy.length ? m['creator.map_delete_blocked']({ routes: usedBy.join(', ') }) : undefined} onclick={remove} data-testid="place-delete">{m['creator.map_delete_place']()}</button>
{#if usedBy.length}<p class="hint">{m['creator.map_delete_blocked']({ routes: usedBy.join(', ') })}</p>{/if}
