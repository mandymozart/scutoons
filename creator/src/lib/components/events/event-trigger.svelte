<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { getAt, type Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { triggerOptions } from '$lib/editor/vn-forms.ts';
	import PropField from '$lib/components/fields/prop-field.svelte';

	/**
	 * A slice event's trigger: click, hover or in view, once; an area on the slice or the whole slice (no rect) – every
	 * trigger takes either (owner 2026-10-03).
	 */
	let { doc, path, event, index, onselect }: { doc: ContentDoc; path: Path; event: any; index: number; onselect?: (index: number) => void } = $props();

	/** whole slice = no `trigger.rect`; off = a centred area to drag on the slice */
	function setWholeSlice(whole: boolean) {
		doc.edit((file) => {
			const trigger = getAt(file, [...path, 'trigger']);
			if (!trigger) return;
			if (whole) delete trigger.rect;
			else trigger.rect = { x: 10, y: 30, width: 80, height: 40 };
		}, `${path.join('.')}.area`);
		onselect?.(index);
	}
</script>

<div class="sc-fields trigger" data-cols="3" data-density="dense" data-align="end">
	<PropField {doc} path={[...path, 'trigger', 'type']} label={m['creator.trigger']()} type="select" options={triggerOptions()} required />
	<PropField {doc} path={[...path, 'trigger', 'once']} label={m['creator.trigger_once']()} type="bool" placeholder="false" />
	<label class="whole" title={m['creator.event_whole_slice_hint']()}>
		<input type="checkbox" checked={!event.trigger?.rect} onchange={(e) => setWholeSlice(e.currentTarget.checked)} data-testid="event-whole-{index}" />
		{m['creator.event_whole_slice']()}
	</label>
</div>

<style>
	.trigger {
		border-top: 1px dashed var(--border-muted);
		padding-top: 0.3rem;
	}
	.whole {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		align-self: end;
		min-height: var(--control-height);
	}
</style>
