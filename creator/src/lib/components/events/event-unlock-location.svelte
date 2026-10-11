<script lang="ts">
	import { mapName, type Path } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { mapPlaces } from '$lib/editor/map.ts';
	import PropField from '$lib/components/fields/prop-field.svelte';

	/**
	 * unlockLocation (agents/MAP.md §3.1, §8.5a): a place of the world map is discovered – the reader gets a toast; in a VN
	 * script, a click trigger or an item tap the map opens, flies to the place and closes again. `silent`: the state only.
	 */
	let { doc, path, event }: { doc: ContentDoc; path: Path; event: any } = $props();

	let places = $state<{ id: string; name: unknown }[]>([]);
	$effect(() => {
		mapPlaces().then((list) => (places = list));
	});
	const options = $derived.by(() => {
		const list = places.map((p) => ({ value: p.id, label: mapName(p.name as never, getLocale()) || p.id })).sort((a, b) => a.label.localeCompare(b.label));
		const current = event?.params?.location;
		// a place that is not (or no longer) on the map stays visible as its id
		if (current && !list.some((o) => o.value === current)) list.unshift({ value: current, label: current });
		return list;
	});
</script>

<div class="sc-fields" data-cols="2" data-density="dense" data-align="end">
	<PropField {doc} path={[...path, 'params', 'location']} label={m['creator.unlock_location_place']()} type="select" {options} placeholder={m['creator.unlock_location_pick']()} />
	<PropField {doc} path={[...path, 'params', 'silent']} label={m['creator.unlock_location_silent']()} type="bool" placeholder="false" />
</div>
<p class="hint">{m['creator.unlock_location_hint']()}</p>
