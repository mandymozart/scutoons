<script lang="ts">
	import { Segmented } from '@scutoons/ui';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { FX_SOUNDS, getAt, TextBoxEffect, TriggerMode, type Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { REVEAL_PRESETS, applyRevealPreset, matchRevealPreset, type RevealPreset } from '$lib/editor/event-presets.ts';
	import PropField from '$lib/components/fields/prop-field.svelte';

	/**
	 * showTextBox: a text box this event reveals (the box gives the look, the event the timing) – one select "choose or
	 * create new" (owner 2026-10-06: the boxes by their text in the header's language, never the id; last "+ New text
	 * box" where the event has a slice), one preset for how it comes in (owner 2026-10-03), every timing field under
	 * Advanced.
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
		/** the boxes it may reveal: id (the value) + label (the start of the text) */
		dialogs: { id: string; label: string }[];
		/** "+ New text box": a new box on the event's slice, revealed by this event and selected (none in a group script) */
		oncreatebox?: (index: number) => void;
	} = $props();

	const box = $derived(event.params?.dialog);
	const NEW = '+new';
	const fieldId = $derived(`f-${path.join('-')}-dialog`);

	function pick(select: HTMLSelectElement) {
		const value = select.value;
		if (value === NEW) {
			// back to the current box until the new one is there (it is selected on the canvas, the text box tab opens)
			select.value = box ?? '';
			oncreatebox?.(index);
			return;
		}
		doc.set([...path, 'params', 'dialog'], value || undefined);
	}
	const effectOptions = () => [
		{ value: TextBoxEffect.NONE, label: m['creator.reveal_effect_none']() },
		{ value: TextBoxEffect.SHIVER, label: m['creator.reveal_preset_shiver']() },
		{ value: TextBoxEffect.PULSE, label: m['creator.reveal_preset_pulse']() },
		{ value: TextBoxEffect.WOBBLE, label: m['creator.reveal_preset_wobble']() },
		{ value: TextBoxEffect.POP, label: m['creator.reveal_preset_pop']() },
		{ value: TextBoxEffect.SLAM, label: m['creator.reveal_preset_slam']() },
		{ value: TextBoxEffect.FADE, label: m['creator.reveal_preset_fade']() },
		{ value: TextBoxEffect.TYPE_ON, label: m['creator.reveal_preset_typeOn']() }
	];
	/** the sound when it appears (agents/FX.md): the effect's own (placeholder), silent or one of the reader's sounds */
	const soundOptions = () => [{ value: 'none', label: m['creator.fx_sound_none']() }, ...FX_SOUNDS.map((s) => ({ value: s, label: m[`creator.fx_sound_${s}`]() }))];
	const preset = $derived(matchRevealPreset(event.params));

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
	<div class="field">
		<label class="sc-label" for={fieldId}>{m['creator.text_box']()}</label>
		<select id={fieldId} value={box ?? ''} onchange={(e) => pick(e.currentTarget)} data-testid="reveal-box">
			{#if !box}<option value="">{m['creator.reveal_choose_box']()}</option>{/if}
			{#each dialogs as d (d.id)}<option value={d.id}>{d.label}</option>{/each}
			{#if box && !dialogs.some((d) => d.id === box)}<option value={box}>{m['creator.reveal_box_missing']()}</option>{/if}
			{#if oncreatebox}<option value={NEW} data-testid="reveal-new-box">{m['creator.reveal_new_box']()}</option>{/if}
		</select>
	</div>
</div>
<Segmented
	grid
	min="5.5rem"
	label={m['creator.reveal_preset']()}
	value={preset}
	onchange={(id) => setPreset(REVEAL_PRESETS.find((p) => p.id === id)!)}
	options={REVEAL_PRESETS.map((p) => ({ value: p.id, label: m[`creator.reveal_preset_${p.id}`](), testid: `reveal-preset-${p.id}` }))}
/>
<div class="sc-fields" data-cols="3" data-density="dense" data-align="end">
	<PropField {doc} path={[...path, 'params', 'sound']} label={m['creator.fx_sound']()} type="select" options={soundOptions()} placeholder={m['creator.fx_sound_own']()} />
</div>
<details class="advanced">
	<summary>{m['creator.advanced']()}</summary>
	{#if box}<p class="hint">{m['creator.reveal_box_hint']()}</p>{/if}
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

<style>
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		min-width: 0;
		grid-column: 1 / -1;
	}
</style>
