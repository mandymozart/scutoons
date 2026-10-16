<script lang="ts">
	import { tick as updated, untrack } from 'svelte';
	import type { Group } from '@scutoons/content';
	import { DEFAULT_SLICE } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import Slice from '../slice.svelte';
	import { stripFrames } from './strip-scroll.ts';
	import { ease, frameLayout, glide, keepPlace, placeFrames, revealTarget, stepTarget, stripFit } from './strip-loop.ts';
	import type { Motion, Overhang, Placement, StripFit, StripLayout } from './strip-loop.ts';
	import { STRIP_SHOW, stripDrag, type StripInput } from './strip-drag.ts';
	import { boxOverhang, decodeImages, rootRem, viewportWidth } from './strip-dom.ts';

	/**
	 * A horizontal group with `renderer.params.endless` (owner 2026-10-15, BACKLOG 10.100, TEXT-ACROSS-SLICES.md §8): the
	 * slices side by side as one screen-high band in the page flow, across the whole width. The reader drags it sideways
	 * (momentum, no snap) and it never ends: a frame that left the window on one side comes in on the other – no clones,
	 * no DOM moves (strip-loop.ts), so every slice and its observers exist once and fire once per real entry. The page
	 * scrolls past it as usual: the vertical wheel and vertical swipes stay the page's (strip-drag.ts). A grab cursor is
	 * the only cue (owner 2026-10-16, Q10). A picture too narrow for the screen narrows the window (centred on ink) or is
	 * whole and still. `sliceOffset`: the page index of the first slice (linked groups share one page).
	 */
	let { group, sliceOffset = 0 }: { group: Group; sliceOffset?: number } = $props();

	let band = $state<HTMLElement>();
	let track = $state<HTMLElement>();
	/** the frames at the band's height, the window and the root font size (px → rem), from the last measure */
	let geo = $state.raw<{ layout: StripLayout; fit: StripFit; rem: number } | null>(null);
	/** where the track and each frame are drawn */
	let placement = $state.raw<Placement | null>(null);
	let dragging = $state(false);
	/** the band is near the screen: every frame loads now (at normal priority) and is decoded */
	let near = $state(false);
	/** the band's size at the last measure, px – the observer skips a notification that changed nothing */
	let measured = { width: 0, height: 0 };

	/** how far the picture has moved right, px (any number – it repeats every turn) */
	let offset = 0;
	let dpr = 1;
	/** how far each frame's boxes reach past its sides, px – a frame wraps only once they are off screen too */
	let overhang: Overhang[] = [];
	/** a running glide (after a fling) or ease (keys, focus) – rAF only while it runs */
	let motion: Motion | null = null;
	let frame = 0;

	const frames = $derived(stripFrames(group.slices ?? [], DEFAULT_SLICE));
	const ratios = $derived(frames.map((f) => f.width / f.height));

	/** px → rem (sizes in rem, also from JS) */
	const rem = (px: number) => `${px / (geo?.rem ?? 16)}rem`;
	const frameEls = () => [...(track?.children ?? [])] as HTMLElement[];
	const frameOf = (target: EventTarget | null) => frameEls().findIndex((el) => target instanceof Node && el.contains(target));

	function render() {
		if (geo) placement = placeFrames(offset, geo.layout, geo.fit, dpr, overhang);
	}

	/** the frames from the band's height, the window from its width; a resize keeps the place */
	function measure() {
		if (!band) return;
		const rect = band.getBoundingClientRect();
		measured = { width: rect.width, height: rect.height };
		dpr = window.devicePixelRatio || 1;
		const layout = frameLayout(ratios, rect.height, dpr);
		overhang = boxOverhang(frameEls(), layout.widths);
		// a running glide / ease would write the old scale back
		stop();
		offset = keepPlace(offset, geo?.layout.turn ?? 0, layout.turn);
		geo = { layout, fit: stripFit(layout, rect.width, dpr, overhang), rem: rootRem() };
		render();
		// again once the frames have their widths (the next DOM update): the first measure has none to read
		void updated().then(refit);
	}

	/** the boxes' overhang from the drawn frames, and the window it allows */
	function refit() {
		if (!geo || !band) return;
		overhang = boxOverhang(frameEls(), geo.layout.widths);
		geo = { ...geo, fit: stripFit(geo.layout, measured.width, dpr, overhang) };
		render();
	}

	function tick(now: number) {
		frame = 0;
		if (!motion) return;
		const at = motion.at(now);
		offset = at.x;
		if (at.done) motion = null;
		render();
		if (motion) frame = requestAnimationFrame(tick);
	}
	function run(next: Motion | null) {
		stop();
		motion = next;
		if (motion) frame = requestAnimationFrame(tick);
	}
	/** stops a running glide / ease – true when one ran (the tap that stopped it is no click) */
	function stop() {
		const ran = motion !== null;
		motion = null;
		if (frame) cancelAnimationFrame(frame);
		frame = 0;
		return ran;
	}
	/** keys, focus: an ease to `to` – with reduced motion a jump (a drag and its glide stay, like native scrolling) */
	function slideTo(to: number) {
		if (!matchMedia('(prefers-reduced-motion: reduce)').matches) return run(ease(offset, to, performance.now()));
		stop();
		offset = to;
		render();
	}

	const input: StripInput = {
		enabled: () => !!geo && geo.fit.mode !== 'still',
		stop,
		move(dx) {
			offset += dx;
			render();
		},
		release(velocity) {
			if (geo) run(glide(offset, velocity, performance.now(), geo.layout.turn));
		},
		step(direction) {
			// presses add up: from the running ease's end
			if (geo) slideTo(stepTarget(motion?.ease ? motion.end : offset, direction, geo.layout));
		},
		reveal(target) {
			const index = frameOf(target);
			if (!geo || !placement || index < 0) return;
			const to = revealTarget(offset, index, placement, geo.layout, geo.fit.window);
			if (to !== null) slideTo(to);
		},
		dragging: (on) => (dragging = on)
	};

	/** the page's arrival (`?label=`, resume, the creator's Reader – chapter +page.svelte): the slice's frame at the left */
	function show(event: Event) {
		const index = frameOf(event.target);
		if (!geo || index < 0) return;
		stop();
		offset = -geo.layout.starts[index];
		render();
	}

	$effect(() => {
		const el = band;
		// the frames' ratios: a new set of slices is measured anew
		void ratios;
		if (!el || !track) return;
		// the band resizes with the window (not its frames: measure sizes them – WebKit reported a ResizeObserver loop)
		const observer = new ResizeObserver(() => {
			const rect = el.getBoundingClientRect();
			if (rect.width !== measured.width || rect.height !== measured.height) measure();
		});
		observer.observe(el);
		const nearing = new IntersectionObserver(
			(entries) => {
				if (!entries.some((entry) => entry.isIntersecting)) return;
				near = true;
				nearing.disconnect();
			},
			{ rootMargin: '100% 0px' }
		);
		nearing.observe(el);
		el.addEventListener(STRIP_SHOW, show);
		untrack(measure);
		return () => {
			observer.disconnect();
			nearing.disconnect();
			el.removeEventListener(STRIP_SHOW, show);
			stop();
		};
	});

	// decoded before they show (strip-dom.ts decodeImages)
	$effect(() => {
		if (near && band) return decodeImages(band);
	});
</script>

<!-- a region with a name, focusable: ← → move it (strip-drag.ts) -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
	class="band"
	class:still={geo?.fit.mode === 'still'}
	class:dragging
	role="region"
	tabindex="0"
	aria-label={m['chapter.strip_endless']()}
	data-endless
	data-fx-window
	data-group-label={group.label}
	bind:this={band}
	{@attach stripDrag(input)}
	{@attach viewportWidth}
>
	<div class="window" style:left={geo ? rem(geo.fit.inset) : null} style:width={geo ? rem(geo.fit.window) : null}>
		<div class="track" bind:this={track} style:transform={placement ? `translate3d(${rem(placement.x)}, 0, 0)` : null}>
			{#each frames as item, index}
				<div
					class="frame"
					style:left={geo && placement ? rem(geo.layout.starts[index] + placement.turns[index] * geo.layout.turn) : null}
					style:width={geo ? rem(geo.layout.widths[index]) : null}
					style:--plane={placement?.planes[index] ?? index + 1}
				>
					<Slice
						slice={item.slice}
						sliceIndex={index}
						pageIndex={sliceOffset + index}
						allSlices={group.slices ?? []}
						preload={near}
					/>
				</div>
			{/each}
		</div>
	</div>
</div>

<style>
	/* in the page flow across the whole width of the viewport (out of the reading column, like the pinned strip), as high
	   as the small viewport but 4rem less than the large one, so the page peeks in above or below – never dvh: following
	   the mobile toolbar, it made the page jump (owner 2026-10-10). At most 10lvh less: from 768px the root is 24px, and
	   4rem made a landscape phone's band too low to loop (844 × 390: 294px high, the four groups whole and still) */
	.band {
		position: relative;
		width: var(--viewport-width, 100vw);
		margin-left: calc(50% - var(--viewport-width, 100vw) / 2);
		height: calc(100vh - min(4rem, 10vh));
		height: min(100svh, calc(100lvh - min(4rem, 10lvh)));
		background: var(--ink);
		/* clip, not hidden: as a scroll container focus and scrollIntoView scrolled it out of step with the track */
		overflow: clip;
		cursor: grab;
		user-select: none;
		-webkit-user-select: none;
		-webkit-touch-callout: none;
	}
	.band.dragging {
		cursor: grabbing;
	}
	.band.still {
		cursor: auto;
	}
	.band:focus-visible {
		outline: var(--border-width-focus) solid var(--paper);
		outline-offset: calc(-1 * var(--border-width-focus));
	}
	/* the window: the whole band, or narrowed so a frame can wrap unseen (strip-loop.ts stripFit) – not `.view`, the old
	   static/css/styles.css caps that at 800px */
	.window {
		position: absolute;
		top: 0;
		bottom: 0;
		left: 0;
		width: 100%;
		overflow: clip;
	}
	/* one translate moves all; a frame wraps by its `left` (a transform would be a stacking context over crossing boxes) */
	.track {
		position: absolute;
		top: 0;
		left: 0;
		height: 100%;
		will-change: transform;
	}
	.frame {
		position: absolute;
		top: 0;
		height: 100%;
	}
	/* each slice a hairline wider than its frame, under the next one: at a fractional device pixel ratio the ink showed
	   through between the frames (Chromium, WebKit – measured) */
	.frame :global(.slice) {
		min-height: 0;
		width: calc(100% + var(--border-width-hairline));
	}
	/* the later slice on screen paints on top, at the wrap seam too (frame N left of frame 1); it reaches across slices
	   once a slice is no stacking context of its own (TEXT-ACROSS-SLICES.md §7 step 1) */
	.frame :global(.slice-layers) {
		z-index: var(--plane, 1);
	}
	.band :global(img) {
		-webkit-user-drag: none;
	}
</style>
