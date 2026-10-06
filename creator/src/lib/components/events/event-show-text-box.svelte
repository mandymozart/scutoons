<script lang="ts">
	import { Segmented } from '@scutoons/ui';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { getAt, TextBoxEffect, TriggerMode, type Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { REVEAL_PRESETS, applyRevealPreset, matchRevealPreset, type RevealPreset } from '$lib/editor/event-presets.ts';
	import PropField from '$lib/components/fields/prop-field.svelte';

	/**
	 * showTextBox: a text box this event reveals (the box gives the look, the event the timing) – pick one or create
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
	const effectOptions = () => [
		{ value: TextBoxEffect.NONE, label: m['creator.reveal_effect_none']() },
		{ value: TextBoxEffect.SHIVER, label: m['creator.reveal_preset_shiver']() },
		{ value: TextBoxEffect.PULSE, label: m['creator.reveal_preset_pulse']() },
		{ value: TextBoxEffect.WOBBLE, label: m['creator.reveal_preset_wobble']() }
	];
	const ono = $derived(matchRevealPreset(event.params));

	/** the reveal preset (effect + timing); the text and mode / wait stay */
	function setPreset(preset: RevealPreset) {
		doc.edit((file) => {
			const e = getAt(file, path);
			if (!e) return;
			e.params ??= {};
			applyRevealPreset(e.params, preset);
		}, `${path.join('.')}.reveal-preset`);
	}
</script>

<div class="sc-fields" data-cols="3" data-density="dense" data-align="end">
	<PropField
		{doc}
		path={[...path, 'params', 'dialog']}
		label={m['creator.text_box']()}
		type="select"
		options={[...dialogs.map((d) => ({ value: d.id, label: d.label })), ...(box && !dialogs.some((d) => d.id === box) ? [{ value: box, label: `${box} (?)` }] : [])]}
		placeholder={m['creator.reveal_choose_box']()}
	/>
	{#if oncreatebox && !box}
		<button type="button" class="secondary" onclick={() => oncreatebox(index)} data-testid="reveal-create-box">{m['creator.reveal_create_box']()}</button>
	{/if}
</div>
<Segmented
	grid
	min="5.5rem"
	label={m['creator.reveal_preset']()}
	value={ono}
	onchange={(id) => setPreset(REVEAL_PRESETS.find((p) => p.id === id)!)}
	options={REVEAL_PRESETS.map((p) => ({ value: p.id, label: m[`creator.reveal_preset_${p.id}`](), testid: `reveal-preset-${p.id}` }))}
/>
<details class="advanced">
	<summary>{m['creator.advanced']()}</summary>
	{#if box}<p class="hint">{m['creator.reveal_box_hint']({ id: box })}</p>{/if}
	<div class="sc-fields" data-cols="3" data-density="dense" data-align="end">
		<PropField {doc} path={[...path, 'params', 'effect']} label={m['creator.reveal_effect']()} type="select" options={effectOptions()} placeholder={m['creator.reveal_effect_none']()} />
		<PropField {doc} path={[...path, 'params', 'duration']} label={m['creator.field_duration']()} type="number" step={100} placeholder="2000" />
		<PropField {doc} path={[...path, 'params', 'delay']} label={m['creator.field_delay']()} type="number" step={100} placeholder="0" />
		<PropField {doc} path={[...path, 'params', 'mode']} label={m['creator.field_mode']()} type="select" options={Object.values(TriggerMode)} placeholder="poly" />
		<PropField {doc} path={[...path, 'params', 'wait']} label={m['creator.vn_wait']()} type="bool" placeholder="false" />
		<PropField
			{doc}
			path={[...path, 'params', 'stay']}
			label={m['creator.reveal_after']()}
			type="select"
			options={[{ value: false, label: m['creator.reveal_disappears']() }]}
			placeholder={m['creator.reveal_stays']()}
		/>
	</div>
</details>
