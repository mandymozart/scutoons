<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { Path } from '@scutoons/content';
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
	const p = (...keys: (string | number)[]) => [...path, ...keys];
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
			<PropField {doc} path={p('variant')} label="variant" type="select" options={['bubble', 'caption', 'narration', 'sfx']} placeholder="bubble (default)" />
			<PropField {doc} path={p('size')} label="size (% width)" type="number" placeholder="4" />
			<PropField {doc} path={p('align')} label="align" type="select" options={['left', 'center', 'right']} placeholder={boxed ? 'left' : 'center'} />
			<PropField {doc} path={p('valign')} label="valign" type="select" options={['top', 'middle', 'bottom']} placeholder="middle" />
			<PropField {doc} path={p('font')} label="font" type="select" options={['lettering', 'mono']} placeholder={boxed ? 'mono' : 'lettering'} />
			<PropField {doc} path={p('weight')} label="weight" type="select" options={[400, 700]} placeholder="400" />
			<PropField {doc} path={p('uppercase')} label="uppercase" type="bool" placeholder={variant === 'sfx' ? 'true (sfx)' : 'false'} />
			<PropField {doc} path={p('rotation')} label="rotation °" type="number" step={1} placeholder="0" />
			<PropField {doc} path={p('color')} label="color" type="color" placeholder={variant === 'sfx' ? '#ffffff' : '#000000'} />
			{#if boxed}
				<PropField {doc} path={p('frame')} label="frame" type="bool" placeholder="true" />
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
		gap: 0.6rem;
	}
	.row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.5rem;
	}
	h3 {
		font-size: 12px;
		margin: 0;
	}
	.grid {
		display: grid;
		gap: 0.4rem 0.5rem;
	}
	.g4 {
		grid-template-columns: repeat(4, minmax(0, 1fr));
	}
	.g3 {
		grid-template-columns: repeat(3, minmax(0, 1fr));
	}
	summary {
		cursor: pointer;
		font-size: 11px;
		margin-bottom: 0.3rem;
	}
</style>
