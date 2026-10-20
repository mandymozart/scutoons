<script lang="ts">
	/**
	 * The strip editor (agents/research/TEXT-ACROSS-SLICES.md §4, step 2): the open group as ONE strip – its slices as
	 * canvases stacked like the reader (vertical) or side by side (a horizontal / endless group), no gap. A box may reach
	 * over a seam (§3, §7): it paints and takes the pointer over the neighbour (slice-canvas.svelte has no stacking context
	 * of its own). The open slice (`active`, the chapter view's slice) holds the selection, the live handles and the test
	 * ids; a pointer on another slice makes that one the open slice; the user's scrolling makes the slice in the view's
	 * centre the open one (while nothing is selected – the editor decides; a zoom never does). Virtualised: a slice mounts within a screen of the view and
	 * unmounts three screens away, its place kept at its last drawn size. The seams: the slice's number always, a hairline
	 * across the art with `seams` (the View toggle).
	 */
	import { tick, untrack } from 'svelte';
	import { DEFAULT_SLICE, groupAxis, type CollagePiece, type Path, type PresetList } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { SLICE_RATIO } from '$lib/editor/defaults.ts';
	import { m } from '$lib/paraglide/messages.js';
	import SliceCanvas, { type CanvasSelection, type ItemKind, type Tool } from './slice-canvas.svelte';
	import type { Rect } from '$lib/editor/canvas-geometry.ts';

	let {
		doc,
		groupPath,
		active,
		locale,
		grid = 0,
		tool = 'text',
		layers = null,
		selection = null,
		live = true,
		zoom = null,
		seams = false,
		pieceName,
		presets,
		onshown,
		onselect,
		oncreate,
		oncentre
	}: {
		doc: ContentDoc;
		groupPath: Path;
		/** the open slice's index */
		active: number;
		locale: string;
		grid?: number;
		tool?: Tool;
		layers?: readonly ItemKind[] | null;
		/** the open slice's selection */
		selection?: CanvasSelection | null;
		live?: boolean;
		zoom?: number | null;
		/** the seams as a hairline across the art */
		seams?: boolean;
		pieceName?: (piece: CollagePiece) => string;
		presets?: PresetList;
		onshown?: (factor: number) => void;
		/** a pointer went down on slice `index` (its box, or empty space: null) */
		onselect?: (index: number, selection: CanvasSelection | null) => void;
		/** a rect was drawn on slice `index` */
		oncreate?: (index: number, tool: Tool, rect: Required<Rect>) => void;
		/** slice `index` lies in the view's centre (scrolled there) */
		oncentre?: (index: number) => void;
	} = $props();

	const group = $derived<any>(doc.get(doc.primary, groupPath));
	const slices = $derived<any[]>(Array.isArray(group?.slices) ? group.slices : []);
	const axis = $derived(groupAxis(group) === 'vertical' ? 'vertical' : 'horizontal');
	/** the strip's width (vertical): the first slice's 1x width – the reading width */
	const sliceW = $derived(slices[0]?.width > 0 ? slices[0].width : DEFAULT_SLICE.width);

	let strip = $state<HTMLDivElement | null>(null);
	const slots: HTMLElement[] = [];
	/** the slices mounted (within a screen of the view) */
	let near = $state<Set<number>>(new Set());
	/** a slot's last drawn size (height / width) – its place while unmounted */
	let ratios = $state<Record<number, number>>({});
	const ratioOf = (i: number) => ratios[i] ?? (slices[i]?.width > 0 && slices[i]?.height > 0 ? slices[i].height / slices[i].width : SLICE_RATIO);
	/** the slice the strip itself made the open one (a pointer, the centre): no scroll back to it */
	let reported = -1;

	const canvases: (SliceCanvas | undefined)[] = $state([]);
	/** the keyboard's nudge acts on the open slice's canvas */
	export function nudge(dxPx: number, dyPx: number) {
		return canvases[active]?.nudge(dxPx, dyPx) ?? false;
	}

	/**
	 * the element that scrolls the strip (the slice area on a desktop, the page column on a phone): the observers' root –
	 * a scroller clips what the viewport would see, so a root margin on the viewport never reaches past it
	 */
	function scrollerOf(el: HTMLElement): HTMLElement | null {
		for (let p = el.parentElement; p && p !== document.body; p = p.parentElement) {
			const cs = getComputedStyle(p);
			if (/(auto|scroll)/.test(axis === 'vertical' ? cs.overflowY : cs.overflowX)) return p;
		}
		return null;
	}

	/** mount within a screen, unmount three screens away (two observers: in / out) */
	$effect(() => {
		const el = strip;
		void slices.length;
		if (!el || typeof IntersectionObserver === 'undefined') {
			near = new Set(slices.map((_, i) => i));
			return;
		}
		const root = scrollerOf(el);
		const index = (e: IntersectionObserverEntry) => Number((e.target as HTMLElement).dataset.strip);
		const enter = new IntersectionObserver(
			(entries) => {
				const add = entries.filter((e) => e.isIntersecting).map(index);
				if (add.length) near = new Set([...near, ...add]);
			},
			{ root, rootMargin: '100%' }
		);
		const leave = new IntersectionObserver(
			(entries) => {
				const gone = entries.filter((e) => !e.isIntersecting).map(index);
				if (gone.length) near = new Set([...near].filter((i) => !gone.includes(i)));
			},
			{ root, rootMargin: '300%' }
		);
		const sizes = new ResizeObserver((entries) => {
			const next = { ...untrack(() => ratios) };
			let changed = false;
			for (const e of entries) {
				const i = Number((e.target as HTMLElement).dataset.strip);
				const { width, height } = e.contentRect;
				if (width > 0 && height > 0 && (e.target as HTMLElement).dataset.mounted === '1' && Math.abs((next[i] ?? 0) - height / width) > 0.001) {
					next[i] = height / width;
					changed = true;
				}
			}
			if (changed) ratios = next;
		});
		for (const slot of slots) {
			if (!slot) continue;
			enter.observe(slot);
			leave.observe(slot);
			sizes.observe(slot);
		}
		return () => {
			enter.disconnect();
			leave.disconnect();
			sizes.disconnect();
		};
	});

	/**
	 * the slice in the view's centre opens – after a scroll the USER made (wheel, finger, keys, the scrollbar): a zoom, a
	 * slice mounting or a box growing moves the scroll too, and must not open another slice under the selection's feet
	 */
	$effect(() => {
		const el = strip;
		if (!el) return;
		const root = scrollerOf(el);
		const target: HTMLElement | Window = root ?? window;
		let userAt = -Infinity;
		let frame = 0;
		const user = (event: Event) => {
			if (event instanceof WheelEvent && (event.ctrlKey || event.metaKey)) return;
			userAt = performance.now();
		};
		const find = () => {
			frame = 0;
			const box = root ? root.getBoundingClientRect() : { top: 0, bottom: innerHeight, left: 0, right: innerWidth };
			const mid = axis === 'vertical' ? (box.top + box.bottom) / 2 : (box.left + box.right) / 2;
			const i = slots.findIndex((slot) => {
				if (!slot) return false;
				const r = slot.getBoundingClientRect();
				return axis === 'vertical' ? r.top <= mid && r.bottom > mid : r.left <= mid && r.right > mid;
			});
			if (i < 0 || i === untrack(() => active)) return;
			reported = i;
			oncentre?.(i);
		};
		const scrolled = () => {
			if (performance.now() - userAt > 1000 || frame) return;
			frame = requestAnimationFrame(find);
		};
		const events = ['wheel', 'touchmove', 'keydown', 'pointerdown'] as const;
		for (const type of events) target.addEventListener(type, user, { passive: true, capture: true });
		target.addEventListener('scroll', scrolled, { passive: true });
		return () => {
			for (const type of events) target.removeEventListener(type, user, { capture: true });
			target.removeEventListener('scroll', scrolled);
			if (frame) cancelAnimationFrame(frame);
		};
	});

	/**
	 * the open slice changed elsewhere (the tree, ‹ ›, a deep link): the strip scrolls it to the view's centre
	 */
	$effect(() => {
		const i = active;
		const slot = slots[i];
		if (!slot || i === reported) return;
		reported = i;
		// only the strip's own scroller moves (scrollIntoView would scroll the shell around it too – under the header)
		const root = strip ? scrollerOf(strip) : null;
		const r = slot.getBoundingClientRect();
		const box = root ? root.getBoundingClientRect() : { top: 0, left: 0, width: innerWidth, height: innerHeight };
		const dy = axis === 'vertical' ? r.top + r.height / 2 - (box.top + box.height / 2) : 0;
		const dx = axis === 'vertical' ? 0 : r.left + r.width / 2 - (box.left + box.width / 2);
		if (root) root.scrollBy({ left: dx, top: dy, behavior: 'instant' });
		else window.scrollBy({ left: dx, top: dy, behavior: 'instant' });
	});

	/** a pointer on slice `i`: it becomes the open slice without a scroll */
	function take(i: number) {
		reported = i;
		picked = true;
	}
	/** the selection came from a pointer on the strip (it is under the pointer – no reveal, the drag would lose it) */
	let picked = false;

	/**
	 * a selection made elsewhere (the Layers list, the keys) shows its box with its handles: the strip scrolls just enough
	 * (2.5rem around it for the rotate / scale handles); a box taller than the view shows its start
	 */
	$effect(() => {
		const sel = selection;
		const i = active;
		if (!sel) return;
		if (picked) {
			picked = false;
			return;
		}
		tick().then(() => {
			const el = slots[i]?.querySelector<HTMLElement>('[data-testid="slice-canvas"] .box.selected');
			const root = strip ? scrollerOf(strip) : null;
			if (!el) return;
			const r = el.getBoundingClientRect();
			const view = root ? root.getBoundingClientRect() : { top: 0, bottom: innerHeight, left: 0, right: innerWidth };
			const margin = 2.5 * (parseFloat(getComputedStyle(document.documentElement).fontSize) || 16);
			const shift = (start: number, end: number, from: number, to: number) =>
				start - margin < from || end - start + 2 * margin > to - from ? start - margin - from : end + margin > to ? end + margin - to : 0;
			const dy = shift(r.top, r.bottom, view.top, view.bottom);
			const dx = shift(r.left, r.right, view.left, view.right);
			if (!dx && !dy) return;
			if (root) root.scrollBy({ left: dx, top: dy, behavior: 'instant' });
			else window.scrollBy({ left: dx, top: dy, behavior: 'instant' });
		});
	});
</script>

<div
	class="strip axis-{axis}"
	class:zoomed={zoom !== null}
	class:seams
	bind:this={strip}
	style:--slice-w="{sliceW}px"
	style:--zoom={zoom}
	data-testid="strip"
	data-axis={axis}
>
	{#each slices as _, i (i)}
		{@const mounted = near.has(i) || i === active}
		<div class="slot" class:active={i === active} bind:this={slots[i]} data-strip={i} data-mounted={mounted ? '1' : '0'} aria-label={m['creator.strip_slice']({ number: i + 1 })}>
			{#if i > 0}<span class="seam" aria-hidden="true"></span>{/if}
			<span class="number" aria-hidden="true" data-testid="strip-number-{i}">{i + 1}</span>
			{#if mounted}
				<SliceCanvas
					bind:this={canvases[i]}
					{doc}
					slicePath={[...groupPath, 'slices', i]}
					{locale}
					{grid}
					{tool}
					{layers}
					selection={i === active ? selection : null}
					live={live && i === active}
					{zoom}
					{pieceName}
					{presets}
					strip={axis}
					open={i === active}
					onshown={i === active ? onshown : undefined}
					onselect={(next) => {
						take(i);
						onselect?.(i, next);
					}}
					oncreate={(t, rect) => {
						take(i);
						oncreate?.(i, t, rect);
					}}
				/>
			{:else}
				<div class="placeholder" style:--ratio={ratioOf(i)}></div>
			{/if}
		</div>
	{/each}
</div>

<style>
	/* vertical: one column at the reading width (zoomed: the slice's 1x width × the zoom), centred; a phone: full width */
	.strip.axis-vertical {
		display: flex;
		flex-direction: column;
		width: min(100%, var(--slice-w));
		margin: 0 auto;
		padding: 0 0 var(--space-15);
	}
	.strip.axis-vertical.zoomed {
		width: calc(var(--slice-w) * var(--zoom));
	}
	@media (max-width: 767px) {
		.strip.axis-vertical:not(.zoomed) {
			width: 100%;
		}
	}
	/* horizontal / endless: side by side, the area's height */
	.strip.axis-horizontal {
		display: flex;
		flex-direction: row;
		width: max-content;
		height: 100%;
	}
	.slot {
		position: relative;
		flex: none;
	}
	.axis-vertical .placeholder {
		width: 100%;
		aspect-ratio: 1 / var(--ratio);
	}
	.axis-horizontal .placeholder {
		height: 100cqh;
		aspect-ratio: 1 / var(--ratio);
	}
	.zoomed.axis-horizontal .placeholder {
		height: auto;
		width: calc(var(--slice-w) * var(--zoom));
	}
	.placeholder {
		background: var(--surface-panel);
	}
	/* the slice's number at its seam: left of the column (vertical), on the art's top-left corner (horizontal) */
	.number {
		position: absolute;
		top: 0;
		right: calc(100% + var(--space-05));
		padding: 0 var(--space-025);
		border: var(--border-width-hairline) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
		font-size: var(--type-caption-size);
		line-height: 1.4;
		pointer-events: none;
	}
	.slot.active .number {
		background: var(--select-live-border);
	}
	.axis-horizontal .number {
		right: auto;
		left: var(--space-025);
		top: var(--space-025);
		z-index: 5;
	}
	@media (max-width: 767px) {
		.axis-vertical .number {
			right: auto;
			left: var(--space-025);
			top: var(--space-025);
			z-index: 5;
		}
	}
	/* the seam as a hairline across the art (the View toggle), over the lettering, never in the pointer's way */
	.seam {
		display: none;
		position: absolute;
		z-index: 5;
		pointer-events: none;
		border: 0 dashed var(--ink);
	}
	.seams .seam {
		display: block;
	}
	.axis-vertical .seam {
		left: 0;
		right: 0;
		top: 0;
		border-top-width: var(--border-width-hairline);
	}
	.axis-horizontal .seam {
		top: 0;
		bottom: 0;
		left: 0;
		border-left-width: var(--border-width-hairline);
	}
</style>
