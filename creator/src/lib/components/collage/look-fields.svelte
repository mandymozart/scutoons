<script lang="ts">
	import { ColorField, PropRow, PropSelect, ScrubNumber, Segmented } from '@scutoons/ui';
	import { BLEND_MODES, pieceStyle, round2, type CollagePiece } from '@scutoons/content';
	import { blurKind, editEffect, fillKind, setBlurRamp, setEffect, setEffectColor, setFillKind, setPlain, RAMP_POINTS, type ClearedEffects, type FillKind } from '$lib/editor/collage-edit.ts';
	import { blendLabel } from '$lib/editor/collage-words.ts';
	import GradientField from '$lib/components/collage/gradient-field.svelte';
	import PointsField from '$lib/components/collage/points-field.svelte';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A piece's colouration fields (research/COLLAGE.md §7): opacity + blend, fill (none / colour / gradient), outline,
	 * shadow, blur (even or a ramp between two points). The fields show the piece as drawn (`pieceStyle`: its look under
	 * its own values); a change makes that key the piece's own (collage-edit.ts – copy-on-write), clearing an effect
	 * turns it off – an outline / shadow colour retyped brings it back as it was (`setEffectColor`); the fill's colour
	 * field never clears the fill (only the kind None does). Lengths are % of the piece's width (the same look at every
	 * size).
	 */
	let { piece, change }: { piece: CollagePiece; change: (edit: (piece: CollagePiece) => void, key: string) => void } = $props();

	const style = $derived(pieceStyle(piece));
	/** the look's values alone (the plain fields' placeholders) */
	const look = $derived(pieceStyle({ preset: piece.preset }));
	const fill = $derived(style.fill);
	const fillKindNow = $derived(fillKind(fill));
	const blurKindNow = $derived(blurKind(style.blur));

	// Segmented sends the chosen option again: the kind it has is no edit (no undo step, the values stay)
	function setFill(kind: FillKind) {
		if (kind !== fillKindNow) change((p) => setFillKind(p, kind), 'fill');
	}

	/** the outlines / shadows a colour field cleared in this inspector (by piece id) – a retype brings them back */
	const cleared: ClearedEffects = new Map();
	const effectColor = (key: 'outline' | 'shadow', color: string | undefined) => change((p) => setEffectColor(p, key, color, cleared), `${key}.color`);
</script>

<PropRow label={m['creator.piece_blend']()} testid="piece-blend-row">
	<ScrubNumber
		icon="eye"
		label={m['creator.piece_opacity']()}
		unit="%"
		step={1}
		min={0}
		max={100}
		value={piece.style?.opacity === undefined ? undefined : round2(piece.style.opacity * 100)}
		placeholder={String(round2((look.opacity ?? 1) * 100))}
		onchange={(v) => change((p) => setPlain(p, 'opacity', v === undefined ? undefined : Math.round(v * 100) / 10000), 'opacity')}
		testid="piece-opacity"
	/>
	<!-- its own line: beside the opacity even two parts of the row cut "(Multiply)" / "(Multiplizieren)" (22rem inspector) -->
	<span data-prop-line>
		<PropSelect
			icon="blend"
			label={m['creator.piece_blend']()}
			value={piece.style?.blend}
			options={BLEND_MODES.map((b) => ({ value: b, label: blendLabel(b) }))}
			placeholder={blendLabel(look.blend ?? 'normal')}
			onchange={(v) => change((p) => setPlain(p, 'blend', v), 'blend')}
			testid="piece-blend"
		/>
	</span>
</PropRow>

<PropRow label={m['creator.piece_fill']()} testid="piece-fill-row">
	<Segmented
		label={m['creator.piece_fill']()}
		square
		value={fillKindNow}
		onchange={setFill}
		options={[
			{ value: 'none', icon: 'slash', title: m['creator.piece_fill_none'](), testid: 'piece-fill-none' },
			{ value: 'color', icon: 'droplet', title: m['creator.piece_fill_color'](), testid: 'piece-fill-color' },
			{ value: 'gradient', icon: 'gradient', title: m['creator.piece_fill_gradient'](), testid: 'piece-fill-gradient' }
		]}
		testid="piece-fill"
	/>
	{#if typeof fill === 'string'}
		<span data-prop-line>
			<!-- emptied to retype: the fill stays (the field would vanish with it) – the kind None turns it off -->
			<ColorField label={m['creator.piece_fill_color']()} value={fill} onchange={(v) => v && change((p) => setEffect(p, 'fill', v), 'fill.color')} testid="piece-fill-value" />
		</span>
	{:else if fill}
		<GradientField value={fill} onchange={(g) => change((p) => setEffect(p, 'fill', g), 'fill.gradient')} />
	{/if}
</PropRow>

<PropRow label={m['creator.text_outline']()} testid="piece-outline-row">
	<ColorField
		label={m['creator.text_outline']()}
		value={style.outline?.color}
		placeholder={m['creator.text_outline_none']()}
		onchange={(v) => effectColor('outline', v)}
		testid="piece-outline"
	/>
	{#if style.outline}
		<ScrubNumber icon="outline-width" label={m['creator.piece_outline_width']()} unit="%" step={0.1} min={0} value={style.outline.width} required onchange={(v) => change((p) => editEffect(p, 'outline', { width: v ?? 0 }), 'outline.width')} testid="piece-outline-width" />
	{/if}
</PropRow>

<PropRow label={m['creator.piece_shadow']()} testid="piece-shadow-row">
	<ColorField
		label={m['creator.piece_shadow']()}
		value={style.shadow?.color}
		placeholder={m['creator.text_outline_none']()}
		onchange={(v) => effectColor('shadow', v)}
		testid="piece-shadow"
	/>
	{#if style.shadow}
		{@const shadow = style.shadow}
		<ScrubNumber
			icon="eye"
			label={m['creator.piece_shadow_opacity']()}
			unit="%"
			step={1}
			min={0}
			max={100}
			value={shadow.opacity === undefined ? undefined : round2(shadow.opacity * 100)}
			placeholder="100"
			onchange={(v) => change((p) => editEffect(p, 'shadow', { opacity: v === undefined ? undefined : Math.round(v * 100) / 10000 }), 'shadow.opacity')}
			testid="piece-shadow-opacity"
		/>
		<span class="line" data-prop-line>
			<ScrubNumber tag="X" label={m['creator.piece_shadow_x']()} unit="%" step={0.1} value={shadow.x} required onchange={(v) => change((p) => editEffect(p, 'shadow', { x: v ?? 0 }), 'shadow.x')} testid="piece-shadow-x" />
			<ScrubNumber tag="Y" label={m['creator.piece_shadow_y']()} unit="%" step={0.1} value={shadow.y} required onchange={(v) => change((p) => editEffect(p, 'shadow', { y: v ?? 0 }), 'shadow.y')} testid="piece-shadow-y" />
			<ScrubNumber icon="blur" label={m['creator.piece_shadow_blur']()} unit="%" step={0.1} min={0} value={shadow.blur} required onchange={(v) => change((p) => editEffect(p, 'shadow', { blur: v ?? 0 }), 'shadow.blur')} testid="piece-shadow-blur" />
		</span>
	{/if}
</PropRow>

<PropRow label={m['creator.piece_blur']()} testid="piece-blur-row">
	<ScrubNumber
		icon="blur"
		label={m['creator.piece_blur']()}
		unit="%"
		step={0.1}
		min={0}
		value={style.blur?.amount}
		placeholder="0"
		onchange={(v) => change((p) => (v === undefined ? setEffect(p, 'blur', undefined) : editEffect(p, 'blur', { amount: v })), 'blur.amount')}
		testid="piece-blur"
	/>
	{#if style.blur}
		{@const blur = style.blur}
		<Segmented
			label={m['creator.piece_blur_kind']()}
			square
			value={blurKindNow}
			onchange={(k) => k !== blurKindNow && change((p) => setBlurRamp(p, k === 'ramp'), 'blur.kind')}
			options={[
				{ value: 'even', icon: 'circle', title: m['creator.piece_blur_even'](), testid: 'piece-blur-even' },
				{ value: 'ramp', icon: 'gradient', title: m['creator.piece_blur_ramp'](), testid: 'piece-blur-ramp' }
			]}
			testid="piece-blur-kind"
		/>
		{#if blur.from && blur.to}
			<PointsField from={blur.from ?? RAMP_POINTS.from} to={blur.to ?? RAMP_POINTS.to} onchange={(from, to) => change((p) => editEffect(p, 'blur', { from, to }), 'blur.points')} />
		{/if}
	{/if}
</PropRow>

<style>
	.line {
		display: flex;
		gap: var(--space-025);
		min-width: 0;
	}
	.line > :global(*) {
		flex: 1 1 0;
		min-width: 0;
	}
</style>
