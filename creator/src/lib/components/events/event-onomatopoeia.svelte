<script lang="ts">
	import { Segmented } from '@scutoons/ui';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { getAt, OnomatopoeiaEffect, TriggerMode, type Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { ONO_PRESETS, applyOnoPreset, matchOnoPreset, type OnoPreset } from '$lib/editor/event-presets.ts';
	import PropField from '$lib/components/fields/prop-field.svelte';

	/**
	 * showOnomatopeia: a text box this event reveals (the box gives the look, the event the timing) – pick one or create
	 * it, one preset for how it comes in (owner 2026-10-03), every timing field under Advanced.
	 */
	let {
		doc,
		path,
		event,
		index,
		dialogs,
		oncreatebox
	}: {
		doc: ContentDoc;
		path: Path;
		event: any;
		index: number;
		dialogs: { id: string; label: string }[];
		oncreatebox?: (index: number) => void;
	} = $props();

	const box = $derived(event.params?.dialog);
	const ono = $derived(matchOnoPreset(event.params));

	/** an onomatopoeia's preset (effect + timing); the text and mode / wait stay */
	function setPreset(preset: OnoPreset) {
		doc.edit((file) => {
			const e = getAt(file, path);
			if (!e) return;
			e.params ??= {};
			applyOnoPreset(e.params, preset);
		}, `${path.join('.')}.ono-preset`);
	}
</script>

<div class="sc-fields" data-cols="3" data-density="dense" data-align="end">
	<PropField
		{doc}
		path={[...path, 'params', 'dialog']}
		label={m['creator.ono_text_box']()}
		type="select"
		options={[...dialogs.map((d) => ({ value: d.id, label: d.label })), ...(box && !dialogs.some((d) => d.id === box) ? [{ value: box, label: `${box} (?)` }] : [])]}
		placeholder={m['creator.ono_choose_box']()}
	/>
	{#if oncreatebox && !box}
		<button type="button" class="secondary" onclick={() => oncreatebox(index)} data-testid="ono-create-box">{m['creator.ono_create_box']()}</button>
	{/if}
</div>
<Segmented
	grid
	min="5.5rem"
	label={m['creator.ono_preset']()}
	value={ono}
	onchange={(id) => setPreset(ONO_PRESETS.find((p) => p.id === id)!)}
	options={ONO_PRESETS.map((p) => ({ value: p.id, label: m[`creator.ono_preset_${p.id}`](), testid: `ono-preset-${p.id}` }))}
/>
<details class="advanced">
	<summary>{m['creator.advanced']()}</summary>
	{#if box}<p class="hint">{m['creator.ono_box_hint']({ id: box })}</p>{/if}
	<div class="sc-fields" data-cols="3" data-density="dense" data-align="end">
		<PropField {doc} path={[...path, 'params', 'effect']} label="effect" type="select" options={Object.values(OnomatopoeiaEffect)} placeholder="none" />
		<PropField {doc} path={[...path, 'params', 'duration']} label="duration ms" type="number" step={100} placeholder="2000" />
		<PropField {doc} path={[...path, 'params', 'delay']} label="delay ms" type="number" step={100} placeholder="0" />
		<PropField {doc} path={[...path, 'params', 'mode']} label="mode" type="select" options={Object.values(TriggerMode)} placeholder="poly" />
		<PropField {doc} path={[...path, 'params', 'wait']} label="wait (tap)" type="bool" placeholder="false" />
		<PropField
			{doc}
			path={[...path, 'params', 'stay']}
			label={m['creator.ono_after']()}
			type="select"
			options={[{ value: false, label: m['creator.ono_disappears']() }]}
			placeholder={m['creator.ono_stays']()}
		/>
	</div>
</details>
