<script lang="ts">
	import { confirmDelete } from '$lib/editor/ask.ts';
	import { CloseIcon, SortGrip, sortable } from '@scutoons/ui';
	import { MAP_NETWORKS, MAP_SOURCE_LANGUAGE, isPlaceWaypoint, mapName, slugId, type LocaleText, type MapNetwork, type MapPath, type WorldMap } from '@scutoons/content';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { moveAt } from '$lib/editor/move.ts';
	import { autoId, editRoute, removeRoute, renameRoute } from '$lib/editor/map.ts';
	import LocaleMapField from '$lib/components/fields/locale-map-field.svelte';
	import MapToonsChips from '$lib/components/map/map-toons-chips.svelte';
	import type { MapSelection } from '$lib/components/map/map-canvas.svelte';

	/**
	 * The selected route (agents/MAP.md §4.4): id (editable until the first publish), names, toons, hidden, waypoints
	 * (a place → select it; a free point: id + name; reorder, remove, reverse), delete.
	 */
	let {
		doc,
		map,
		route,
		toons,
		languages,
		locked,
		onselect
	}: {
		doc: ContentDoc;
		map: WorldMap;
		route: MapPath;
		toons: { id: string; name: string }[];
		languages: string[];
		/** the id is in the published map (fixed: events reference it) */
		locked: boolean;
		onselect: (selection: MapSelection) => void;
	} = $props();

	const locale = $derived(getLocale());

	function changeId(value: string) {
		onselect({ kind: 'route', id: renameRoute(doc, route.id, value) });
	}
	function setName(name: LocaleText) {
		const before = route.name?.[MAP_SOURCE_LANGUAGE];
		const id = route.id;
		editRoute(doc, id, (r) => (r.name = name), 'route-name');
		const next = name[MAP_SOURCE_LANGUAGE]?.trim();
		if (!locked && next && autoId(id, before, 'weg') && slugId(next) && slugId(next) !== id) onselect({ kind: 'route', id: renameRoute(doc, id, next, 'route-name') });
	}
	async function remove() {
		if (!(await confirmDelete(m['creator.map_delete_confirm']({ name: mapName(route.name, locale) || route.id })))) return;
		removeRoute(doc, route.id);
		onselect(null);
	}
	const moveWaypoint = (from: number, to: number) => editRoute(doc, route.id, (r) => void moveAt(r.waypoints, from, to));
	const removeWaypoint = (i: number) => editRoute(doc, route.id, (r) => r.waypoints.splice(i, 1));
</script>

<h2>{mapName(route.name, locale) || route.id}</h2>
<label class="field">
	<span>{m['creator.map_id']()}</span>
	<input type="text" value={route.id} disabled={locked} title={locked ? m['creator.map_id_locked']() : undefined} onchange={(e) => changeId(e.currentTarget.value)} data-testid="route-id" />
</label>
<LocaleMapField value={route.name} {languages} label={m['creator.map_name']()} required testid="route-name" onchange={setName} />
<MapToonsChips {toons} value={route.toons} onchange={(next) => editRoute(doc, route.id, (r) => (r.toons = next))} />
<!-- a ride line (research/CAVE.md D8): its places are the stops; readers ride it for cucumbers, never walk it -->
<label class="field">
	{m['creator.map_network']()}
	<select value={route.network ?? ''} onchange={(e) => editRoute(doc, route.id, (r) => { const v = e.currentTarget.value as MapNetwork | ''; if (v) r.network = v; else delete r.network; })} data-testid="route-network">
		<option value="">{m['creator.map_network_none']()}</option>
		{#each MAP_NETWORKS as n (n)}<option value={n}>{m[`creator.map_network_${n}`]()}</option>{/each}
	</select>
</label>
<label class="check"><input type="checkbox" checked={!!route.hidden} onchange={(e) => editRoute(doc, route.id, (r) => { if (e.currentTarget.checked) r.hidden = true; else delete r.hidden; })} /> {m['creator.map_hidden']()}</label>
<fieldset>
	<legend>{m['creator.map_waypoints']()} ({route.waypoints.length})</legend>
	<p class="hint">{m['creator.map_insert_hint']()}</p>
	<ol class="waypoints" use:sortable={{ onmove: moveWaypoint }} data-testid="route-waypoints">
		{#each route.waypoints as w, i (i)}
			<li data-sort-row>
				<SortGrip label={m['creator.sort_move']({ name: isPlaceWaypoint(w) ? w.location : mapName(w.name, locale) || w.id })} title={m['creator.sort_hint']()} />
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
					<button type="button" class="ghost" aria-label={m['creator.map_remove_waypoint']()} title={m['creator.map_remove_waypoint']()} onclick={() => removeWaypoint(i)}><CloseIcon /></button>
				</span>
			</li>
		{/each}
	</ol>
	<button type="button" class="ghost" onclick={() => editRoute(doc, route.id, (r) => r.waypoints.reverse())}>{m['creator.map_reverse']()}</button>
</fieldset>
<button type="button" class="danger" onclick={remove} data-testid="route-delete">{m['creator.map_delete_route']()}</button>

<style>
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
	}
	.waypoints .tools {
		display: flex;
		flex: none;
		margin-left: auto;
	}
	.waypoints .tools :global(button) {
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
</style>
