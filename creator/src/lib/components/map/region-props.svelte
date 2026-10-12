<script lang="ts">
	import { confirmDelete } from '$lib/editor/ask.ts';
	import { regionBrush } from '$lib/editor/region-brush.svelte.ts';
	import { Icon } from '@scutoons/ui';
	import { BRUSH_MAX, BRUSH_MIN, MAP_SOURCE_LANGUAGE, mapName, runsSize, slugId, type LocaleText, type MapRegion } from '@scutoons/content';
	import { MediaSlot } from '@scutoons/media';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { autoId, editRegion, removeRegion, renameRegion } from '$lib/editor/map.ts';
	import LocaleMapField from '$lib/components/fields/locale-map-field.svelte';
	import type { MapSelection } from '$lib/components/map/map-canvas.svelte';

	/**
	 * The selected region (agents/MAP.md §8.10, owner 2026-10-12): id (editable until the first publish), name, the welcome
	 * sign with its claim, the greeting (said by the representative – or the narrator), the in-depth description, the
	 * vegetation, the colour on the map, the elected representative (elections later – set by hand), the cover painted with
	 * the hex brush (1–256 cells), delete.
	 */
	let {
		doc,
		region,
		languages,
		characters,
		locked,
		onselect
	}: {
		doc: ContentDoc;
		region: MapRegion;
		languages: string[];
		/** the registry's characters (the representative) */
		characters: { id: string; name: string }[];
		/** the id is in the published map (fixed: characters and events reference it) */
		locked: boolean;
		onselect: (selection: MapSelection) => void;
	} = $props();

	const locale = $derived(getLocale());
	const cells = $derived(runsSize(region.cells ?? []));
	const painting = $derived(regionBrush.region === region.id);

	function set<K extends keyof MapRegion>(key: K, value: MapRegion[K] | undefined, undoKey?: string) {
		editRegion(doc, region.id, (r) => {
			const empty = value === undefined || value === '' || (typeof value === 'object' && value !== null && !Array.isArray(value) && !Object.values(value).some((v) => typeof v === 'string' && v.trim()));
			if (empty && key !== 'name') delete r[key];
			else r[key] = value as MapRegion[K];
		}, undoKey ?? `region-${String(key)}:${region.id}`);
	}
	function setName(name: LocaleText) {
		const before = region.name?.[MAP_SOURCE_LANGUAGE];
		const id = region.id;
		set('name', name, 'region-name');
		const next = name[MAP_SOURCE_LANGUAGE]?.trim();
		if (!locked && next && autoId(id, before, 'region') && slugId(next) && slugId(next) !== id) onselect({ kind: 'region', id: renameRegion(doc, id, next, 'region-name') });
	}
	async function remove() {
		const name = mapName(region.name, locale) || region.id;
		if (!(await confirmDelete(m['creator.map_delete_confirm']({ name })))) return;
		regionBrush.stop();
		removeRegion(doc, region.id);
		onselect(null);
	}
</script>

<h2>{mapName(region.name, locale) || region.id}</h2>
<div class="tab" data-tab="region">
	<label class="field">
		<span>{m['creator.map_id']()}</span>
		<input type="text" value={region.id} disabled={locked} title={locked ? m['creator.map_id_locked']() : undefined} onchange={(e) => onselect({ kind: 'region', id: renameRegion(doc, region.id, e.currentTarget.value) })} data-testid="region-id" />
	</label>
	<LocaleMapField value={region.name} {languages} label={m['creator.map_name']()} required testid="region-name" onchange={setName} />
	<LocaleMapField value={region.claim ?? {}} {languages} label={m['creator.region_claim']()} testid="region-claim" onchange={(v) => set('claim', v)} />

	<!-- the cover: the map's own hex cells, painted with the brush (1–256 – owner 2026-10-12) -->
	<fieldset class="ground" data-testid="region-cover">
		<legend>{m['creator.region_cover']()}</legend>
		<p class="hint">{m['creator.region_cells']({ count: cells.toLocaleString(locale) })}</p>
		<div class="row">
			<button type="button" class:secondary={!painting} aria-pressed={painting} onclick={() => regionBrush.toggle(region.id)} data-testid="region-paint"><Icon name="hexagon" /> {m['creator.map_cells_paint']()}</button>
			{#if painting}
				<span class="sc-seg" role="group" aria-label={m['creator.region_cover']()}>
					<button type="button" class:on={regionBrush.mode === 'add'} aria-pressed={regionBrush.mode === 'add'} onclick={() => (regionBrush.mode = 'add')} data-testid="region-add">{m['creator.map_cells_add']()}</button>
					<button type="button" class:on={regionBrush.mode === 'remove'} aria-pressed={regionBrush.mode === 'remove'} onclick={() => (regionBrush.mode = 'remove')} data-testid="region-remove">{m['creator.map_cells_remove']()}</button>
				</span>
			{/if}
		</div>
		{#if painting}
			<label class="field"><span>{m['creator.map_brush_size']()} · {regionBrush.size}</span>
				<input type="range" min={BRUSH_MIN} max={BRUSH_MAX} step="1" bind:value={regionBrush.size} data-testid="region-brush-size" />
			</label>
		{/if}
	</fieldset>

	<label class="field">
		<span>{m['creator.region_colour']()}</span>
		<input type="color" value={region.colour ?? '#888888'} onchange={(e) => set('colour', e.currentTarget.value)} data-testid="region-colour" />
	</label>
</div>

<div class="tab" data-tab="region">
	<!-- the welcome (agents/MAP.md §8.10): the sign behind, the representative (or the narrator) says the greeting -->
	<MediaSlot toon="" value={region.sign?.src || undefined} label={m['creator.region_sign']()} place={`catpope/map/regions/${region.id}/`} webp onchange={(path) => set('sign', path ? { src: path } : undefined)} testid="region-sign" />
	<LocaleMapField value={region.greeting ?? {}} {languages} label={m['creator.region_greeting']()} multiline testid="region-greeting" onchange={(v) => set('greeting', v)} />
	<label class="field">
		<span>{m['creator.region_representative']()}</span>
		<select value={region.representative ?? ''} onchange={(e) => set('representative', e.currentTarget.value || undefined)} data-testid="region-representative">
			<option value="">{m['creator.region_narrator']()}</option>
			{#each characters as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
			{#if region.representative && !characters.some((c) => c.id === region.representative)}<option value={region.representative}>{region.representative} (?)</option>{/if}
		</select>
	</label>
	<p class="hint">{m['creator.region_representative_hint']()}</p>
	<LocaleMapField value={region.description ?? {}} {languages} label={m['creator.region_description']()} multiline testid="region-description" onchange={(v) => set('description', v)} />
	<label class="field">
		<span>{m['creator.region_vegetation']()}</span>
		<textarea rows="3" value={region.vegetation ?? ''} onchange={(e) => set('vegetation', e.currentTarget.value.trim() || undefined)} data-testid="region-vegetation"></textarea>
	</label>
	<button type="button" class="danger" onclick={remove} data-testid="region-delete">{m['creator.region_delete']()}</button>
</div>
