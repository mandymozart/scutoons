<script lang="ts">
	import { mapName, type Path } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { page } from '$app/state';
	import { mapPlaces } from '$lib/editor/map.ts';
	import PropField from '$lib/components/fields/prop-field.svelte';

	/**
	 * unlockLocation (agents/MAP.md §3.1, §8.5a): a place of the world map is discovered – the reader gets a toast; in a VN
	 * script, a click trigger or an item tap the map opens, flies to the place and closes again. `silent`: the state only;
	 * `map`: open (also when scrolled into) / toast only / empty = by the trigger (owner 2026-10-11).
	 * `avatar`: the same place picker for setAvatarLocation (owner 2026-10-15) – the reader's avatar stands there, nothing else.
	 */
	let { doc, path, event, avatar = false }: { doc: ContentDoc; path: Path; event: any; avatar?: boolean } = $props();

	let places = $state<{ id: string; name: unknown; toons: string[] }[]>([]);
	/** the toon of the page: its places first, the other toons' after them (allowed – shown as plain markers there) */
	const toon = $derived(String(page.params.toonId ?? ''));
	$effect(() => {
		mapPlaces().then((list) => (places = list));
	});
	const options = $derived.by(() => {
		const name = (p: { id: string; name: unknown }) => mapName(p.name as never, getLocale()) || p.id;
		const own = places.filter((p) => !toon || p.toons.includes(toon)).map((p) => ({ value: p.id, label: name(p) })).sort((a, b) => a.label.localeCompare(b.label));
		const others = places.filter((p) => toon && !p.toons.includes(toon)).map((p) => ({ value: p.id, label: `${name(p)} · ${m['creator.unlock_location_other_toon']({ toons: p.toons.join(', ') || '–' })}` })).sort((a, b) => a.label.localeCompare(b.label));
		const list = [...own, ...others];
		const current = event?.params?.location;
		// a place that is not (or no longer) on the map stays visible as its id
		if (current && !list.some((o) => o.value === current)) list.unshift({ value: current, label: current });
		return list;
	});
</script>

<div class="sc-fields" data-cols="2" data-density="dense" data-align="end">
	<PropField {doc} path={[...path, 'params', 'location']} label={m['creator.unlock_location_place']()} type="select" {options} placeholder={m['creator.unlock_location_pick']()} />
	{#if !avatar}
		<PropField {doc} path={[...path, 'params', 'silent']} label={m['creator.unlock_location_silent']()} type="bool" placeholder="false" />
		<!-- how the map shows it (owner 2026-10-11): empty = by the trigger -->
		<PropField {doc} path={[...path, 'params', 'map']} label={m['creator.unlock_location_map']()} type="select" options={[{ value: 'open', label: m['creator.unlock_location_map_open']() }, { value: 'toast', label: m['creator.unlock_location_map_toast']() }]} placeholder={m['creator.unlock_location_map_auto']()} />
	{/if}
</div>
<p class="hint">{avatar ? m['creator.set_avatar_location_hint']() : m['creator.unlock_location_hint']()}</p>
