<script lang="ts">
	import { ColorField, ScrubNumber } from '@scutoons/ui';
	import type { PieceGradient } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A piece's gradient fill (research/COLLAGE.md §7 "Fill"): one line – the colour it starts with, the colour it ends
	 * with, the direction (CSS degrees, 0 = to the top; empty = 180, top to bottom). Stops between them stay as they
	 * are (more than two is a later UI). The whole gradient goes to `onchange` (one value, one undo step per field).
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

<span class="gradient" data-prop-line data-testid="piece-gradient">
	<ColorField label={m['creator.piece_gradient_from']()} value={stops[0]?.color} onchange={(c) => color('from', c)} testid="piece-gradient-from" />
	<ColorField label={m['creator.piece_gradient_to']()} value={stops[stops.length - 1]?.color} onchange={(c) => color('to', c)} testid="piece-gradient-to" />
	<ScrubNumber icon="compass" label={m['creator.piece_gradient_angle']()} unit="°" step={1} value={value.angle} placeholder="180" onchange={angle} testid="piece-gradient-angle" />
</span>

<style>
	.gradient {
		display: flex;
		gap: var(--space-025);
		min-width: 0;
	}
	.gradient > :global(*) {
		flex: 1 1 0;
		min-width: 0;
	}
</style>
