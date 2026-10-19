<script lang="ts">
	/**
	 * One lane of the Timeline (timeline.svelte – read its header first): its label (sticky) and its items on the axis – a
	 * listbox with a roving tabindex: ← → Home End move the selection, Enter selects (never Space – the app's play key);
	 * selected = inverted (.sc-selected) + an inset ink frame (a picture hides the inversion). A press on an item is a click
	 * unless it moves (then it seeks); on a lane with `selectable: false` it seeks at once like the background, and the keys
	 * only move the focus (the studio's take: a click scrubs, like the old TakeTimeline). With the listbox itself focused (a
	 * press on its background) the keys start from the lane's current item, not the first one. A `trim` shows the cut parts
	 * hatched; with `ontrim` its in / out are two sliders – siblings of the listbox, which owns only options – dragged
	 * (snapped to `lane.step`) or stepped by it (Shift ×10, Alt ×0.1), controlled by the props. Positions are % of the
	 * span; an instant / sliver keeps 0.25rem.
	 */
	import { tick, type Snippet } from 'svelte';
	import { clamp, stepFor } from './scrub.ts';
	import { clampTrim, formatTime, FRAME, frac, ordered, type Ms, type TimelineItem, type TimelineLane, type TimelineTrim } from './timeline.ts';
	import { uiLabels } from './ui-labels.ts';

	type Edge = 'in' | 'out';
	let { lane, span, selected, item, press, timeAtX, onselect, ontrim }: {
		lane: TimelineLane;
		span: Ms;
		selected: readonly string[];
		item?: Snippet<[TimelineItem, TimelineLane]>;
		/** the timeline's press on `el`: a seek drag – with `click`, a click unless it moves */
		press: (e: PointerEvent, el: HTMLElement, click?: () => void) => void;
		/** a pointer's clientX → ms on the axis */
		timeAtX: (clientX: number) => Ms;
		onselect?: (laneId: string, itemId: string) => void;
		ontrim?: (laneId: string, itemId: string, trim: { in: Ms; out: Ms }, final: boolean) => void;
	} = $props();

	const t = uiLabels();
	const uid = $props.id();
	const EDGES: Edge[] = ['in', 'out'];
	let box = $state<HTMLElement>();
	let active = $state<string | null>(null);
	const items = $derived(ordered(lane.items));
	const trimmed = $derived(items.flatMap((it) => (it.trim ? [{ it, trim: it.trim }] : [])));
	const step = $derived(lane.step ?? FRAME);
	/** the lane's current item: its selected one, else the one focused last; its one tab stop: that, else the first */
	const current = $derived(items.find((i) => selected.includes(i.id))?.id ?? (items.some((i) => i.id === active) ? active : undefined));
	const stop = $derived(current ?? items[0]?.id);
	const pct = (ms: Ms) => `${frac(ms, span) * 100}%`;
	const size = (it: TimelineItem) => Math.max(0, it.length ?? 0);
	const name = (it: TimelineItem) => it.label ?? it.id;
	/** a handle's range: in from 0 to out − min, out from in + min to the length */
	const bounds = (it: TimelineItem, trim: TimelineTrim, edge: Edge) => (edge === 'in' ? [0, trim.out - trim.min] : [trim.in + trim.min, size(it)]);
	const emit = (it: TimelineItem, trim: TimelineTrim, final: boolean) => ontrim?.(lane.id, it.id, { in: trim.in, out: trim.out }, final);

	/** the app picks this lane's items (`selectable: false`: a press on one seeks, the keys only move the focus) */
	const pick = $derived(lane.selectable === false ? undefined : onselect);

	function down(e: PointerEvent) {
		const id = (e.target as HTMLElement).closest<HTMLElement>('[data-item]')?.dataset.item;
		if (e.button === 0 && box) press(e, box, id && pick ? () => pick(lane.id, id) : undefined);
	}
	function keys(e: KeyboardEvent) {
		// the focused option – or, on the listbox itself (a press on the lane's background focuses it), the current item
		// (none: → / Home pick the first, as before)
		const at = (e.target as HTMLElement).closest<HTMLElement>('[data-item]')?.dataset.item ?? current;
		const i = items.findIndex((x) => x.id === at);
		const to = ({ ArrowLeft: i - 1, ArrowRight: i + 1, Home: 0, End: items.length - 1 } as Record<string, number>)[e.key];
		if (e.key === 'Enter' && i >= 0) pick?.(lane.id, items[i].id);
		if (to === undefined || !items.length || e.altKey || e.ctrlKey || e.metaKey) return;
		e.preventDefault();
		const next = items[clamp(to, 0, items.length - 1)];
		active = next.id;
		pick?.(lane.id, next.id);
		tick().then(() => ([...(box?.children ?? [])] as HTMLElement[]).find((c) => c.dataset.item === next.id)?.focus());
	}
	function trimDown(e: PointerEvent, it: TimelineItem, trim: TimelineTrim, edge: Edge) {
		if (e.button !== 0) return;
		const [el, id, x0, off] = [e.currentTarget as HTMLElement, e.pointerId, timeAtX(e.clientX), new AbortController()];
		el.setPointerCapture(id);
		let [last, moved] = [trim, false];
		const move = (ev: PointerEvent) => {
			if (ev.pointerId !== id) return;
			// by how far it moved (a grab beside the point does not jump), snapped to the lane's step: what the shot will show
			last = clampTrim({ ...last, [edge]: Math.round((trim[edge] + timeAtX(ev.clientX) - x0) / step) * step }, size(it), edge);
			moved = true;
			emit(it, last, false);
		};
		const end = (ev: PointerEvent) => {
			if (ev.pointerId !== id) return;
			off.abort();
			if (moved) emit(it, last, true);
		};
		el.addEventListener('pointermove', move, off);
		el.addEventListener('pointerup', end, off);
		el.addEventListener('pointercancel', end, off);
	}
	function trimKey(e: KeyboardEvent, it: TimelineItem, trim: TimelineTrim, edge: Edge) {
		const s = stepFor(step, e);
		const by = ({ ArrowLeft: -s, ArrowDown: -s, ArrowRight: s, ArrowUp: s, Home: -Infinity, End: Infinity } as Record<string, number>)[e.key];
		if (by === undefined || e.ctrlKey || e.metaKey) return;
		e.preventDefault();
		emit(it, clampTrim({ ...trim, [edge]: trim[edge] + by }, size(it), edge), true);
	}
</script>

<div class="label" title={lane.label}><span class="sc-label" id="{uid}-label">{lane.label}</span></div>
<div class="track">
	<div bind:this={box} class="items" role="listbox" aria-orientation="horizontal" aria-labelledby="{uid}-label" tabindex="-1"
		onpointerdown={down} onkeydown={keys} onfocusin={(e) => (active = (e.target as HTMLElement).dataset.item ?? active)}>
		{#each items as it (it.id)}
			{@const on = selected.includes(it.id)}
			<div class="item" class:sc-selected={on} role="option" aria-selected={on} aria-label={name(it)} tabindex={it.id === stop ? 0 : -1}
				title="{name(it)} · {formatTime(size(it), 's', 1)}" data-item={it.id} data-state={it.state}
				data-control-exempt="timeline item: a span of time, as wide as it lasts"
				style:left="min({pct(it.start)}, 100% - 0.25rem)" style:width="{(frac(it.start + size(it), span) - frac(it.start, span)) * 100}%">
				{#if item}{@render item(it, lane)}{:else}<span class="name">{it.label ?? ''}</span>{/if}
				{#if it.trim && size(it)}
					<span class="cut" style:width="{(it.trim.in / size(it)) * 100}%"></span>
					<span class="cut end" style:width="{(1 - it.trim.out / size(it)) * 100}%"></span>
				{/if}
			</div>
		{/each}
	</div>
	{#if ontrim}
		{#each trimmed as { it, trim } (it.id)}
			{#each EDGES as edge (edge)}
				{@const [lo, hi] = bounds(it, trim, edge)}
				{@const what = `${t(edge === 'in' ? 'timelineTrimIn' : 'timelineTrimOut')} · ${name(it)}`}
				<div class="handle {edge}" role="slider" tabindex="0" aria-label={what} aria-orientation="horizontal" aria-valuemin={lo} aria-valuemax={hi}
					aria-valuenow={trim[edge]} aria-valuetext={formatTime(trim[edge], 's')} title="{what} · {formatTime(trim[edge], 's')}"
					data-control-exempt="timeline trim handle" style:left={pct(it.start + trim[edge])}
					onpointerdown={(e) => trimDown(e, it, trim, edge)} onkeydown={(e) => trimKey(e, it, trim, edge)}></div>
			{/each}
		{/each}
	{/if}
</div>

<style>
	/* the label column (--timeline-label): sticky, so a scrolled strip keeps its lane names */
	.label {
		position: sticky;
		left: 0;
		z-index: 4;
		display: flex;
		align-items: center;
		min-width: 0;
		padding: 0 var(--space-05);
		background: var(--paper);
		border-right: var(--border-width-regular) solid var(--ink);
		border-bottom: var(--border-width-hairline) solid var(--ink);
	}
	.label > span, .name {
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}
	.track {
		position: relative;
		min-width: 0;
		overflow-x: clip; /* an instant / a grip at the span's end: no scrollbar at zoom 1 */
		border-bottom: var(--border-width-hairline) solid var(--ink);
	}
	.items { position: absolute; inset: 0; user-select: none; }
	.item {
		position: absolute;
		top: 0;
		bottom: 0;
		box-sizing: border-box;
		min-width: 0.25rem;
		overflow: hidden;
		/* the label hides on narrow items; the app's snippet can ask the item too (@container) */
		container-type: inline-size;
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-hairline) solid var(--ink);
		scroll-margin-inline: var(--timeline-label) 0;
	}
	/* a picture (the app's snippet) covers the inversion: the selected item also gets an inset ink frame */
	.item.sc-selected:not(:focus-visible) { outline: var(--border-width-focus) solid var(--ink); outline-offset: calc(-1 * var(--border-width-focus)); }
	.name {
		display: block;
		padding: 0 var(--space-025);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	@container (max-width: 2rem) {
		.name {
			display: none;
		}
	}
	/* the cut-away part of a trimmed item: hatched in the item's text colour (ink, paper when selected) */
	.cut {
		position: absolute;
		inset: 0 auto 0 0;
		opacity: 0.4;
		pointer-events: none;
		background: repeating-linear-gradient(135deg, transparent 0 0.25rem, currentColor 0 0.375rem);
	}
	.cut.end { inset: 0 0 0 auto; }
	/* a trim handle: a 3px ink bar on the trim point, its grip 0.75rem centred on it (a Divider's hit zone) – so half of it
	   stays grabbable at the strip's ends (a take at 0 under the sticky lane label, an out point on the clipped end) */
	.handle {
		position: absolute;
		top: 0;
		bottom: 0;
		z-index: 1;
		width: 0.75rem;
		margin-left: -0.375rem;
		cursor: ew-resize;
		touch-action: none;
		background: linear-gradient(var(--ink), var(--ink)) center / var(--border-width-focus) 100% no-repeat;
	}
	.handle:hover { background-size: 0.25rem 100%; }
</style>
