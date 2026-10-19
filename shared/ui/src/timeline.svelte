<script lang="ts">
	/**
	 * The Timeline (agents/DOCKING.md "Bottom region"; owner 2026-10-19: "the timeline underneath the other windows was
	 * also good" – one timeline for the storyboard studio now, the creator and the live mode, research/LIVE.md, later):
	 * lanes of items (timeline-lane.svelte) on ONE linear axis in ms (timeline.ts – a shot as wide as its reading time, no
	 * per-item minimum that bends the ruler), the ruler with ticks + markers, the playhead, zoom (1 = the span fits).
	 * No transport: the app owns the clock and plays – no Play button, no Space, never PageUp / PageDown (the score's
	 * pedal keys, research/SCORES.md §4). It shows and asks (onseek / onselect / ontrim); without them it is read-only.
	 * Pointer: press + drag on the ruler or a lane's background seeks (a finger pans a lane, a tap seeks); a click on an
	 * item selects (a lane with `selectable: false` seeks there too); Ctrl / ⌘ + wheel zooms around the pointer; + / − / 0
	 * (fit) while the focus is inside. `reveal` brings a range into view when it changes (zoomed in to `revealFill` of the
	 * view, e.g. a take to trim precisely) and when the app sets the zoom (its remembered zoom per step – then only panned
	 * there, the zoom stays); `follow` keeps the playhead in view – a manual scroll pauses it until the playhead is back in
	 * view or the next playback starts.
	 * One group, not a landmark: the AppShell's bottom region around it is the region.
	 * Sizes: the strip is --zoom × the view wide (cqi of the scroller), everything on it in % – no px, no root font size.
	 */
	import { tick, untrack, type Snippet } from 'svelte';
	import Lane from './timeline-lane.svelte';
	import { clamp, stepFor } from './scrub.ts';
	import { followView, formatTime, frac, inView, panView, revealView, tickStep, ticks, timeAt, zoomAround, ZOOM_MAX, ZOOM_MIN } from './timeline.ts';
	import type { Ms, TimelineItem, TimelineLane, TimelineMarker, View } from './timeline.ts';
	import { uiLabels } from './ui-labels.ts';

	let { lanes, span, markers = [], playhead, selected = [], zoom = $bindable(1), reveal, revealFill = 0, follow = false, label, item, onseek, onselect, ontrim }: {
		lanes: TimelineLane[];
		/** the axis' length – everything on it is a share of it */
		span: Ms;
		/** scenes, groups, rows, bars: a label with a line on the ruler */
		markers?: TimelineMarker[];
		playhead?: Ms;
		/** item ids, shown inverted */
		selected?: readonly string[];
		/** 1 – 64: how many views wide the strip is */
		zoom?: number;
		/** a range brought into view whenever it changes (e.g. the selected shot); `revealFill` 0.5 also zooms in until it takes half the view */
		reveal?: readonly [Ms, Ms];
		revealFill?: number;
		follow?: boolean;
		/** the group's accessible name */
		label: string;
		/** an item's inside (the studio: the frame, the review dot, the handheld mark); default: its label */
		item?: Snippet<[TimelineItem, TimelineLane]>;
		onseek?: (t: Ms, final: boolean) => void;
		onselect?: (laneId: string, itemId: string) => void;
		ontrim?: (laneId: string, itemId: string, trim: { in: Ms; out: Ms }, final: boolean) => void;
	} = $props();

	const t = uiLabels();
	let scroller = $state<HTMLElement>();
	let strip = $state<HTMLElement>();
	/** measured: the scroller's and the label column's widths, the scroll position (px, only ever used as ratios) */
	let fit = $state(0), labelW = $state(0), scrollPx = $state(0);
	/** `ours`: the scrollLeft set here last (its scroll events are not the person's); `revealed`: the last reveal's values, '' = none;
	 *  `applied`: the zoom set here last (another one came from the app; NaN = none yet); `applies`: the last apply wins */
	let [ours, paused, revealed, applied, applies] = [-1, false, '', NaN, 0];
	const z = $derived(clamp(zoom, ZOOM_MIN, ZOOM_MAX));
	const viewW = $derived(Math.max(1, fit - labelW));
	/** a tick label needs about 4rem: 0.8 of the measured 5rem label column (never the root font size) */
	const perView = $derived(labelW ? Math.max(2, Math.floor(viewW / (0.8 * labelW))) : 8);
	const from = $derived(scrollPx / (viewW * z));
	/** only the ticks around the view are drawn (a long show at zoom 64 would make thousands) */
	const rulerTicks = $derived(ticks(span, z, perView, [timeAt(from - 1 / z, span), timeAt(from + 2 / z, span)]));
	const step = $derived(tickStep(span / z, perView));
	const rows = $derived(['var(--control-height)', ...lanes.map((l) => (l.tall ? 'minmax(var(--control-height), 1fr)' : 'var(--control-height)'))].join(' '));
	const head = $derived(playhead === undefined ? null : frac(playhead, span));
	const pct = (ms: Ms) => `${frac(ms, span) * 100}%`;
	/** the view now and a pointer's time, read from the DOM (the strip as laid out) */
	const current = (): View => ({ zoom: z, scrollFrac: (scroller?.scrollLeft ?? 0) / (strip?.getBoundingClientRect().width || 1) });
	const timeAtX = (clientX: number): Ms => (strip ? timeAt((clientX - strip.getBoundingClientRect().left) / strip.getBoundingClientRect().width, span) : 0);
	async function apply(v: View) {
		const n = ++applies;
		applied = clamp(v.zoom, ZOOM_MIN, ZOOM_MAX);
		zoom = v.zoom;
		await tick();
		// two in one turn (a reveal and the app's zoom on mount): only the last scrolls (the first's share is of the old width)
		if (n !== applies || !scroller || !strip) return;
		scroller.scrollLeft = v.scrollFrac * strip.getBoundingClientRect().width;
		ours = scrollPx = scroller.scrollLeft;
	}
	function scrolled() {
		scrollPx = scroller?.scrollLeft ?? 0;
		if (Math.abs(scrollPx - ours) <= 1) return;
		ours = -1;
		// the person scrolled: following pauses while the playhead is out of view
		if (follow && head !== null) paused = !inView(current(), head);
	}
	/** a press on `el` (that pointer only): seeks while it moves (at once for a mouse on the background / the ruler) – with `click`, a click unless it moves */
	function press(e: PointerEvent, el: HTMLElement, click?: () => void, now = !click && e.pointerType === 'mouse') {
		if (!onseek && !click) return;
		const [x0, id, off] = [e.clientX, e.pointerId, new AbortController()];
		let [at, moving] = [timeAtX(x0), now];
		el.setPointerCapture(id);
		if (moving) onseek?.(at, false);
		const move = (ev: PointerEvent) => {
			if (ev.pointerId !== id || (!moving && Math.abs(ev.clientX - x0) < (ev.pointerType === 'mouse' ? 3 : 6))) return;
			if (!moving && ev.pointerType !== 'mouse') return off.abort(); // a finger pans a lane (the browser scrolls): no seek, no tap
			moving = true;
			onseek?.((at = timeAtX(ev.clientX)), false);
		};
		const end = (ev: PointerEvent) => {
			if (ev.pointerId !== id) return;
			off.abort();
			if (moving) onseek?.(at, true);
			else if (ev.type === 'pointerup') (click ?? (() => onseek?.(at, true)))();
		};
		el.addEventListener('pointermove', move, off);
		el.addEventListener('pointerup', end, off);
		el.addEventListener('pointercancel', end, off);
	}
	function wheel(e: WheelEvent) {
		if (!(e.ctrlKey || e.metaKey) || !strip) return;
		e.preventDefault();
		const [v, r] = [current(), strip.getBoundingClientRect()];
		const dy = clamp(e.deltaMode ? e.deltaY * 33 : e.deltaY, -120, 120); // lines → about a notch; a pinch comes in small steps
		apply(zoomAround(z, Math.exp(-dy / 500), clamp(((e.clientX - r.left) / r.width - v.scrollFrac) * z, 0, 1), v.scrollFrac));
	}
	/** + / − zoom around the playhead (else the middle), 0 fits – anywhere inside, never with a modifier (browser zoom) */
	function keys(e: KeyboardEvent) {
		const factor = ({ '+': 1.25, '=': 1.25, '-': 0.8, '0': 0 } as Record<string, number>)[e.key];
		if (factor === undefined || e.ctrlKey || e.metaKey || e.altKey || (e.target as HTMLElement).closest('input, textarea, select, [contenteditable]')) return;
		e.preventDefault();
		const v = current();
		apply(factor ? zoomAround(z, factor, head !== null && inView(v, head) ? (head - v.scrollFrac) * z : 0.5, v.scrollFrac) : { zoom: 1, scrollFrac: 0 });
	}
	/** the playhead's slider: ← → one ruler step (Shift ×10, Alt ×0.1), Home / End */
	function rulerKey(e: KeyboardEvent) {
		const s = stepFor(step, e);
		const by = ({ ArrowLeft: -s, ArrowDown: -s, ArrowRight: s, ArrowUp: s, Home: -Infinity, End: Infinity } as Record<string, number>)[e.key];
		if (by === undefined || !onseek || e.ctrlKey || e.metaKey) return;
		e.preventDefault();
		onseek(clamp((playhead ?? 0) + by, 0, span), true);
	}

	$effect(() => {
		const [r, key] = [reveal, reveal ? `${reveal[0]}:${reveal[1]}` : ''];
		if (!strip || key === revealed) return;
		revealed = key;
		if (r) untrack(() => apply(revealView(current(), r, span, revealFill)));
	});
	// the app set the zoom (bind:zoom – the studio's remembered zoom per step): the scroll kept its px from the other zoom
	// and the range above is unchanged – it comes back into view at that zoom; the first run is the mount
	$effect(() => {
		const now = z;
		if (!strip || now === applied) return;
		const outside = !Number.isNaN(applied);
		applied = now;
		untrack(() => outside && reveal && apply(panView(current(), reveal, span)));
	});
	$effect(() => {
		// a new playback always starts following (a scroll during the last one paused it)
		if (!follow) return void (paused = false);
		const f = head;
		if (f === null || !strip) return;
		untrack(() => {
			const v = current();
			if (paused) paused = !inView(v, f);
			else if (!inView(v, f)) apply(followView(v, f));
		});
	});
</script>

<!-- the zoom keys belong to the whole timeline, wherever the focus is inside it -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="sc-timeline" role="group" aria-label={label} style:--zoom={z} onkeydown={keys}>
	<div class="scroller" bind:this={scroller} bind:clientWidth={fit} onscroll={scrolled} onwheel={wheel}>
		<div class="body" style:grid-template-rows={rows}>
			<div class="corner" aria-hidden="true" bind:clientWidth={labelW}>{playhead === undefined ? '' : formatTime(playhead, 'clock', 1)}</div>
			<div class="ruler" role="slider" tabindex="0" aria-label={t('timelinePlayhead')} aria-orientation="horizontal" aria-valuemin={0} aria-valuemax={span}
				aria-valuenow={playhead ?? 0} aria-valuetext={formatTime(playhead ?? 0, 'clock', 1)} aria-readonly={!onseek} title={t('timelineHint')}
				onpointerdown={(e) => e.button === 0 && press(e, e.currentTarget, undefined, true)} onkeydown={rulerKey}>
				{#each rulerTicks as k (k.at)}<span class="tick" style:left={pct(k.at)}>{k.label}</span>{/each}
				{#each markers as m}<span class="marker" style:left={pct(m.at)}>{m.label}</span>{/each}
			</div>
			{#each lanes as lane (lane.id)}
				<Lane {lane} {span} {selected} {item} {press} {timeAtX} {onselect} {ontrim} />
			{/each}
			<div class="strip" bind:this={strip} aria-hidden="true">
				{#if head !== null}<span class="head" style:left="{head * 100}%"></span>{/if}
			</div>
		</div>
	</div>
</div>

<style>
	.sc-timeline {
		--timeline-label: 5rem;
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		background: var(--paper);
	}
	.scroller {
		flex: 1 1 auto;
		min-height: 0;
		overflow: auto;
		overscroll-behavior-x: contain;
		/* 100cqi = the scroller's width: the strip gets its size without being measured */
		container-type: inline-size;
	}
	.body {
		position: relative;
		display: grid;
		grid-template-columns: var(--timeline-label) minmax(0, 1fr);
		width: calc(var(--timeline-label) + var(--zoom) * (100cqi - var(--timeline-label)));
		min-height: 100%;
	}
	.corner, .ruler {
		position: sticky;
		top: 0;
		z-index: 2;
		background: var(--paper);
		border-bottom: var(--border-width-regular) solid var(--ink);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-variant-numeric: tabular-nums;
	}
	.corner {
		left: 0;
		z-index: 5;
		display: flex;
		align-items: center;
		padding: 0 var(--space-05);
		border-right: var(--border-width-regular) solid var(--ink);
	}
	.ruler { overflow: hidden; cursor: ew-resize; touch-action: none; user-select: none; }
	/* markers (scenes) on the upper half with a 2px line, ticks on the lower half with a hairline */
	.tick, .marker {
		position: absolute;
		padding-left: var(--space-025);
		white-space: nowrap;
	}
	.tick { bottom: 0; color: var(--text-muted); border-left: var(--border-width-hairline) solid var(--ink); }
	.marker { top: 0; border-left: var(--border-width-regular) solid var(--ink); }
	/* the playhead's layer over the ruler and the lanes, under the sticky labels; clipped: a playhead at the end sticks out */
	.strip { position: absolute; inset: 0 0 0 var(--timeline-label); z-index: 3; pointer-events: none; overflow: clip; }
	/* an ink line with paper edges – it reads on an inverted item and on pictures too; a cap on the ruler */
	.head {
		position: absolute;
		top: 0;
		bottom: 0;
		width: var(--border-width-regular);
		margin-left: calc(var(--border-width-regular) / -2);
		background: var(--ink);
		outline: var(--border-width-hairline) solid var(--paper);
	}
	.head::before {
		content: '';
		position: absolute;
		top: 0;
		left: calc(50% - 0.375rem);
		width: 0.75rem;
		height: 0.5rem;
		background: var(--ink);
		clip-path: polygon(0 0, 100% 0, 50% 100%);
	}
</style>
