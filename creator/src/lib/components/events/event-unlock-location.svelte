<script lang="ts">
	import type { Path } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { TARGETS } from '$lib/editor/address-targets.ts';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import AddressField from '$lib/components/fields/address-field.svelte';

	/**
	 * unlockLocation (agents/MAP.md §3.1, §8.5a): a place of the world map is discovered – the reader gets a toast; in a VN
	 * script, a click trigger or an item tap the map opens, flies to the place and closes again. `silent`: the state only;
	 * `map`: open (also when scrolled into) / toast only / empty = by the trigger (owner 2026-10-11).
	 * `avatar`: the same place field for setAvatarLocation (owner 2026-10-15) – the reader's avatar stands there, nothing else.
	 * The place: ONE typeahead over the toon's address book (#477); another toon's place (allowed – a plain marker there)
	 * is typed as its id.
	 */
	let { doc, path, avatar = false }: { doc: ContentDoc; path: Path; avatar?: boolean } = $props();
</script>

<div class="sc-fields" data-cols="2" data-density="dense" data-align="end">
	<AddressField {doc} path={[...path, 'params']} kinds={TARGETS.location.kinds} fields={TARGETS.location.fields} free="location" label={m['creator.unlock_location_place']()} testid="location" />
	{#if !avatar}
		<PropField {doc} path={[...path, 'params', 'silent']} label={m['creator.unlock_location_silent']()} type="bool" placeholder="false" />
		<!-- how the map shows it (owner 2026-10-11): empty = by the trigger -->
		<PropField {doc} path={[...path, 'params', 'map']} label={m['creator.unlock_location_map']()} type="select" options={[{ value: 'open', label: m['creator.unlock_location_map_open']() }, { value: 'toast', label: m['creator.unlock_location_map_toast']() }]} placeholder={m['creator.unlock_location_map_auto']()} />
	{/if}
</div>
<p class="hint">{avatar ? m['creator.set_avatar_location_hint']() : m['creator.unlock_location_hint']()}</p>
