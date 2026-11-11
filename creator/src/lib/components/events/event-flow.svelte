<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import AddressField from '$lib/components/fields/address-field.svelte';
	import EventTarget from '$lib/components/events/event-target.svelte';
	import { TARGETS } from '$lib/editor/address-targets.ts';

	/**
	 * The story-logic events (CUTOVER 10.27): jumpTo (a target, wait), label (a named point a divert continues at),
	 * divert (to a point), setVariable (set or add; the variable is ONE typeahead over the toon's address book, #477 –
	 * a new one is typed as written: `x`, `global.x`, `toptoy.x`).
	 */
	let {
		doc,
		path,
		event,
		pointNames,
		groupLabels
	}: { doc: ContentDoc; path: Path; event: any; pointNames: string[]; groupLabels: string[] } = $props();
</script>

<div class="sc-fields" data-cols="3" data-density="dense" data-align="end">
	{#if event.type === 'jumpTo'}
		<EventTarget {doc} path={[...path, 'params', 'target']} target={event.params?.target} {groupLabels} />
		<PropField {doc} path={[...path, 'params', 'wait']} label={m['creator.vn_wait']()} type="bool" placeholder="false" />
	{:else if event.type === 'label'}
		<PropField {doc} path={[...path, 'params', 'name']} label={m['creator.vn_point_name']()} required />
	{:else if event.type === 'divert'}
		<PropField {doc} path={[...path, 'params', 'to']} label={m['creator.vn_point']()} type="select" options={pointNames} required />
	{:else if event.type === 'setVariable'}
		<AddressField {doc} path={[...path, 'params']} spec={TARGETS.variable} free="variable" label={m['creator.field_variable']()} testid="set-variable" />
		<PropField {doc} path={[...path, 'params', 'op']} label={m['creator.vn_action']()} type="select" options={[{ value: 'add', label: m['creator.vn_action_add']() }]} placeholder={m['creator.vn_action_set']()} />
		<PropField {doc} path={[...path, 'params', 'value']} label={m['creator.field_value']()} type={event.params?.op === 'add' ? 'number' : 'text'} required />
	{/if}
</div>
{#if event.type === 'label'}
	<p class="hint">{m['creator.vn_point_hint']()}</p>
{:else if event.type === 'divert' && !pointNames.length}
	<p class="hint">{m['creator.vn_no_points']()}</p>
{/if}
