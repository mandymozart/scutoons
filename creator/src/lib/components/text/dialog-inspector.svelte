<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { Segmented } from '@scutoons/ui';
	import { getAt, type Path } from '@scutoons/content';
	import { TEXT_PRESETS, applyTextPreset, matchTextPreset, type TextPreset } from '$lib/editor/text-presets.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import LocaleText from '$lib/components/fields/locale-text.svelte';

	let {
		doc,
		path,
		onremove
	}: {
		doc: ContentDoc;
		/** path of the dialog entry */
		path: Path;
		onremove?: () => void;
	} = $props();

	const dialog = $derived(doc.get(doc.primary, path));
	const variant = $derived(dialog?.variant ?? 'bubble');
	const boxed = $derived(variant === 'caption' || variant === 'narration');
	const p = (...keys: (string | number)[]) => [...path, ...keys];
	/** the preset the look matches (none = changed by hand) */
	const preset = $derived(matchTextPreset(dialog));

	/** a preset's look, in every locale (words, id, position and size stay; owner 2026-10-03) */
	function setPreset(next: TextPreset) {
		doc.edit((file) => {
			const d = getAt(file, path);
			if (d) applyTextPreset(d, next);
		}, `${path.join('.')}.preset`);
	}

</script>

{#if dialog}
	<div class="inspector" data-testid="dialog-inspector">
		<div class="row head">
			<h3>{m['creator.text_boxes']()} · {dialog.id ?? '–'}</h3>
			{#if onremove}<button class="secondary" onclick={onremove}>{m['creator.remove']()}</button>{/if}
		</div>

		<!-- the look: one of the presets (owner 2026-10-03); the fields below fine-tune it -->
		<Segmented
			grid
			label={m['creator.preset']()}
			value={preset}
			onchange={(id) => setPreset(TEXT_PRESETS.find((t) => t.id === id)!)}
			options={TEXT_PRESETS.map((t) => ({ value: t.id, label: m[`creator.preset_${t.id}`](), testid: `dialog-preset-${t.id}` }))}
		/>

		<LocaleText {doc} path={p('text')} rows={4} />

		<!-- two fields per row (the reader's type size needs the width, DESIGN.md "Brutalist tools") -->
		<!-- the fine-tuning, folded (owner 2026-10-03: "where we have presets … an advanced collapsible"); position and size
		     are usually set on the canvas -->
		<details class="advanced" data-testid="dialog-advanced">
		<summary>{m['creator.advanced']()}</summary>
		<div class="sc-fields" data-cols="2" data-density="panel">
			<PropField {doc} path={p('rect', 'x')} label="x %" type="number" required />
			<PropField {doc} path={p('rect', 'y')} label="y %" type="number" required />
			<PropField {doc} path={p('rect', 'width')} label="width %" type="number" required />
			<PropField {doc} path={p('rect', 'height')} label="height %" type="number" placeholder="auto" />
		</div>

		<div class="sc-fields" data-cols="2" data-density="panel">
			<PropField {doc} path={p('id')} label="id" placeholder="s1c2-3-1" />
			<PropField {doc} path={p('size')} label="size (% width)" type="number" placeholder="4" />
			<PropField {doc} path={p('align')} label="align" type="select" options={['left', 'center', 'right']} placeholder={boxed ? 'left' : 'center'} />
			<PropField {doc} path={p('valign')} label="valign" type="select" options={['top', 'middle', 'bottom']} placeholder="middle" />
			<PropField {doc} path={p('font')} label="font" type="select" options={['lettering', 'mono']} placeholder={boxed ? 'mono' : 'lettering'} />
			<PropField {doc} path={p('weight')} label="weight" type="select" options={[400, 700]} placeholder="400" />
			<PropField {doc} path={p('uppercase')} label="uppercase" type="bool" placeholder="false" />
			<PropField {doc} path={p('letterSpacing')} label={m['creator.text_letter_spacing']()} type="number" step={0.01} min={-0.5} max={2} placeholder={(dialog.font ?? (boxed ? 'mono' : 'lettering')) === 'lettering' ? '0.03' : '0'} />
			<PropField {doc} path={p('rotation')} label="rotation °" type="number" step={1} placeholder="0" />
			<PropField {doc} path={p('scale')} label="scale" type="number" step={0.05} min={0.1} max={10} placeholder="1" />
			<PropField {doc} path={p('color')} label="color" type="color" placeholder="#000000" />
			<PropField {doc} path={p('stroke')} label={m['creator.text_outline']()} type="color" placeholder={m['creator.text_outline_none']()} />
			{#if dialog.stroke}
				<PropField {doc} path={p('strokeWidth')} label={m['creator.text_outline_width']()} type="number" step={0.02} min={0} placeholder="0.14" />
			{/if}
			{#if boxed}
				<PropField {doc} path={p('frame')} label={m['creator.text_box_frame']()} type="bool" placeholder="true" />
			{/if}
		</div>
		</details>

		<!-- deprecated (owner 2026-10-03): the art is re-imported without baked text; only shown where a box still has a mask -->
		{#if dialog.mask !== undefined}
		<details open>
			<summary>mask (deprecated – cover baked text; remove once the text-free art is in)</summary>
			<div class="sc-fields" data-cols="2" data-density="panel">
				<PropField {doc} path={p('mask')} label="mask" type="select" options={[{ value: true, label: 'true (#fff)' }, '#ffffff', '#000000']} placeholder="none" />
				<PropField {doc} path={p('maskShape')} label="maskShape" type="select" options={['rect', 'ellipse']} placeholder="rect" />
				<PropField {doc} path={p('pad')} label="pad (% width)" type="number" placeholder="0.5" />
			</div>
		</details>
		{/if}
	</div>
{/if}

<style>
	.advanced[open] {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
	.inspector {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: var(--space-05);
	}
	h3 {
		margin: 0;
	}
</style>
