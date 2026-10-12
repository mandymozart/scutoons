<script lang="ts">
	import { mapName, type Path } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { mapRegions } from '$lib/editor/map.ts';
	import PropField from '$lib/components/fields/prop-field.svelte';

	/**
	 * migrate (agents/MAP.md §8.10, owner 2026-10-12): the reader's avatar (no `who`) or a character is registered in a
	 * region of the world map; then the welcome – the region's sign, its representative (or the narrator) saying the
	 * greeting. `silent`: the registration only.
	 */
	let { doc, path, event, charOptions }: { doc: ContentDoc; path: Path; event: any; charOptions: { value: string; label: string }[] } = $props();

	let regions = $state<{ id: string; name: unknown }[]>([]);
	$effect(() => {
		mapRegions().then((list) => (regions = list));
	});
	const options = $derived.by(() => {
		const list = regions.map((r) => ({ value: r.id, label: mapName(r.name as never, getLocale()) || r.id })).sort((a, b) => a.label.localeCompare(b.label));
		const current = event?.params?.region;
		// a region that is not (or no longer) on the map stays visible as its id
		if (current && !list.some((o) => o.value === current)) list.unshift({ value: current, label: current });
		return list;
	});
	const whoOptions = $derived.by(() => {
		const current = event?.params?.who;
		const list = [...charOptions];
		if (current && !list.some((o) => o.value === current)) list.unshift({ value: current, label: current });
		return list;
	});
</script>

<div class="sc-fields" data-cols="3" data-density="dense" data-align="end">
	<PropField {doc} path={[...path, 'params', 'region']} label={m['creator.migrate_region']()} type="select" {options} placeholder={m['creator.migrate_pick']()} />
	<!-- empty = the reader's own avatar -->
	<PropField {doc} path={[...path, 'params', 'who']} label={m['creator.migrate_who']()} type="select" options={whoOptions} placeholder={m['creator.migrate_reader']()} />
	<PropField {doc} path={[...path, 'params', 'silent']} label={m['creator.migrate_silent']()} type="bool" placeholder="false" />
</div>
<p class="hint">{m['creator.migrate_hint']()}</p>
