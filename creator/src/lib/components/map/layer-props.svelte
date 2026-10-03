<script lang="ts">
	import type { MapLayer, WorldMap } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
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

	function edit(change: (l: MapLayer) => void, key?: string) {
		const i = index;
		editMap(doc, (map) => {
			if (map.layers[i]) change(map.layers[i]);
		}, key);
	}
	function remove() {
		if (!confirm(m['creator.map_delete_confirm']({ name: layer.id }))) return;
		const i = index;
		editMap(doc, (map) => map.layers.splice(i, 1));
		onselect(null);
	}
</script>

<h2>{layer.id}</h2>
<label class="field"><span>{m['creator.map_layer_id']()}</span><input type="text" value={layer.id} onchange={(e) => { const id = e.currentTarget.value.trim(); if (!id) return; edit((l) => (l.id = id)); onselect({ kind: 'layer', id }); }} /></label>
<label class="field"><span>{m['creator.map_layer_image']()}</span><input type="text" value={layer.image} onchange={(e) => edit((l) => (l.image = e.currentTarget.value.trim()))} data-testid="layer-image" /></label>
{#if layer.image}<img class="layer-preview" src={asset(layer.image)} alt="" />{/if}
<span class="lbl">{m['creator.map_layer_zoom']()}</span>
<div class="row">
	<input type="number" step="0.25" min="0" value={layer.minZoom ?? ''} aria-label="minZoom" onchange={(e) => edit((l) => { const v = numberOrUnset(e.currentTarget.value); if (v === undefined) delete l.minZoom; else l.minZoom = v; })} />
	<input type="number" step="0.25" min="0" value={layer.maxZoom ?? ''} aria-label="maxZoom" onchange={(e) => edit((l) => { const v = numberOrUnset(e.currentTarget.value); if (v === undefined) delete l.maxZoom; else l.maxZoom = v; })} />
</div>
<span class="lbl">{m['creator.map_world_size']()}</span>
<div class="row">
	<input type="number" min="1" value={map.width} aria-label="width" onchange={(e) => editMap(doc, (w) => (w.width = Math.max(1, Math.round(Number(e.currentTarget.value)))))} />
	<input type="number" min="1" value={map.height} aria-label="height" onchange={(e) => editMap(doc, (w) => (w.height = Math.max(1, Math.round(Number(e.currentTarget.value)))))} />
</div>
<button type="button" class="danger" onclick={remove}>{m['creator.map_delete_layer']()}</button>

<style>
	.layer-preview {
		width: 100%;
		max-height: 8rem;
		object-fit: contain;
		background: var(--primary-cyan, #00d1ff);
	}
</style>
