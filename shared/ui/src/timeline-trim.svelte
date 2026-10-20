<script lang="ts">
	/**
	 * A lane's trim (timeline-lane.svelte; research/TIMELINE.md §4 "C · Studio" – its prototype studio-layout/
	 * trim-prototype.html, variant C): per trimmed item three sliders – siblings of the lane's listbox, which owns only
	 * options –, controlled by the props (`ontrim` asks, the app sets the item's `trim`):
	 * - in / out: an ink line (paper edges – it reads on pictures) grabbable along the lane's height, a bracket cursor
	 *   ([ / ]) and a tab at its top inside the kept part; a coarse pointer (a finger) gets a 2.75rem zone, a bigger tab;
	 * - slip: a bar under the kept part's middle – or Alt-drag on the kept picture (`slip`, from the lane's press) – moves
	 *   both edges, the length kept, inside 0..length (none while nothing is cut).
	 * A drag moves by how far the pointer went (a grab beside the line never jumps), the moved point snapped to the lane's
	 * `step` (what the shot will show); ← → a step, snapped too (Shift ×10; Alt ×0.1, unsnapped), Home / End on a focused
	 * slider. Held still 450 ms the Timeline zooms to about a frame per picture around it, let go: the zoom before
	 * (`host.hold`); at the visible strip's ends it pans (`host.pan`); while a drag runs the view stays put (`host.grip`:
	 * no follow, no reveal). While trimming (a drag and 1.2 s after its release; a key: 1.2 s) readouts: in / out next to
	 * their lines, outside the kept part (inside a row lower at the visible ends), the kept length in its middle.
	 * Contract: ontrim(lane, item, { in, out }, final, edge) – a drag = calls with final = false (only when the trim
	 * changed), then ONE with final = true on release (also when its element goes away); Escape / pointercancel = the
	 * values at the start, final = true; a press without a change: no call; a key: final = true each.
	 */
	import { stepFor } from './scrub.ts';
	import { formatTime, frac, trimBy, trimReadouts, type Ms, type TimelineItem, type TimelineLane, type TimelineTrim, type TrimEdge, type TrimHost } from './timeline.ts';
	import { uiLabels } from './ui-labels.ts';

	type Trimmed = { it: TimelineItem; trim: TimelineTrim };
	let { lane, items, span, step, host, ontrim }: {
		lane: TimelineLane;
		/** the lane's trimmed items */
		items: Trimmed[];
		span: Ms;
		step: Ms;
		host: TrimHost;
		ontrim: (laneId: string, itemId: string, trim: { in: Ms; out: Ms }, final: boolean, edge: TrimEdge) => void;
	} = $props();

	const t = uiLabels();
	const EDGES = ['in', 'out'] as const;
	const HOLD = 450;
	/** the bracket cursors (the prototype's): SVG images, their hot spot in the image's own pixels */
	const svg = (d: string, x: number, or: string) =>
		`url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24'%3E%3Cg fill='none'%3E%3Cpath d='${d}' stroke='white' stroke-width='4'/%3E%3Cpath d='${d}' stroke='black' stroke-width='2'/%3E%3C/g%3E%3C/svg%3E") ${x} 12, ${or}`;
	const CURSOR: Record<TrimEdge, string> = {
		in: svg('M15 4H10V20H15M3 12H8M5 9L2 12L5 15', 10, 'w-resize'),
		out: svg('M9 4H14V20H9M21 12H16M19 9L22 12L19 15', 14, 'e-resize'),
		slip: svg('M7 5H4V19H7M17 5H20V19H17M8 12H16M10 9L7 12L10 15M14 9L17 12L14 15', 12, 'ew-resize')
	};
	const pct = (ms: Ms) => `${frac(ms, span) * 100}%`;
	const size = (it: TimelineItem) => Math.max(0, it.length ?? 0);
	const name = (it: TimelineItem) => it.label ?? it.id;
	const range = (trim: TimelineTrim) => `${formatTime(trim.in, 's')} – ${formatTime(trim.out, 's')}`;
	/** room to slip: something is cut */
	const slack = (it: TimelineItem, trim: TimelineTrim) => size(it) - (trim.out - trim.in);
	/** a handle's range: in from 0 to out − min, out from in + min to the length */
	const bounds = (it: TimelineItem, trim: TimelineTrim, edge: 'in' | 'out') => (edge === 'in' ? [0, trim.out - trim.min] : [trim.in + trim.min, size(it)]);
	const emit = (it: TimelineItem, trim: TimelineTrim, final: boolean, edge: TrimEdge) => ontrim(lane.id, it.id, { in: trim.in, out: trim.out }, final, edge);

	/** the trim in progress (a drag, or a key's 1.2 s): the readouts show for it */
	let active = $state<{ id: string; edge: TrimEdge } | null>(null);
	let peek: ReturnType<typeof setTimeout> | undefined;
	/** ends the drag in progress (the trim goes away under it) */
	let stop: (() => void) | undefined;
	let box = $state<HTMLElement>();
	let chipIn = $state<HTMLElement>(), chipOut = $state<HTMLElement>();
	const shown = $derived(active ? (items.find((x) => x.it.id === active?.id) ?? null) : null);
	const text = $derived(shown ? trimReadouts(shown.trim, step) : null);

	/** the readouts of `id` for 1.2 s (a key, a drag's release) */
	const linger = (id: string, edge: TrimEdge) => ((active = { id, edge }), (peek = setTimeout(() => (active = null), 1200)));
	/** a drag of `edge` from `e` on `el` (a grip, the slip bar, the lane's listbox for an Alt-drag) */
	function drag(e: PointerEvent, el: HTMLElement, it: TimelineItem, start: TimelineTrim, edge: TrimEdge) {
		const [id, off, cursor] = [e.pointerId, new AbortController(), el.style.cursor];
		let [base, x0, x, still, last] = [start, host.timeAtX(e.clientX), e.clientX, e.clientX, start];
		let [moved, held, raf, timer] = [false, false, 0, undefined as ReturnType<typeof setTimeout> | undefined];
		el.setPointerCapture(id);
		el.style.cursor = CURSOR[edge];
		clearTimeout(peek);
		active = { id: it.id, edge };
		host.grip(true);
		// only a changed trim is a call (a click's jitter on a grip asks nothing)
		const to = (clientX: number) => {
			const next = trimBy(base, size(it), edge, host.timeAtX(clientX) - x0, step);
			if (next.in === last.in && next.out === last.out) return;
			[last, moved] = [next, true];
			emit(it, last, false, edge);
		};
		const arm = () => (clearTimeout(timer), held || (timer = setTimeout(hold, HOLD)));
		async function hold() {
			held = true;
			const at = edge === 'slip' ? host.timeAtX(x) : it.start + last[edge];
			await host.hold(at, step, el.closest<HTMLElement>('[data-axis]')?.clientHeight ?? 0);
			// the view changed under the still pointer: go on from here (no jump)
			[base, x0] = [last, host.timeAtX(x)];
		}
		// every frame: a grip at the visible end pans (once it moved or was held); its element gone (the item lost its trim
		// – a removed element's lostpointercapture never reaches it): the drag ends
		const frame = () => {
			if (!el.isConnected) return end(null, false);
			if ((moved || held) && host.pan(x)) to(x);
			raf = requestAnimationFrame(frame);
		};
		const move = (ev: PointerEvent) => {
			if (ev.pointerId !== id) return;
			x = ev.clientX;
			// moved off the still point: the hold waits again
			if (Math.abs(x - still) > (ev.pointerType === 'mouse' ? 4 : 10)) {
				still = x;
				arm();
			}
			to(x);
		};
		const end = (ev: Event | null, back: boolean) => {
			if (ev instanceof PointerEvent && ev.pointerId !== id) return;
			off.abort();
			stop = undefined;
			clearTimeout(timer);
			cancelAnimationFrame(raf);
			if (el.hasPointerCapture(id)) el.releasePointerCapture(id);
			el.style.cursor = cursor;
			if (held) host.unhold();
			host.grip(false);
			if (moved && !back) linger(it.id, edge);
			else active = null;
			if (moved) emit(it, back ? start : last, true, edge);
		};
		const esc = (ev: KeyboardEvent) => ev.key === 'Escape' && (ev.preventDefault(), ev.stopImmediatePropagation(), end(ev, true));
		el.addEventListener('pointermove', move, off);
		el.addEventListener('pointerup', (ev) => end(ev, false), off);
		el.addEventListener('pointercancel', (ev) => end(ev, true), off);
		el.addEventListener('lostpointercapture', (ev) => end(ev, false), off);
		addEventListener('keydown', esc, { capture: true, signal: off.signal });
		stop = () => end(null, false);
		arm();
		raf = requestAnimationFrame(frame);
	}
	function down(e: PointerEvent, it: TimelineItem, trim: TimelineTrim, edge: TrimEdge) {
		if (e.button === 0) drag(e, e.currentTarget as HTMLElement, it, trim, edge);
	}
	/** Alt + a press on a trimmed item's kept part (the lane's listbox): a slip – false when it is none */
	export function slip(e: PointerEvent, id: string): boolean {
		const x = items.find((i) => i.it.id === id);
		const at = x ? host.timeAtX(e.clientX) - x.it.start : NaN;
		if (!x || !(at >= x.trim.in && at < x.trim.out) || slack(x.it, x.trim) < 1) return false;
		drag(e, e.currentTarget as HTMLElement, x.it, x.trim, 'slip');
		return true;
	}
	function key(e: KeyboardEvent, it: TimelineItem, trim: TimelineTrim, edge: TrimEdge) {
		const s = stepFor(step, e);
		const by = ({ ArrowLeft: -s, ArrowDown: -s, ArrowRight: s, ArrowUp: s, Home: -Infinity, End: Infinity } as Record<string, number>)[e.key];
		if (by === undefined || e.ctrlKey || e.metaKey) return;
		e.preventDefault();
		emit(it, trimBy(trim, size(it), edge, by, e.altKey ? 0 : step), true, edge);
		clearTimeout(peek);
		linger(it.id, edge);
	}

	/** the in / out readouts sit outside the kept part; one that would leave the visible part turns inwards (a row lower),
	 *  one whose line is out of view hides – measured, written to the chips' attributes (never to state) */
	function place() {
		const [area, s] = [host.area(), shown];
		if (!box || !area || !s) return;
		const r = box.getBoundingClientRect();
		for (const [el, t, side] of [[chipIn, s.trim.in, -1], [chipOut, s.trim.out, 1]] as const) {
			if (!el) continue;
			const [x, c, flipped] = [r.left + frac(s.it.start + t, span) * r.width, el.getBoundingClientRect(), el.hasAttribute('data-flip')];
			// its gap to the line, on whichever side it is now
			const gap = (side < 0) !== flipped ? x - c.right : c.left - x;
			el.toggleAttribute('data-flip', side < 0 ? x - gap - c.width < area.left : x + gap + c.width > area.right);
			el.toggleAttribute('data-away', x < area.left || x > area.right);
		}
	}
	// while they show: placed now and every frame (the view moves under them – a pan, a hold, its release)
	$effect(() => {
		if (!shown || !chipIn || !chipOut) return;
		place();
		let raf = requestAnimationFrame(function again() { place(); raf = requestAnimationFrame(again); });
		return () => cancelAnimationFrame(raf);
	});
	$effect(() => () => (stop?.(), clearTimeout(peek)));
</script>

{#each items as { it, trim } (it.id)}
	{#each EDGES as edge (edge)}
		{@const [lo, hi] = bounds(it, trim, edge)}
		{@const what = `${t(edge === 'in' ? 'timelineTrimIn' : 'timelineTrimOut')} · ${name(it)}`}
		<div class="grip {edge}" class:on={active?.id === it.id && active.edge === edge} role="slider" tabindex="0" aria-label={what} aria-orientation="horizontal"
			aria-valuemin={lo} aria-valuemax={hi} aria-valuenow={trim[edge]} aria-valuetext={formatTime(trim[edge], 's')} title="{what} · {formatTime(trim[edge], 's')}"
			data-control-exempt="timeline trim handle" style:left={pct(it.start + trim[edge])} style:cursor={CURSOR[edge]}
			onpointerdown={(e) => down(e, it, trim, edge)} onkeydown={(e) => key(e, it, trim, edge)}><span class="flag" aria-hidden="true">{edge === 'in' ? '[' : ']'}</span></div>
	{/each}
	{#if slack(it, trim) >= 1}
		{@const what = `${t('timelineSlip')} · ${name(it)}`}
		<div class="slip" class:on={active?.id === it.id && active.edge === 'slip'} role="slider" tabindex="0" aria-label={what} aria-orientation="horizontal"
			aria-valuemin={0} aria-valuemax={slack(it, trim)} aria-valuenow={trim.in} aria-valuetext={range(trim)} title="{what} · {range(trim)}"
			data-control-exempt="timeline slip bar" style:left={pct(it.start + (trim.in + trim.out) / 2)} style:cursor={CURSOR.slip}
			onpointerdown={(e) => down(e, it, trim, 'slip')} onkeydown={(e) => key(e, it, trim, 'slip')}></div>
	{/if}
{/each}
{#if shown && text}
	<div class="readouts" bind:this={box} aria-hidden="true">
		<span class="sc-chip at-in" bind:this={chipIn} style:left={pct(shown.it.start + shown.trim.in)}>{text.in}</span>
		<span class="sc-chip at-out" bind:this={chipOut} style:left={pct(shown.it.start + shown.trim.out)}>{text.out}</span>
		<span class="sc-chip at-kept" style:left={pct(shown.it.start + (shown.trim.in + shown.trim.out) / 2)}>{text.kept}</span>
	</div>
{/if}

<style>
	/* the sizes a finger needs where the pointer is one (TIMELINE §4 item 1): the grip zone, the tab, the slip bar */
	.grip, .slip, .readouts {
		--grip: 0.75rem;
		--flag-w: 1rem;
		--flag-h: 1.75rem;
		--slip-w: 3rem;
		--slip-h: 1.25rem;
	}
	@media (pointer: coarse) {
		.grip, .slip, .readouts {
			--grip: 2.75rem;
			--flag-w: 1.75rem;
			--flag-h: var(--control-height);
			--slip-w: 5rem;
			--slip-h: var(--control-height);
		}
	}
	/* a grip: the line on the trim point, its zone centred on it (a Divider's hit zone) – so half of it stays grabbable at
	   the strip's ends (a take at 0 under the sticky lane label, an out point on the clipped end) */
	.grip {
		position: absolute;
		top: 0;
		bottom: 0;
		z-index: 1;
		width: var(--grip);
		margin-left: calc(var(--grip) / -2);
		touch-action: none;
	}
	.grip::before {
		content: '';
		position: absolute;
		top: 0;
		bottom: 0;
		left: 50%;
		width: var(--border-width-focus);
		transform: translateX(-50%);
		background: var(--ink);
		outline: var(--border-width-hairline) solid var(--paper);
	}
	.grip:hover::before, .grip.on::before { width: 0.25rem; }
	/* the tab (a flag) at the line's top, inside the kept part */
	.flag {
		position: absolute;
		top: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		width: var(--flag-w);
		height: var(--flag-h);
		background: var(--ink);
		color: var(--paper);
		font-size: var(--type-caption-size);
	}
	.grip.in .flag { left: 50%; }
	.grip.out .flag { right: 50%; }
	/* the slip bar under the kept part's middle: ink with a paper grip pattern */
	.slip {
		position: absolute;
		bottom: 0;
		z-index: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		width: var(--slip-w);
		height: var(--slip-h);
		margin-left: calc(var(--slip-w) / -2);
		background: var(--ink);
		outline: var(--border-width-hairline) solid var(--paper);
		touch-action: none;
	}
	.slip::before {
		content: '';
		width: 50%;
		height: 40%;
		background: repeating-linear-gradient(90deg, var(--paper) 0 var(--border-width-regular), transparent 0 0.375rem);
	}
	.grip:focus-visible, .slip:focus-visible { outline: var(--border-width-focus) solid var(--ink); outline-offset: var(--border-width-hairline); }
	/* the readouts, one row under the tabs: in left of its line, out right of its line (outside the kept part, clear of the
	   grips), the kept length in its middle; one turned inwards at a visible end (data-flip) a row lower – two rows fit the
	   studio's lane (≈ 7rem) above the slip bar (three covered it); clipped: never a scrollbar that would resize the strip */
	.readouts { --gap: calc(var(--grip) / 2 + var(--space-025)); --row: calc(var(--flag-h) + var(--space-05)); position: absolute; inset: 0; z-index: 1; overflow: clip; pointer-events: none; }
	.sc-chip { position: absolute; top: var(--row); text-transform: none; letter-spacing: normal; font-variant-numeric: tabular-nums; }
	.at-in { transform: translateX(calc(-100% - var(--gap))); }
	.at-out { transform: translateX(var(--gap)); }
	.at-kept { transform: translateX(-50%); }
	/* data-flip / data-away: written by place() */
	.sc-chip:global([data-flip]) { top: calc(var(--row) + 1.75rem); }
	.at-in:global([data-flip]) { transform: translateX(var(--gap)); }
	.at-out:global([data-flip]) { transform: translateX(calc(-100% - var(--gap))); }
	.sc-chip:global([data-away]) { visibility: hidden; }
</style>
