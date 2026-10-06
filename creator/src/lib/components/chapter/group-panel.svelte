<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { Path } from '@scutoons/content';
	import { Renderer } from '@scutoons/content';
	import { CloseIcon, Icon } from '@scutoons/ui';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import EventTarget from '$lib/components/events/event-target.svelte';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * Group settings: label, renderer type + params, linked groups (owner 2026-10-06: `continues`, and `end` on a group
	 * that doesn't continue – @scutoons/content chains.ts). For ref groups the doc is the referenced scene.
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
	const params = $derived(group?.renderer?.params ?? {});
	const p = (...keys: (string | number)[]) => [...groupPath, ...keys];

	const addEnd = () => doc.set(p('end'), { target: { type: 'chapter', value: String(chapterNumber + 1) } });
	const removeEnd = () => doc.set(p('end'), undefined);
</script>

<section class="group-panel" data-testid="group-panel">
	<!-- the label is the name; the scene's storage key is an internal ID (owner 2026-10-03) -->
	<h3>{m['creator.group_named']({ label: group?.label ?? '' })}</h3>
	<div class="sc-fields" data-cols="3" data-density="panel">
		<PropField {doc} path={p('label')} label={m['creator.group_name']()} title={m['creator.group_name_hint']()} />
		<PropField {doc} path={p('renderer', 'type')} label={m['creator.group_layout']()} type="select" options={[{ value: Renderer.VERTICAL, label: m['creator.layout_vertical']() }, { value: Renderer.HORIZONTAL, label: m['creator.layout_horizontal']() }]} required />
		{#if group?.interaction !== undefined}
			<PropField {doc} path={p('interaction')} label={m['creator.group_interaction']()} />
		{/if}
		{#if !last || group?.continues !== undefined}
			<PropField {doc} path={p('continues')} label={m['creator.group_continues']()} type="bool" placeholder="false" hint={m['creator.group_continues_hint']()} />
		{/if}
	</div>
	<!-- the group's end (only where the page ends: not on a group that continues) -->
	{#if group?.continues !== true}
		{#if group?.end}
			<fieldset class="end" data-testid="group-end">
				<legend title={m['creator.group_end_hint']()}>{m['creator.group_end']()}</legend>
				<div class="sc-fields" data-cols="3" data-density="panel">
					<EventTarget {doc} path={p('end', 'target')} target={group.end.target} {groupLabels} />
					<PropField {doc} path={p('end', 'auto')} label={m['creator.group_end_auto']()} type="bool" placeholder="false" hint={m['creator.group_end_auto_hint']()} />
				</div>
				<button type="button" class="sc-round remove" onclick={removeEnd} title={m['creator.group_end_remove']()} aria-label={m['creator.group_end_remove']()}><CloseIcon /></button>
			</fieldset>
		{:else}
			<div>
				<button type="button" class="secondary" onclick={addEnd} title={m['creator.group_end_hint']()} data-testid="group-end-add"><Icon name="plus" /> {m['creator.group_end_add']()}</button>
			</div>
		{/if}
	{/if}
	<!-- the swiper's settings only for a horizontal group (owner 2026-10-03); full width (owner 2026-10-06) replaces the
	     swiper with the screen-high strip, so its settings are hidden then -->
	{#if group?.renderer?.type === Renderer.HORIZONTAL}
	<div class="sc-fields" data-cols="3" data-density="panel">
		<PropField {doc} path={p('renderer', 'params', 'fullWidth')} label={m['creator.group_full_width']()} type="bool" placeholder="false" hint={m['creator.group_full_width_hint']()} />
	</div>
	{#if !params.fullWidth}
	<details open={Object.keys(params).length > 0}>
		<summary>{m['creator.swiper']()}</summary>
		<div class="sc-fields" data-cols="4" data-density="panel">
			<PropField {doc} path={p('renderer', 'params', 'autoPlay', 'enabled')} label={m['creator.swiper_autoplay']()} type="bool" placeholder="false" />
			<PropField {doc} path={p('renderer', 'params', 'autoPlay', 'delay')} label={m['creator.swiper_delay']()} type="number" step={100} placeholder="3000" />
			<PropField {doc} path={p('renderer', 'params', 'loop')} label={m['creator.swiper_loop']()} type="bool" placeholder="false" />
			<PropField {doc} path={p('renderer', 'params', 'flip')} label={m['creator.swiper_flip']()} type="bool" placeholder="false" />
			<PropField {doc} path={p('renderer', 'params', 'cube')} label={m['creator.swiper_cube']()} type="bool" placeholder="false" />
			<PropField {doc} path={p('renderer', 'params', 'keyboard')} label={m['creator.swiper_keyboard']()} type="bool" placeholder="false" />
			<PropField {doc} path={p('renderer', 'params', 'freeMode')} label={m['creator.swiper_free']()} type="bool" placeholder="false" />
		</div>
	</details>
	{/if}
	{/if}
</section>

<style>
	.group-panel {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
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
