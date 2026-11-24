<script lang="ts">
	import { Toggle } from '@scutoons/ui/editor';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { getAt, type Path, type Repeat } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { triggerOptions } from '$lib/editor/vn-forms.ts';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import RepeatsField from '$lib/components/fields/repeats-field.svelte';

	/**
	 * A slice event's trigger: click, hover or in view, how often (Repeats – every time, once per page view, once for good,
	 * every n …; agents/EXPRESSIONS.md §5); an area on the slice or the whole slice (no rect) – every
	 * trigger takes either (owner 2026-10-03).
	 */
	let { doc, path, event, index, onselect }: { doc: ContentDoc; path: Path; event: any; index: number; onselect?: (index: number) => void } = $props();

	/**
	 * How often: events with the same trigger run as one (the reader groups them), so the rule changes for all of them –
	 * a message, its choices and the showChoices stay one run
	 */
	function setRepeat(next: Repeat | undefined) {
		doc.edit((file) => {
			const list = getAt(file, path.slice(0, -1));
			const own = getAt(file, [...path, 'trigger']);
			if (!Array.isArray(list) || !own) return;
			const key = JSON.stringify(own);
			for (const e of list) {
				if (!e?.trigger || JSON.stringify(e.trigger) !== key) continue;
				if (next) e.trigger.repeat = structuredClone(next);
				else delete e.trigger.repeat;
			}
		}, `${path.join('.')}.repeat`);
	}

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
	<RepeatsField id="repeat-{path.join('-')}" testid="event-repeat-{index}" value={event.trigger?.repeat} onchange={setRepeat} />
	<span class="whole"><Toggle shown on={!event.trigger?.rect} label={m['creator.event_whole_slice']()} title={m['creator.event_whole_slice_hint']()} onchange={setWholeSlice} data-testid="event-whole-{index}" /></span>
</div>

<style>
	.trigger {
		border-top: 1px dashed var(--tool-line);
		padding-top: 0.3rem;
	}
	.whole {
		display: flex;
		align-self: end;
	}
</style>
