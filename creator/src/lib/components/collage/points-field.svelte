<script lang="ts">
	import { ScrubNumber } from '@scutoons/ui';
	import type { PiecePoint } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A progressive blur's two points (research/COLLAGE.md §7 "Blur"): sharp at the first, fully blurred from the second,
	 * each in % of the piece's own box (0 0 top left … 100 100 bottom right – they move and turn with the piece). Two
	 * lines in a property row; both points go to `onchange` together.
	 */
	let { from, to, onchange }: { from: PiecePoint; to: PiecePoint; onchange: (from: PiecePoint, to: PiecePoint) => void } = $props();

	const at = (point: PiecePoint, key: 'x' | 'y', v: number | undefined): PiecePoint => ({ ...point, [key]: v ?? 0 });
</script>

<span class="line" data-prop-line data-testid="piece-blur-from">
	<ScrubNumber tag="X1" label={m['creator.piece_blur_from_x']()} unit="%" step={1} value={from.x} required onchange={(v) => onchange(at(from, 'x', v), to)} testid="piece-blur-from-x" />
	<ScrubNumber tag="Y1" label={m['creator.piece_blur_from_y']()} unit="%" step={1} value={from.y} required onchange={(v) => onchange(at(from, 'y', v), to)} testid="piece-blur-from-y" />
</span>
<span class="line" data-prop-line data-testid="piece-blur-to">
	<ScrubNumber tag="X2" label={m['creator.piece_blur_to_x']()} unit="%" step={1} value={to.x} required onchange={(v) => onchange(from, at(to, 'x', v))} testid="piece-blur-to-x" />
	<ScrubNumber tag="Y2" label={m['creator.piece_blur_to_y']()} unit="%" step={1} value={to.y} required onchange={(v) => onchange(from, at(to, 'y', v))} testid="piece-blur-to-y" />
</span>

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
