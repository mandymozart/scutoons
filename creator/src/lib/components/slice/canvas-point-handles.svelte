<script lang="ts">
	import type { PiecePoint } from '@scutoons/content';
	import { pointInBox, type Rect } from '$lib/editor/canvas-geometry.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A collage piece's blur ramp on the slice canvas (research/COLLAGE.md §3, §7 "Blur"): two round dots on the selected
	 * piece – sharp at the first, fully blurred from the second – in % of the piece's own box, so they sit inside its
	 * (turned) canvas box and move and turn with it. A drag moves one dot (`pointInBox`: the pointer in the box's own
	 * frame); `onbegin` runs once per drag (one undo step), `onmove` with both points on every move.
	 */
	let {
		from,
		to,
		box,
		rotation,
		overlay,
		onbegin,
		onmove
	}: {
		from: PiecePoint;
		to: PiecePoint;
		/** the piece's drawn box in % of the slice (before its turn) */
		box: Required<Rect>;
		rotation: number;
		/** the canvas overlay (the slice's box on screen) */
		overlay: HTMLElement | null;
		onbegin: () => void;
		onmove: (from: PiecePoint, to: PiecePoint) => void;
	} = $props();

	let dragging = $state<{ which: 'from' | 'to'; pointer: number; moved: boolean } | null>(null);

	function down(event: PointerEvent, which: 'from' | 'to') {
		if (event.button !== 0) return;
		// the box under it would start a move
		event.stopPropagation();
		event.preventDefault();
		dragging = { which, pointer: event.pointerId, moved: false };
		try {
			(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
		} catch {
			/* not a real pointer */
		}
	}
	function move(event: PointerEvent) {
		const d = dragging;
		if (!d || d.pointer !== event.pointerId || !overlay) return;
		event.stopPropagation();
		const r = overlay.getBoundingClientRect();
		if (!r.width || !r.height) return;
		const pointer = { x: ((event.clientX - r.left) / r.width) * 100, y: ((event.clientY - r.top) / r.height) * 100 };
		const point = pointInBox(pointer, box, { width: r.width, height: r.height }, rotation);
		if (!d.moved) {
			d.moved = true;
			onbegin();
		}
		onmove(d.which === 'from' ? point : from, d.which === 'to' ? point : to);
	}
	function up(event: PointerEvent) {
		if (!dragging || dragging.pointer !== event.pointerId) return;
		event.stopPropagation();
		dragging = null;
	}
</script>

{#each [['from', from], ['to', to]] as const as [which, point] (which)}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<span
		class="dot {which}"
		class:dragging={dragging?.which === which}
		style:left="{point.x}%"
		style:top="{point.y}%"
		title={which === 'from' ? m['creator.piece_blur_from_dot']() : m['creator.piece_blur_to_dot']()}
		data-testid="piece-blur-dot-{which}"
		onpointerdown={(e) => down(e, which)}
		onpointermove={move}
		onpointerup={up}
		onpointercancel={up}
	></span>
{/each}

<style>
	/* round like the rotate / scale handles; sharp = paper, blurred = ink */
	.dot {
		position: absolute;
		z-index: 1;
		width: 0.875rem;
		height: 0.875rem;
		box-sizing: border-box;
		border: var(--border-width-regular) solid var(--ink);
		border-radius: 50%;
		background: var(--paper);
		transform: translate(-50%, -50%);
		cursor: grab;
		touch-action: none;
	}
	.dot.to {
		background: var(--ink);
		border-color: var(--paper);
		outline: var(--border-width-regular) solid var(--ink);
	}
	.dot.dragging {
		cursor: grabbing;
	}
	/* touch screens: a 2.75rem target around it (like the canvas handles) */
	@media (pointer: coarse) {
		.dot::before {
			content: '';
			position: absolute;
			left: 50%;
			top: 50%;
			width: 2.75rem;
			height: 2.75rem;
			transform: translate(-50%, -50%);
		}
	}
</style>
