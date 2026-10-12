<script lang="ts">
	import type { WorldMap } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import PlaceProps from '$lib/components/map/place-props.svelte';
	import RouteProps from '$lib/components/map/route-props.svelte';
	import LayerProps from '$lib/components/map/layer-props.svelte';
	import RegionProps from '$lib/components/map/region-props.svelte';
	import type { MapSelection } from '$lib/components/map/map-canvas.svelte';
	import type { MapViewport } from '@scutoons/reader';

	/**
	 * The builder's properties panel (agents/MAP.md §4.3–4.5): the form of what is selected – a place, a route or a
	 * layer (COMPONENTS.md step 9: one component each) – and the panel's small form vocabulary they share.
	 */
	let {
		doc,
		map,
		selection,
		toons,
		languages,
		publishedIds,
		onselect,
		view,
		bust = 0,
		characters = []
	}: {
		/** the registry's characters (a region's representative) */
		characters?: { id: string; name: string }[];
		doc: ContentDoc;
		map: WorldMap;
		selection: MapSelection;
		toons: { id: string; name: string }[];
		languages: string[];
		/** ids in the published map (fixed: events reference them) */
		publishedIds: { places: Set<string>; routes: Set<string>; regions?: Set<string> };
		onselect: (selection: MapSelection) => void;
		/** the builder's canvas view (a place's "Use the current view") */
		view: MapViewport;
		/** set after an import (a timestamp): the place image's preview loads again */
		bust?: number;
	} = $props();

	const place = $derived(selection?.kind === 'place' ? (map.locations.find((p) => p.id === selection.id) ?? null) : null);
	const route = $derived(selection?.kind === 'route' ? (map.routes.find((r) => r.id === selection.id) ?? null) : null);
	const layerIndex = $derived(selection?.kind === 'layer' ? (map.layers ?? []).findIndex((l) => l.id === selection.id) : -1);
	const layer = $derived(layerIndex >= 0 ? map.layers[layerIndex] : null);
	const region = $derived(selection?.kind === 'region' ? (map.regions?.find((r) => r.id === selection.id) ?? null) : null);
</script>

<div class="props" data-testid="map-props">
	{#if place}
		<PlaceProps {doc} {map} {place} {toons} {languages} locked={publishedIds.places.has(place.id)} {onselect} {view} {bust} />
	{:else if route}
		<RouteProps {doc} {map} {route} {toons} {languages} locked={publishedIds.routes.has(route.id)} {onselect} />
	{:else if layer}
		<LayerProps {doc} {map} {layer} index={layerIndex} {onselect} />
	{:else if region}
		<RegionProps {doc} {region} {languages} {characters} locked={!!publishedIds.regions?.has(region.id)} {onselect} />
	{/if}
</div>

<style>
	/* the panel's form vocabulary, shared by the place / route / layer forms inside it */
	.props {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		padding: 0.75rem;
	}
	.props :global(h2) {
		margin: 0;
		word-break: break-word;
	}
	.props :global(.field) {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		min-width: 0;
		flex: 1;
	}
	.props :global(.field.small) {
		flex: 0 1 5.5rem;
	}
	.props :global(.field span),
	.props :global(.lbl) {
		font-size: var(--type-caption-size);
		color: var(--muted);
	}
	.props :global(.row) {
		display: flex;
		gap: 0.4rem;
		align-items: flex-end;
		flex-wrap: wrap;
	}
	.props :global(.row > input),
	.props :global(.row > select) {
		flex: 1;
		min-width: 0;
	}
	/* the panel's own groups – not a field component's fieldset (LocaleMapField has its own) */
	.props :global(fieldset:not(.locale-map)) {
		border: 1px solid var(--border-subtle);
		margin: 0;
		padding: 0.4rem 0.5rem 0.5rem;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		min-width: 0;
	}
	.props :global(fieldset:not(.locale-map) > legend) {
		font-size: var(--type-caption-size);
		font-weight: var(--font-weight-regular);
	}
	.props :global(.check) {
		display: flex;
		gap: 0.35rem;
		align-items: center;
	}
	.props :global(.hint) {
		font-size: var(--type-caption-size);
		color: var(--muted);
		margin: 0;
	}
	.props :global(.bad) {
		color: var(--error);
	}
	.props :global(.warn) {
		color: var(--warn);
	}
	.props :global(.danger) {
		align-self: flex-start;
	}
</style>
