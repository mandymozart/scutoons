<script lang="ts">
	import {
		TAIL_CORNER_INSET_REM,
		dialogTail,
		followTip,
		freeTailFrom,
		nearestBase,
		round2,
		tailBase,
		toLocal,
		toSlice,
		type FreeTail,
		type Path,
		type TailFrame
	} from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { Rect } from '$lib/editor/canvas-geometry.ts';
	import { SLICE_RATIO } from '$lib/editor/defaults.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A drawn bubble's tail on the slice canvas (owner 2026-10-18, DESIGN.md Part 2 "Tails"): two handles while the bubble
	 * is selected – the TIP (free: anywhere on the slice and past its edge, an off-panel speaker; shown at the edge while
	 * it lies outside) and the BASE (slides along the bubble's outline, jumps sides at the corners, kept 1rem from them).
	 * One undo step per drag; an old fixed tail becomes a free one on its first edit. The arrow keys nudge a focused handle
	 * (the tip by 0.5 % of the slice, Shift × 10; the base along its side). A base that can't reach the tip jumps to the
	 * side facing it ("Basis springt mit") – drawn there and stored there with the edit.
	 */
	let {
		doc,
		path,
		dialog,
		box,
		rotation,
		scale,
		overlay
	}: {
		doc: ContentDoc;
		/** the text box's path */
		path: Path;
		/** the text box as shown */
		dialog: any;
		/** its drawn box in % of the slice (before its turn) */
		box: Required<Rect>;
		rotation: number;
		scale: number;
		/** the canvas overlay (the slice's box on screen) */
		overlay: HTMLElement | null;
	} = $props();

	let size = $state({ w: 0, h: 0 });
	$effect(() => {
		const el = overlay;
		if (!el) return;
		const ro = new ResizeObserver(() => (size = { w: el.clientWidth, h: el.clientHeight }));
		ro.observe(el);
		size = { w: el.clientWidth, h: el.clientHeight };
		return () => ro.disconnect();
	});

	const ratio = $derived(size.w > 0 ? size.h / size.w : SLICE_RATIO);
	const tailBox = $derived({ left: box.x, top: box.y, width: box.width, height: box.height });
	/** the tail as it is drawn: a free one, or an old fixed one as the free tail that looks the same */
	const tail = $derived.by<FreeTail | null>(() => {
		const t = dialogTail(dialog);
		if (!t) return null;
		return typeof t === 'string' ? freeTailFrom(t, tailBox, dialog?.size ?? 4, ratio) : t;
	});
	const frame = $derived<TailFrame>({
		left: (box.x / 100) * size.w,
		top: (box.y / 100) * size.h,
		width: (box.width / 100) * size.w,
		height: (box.height / 100) * size.h,
		rotation,
		scale
	});
	const rem = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
	const inset = () => (TAIL_CORNER_INSET_REM * rem()) / (scale || 1);
	/** the base as it is drawn: on the side facing the tip when the stored one can't reach it ("Basis springt mit") */
	const shownBase = $derived(tail ? followTip(tail, frame, size).base : null);
	/** the base handle on the slice (%) */
	const basePos = $derived.by(() => {
		if (!tail || !shownBase || !size.w) return null;
		const b = tailBase(shownBase.side, shownBase.at, frame.width, frame.height, inset());
		const p = toSlice(frame, b.point);
		return { x: (p.x / size.w) * 100, y: (p.y / size.h) * 100 };
	});
	const off = $derived(!!tail && (tail.tip.x < 0 || tail.tip.x > 100 || tail.tip.y < 0 || tail.tip.y > 100));
	const clamp = (v: number) => Math.min(100, Math.max(0, v));

	/**
	 * writes the tail (the drag's undo step was taken at its start); a base that can't reach the tip is stored on the side
	 * facing it, in the same step (owner 2026-10-18, "Basis springt mit")
	 */
	function write(next: FreeTail) {
		const entry = doc.get(doc.primary, path);
		if (entry) entry.tail = followTip(next, frame, size);
	}

	let dragging = $state<{ which: 'tip' | 'base'; pointer: number; moved: boolean } | null>(null);

	function down(event: PointerEvent, which: 'tip' | 'base') {
		if (event.button !== 0) return;
		// the box / overlay under it would start a move or a new box
		event.stopPropagation();
		event.preventDefault();
		dragging = { which, pointer: event.pointerId, moved: false };
		(event.currentTarget as HTMLElement).focus();
		try {
			(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
		} catch {
			/* not a real pointer */
		}
	}
	function move(event: PointerEvent) {
		const d = dragging;
		if (!d || d.pointer !== event.pointerId || !overlay || !tail) return;
		event.stopPropagation();
		const r = overlay.getBoundingClientRect();
		if (!r.width || !r.height) return;
		// not clamped: the tip may go past the slice's edge
		const px = { x: event.clientX - r.left, y: event.clientY - r.top };
		if (!d.moved) {
			d.moved = true;
			doc.checkpoint();
		}
		if (d.which === 'tip') write({ ...tail, tip: { x: round2((px.x / r.width) * 100), y: round2((px.y / r.height) * 100) } });
		else write({ ...tail, base: nearestBase(toLocal(frame, { x: (px.x / r.width) * size.w, y: (px.y / r.height) * size.h }), frame.width, frame.height, inset()) });
	}
	function up(event: PointerEvent) {
		if (!dragging || dragging.pointer !== event.pointerId) return;
		event.stopPropagation();
		dragging = null;
	}

	const ARROWS: Record<string, [number, number]> = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
	function key(event: KeyboardEvent, which: 'tip' | 'base') {
		const arrow = ARROWS[event.key];
		if (!arrow || !tail) return;
		// the canvas' own nudge would move the box
		event.preventDefault();
		event.stopPropagation();
		const k = event.shiftKey ? 10 : 1;
		doc.checkpoint(`${path.join('.')}.tail.${which}`);
		if (which === 'tip') {
			write({ ...tail, tip: { x: round2(tail.tip.x + arrow[0] * 0.5 * k), y: round2(tail.tip.y + arrow[1] * 0.5 * (k / ratio)) } });
			return;
		}
		// along the side: right / down = towards its end
		const now = shownBase ?? tail.base;
		const horizontal = now.side === 'top' || now.side === 'bottom';
		const step = (horizontal ? arrow[0] : arrow[1]) * 0.02 * k;
		if (!step) return;
		const b = tailBase(now.side, now.at + step, frame.width, frame.height, inset());
		write({ ...tail, base: { side: now.side, at: Math.round(b.at * 1000) / 1000 } });
	}
</script>

{#if tail && basePos}
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<span
		class="handle base"
		class:dragging={dragging?.which === 'base'}
		style:left="{basePos.x}%"
		style:top="{basePos.y}%"
		title={m['creator.tail_base_handle']()}
		aria-label={m['creator.tail_base_handle']()}
		role="button"
		tabindex="0"
		data-testid="tail-base"
		data-side={shownBase?.side}
		onpointerdown={(e) => down(e, 'base')}
		onpointermove={move}
		onpointerup={up}
		onpointercancel={up}
		onkeydown={(e) => key(e, 'base')}
	></span>
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<span
		class="handle tip"
		class:off
		class:dragging={dragging?.which === 'tip'}
		style:left="{clamp(tail.tip.x)}%"
		style:top="{clamp(tail.tip.y)}%"
		title={off ? m['creator.tail_tip_off']() : m['creator.tail_tip_handle']()}
		aria-label={m['creator.tail_tip_handle']()}
		role="button"
		tabindex="0"
		data-testid="tail-tip"
		onpointerdown={(e) => down(e, 'tip')}
		onpointermove={move}
		onpointerup={up}
		onpointercancel={up}
		onkeydown={(e) => key(e, 'tip')}
	></span>
{/if}

<style>
	/* the tip: a round ink dot (like the canvas' round handles); the base: a paper diamond on the outline */
	.handle {
		position: absolute;
		/* over the selected box (z 3, slice-canvas.svelte) */
		z-index: 4;
		width: 0.875rem;
		height: 0.875rem;
		box-sizing: border-box;
		border: var(--border-width-regular) solid var(--ink);
		transform: translate(-50%, -50%);
		cursor: grab;
		touch-action: none;
	}
	.tip {
		border-radius: 50%;
		background: var(--ink);
		outline: var(--border-width-regular) solid var(--paper);
	}
	/* past the slice's edge: shown on the edge, hollow */
	.tip.off {
		background: var(--paper);
		border-style: dashed;
	}
	.base {
		background: var(--paper);
		transform: translate(-50%, -50%) rotate(45deg);
	}
	.handle.dragging {
		cursor: grabbing;
	}
	.handle:focus-visible {
		outline: var(--border-width-regular) solid var(--select-live-text);
		outline-offset: 0.125rem;
	}
	/* touch screens: a 2.75rem target around it (like the canvas handles) */
	@media (pointer: coarse) {
		.handle::before {
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
