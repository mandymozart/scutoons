<script lang="ts">
	import { ColorField, ScrubNumber } from '@scutoons/ui';
	import type { PieceGradient } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A piece's gradient fill (research/COLLAGE.md §7 "Fill"), inside the Fill row: the direction first (CSS degrees,
	 * 0 = to the top; empty = 180, top to bottom) – on the row's first line beside the kind –, then on a line of their own
	 * the colour it starts with and the one it ends with: half the row each when both hexes fit whole, else one line each
	 * (the 22rem inspector). Stops between them stay as they are (more than two is a later UI). The whole gradient goes
	 * to `onchange` (one value, one undo step per field).
	 */
	let { value, onchange }: { value: PieceGradient; onchange: (next: PieceGradient) => void } = $props();

	const stops = $derived(value.stops ?? []);

	function color(end: 'from' | 'to', next: string | undefined) {
		if (!next || stops.length < 2) return;
		const copy: PieceGradient = JSON.parse(JSON.stringify(value));
		copy.stops[end === 'from' ? 0 : copy.stops.length - 1].color = next;
		onchange(copy);
	}
	function angle(next: number | undefined) {
		const copy: PieceGradient = JSON.parse(JSON.stringify(value));
		if (next === undefined) delete copy.angle;
		else copy.angle = next;
		onchange(copy);
	}
</script>

<ScrubNumber icon="compass" label={m['creator.piece_gradient_angle']()} unit="°" step={1} value={value.angle} placeholder="180" onchange={angle} testid="piece-gradient-angle" />
<!-- 7.5rem: a colour field with a whole #rrggbb (its swatch's padding + 7 characters) -->
<span class="sc-fields" data-cols="fit" data-density="dense" data-prop-line style:--fields-fit-min="7.5rem" data-testid="piece-gradient">
	<ColorField label={m['creator.piece_gradient_from']()} value={stops[0]?.color} onchange={(c) => color('from', c)} testid="piece-gradient-from" />
	<ColorField label={m['creator.piece_gradient_to']()} value={stops[stops.length - 1]?.color} onchange={(c) => color('to', c)} testid="piece-gradient-to" />
</span>
