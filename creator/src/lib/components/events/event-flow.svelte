<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import EventTarget from '$lib/components/events/event-target.svelte';

	/**
	 * The story-logic events (CUTOVER 10.27): jumpTo (a target, wait), label (a named point a divert continues at),
	 * divert (to a point), setVariable (set or add).
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
		<PropField {doc} path={[...path, 'params', 'wait']} label="wait (tap)" type="bool" placeholder="false" />
	{:else if event.type === 'label'}
		<PropField {doc} path={[...path, 'params', 'name']} label={m['creator.vn_point_name']()} required />
	{:else if event.type === 'divert'}
		<PropField {doc} path={[...path, 'params', 'to']} label={m['creator.vn_point']()} type="select" options={pointNames} required />
	{:else if event.type === 'setVariable'}
		<PropField {doc} path={[...path, 'params', 'variable']} label="variable" required />
		<PropField {doc} path={[...path, 'params', 'op']} label="op" type="select" options={[{ value: 'add', label: 'add (+)' }]} placeholder="set (=)" />
		<PropField {doc} path={[...path, 'params', 'value']} label="value" type={event.params?.op === 'add' ? 'number' : 'text'} required />
	{/if}
</div>
{#if event.type === 'label'}
	<p class="hint">{m['creator.vn_point_hint']()}</p>
{:else if event.type === 'divert' && !pointNames.length}
	<p class="hint">{m['creator.vn_no_points']()}</p>
{/if}
