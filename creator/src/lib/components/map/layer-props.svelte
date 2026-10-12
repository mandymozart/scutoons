<script lang="ts">
	import { confirmDelete } from '$lib/editor/ask.ts';
	import { FOG_RADIUS_MAX, FOG_RADIUS_MIN, type MapLayer, type WorldMap } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { MediaSlot } from '@scutoons/media';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { editMap, numberOrUnset } from '$lib/editor/map.ts';
	import type { MapSelection } from '$lib/components/map/map-canvas.svelte';

	/** The selected background layer (agents/MAP.md §4.5): id, image, the zoom range it shows in, the world's size, delete. */
	let {
		doc,
		map,
		layer,
		index,
		onselect
	}: { doc: ContentDoc; map: WorldMap; layer: MapLayer; index: number; onselect: (selection: MapSelection) => void } = $props();

	/** where an uploaded layer goes: the folder of the first layer's image, else the map's folder */
	const layerFolder = $derived((map.layers.find((l) => l.image)?.image ?? 'catpope/map/x').replace(/[^/]+$/, ''));

	function edit(change: (l: MapLayer) => void, key?: string) {
		const i = index;
		editMap(doc, (map) => {
			if (map.layers[i]) change(map.layers[i]);
		}, key);
	}
	async function remove() {
		if (!(await confirmDelete(m['creator.map_delete_confirm']({ name: layer.id })))) return;
		const i = index;
		editMap(doc, (map) => map.layers.splice(i, 1));
		onselect(null);
	}
</script>

<h2>{layer.id}</h2>
<label class="field"><span>{m['creator.map_layer_id']()}</span><input type="text" value={layer.id} onchange={(e) => { const id = e.currentTarget.value.trim(); if (!id) return; edit((l) => (l.id = id)); onselect({ kind: 'layer', id }); }} /></label>
<!-- the asset picker (owner 2026-10-11: no bare URL field): an SVG or a WebP; an uploaded PNG / JPEG becomes a WebP -->
<MediaSlot toon="" value={layer.image || undefined} label={m['creator.map_layer_image']()} place={layerFolder} webp onchange={(path) => edit((l) => (l.image = path ?? ''))} testid="layer-image" />
<span class="lbl">{m['creator.map_layer_zoom']()}</span>
<div class="row">
	<input type="number" step="0.25" min="0" value={layer.minZoom ?? ''} aria-label={m['creator.map_min_zoom']()} onchange={(e) => edit((l) => { const v = numberOrUnset(e.currentTarget.value); if (v === undefined) delete l.minZoom; else l.minZoom = v; })} />
	<input type="number" step="0.25" min="0" value={layer.maxZoom ?? ''} aria-label={m['creator.map_max_zoom']()} onchange={(e) => edit((l) => { const v = numberOrUnset(e.currentTarget.value); if (v === undefined) delete l.maxZoom; else l.maxZoom = v; })} />
</div>
<!-- fog of war (owner 2026-10-12, agents/MAP.md §8.9): the radius around the reader's unlocked places -->
<label class="field"><span>{m['creator.map_layer_fog']()}</span><input type="number" step="1" min={FOG_RADIUS_MIN} max={FOG_RADIUS_MAX} value={layer.fog?.radius ?? ''} title={m['creator.map_layer_fog_hint']()} onchange={(e) => edit((l) => { const v = numberOrUnset(e.currentTarget.value); if (v === undefined) delete l.fog; else l.fog = { radius: Math.min(FOG_RADIUS_MAX, Math.max(FOG_RADIUS_MIN, v)) }; })} data-testid="layer-fog" /></label>
<span class="lbl">{m['creator.map_world_size']()}</span>
<div class="row">
	<input type="number" min="1" value={map.width} aria-label={m['creator.map_width']()} onchange={(e) => editMap(doc, (w) => (w.width = Math.max(1, Math.round(Number(e.currentTarget.value)))))} />
	<input type="number" min="1" value={map.height} aria-label={m['creator.map_height']()} onchange={(e) => editMap(doc, (w) => (w.height = Math.max(1, Math.round(Number(e.currentTarget.value)))))} />
</div>
<button type="button" class="danger" onclick={remove}>{m['creator.map_delete_layer']()}</button>


