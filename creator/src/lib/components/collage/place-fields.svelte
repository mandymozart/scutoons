<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { PropRow, ScrubNumber, Segmented } from '@scutoons/ui';
	import type { CollagePiece, Path } from '@scutoons/content';
	import AnchorField from '$lib/components/fields/anchor-field.svelte';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * Where a piece sits (research/COLLAGE.md §7): on a slice the anchor point (the 3×3 picker) at X / Y in % of the
	 * picture; on a score a bar (+ staff) of the file – x in % of the bar's time, y in staff spaces below the staff's top
	 * line – or, as a background, % of the page that holds the bar (the piece's centre: no anchor picker). Then the width
	 * (% / staff spaces – the height follows the file, never distorted), the rotation, and on scores over / behind the
	 * notes. Plain fields: one undo step per field (doc.set coalesces a scrub).
	 */
	let { doc, path, piece, host }: { doc: ContentDoc; path: Path; piece: CollagePiece; host: 'slice' | 'score' } = $props();

	const p = (...keys: (string | number)[]) => [...path, ...keys];
	const onPage = $derived(host === 'score' && !!piece.at?.page);
	/** the unit of y and the width: % on a slice and a page, staff spaces on the music */
	const sp = $derived(host === 'score' && !onPage ? m['creator.piece_unit_sp']() : '%');
</script>

<PropRow label={m['creator.prop_position']()} testid="piece-position">
	{#if host === 'slice'}
		<AnchorField {doc} {path} bare />
	{:else}
		<Segmented
			label={m['creator.piece_at']()}
			square
			radio
			value={onPage ? 'page' : 'bar'}
			onchange={(v) => doc.set(p('at', 'page'), v === 'page' ? true : undefined)}
			options={[
				{ value: 'bar', icon: 'music', title: m['creator.piece_at_bar'](), testid: 'piece-at-bar' },
				{ value: 'page', icon: 'file', title: m['creator.piece_at_page'](), testid: 'piece-at-page' }
			]}
			testid="piece-at"
		/>
		<input type="text" aria-label={m['creator.piece_bar']()} title={m['creator.piece_bar']()} placeholder="#" value={piece.at?.bar ?? ''} oninput={(e) => doc.set(p('at', 'bar'), e.currentTarget.value.trim())} data-testid="piece-bar" />
		<ScrubNumber tag="St" label={m['creator.piece_staff']()} step={1} min={1} value={piece.at?.staff} placeholder="1" onchange={(v) => doc.set(p('at', 'staff'), v === undefined ? undefined : Math.max(1, Math.round(v)))} testid="piece-staff" />
	{/if}
	<span class="line" data-prop-line={host === 'score' || undefined}>
		<ScrubNumber tag="X" label={m['creator.field_x']()} unit="%" step={0.1} value={piece.rect?.x} required onchange={(v) => doc.set(p('rect', 'x'), v ?? 0)} testid="piece-x" />
		<ScrubNumber tag="Y" label={host === 'score' && !onPage ? m['creator.piece_y_sp']() : m['creator.field_y']()} unit={sp} step={0.1} value={piece.rect?.y} required onchange={(v) => doc.set(p('rect', 'y'), v ?? 0)} testid="piece-y" />
	</span>
</PropRow>

<PropRow label={m['creator.prop_dimensions']()} testid="piece-size">
	<ScrubNumber tag="W" label={host === 'score' && !onPage ? m['creator.piece_width_sp']() : m['creator.field_width']()} unit={sp} step={0.1} min={0.1} value={piece.rect?.width} required onchange={(v) => doc.set(p('rect', 'width'), v ?? 1)} testid="piece-width" />
	<ScrubNumber icon="rotate-cw" label={m['creator.field_rotation']()} unit="°" step={1} value={piece.rotation} placeholder="0" onchange={(v) => doc.set(p('rotation'), v || undefined)} testid="piece-rotation" />
</PropRow>

{#if host === 'score'}
	<PropRow label={m['creator.piece_side']()} testid="piece-side-row">
		<Segmented
			label={m['creator.piece_side']()}
			stretch
			radio
			value={piece.behind ? 'behind' : null}
			fallback="over"
			fallbackNote={m['creator.field_default_plain']()}
			onclear={() => doc.set(p('behind'), undefined)}
			onchange={(v) => doc.set(p('behind'), v === 'behind' ? true : undefined)}
			options={[
				{ value: 'over', label: m['creator.piece_over'](), title: m['creator.piece_over_hint'](), testid: 'piece-over' },
				{ value: 'behind', label: m['creator.piece_behind'](), title: m['creator.piece_behind_hint'](), testid: 'piece-behind' }
			]}
			testid="piece-side"
		/>
	</PropRow>
{/if}

<style>
	/* X / Y: one part of the row (a slice: beside the anchor picker), their own line on a score */
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
