<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { FlowLayout, Path } from '@scutoons/content';
	import { groupLayout, rendererOf, setAt } from '@scutoons/content';
	import { CloseIcon, Icon } from '@scutoons/ui';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import EventTarget from '$lib/components/events/event-target.svelte';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * Group settings: label, layout (the renderer), linked groups (owner 2026-10-06: `continues`, and `end` on a group
	 * that doesn't continue – @scutoons/content chains.ts). For ref groups the doc is the referenced scene. Compact
	 * (owner 2026-10-06): one tight field grid, explanations as the labels' tooltips.
	 */
	let {
		doc,
		groupPath,
		refSrc = null,
		toon = '',
		last = false,
		chapterNumber = 0,
		groupLabels = []
	}: {
		doc: ContentDoc;
		groupPath: Path;
		refSrc?: string | null;
		toon?: string;
		/** the chapter's last group: nothing to continue into */
		last?: boolean;
		/** the chapter's number (a new end points to the next chapter) */
		chapterNumber?: number;
		/** the chapter's group labels (the end's target group) */
		groupLabels?: string[];
	} = $props();

	const group = $derived(doc.get(doc.primary, groupPath));
	const layout = $derived(groupLayout(group));
	const p = (...keys: (string | number)[]) => [...groupPath, ...keys];

	const addEnd = () => doc.set(p('end'), { target: { type: 'chapter', value: String(chapterNumber + 1) } });
	const removeEnd = () => doc.set(p('end'), undefined);
	// the Layout select (owner 2026-10-16, FEATURES 10.100 §8) writes the whole renderer in one undo step: Vertical and
	// Horizontal (the pinned strip) without params, Endless with `endless`. The preset of a group turned horizontal
	// (owner 2026-10-06, both kinds): it continues (strips read best inside a chain) – only when nothing is set yet, a
	// chosen continues / end stays.
	const setLayout = (next: unknown) => {
		const preset = layout === 'vertical' && next !== 'vertical' && !last && group?.continues === undefined && !group?.end;
		doc.edit((file) => {
			setAt(file, p('renderer'), rendererOf(next as FlowLayout));
			if (preset) setAt(file, p('continues'), true);
		}, p('renderer').join('.'));
	};
	const layouts = $derived([
		{ value: 'vertical', label: m['creator.layout_vertical']() },
		{ value: 'strip', label: m['creator.layout_horizontal']() },
		{ value: 'endless', label: m['creator.layout_endless']() }
	]);
</script>

<section class="group-panel" data-testid="group-panel">
	<!-- the label is the name; the scene's storage key is an internal ID (owner 2026-10-03) -->
	<h3>{m['creator.group_named']({ label: group?.label ?? '' })}</h3>
	<!-- compact (owner 2026-10-06): one tight grid, the explanations in the labels' tooltips -->
	<div class="sc-fields" data-cols="fill" data-density="panel" data-align="end">
		<PropField {doc} path={p('label')} label={m['creator.group_name']()} title={m['creator.group_name_hint']()} />
		<PropField {doc} path={p('renderer')} label={m['creator.group_layout']()} title={m['creator.group_layout_hint']()} type="select" options={layouts} required source={{ value: layout, write: setLayout }} testid="group-layout" />
		{#if !last || group?.continues !== undefined}
			<PropField {doc} path={p('continues')} label={m['creator.group_continues']()} title={m['creator.group_continues_hint']()} type="bool" placeholder="false" />
		{/if}
		<!-- the group's end (only where the page ends: not on a group that continues) -->
		{#if group?.continues !== true && !group?.end}
			<div class="add-end">
				<button type="button" class="secondary" onclick={addEnd} title={m['creator.group_end_hint']()} data-testid="group-end-add"><Icon name="plus" /> {m['creator.group_end_add']()}</button>
			</div>
		{/if}
	</div>
	{#if group?.continues !== true && group?.end}
		<fieldset class="end" data-testid="group-end">
			<legend title={m['creator.group_end_hint']()}>{m['creator.group_end']()}</legend>
			<div class="sc-fields" data-cols="fill" data-density="panel" data-align="end">
				<EventTarget {doc} path={p('end', 'target')} target={group.end.target} {groupLabels} />
				<PropField {doc} path={p('end', 'auto')} label={m['creator.group_end_auto']()} title={m['creator.group_end_auto_hint']()} type="bool" placeholder="false" />
			</div>
			<button type="button" class="sc-round remove" onclick={removeEnd} title={m['creator.group_end_remove']()} aria-label={m['creator.group_end_remove']()}><CloseIcon /></button>
		</fieldset>
	{/if}
</section>

<style>
	.group-panel {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
	h3 {
		margin: 0;
	}
	/* a field group without a frame (legend = label), the remove button at its top right */
	.end {
		position: relative;
		margin: 0;
		padding: 0;
		border: 0;
		min-width: 0;
	}
	.end .remove {
		position: absolute;
		top: 0;
		right: 0;
	}
</style>
