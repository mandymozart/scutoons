<script lang="ts">
	import { untrack } from 'svelte';
	import { tileLevel, visibleTiles, type MapFog, type MapTiles } from '@scutoons/content';
	import type { MapViewport } from './viewport.svelte.ts';
	import { fogGradients, fogMaskUrl } from './fog-mask.ts';

	/**
	 * The world map's terrain (agents/MAP.md; owner 2026-10-11: "MapTerrain sollte sowieso shared sein"): the background
	 * layers in world size under ONE camera transform, shared by the reader's map and the creator's map builder – so the
	 * renderer can change in one place later (a raster / tiled / canvas terrain, BACKLOG 10.63). A layer shows within its
	 * zoom range (`minZoom` / `maxZoom`) and when `visible` lets it (the builder's eye). While the camera moves the world is
	 * one composited layer (`will-change`), so a zoom is not re-rastered per frame.
	 *
	 * Fog of war (owner 2026-10-12, agents/MAP.md §8.9): with `fog` (the reader's unlocked places) a layer with `fog` shows
	 * only around those places – one mask image for the layer (fog-mask.ts), not one per tile; a place that joins the list
	 * later grows its disc (the reveal). Without `fog` (the builder) every layer shows whole.
	 */
	let {
		view,
		width,
		height,
		layers,
		url,
		visible = () => true,
		zoomRange = true,
		fog = null,
		testid
	}: {
		view: MapViewport;
		/** the world's size (world units = px at zoom 1) */
		width: number;
		height: number;
		layers: readonly { id: string; image?: string; minZoom?: number; maxZoom?: number; tiles?: MapTiles; fog?: MapFog }[];
		/** a layer's image path → its URL */
		url: (path: string) => string;
		/** may this layer show (the builder hides some); default all */
		visible?: (id: string) => boolean;
		/** false: every layer at every zoom (the builder shows them all to edit) */
		zoomRange?: boolean;
		/** the places the reader has unlocked (their points): fog layers show only around them; null = no fog (whole) */
		fog?: readonly { id: string; x: number; y: number; scale?: number }[] | null;
		testid?: string;
	} = $props();

	// ── fog of war: per radius one mask image of the settled places; a place new to the list grows first (≈ 1.6 s) ──
	const GROW_MS = 1600;
	type FogPlace = { id: string; x: number; y: number; scale?: number };
	/** the places already in the mask image, and those growing (with their start) */
	let known: Set<string> | null = null;
	let growing = $state<{ p: FogPlace; start: number }[]>([]);
	let now = $state(0);
	let masks = $state<Record<number, string | null>>({});
	const radii = $derived([...new Set(layers.filter((l) => l.fog).map((l) => l.fog!.radius))]);
	$effect(() => {
		const list = fog;
		if (!list) {
			known = null;
			return;
		}
		untrack(() => {
			const reduced = globalThis.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
			// the first list after mount is there already; later additions grow (unless motion is reduced)
			if (known && !reduced) {
				// a route's corridor points (`scale`) go straight into the mask image – hundreds of growing CSS gradients would be
				// too much for a phone; the places grow
				const fresh = list.filter((p) => p.scale === undefined && !known!.has(p.id) && !growing.some((g) => g.p.id === p.id));
				if (fresh.length) growing = [...growing, ...fresh.map((p) => ({ p, start: performance.now() }))];
			}
			known = new Set(list.map((p) => p.id));
			if (growing.length) tick();
		});
	});
	let frame = 0;
	function tick() {
		cancelAnimationFrame(frame);
		frame = requestAnimationFrame(() => {
			now = performance.now();
			if (growing.some((g) => now - g.start < GROW_MS)) tick();
		});
	}
	$effect(() => () => cancelAnimationFrame(frame));
	/** the settled places (in the mask image): the list without those still growing */
	const growingDone = $derived(growing.length > 0 && growing.every((g) => now - g.start >= GROW_MS));
	const settled = $derived(fog ? fog.filter((p) => growingDone || !growing.some((g) => g.p.id === p.id)) : []);
	let maskRun = 0;
	/** redrawn only when the settled places or the radii change (not on every new list of the same places) */
	const maskKey = $derived(fog ? `${width}x${height}|${radii.join(',')}|${settled.map((p) => `${p.id}@${p.x},${p.y}`).join(';')}` : '');
	$effect(() => {
		if (!maskKey) return;
		const places = untrack(() => settled), rs = untrack(() => radii), size = { width: untrack(() => width), height: untrack(() => height) };
		const run = ++maskRun;
		void Promise.all(rs.map(async (r) => [r, await fogMaskUrl(places, r, size)] as const)).then((pairs) => {
			if (run !== maskRun) return pairs.forEach(([, u]) => u && URL.revokeObjectURL(u));
			const old = masks;
			masks = Object.fromEntries(pairs);
			Object.values(old).forEach((u) => u && URL.revokeObjectURL(u));
			// the grown places in this image: their gradients go
			if (growing.some((g) => places.some((p) => p.id === g.p.id))) growing = growing.filter((g) => !places.some((p) => p.id === g.p.id));
		});
	});
	$effect(() => () => Object.values(untrack(() => masks)).forEach((u) => u && URL.revokeObjectURL(u)));
	const ease = (t: number) => 1 - (1 - Math.min(1, Math.max(0, t))) ** 3;
	/** a fog layer's mask (CSS): the growing places' gradients over the settled mask image */
	function maskOf(f: MapFog, tile?: { left: number; top: number }): string {
		// a grown place keeps its gradients until the new mask image (with it) is there – no flicker
		const layers = growing.flatMap((g) => fogGradients(g.p, f.radius, ease((now - g.start) / GROW_MS)));
		const url = masks[f.radius];
		if (url) layers.push(`url(${url})`);
		const image = layers.length ? layers.join(', ') : 'linear-gradient(transparent, transparent)';
		const css = `mask-image: ${image}; -webkit-mask-image: ${image}`;
		if (!tile) return css;
		// one tile: the world-sized mask, shifted to the tile's place
		const box = `mask-size: ${width}px ${height}px; -webkit-mask-size: ${width}px ${height}px; mask-repeat: no-repeat; -webkit-mask-repeat: no-repeat`;
		const at = `mask-position: ${-tile.left}px ${-tile.top}px; -webkit-mask-position: ${-tile.left}px ${-tile.top}px`;
		return `${css}; ${box}; ${at}`;
	}

	// ── tiles (BACKLOG 10.63): a layer with a pyramid draws the tiles on screen over its image (the base and fallback) –
	//    the level follows the zoom only at rest (no new downloads mid-gesture); the tiles follow a pan at once ──
	let restZoom = $state(1);
	$effect(() => {
		if (!view.moving) restZoom = view.zoom;
	});
	const dpr = typeof window === 'undefined' ? 1 : window.devicePixelRatio || 1;
	function tilesOf(tiles: MapTiles) {
		const level = tileLevel(restZoom, dpr, tiles.levels);
		if (level < 1) return []; // level 0 = the image itself
		const a = view.toWorld({ x: 0, y: 0 });
		const b = view.toWorld({ x: view.width, y: view.height });
		const margin = 256 / 2 ** level; // one tile around the screen
		return visibleTiles({ x: a.x - margin, y: a.y - margin, width: b.x - a.x + margin * 2, height: b.y - a.y + margin * 2 }, level, { width, height });
	}

	const inRange = (l: { minZoom?: number; maxZoom?: number }) => (l.minZoom === undefined || view.zoom >= l.minZoom) && (l.maxZoom === undefined || view.zoom <= l.maxZoom);
</script>

<!-- zoomed in, a fog layer masks each tile on its own (owner 2026-10-12, the phone: "after zooming in the fog of war
     does not work any more" – one masked layer as big as the zoomed world is more than a phone's GPU keeps a mask for) -->
{#snippet fogTiles(layer: (typeof layers)[number], f: MapFog, list: ReturnType<typeof tilesOf>)}
	{#each list as t (`${t.z}/${t.x}_${t.y}`)}
		<img class="tile" src={url(`${layer.tiles!.path}/${t.z}/${t.x}_${t.y}.webp`)} alt="" draggable="false" style="left: {t.left}px; top: {t.top}px; width: {t.size}px; height: {t.size}px; {maskOf(f, t)}" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.visibility = 'hidden')} data-testid="map-fog-tile" />
	{/each}
{/snippet}

{#snippet drawn(layer: (typeof layers)[number])}
	<img class="layer" src={url(layer.image!)} alt="" {width} {height} draggable="false" />
	{#if layer.tiles}
		{#each tilesOf(layer.tiles) as t (`${t.z}/${t.x}_${t.y}`)}
			<img class="tile" src={url(`${layer.tiles.path}/${t.z}/${t.x}_${t.y}.webp`)} alt="" draggable="false" style:left="{t.left}px" style:top="{t.top}px" style:width="{t.size}px" style:height="{t.size}px" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.visibility = 'hidden')} />
		{/each}
	{/if}
{/snippet}

<div class="terrain" class:moving={view.moving} style:transform={view.transform} style:width="{width}px" style:height="{height}px" data-testid={testid}>
	{#each layers as layer (layer.id)}
		{#if layer.image && (!zoomRange || inRange(layer)) && visible(layer.id)}
			{#if layer.fog && fog}
				{#if fog.length || growing.length}
					{@const list = layer.tiles ? tilesOf(layer.tiles) : []}
					{#if list.length}
						{@render fogTiles(layer, layer.fog, list)}
					{:else}
						<div class="fog" style={maskOf(layer.fog)} data-testid="map-fog-{layer.id}">
							{@render drawn(layer)}
						</div>
					{/if}
				{/if}
			{:else}
				{@render drawn(layer)}
			{/if}
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
	/* a fog layer: its mask in world proportions (one image, scaled with the camera) */
	.fog {
		position: absolute;
		inset: 0;
		mask-size: 100% 100%;
		-webkit-mask-size: 100% 100%;
		mask-repeat: no-repeat;
		-webkit-mask-repeat: no-repeat;
		pointer-events: none;
	}
	/* in world units inside the camera transform */
	.tile {
		position: absolute;
		pointer-events: none;
	}
</style>
