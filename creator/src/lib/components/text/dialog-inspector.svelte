<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { ColorField, LinkedPair, PropRow, PropSelect, ScrubNumber, Segmented } from '@scutoons/ui';
	import { DEFAULT_SLICE, dialogFit, getAt, round2, type DialogFit, type Path } from '@scutoons/content';
	import { drawnHeights, drawnKey, drawnWidths } from '$lib/editor/drawn-heights.svelte.ts';
	import { TEXT_SIZE_STEP, TEXT_SIZE_UNITS, sizeToUnit, unitToSize } from '$lib/editor/text-size-unit.ts';
	import { textSizeUnit } from '$lib/editor/text-size-unit.svelte.ts';
	import { TEXT_PRESETS, applyTextPreset, matchTextPreset, type TextPreset } from '$lib/editor/text-presets.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { alignLabel } from '$lib/editor/cover-words.ts';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import LocaleText from '$lib/components/fields/locale-text.svelte';
	import AnchorField from '$lib/components/fields/anchor-field.svelte';
	import TailField from '$lib/components/text/tail-field.svelte';
	import { drawnBox } from '$lib/editor/tail-edit.ts';

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
	const boxed = $derived(variant === 'caption' || variant === 'narration' || variant === 'lyrics');
	const p = (...keys: (string | number)[]) => [...path, ...keys];
	const ALIGNS = ['left', 'center', 'right'] as const;
	const VALIGNS = ['top', 'middle', 'bottom'] as const;
	const valignLabel = (v: string) => ({ top: m['creator.video_position_top'](), middle: m['creator.video_position_center'](), bottom: m['creator.video_position_bottom']() })[v] ?? v;
	/** a field's writer for the compact controls (undefined removes the key – the default applies) */
	const set = (...keys: (string | number)[]) => (v: unknown) => doc.set(p(...keys), v);
	const font = $derived(dialog?.font ?? (boxed ? 'mono' : 'lettering'));
	let keepRatio = $state(false);

	/** the font size in the chosen unit (owner 2026-10-17: rem by default, pt / px to switch; stored as % of the slice) */
	const sliceWidth = $derived(doc.get(doc.primary, path.slice(0, -2))?.width || DEFAULT_SLICE.width);
	const unit = $derived(textSizeUnit.value);
	const sizeShown = $derived(typeof dialog?.size === 'number' ? sizeToUnit(dialog.size, unit, sliceWidth) : undefined);
	const writeSize = (v: number | undefined) => doc.set(p('size'), v === undefined ? undefined : unitToSize(v, unit, sliceWidth));

	/**
	 * Box sizing (owner 2026-10-17, Figma's three modes – dialog-fit.ts, DESIGN.md Part 2 "Box sizing"). The box keeps its
	 * drawn size when the mode changes (the canvas' measured width / height); auto width keeps the width as its max width.
	 * Stored as before: only auto width writes `fit`; auto height = no height, fixed = a height.
	 */
	const sizing = $derived(dialog ? dialogFit(dialog) : 'height');
	function setFit(next: DialogFit) {
		const key = drawnKey(path);
		const width = drawnWidths[key];
		const height = drawnHeights[key];
		doc.edit((file) => {
			const d = getAt(file, path);
			if (!d?.rect) return;
			const from = dialogFit(d);
			if (next === 'width') {
				d.fit = 'width';
				delete d.rect.height;
				return;
			}
			delete d.fit;
			if (from === 'width') d.rect.width = round2(width ?? d.rect.width ?? 50);
			if (next === 'height') delete d.rect.height;
			else d.rect.height = d.rect.height ?? round2(height ?? 10);
		}, `${path.join('.')}.fit`);
	}

	/** the preset the look matches (none = changed by hand) */
	const preset = $derived(matchTextPreset(dialog));

	/** a preset's look, in every locale (words, id, position and size stay; owner 2026-10-03); speech / thought bring a tail */
	function setPreset(next: TextPreset) {
		const slice = doc.get(doc.primary, path.slice(0, -2));
		const ratio = slice?.width > 0 && slice?.height > 0 ? slice.height / slice.width : undefined;
		doc.edit((file) => {
			const d = getAt(file, path);
			if (d) applyTextPreset(d, next, d.rect ? drawnBox(path, d) : undefined, ratio);
		}, `${path.join('.')}.preset`);
	}

</script>

{#if dialog}
	<div class="inspector" data-testid="dialog-inspector">
		<div class="row head">
			<h3>{m['creator.text_box']()} · {dialog.id ?? '–'}</h3>
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

		<!-- a drawn bubble's tail kind (owner 2026-10-18); its tip and base are dragged on the canvas -->
		<TailField {doc} {path} />

		<LocaleText {doc} path={p('text')} rows={4} expressions />

		<!-- the fine-tuning, folded (owner 2026-10-03: "where we have presets … an advanced collapsible"); position and size
		     are usually set on the canvas. Compact property rows (owner 2026-10-17, agents/research/EDITOR-INPUTS.md):
		     drag a field's handle to change it, click to type. -->
		<details class="advanced" data-testid="dialog-advanced">
		<summary>{m['creator.advanced']()}</summary>
		<div class="props">
			<!-- X / Y = the anchor point's position (owner 2026-10-17, DESIGN.md Part 2 "Anchor"); the 3×3 picker leads the row -->
			<PropRow label={m['creator.prop_position']()} testid="dialog-position">
				<AnchorField {doc} {path} bare />
				<ScrubNumber tag="X" label={m['creator.field_x']()} unit="%" step={0.1} value={dialog.rect?.x} onchange={(v) => doc.set(p('rect', 'x'), v ?? 0)} required testid="dialog-x" />
				<ScrubNumber tag="Y" label={m['creator.field_y']()} unit="%" step={0.1} value={dialog.rect?.y} onchange={(v) => doc.set(p('rect', 'y'), v ?? 0)} required testid="dialog-y" />
			</PropRow>
			<!-- the sizing mode (auto width / auto height / fixed, Figma's three), then W ⟷ H on the row's second line -->
			<PropRow label={m['creator.prop_dimensions']()} testid="dialog-dimensions">
				<Segmented
					label={m['creator.prop_fit']()}
					stretch
					square
					radio
					value={sizing}
					onchange={(f) => setFit(f)}
					options={(['width', 'height', 'fixed'] as const).map((f) => ({ value: f, icon: `fit-${f}`, title: m[`creator.fit_${f}`](), testid: `dialog-fit-${f}` }))}
					testid="dialog-fit"
				/>
				<span class="size-pair" data-prop-line>
					<LinkedPair
						a={dialog.rect?.width}
						b={sizing === 'fixed' ? dialog.rect?.height : undefined}
						first={{ tag: 'W', label: sizing === 'width' ? m['creator.field_max_width']() : m['creator.field_width'](), required: sizing !== 'width', placeholder: sizing === 'width' ? 'auto' : '', testid: 'dialog-width' }}
						second={{ tag: 'H', label: m['creator.field_height'](), placeholder: 'auto', disabled: sizing === 'width', testid: 'dialog-height' }}
						unit="%"
						step={0.1}
						min={0}
						lockLabel={m['creator.keep_proportions']()}
						bind:locked={keepRatio}
						onchange={(w, h) => {
							const key = `${path.join('.')}.rect.size`;
							doc.set(p('rect', 'width'), sizing === 'width' ? w : (w ?? 0), key);
							if (sizing === 'width') return;
							doc.set(p('rect', 'height'), h, key);
							// a height makes it a fixed size (stored as before: the height decides)
							if (h !== undefined && dialog.fit) doc.set(p('fit'), undefined, key);
						}}
					/>
				</span>
			</PropRow>

			<!-- no weight control (owner 2026-10-18: "keep it reduced, focus on certain things"); a stored weight still renders -->
			<PropRow label={m['creator.prop_type']()}>
				<PropSelect label={m['creator.text_font']()} icon="type" value={dialog.font} options={[{ value: 'lettering', label: 'lettering' }, { value: 'mono', label: 'mono' }]} placeholder={boxed ? 'mono' : 'lettering'} onchange={set('font')} testid="dialog-font" />
			</PropRow>
			<PropRow label={m['creator.prop_text_size']()} hint={m['creator.text_size_hint']()}>
				<!-- the unit shows in the switch beside it, not again in the field -->
				<ScrubNumber icon="type" label="{m['creator.prop_text_size']()} ({unit})" step={TEXT_SIZE_STEP[unit]} min={0} value={sizeShown} placeholder={String(sizeToUnit(4, unit, sliceWidth))} onchange={writeSize} testid="dialog-size" />
				<Segmented label={m['creator.text_size_unit']()} value={unit} onchange={(u) => textSizeUnit.set(u)} options={TEXT_SIZE_UNITS.map((u) => ({ value: u, label: u }))} testid="dialog-size-unit" />
			</PropRow>
			<PropRow label={m['creator.prop_letters']()}>
				<Segmented
					label={m['creator.text_uppercase']()}
					stretch
					value={dialog.uppercase === undefined ? null : dialog.uppercase ? 'upper' : 'as'}
					fallback="as"
					fallbackNote={m['creator.field_default_plain']()}
					onclear={() => doc.set(p('uppercase'), undefined)}
					onchange={(c) => doc.set(p('uppercase'), c === 'upper')}
					options={[{ value: 'as', label: 'Aa', title: m['creator.text_case_as_written']() }, { value: 'upper', label: 'AA', title: m['creator.text_uppercase']() }]}
					testid="dialog-case"
				/>
			</PropRow>
			<PropRow label={m['creator.prop_spacing']()}>
				<ScrubNumber icon="letter-spacing" label={m['creator.text_letter_spacing']()} unit="em" step={0.01} min={-0.5} max={2} value={dialog.letterSpacing} placeholder={font === 'lettering' ? '0.03' : '0'} onchange={set('letterSpacing')} testid="dialog-letter-spacing" />
				<ScrubNumber icon="line-height" label={m['creator.text_line_height']()} step={0.05} min={0.5} max={3} value={dialog.lineHeight} placeholder={font === 'lettering' ? '1.1' : '1.35'} onchange={set('lineHeight')} testid="dialog-line-height" />
			</PropRow>
			<PropRow label={m['creator.text_align']()}>
				<Segmented
					label={m['creator.text_align']()}
					stretch
					square
					value={dialog.align}
					fallback={boxed ? 'left' : 'center'}
					fallbackNote={m['creator.field_default_plain']()}
					onclear={() => doc.set(p('align'), undefined)}
					onchange={set('align')}
					options={ALIGNS.map((a) => ({ value: a, icon: `align-${a}`, title: alignLabel(a) }))}
					testid="dialog-align"
				/>
				<Segmented
					label={m['creator.text_valign']()}
					stretch
					square
					value={dialog.valign}
					fallback="middle"
					fallbackNote={m['creator.field_default_plain']()}
					onclear={() => doc.set(p('valign'), undefined)}
					onchange={set('valign')}
					options={VALIGNS.map((v) => ({ value: v, icon: `valign-${v}`, title: valignLabel(v) }))}
					testid="dialog-valign"
				/>
			</PropRow>
			<PropRow label={m['creator.prop_transform']()}>
				<ScrubNumber icon="rotate-cw" label={m['creator.field_rotation']()} unit="°" step={1} value={dialog.rotation} placeholder="0" onchange={set('rotation')} testid="dialog-rotation" />
				<ScrubNumber icon="maximize-2" label={m['creator.field_scale']()} step={0.05} min={0.1} max={10} value={dialog.scale} placeholder="1" onchange={set('scale')} testid="dialog-scale" />
			</PropRow>

			<PropRow label={m['creator.field_color']()}>
				<ColorField label={m['creator.field_color']()} value={dialog.color} placeholder="#000000" onchange={set('color')} testid="dialog-color" />
			</PropRow>
			<PropRow label={m['creator.text_outline']()}>
				<ColorField label={m['creator.text_outline']()} value={dialog.stroke} placeholder={m['creator.text_outline_none']()} onchange={set('stroke')} testid="dialog-stroke" />
				{#if dialog.stroke}
					<ScrubNumber icon="outline-width" label={m['creator.text_outline_width']()} unit="em" step={0.02} min={0} value={dialog.strokeWidth} placeholder="0.14" onchange={set('strokeWidth')} testid="dialog-stroke-width" />
				{/if}
			</PropRow>
			{#if boxed}
				<PropRow label={m['creator.text_box_frame']()}>
					<Segmented
						label={m['creator.text_box_frame']()}
						stretch
						value={dialog.frame === undefined ? null : dialog.frame ? 'yes' : 'no'}
						fallback="yes"
						fallbackNote={m['creator.field_default_plain']()}
						onclear={() => doc.set(p('frame'), undefined)}
						onchange={(f) => doc.set(p('frame'), f === 'yes')}
						options={[{ value: 'yes', label: m['creator.yes']() }, { value: 'no', label: m['creator.no']() }]}
						testid="dialog-frame"
					/>
				</PropRow>
			{/if}
			<PropRow label={m['creator.field_id']()}>
				<input type="text" aria-label={m['creator.field_id']()} value={dialog.id ?? ''} placeholder="s1c2-3-1" oninput={(e) => doc.set(p('id'), e.currentTarget.value || undefined)} data-testid="dialog-id" />
			</PropRow>
		</div>
		</details>

		<!-- deprecated (owner 2026-10-03): the art is re-imported without baked text; only shown where a box still has a mask -->
		{#if dialog.mask !== undefined}
		<details open>
			<summary>{m['creator.text_mask_legacy']()}</summary>
			<div class="sc-fields" data-cols="2" data-density="panel">
				<PropField {doc} path={p('mask')} label={m['creator.text_mask']()} type="select" options={[{ value: true, label: 'true (#fff)' }, '#ffffff', '#000000']} placeholder={m['creator.field_none']()} />
				<PropField {doc} path={p('maskShape')} label={m['creator.text_mask_shape']()} type="select" options={['rect', 'ellipse']} placeholder="rect" />
				<PropField {doc} path={p('pad')} label={m['creator.text_mask_pad']()} type="number" placeholder="0.5" />
			</div>
		</details>
		{/if}
	</div>
{/if}

<style>
	.size-pair {
		display: block;
	}
	.props {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
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
