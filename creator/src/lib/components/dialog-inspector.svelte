<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { DIALOG_TAILS, getAt, type Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import PropField from './prop-field.svelte';
	import LocaleText from './locale-text.svelte';

	let {
		doc,
		path,
		editLocale = null,
		onremove
	}: {
		doc: ContentDoc;
		/** path of the dialog entry */
		path: Path;
		/** geometry scope: null = all locales, else only this locale */
		editLocale?: string | null;
		onremove?: () => void;
	} = $props();

	const dialog = $derived(doc.get(doc.primary, path));
	const variant = $derived(dialog?.variant ?? 'bubble');
	const boxed = $derived(variant === 'caption' || variant === 'narration');
	/** a drawn speech bubble (bubble + frame) */
	const drawnBubble = $derived(variant === 'bubble' && dialog?.frame === true);
	const p = (...keys: (string | number)[]) => [...path, ...keys];

	const VARIANTS = $derived([
		{ value: 'bubble', label: m['creator.variant_bubble']() },
		{ value: 'caption', label: m['creator.variant_caption']() },
		{ value: 'narration', label: m['creator.variant_narration']() }
	]);

	/** the variant, in every locale; a bubble on text-free art gets its drawn shape (frame) right away */
	function setVariant(next: string) {
		doc.structural((file) => {
			const d = getAt(file, path);
			if (!d) return;
			d.variant = next;
			if (next === 'bubble' && d.frame === undefined) d.frame = true;
		}, `${path.join('.')}.variant`);
	}

</script>

{#if dialog}
	<div class="inspector" data-testid="dialog-inspector">
		<div class="row head">
			<h3>{m['creator.text_boxes']()} · {dialog.id ?? '–'}</h3>
			{#if onremove}<button class="secondary" onclick={onremove}>{m['creator.remove']()}</button>{/if}
		</div>

		<LocaleText {doc} path={p('text')} rows={4} />

		<div class="grid g4">
			<PropField {doc} path={p('rect', 'x')} label="x %" type="number" required editLocale={editLocale} />
			<PropField {doc} path={p('rect', 'y')} label="y %" type="number" required editLocale={editLocale} />
			<PropField {doc} path={p('rect', 'width')} label="width %" type="number" required editLocale={editLocale} />
			<PropField {doc} path={p('rect', 'height')} label="height %" type="number" placeholder="auto" editLocale={editLocale} />
		</div>
		{#if editLocale}<p class="hint">Geometry: only {editLocale} (override).</p>{/if}

		<div class="grid g3">
			<PropField {doc} path={p('id')} label="id" placeholder="s1c2-3-1" />
			<div class="field">
				<label for="variant-{path.join('-')}">variant</label>
				<select id="variant-{path.join('-')}" value={dialog.variant ?? 'bubble'} onchange={(e) => setVariant(e.currentTarget.value)} data-testid="dialog-variant">
					{#each VARIANTS as v (v.value)}<option value={v.value}>{v.label}</option>{/each}
				</select>
			</div>
			<PropField {doc} path={p('size')} label="size (% width)" type="number" placeholder="4" />
			<PropField {doc} path={p('align')} label="align" type="select" options={['left', 'center', 'right']} placeholder={boxed ? 'left' : 'center'} />
			<PropField {doc} path={p('valign')} label="valign" type="select" options={['top', 'middle', 'bottom']} placeholder="middle" />
			<PropField {doc} path={p('font')} label="font" type="select" options={['lettering', 'mono']} placeholder={boxed ? 'mono' : 'lettering'} />
			<PropField {doc} path={p('weight')} label="weight" type="select" options={[400, 700]} placeholder="400" />
			<PropField {doc} path={p('uppercase')} label="uppercase" type="bool" placeholder="false" />
			<PropField {doc} path={p('rotation')} label="rotation °" type="number" step={1} placeholder="0" />
			<PropField {doc} path={p('scale')} label="scale" type="number" step={0.05} min={0.1} max={10} placeholder="1" />
			<PropField {doc} path={p('color')} label="color" type="color" placeholder="#000000" />
			<PropField {doc} path={p('stroke')} label={m['creator.text_outline']()} type="color" placeholder={m['creator.text_outline_none']()} />
			{#if dialog.stroke}
				<PropField {doc} path={p('strokeWidth')} label={m['creator.text_outline_width']()} type="number" step={0.02} min={0} placeholder="0.14" />
			{/if}
			{#if boxed}
				<PropField {doc} path={p('frame')} label="frame" type="bool" placeholder="true" />
			{:else if variant === 'bubble'}
				<PropField {doc} path={p('frame')} label={m['creator.bubble_frame']()} type="bool" placeholder="false" />
				{#if drawnBubble}
					<PropField {doc} path={p('tail')} label={m['creator.bubble_tail']()} type="select" options={DIALOG_TAILS.filter((t) => t !== 'bottom-left')} placeholder="bottom-left" />
				{/if}
			{/if}
		</div>

		<details open={dialog.mask !== undefined}>
			<summary>mask (cover baked text)</summary>
			<div class="grid g3">
				<PropField {doc} path={p('mask')} label="mask" type="select" options={[{ value: true, label: 'true (#fff)' }, '#ffffff', '#000000']} placeholder="none" />
				<PropField {doc} path={p('maskShape')} label="maskShape" type="select" options={['rect', 'ellipse']} placeholder="rect" />
				<PropField {doc} path={p('pad')} label="pad (% width)" type="number" placeholder="0.5" />
			</div>
		</details>
	</div>
{/if}

<style>
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
	/* two fields per row: the reader's type size needs the width (DESIGN.md "Brutalist tools") */
	.grid {
		display: grid;
		gap: var(--space-075) var(--space-1);
	}
	.g4,
	.g3 {
		grid-template-columns: repeat(2, minmax(0, 1fr));
	}
	summary {
		margin-bottom: var(--space-05);
	}
	/* the variant select (its own change handler) – laid out like a PropField */
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		min-width: 0;
	}
	.field label {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
	}
</style>
