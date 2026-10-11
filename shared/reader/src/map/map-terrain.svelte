<script lang="ts">
	import type { MapViewport } from './viewport.svelte.ts';

	/**
	 * The world map's terrain (agents/MAP.md; owner 2026-10-11: "MapTerrain sollte sowieso shared sein"): the background
	 * layers in world size under ONE camera transform, shared by the reader's map and the creator's map builder – so the
	 * renderer can change in one place later (a raster / tiled / canvas terrain, BACKLOG 10.63). A layer shows within its
	 * zoom range (`minZoom` / `maxZoom`) and when `visible` lets it (the builder's eye). While the camera moves the world is
	 * one composited layer (`will-change`), so a zoom is not re-rastered per frame.
	 */
	let {
		view,
		width,
		height,
		layers,
		url,
		visible = () => true,
		zoomRange = true,
		testid
	}: {
		view: MapViewport;
		/** the world's size (world units = px at zoom 1) */
		width: number;
		height: number;
		layers: readonly { id: string; image?: string; minZoom?: number; maxZoom?: number }[];
		/** a layer's image path → its URL */
		url: (path: string) => string;
		/** may this layer show (the builder hides some); default all */
		visible?: (id: string) => boolean;
		/** false: every layer at every zoom (the builder shows them all to edit) */
		zoomRange?: boolean;
		testid?: string;
	} = $props();

	const inRange = (l: { minZoom?: number; maxZoom?: number }) => (l.minZoom === undefined || view.zoom >= l.minZoom) && (l.maxZoom === undefined || view.zoom <= l.maxZoom);
</script>

<div class="terrain" class:moving={view.moving} style:transform={view.transform} style:width="{width}px" style:height="{height}px" data-testid={testid}>
	{#each layers as layer (layer.id)}
		{#if layer.image && (!zoomRange || inRange(layer)) && visible(layer.id)}
			<img class="layer" src={url(layer.image)} alt="" {width} {height} draggable="false" />
		{/if}
	{/each}
</div>

<style>
	.terrain {
		position: absolute;
		top: 0;
		left: 0;
		transform-origin: 0 0;
		pointer-events: none;
	}
	.terrain.moving {
		will-change: transform;
	}
	.layer {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
	}
</style>
