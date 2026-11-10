<script lang="ts">
	import type { Path } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { TARGETS } from '$lib/editor/address-targets.ts';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import AddressField from '$lib/components/fields/address-field.svelte';

	/**
	 * migrate (agents/MAP.md §8.10, owner 2026-10-12): the reader's avatar (no `who`) or a character is registered in a
	 * region of the world map; then the welcome – the region's sign, its representative (or the narrator) saying the
	 * greeting. `silent`: the registration only. The region: ONE typeahead over the toon's address book (#477 slice D; a
	 * region no longer on the map stays shown as its id, dashed).
	 */
	let { doc, path, event, charOptions }: { doc: ContentDoc; path: Path; event: any; charOptions: { value: string; label: string }[] } = $props();

	const whoOptions = $derived.by(() => {
		const current = event?.params?.who;
		const list = [...charOptions];
		if (current && !list.some((o) => o.value === current)) list.unshift({ value: current, label: current });
		return list;
	});
</script>

<div class="sc-fields" data-cols="3" data-density="dense" data-align="end">
	<AddressField {doc} path={[...path, 'params']} spec={TARGETS.region} label={m['creator.migrate_region']()} testid="migrate-region" />
	<!-- empty = the reader's own avatar -->
	<PropField {doc} path={[...path, 'params', 'who']} label={m['creator.migrate_who']()} type="select" options={whoOptions} placeholder={m['creator.migrate_reader']()} />
	<PropField {doc} path={[...path, 'params', 'silent']} label={m['creator.migrate_silent']()} type="bool" placeholder="false" />
</div>
<p class="hint">{m['creator.migrate_hint']()}</p>
