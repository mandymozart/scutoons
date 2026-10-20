<script lang="ts">
	/**
	 * One lane of the Timeline (timeline.svelte – read its header first): its label (sticky) and its items on the axis – a
	 * listbox with a roving tabindex: ← → Home End move the selection, Enter selects (never Space – the app's play key);
	 * selected = inverted (.sc-selected) + an inset ink frame (a picture hides the inversion). A press on an item is a click
	 * unless it moves (then it seeks); on a lane with `selectable: false` it seeks at once like the background, and the keys
	 * only move the focus (e.g. a take to scrub: a click scrubs). With the listbox itself focused (a press on its
	 * background) the keys start from the lane's current item, not the first one. A `trim` shows the cut parts hatched;
	 * with `ontrim` its in / out lines and its slip bar are sliders (timeline-trim.svelte – siblings of the listbox, which
	 * owns only options), and Alt + a press on the kept part slips it. Positions are % of the span; an instant / sliver
	 * keeps 0.25rem. The track is `data-axis` + `data-lane` (the Timeline's hover finds the lane there); while the Timeline
	 * previews items on hover (`previewed`) they carry no title tooltip – one box at a time.
	 */
	import { tick, type Snippet } from 'svelte';
	import { clamp } from './scrub.ts';
	import { formatTime, FRAME, frac, ordered, type Ms, type TimelineItem, type TimelineLane, type TrimEdge, type TrimHost } from './timeline.ts';
	import Trim from './timeline-trim.svelte';

	let { lane, span, selected, item, previewed = false, press, host, onselect, ontrim }: {
		lane: TimelineLane;
		span: Ms;
		selected: readonly string[];
		item?: Snippet<[TimelineItem, TimelineLane]>;
		/** the Timeline shows a hover preview: no title tooltip on the items */
		previewed?: boolean;
		/** the timeline's press on `el`: a seek drag – with `click`, a click unless it moves */
		press: (e: PointerEvent, el: HTMLElement, click?: () => void) => void;
		/** the Timeline's side of a trim drag: the time under a pointer, hold-to-zoom, panning, the visible part */
		host: TrimHost;
		onselect?: (laneId: string, itemId: string) => void;
		ontrim?: (laneId: string, itemId: string, trim: { in: Ms; out: Ms }, final: boolean, edge: TrimEdge) => void;
	} = $props();

	const uid = $props.id();
	let box = $state<HTMLElement>();
	let trimmer = $state<{ slip: (e: PointerEvent, id: string) => boolean }>();
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

	/** the app picks this lane's items (`selectable: false`: a press on one seeks, the keys only move the focus) */
	const pick = $derived(lane.selectable === false ? undefined : onselect);

	function down(e: PointerEvent) {
		const id = (e.target as HTMLElement).closest<HTMLElement>('[data-item]')?.dataset.item;
		if (e.button !== 0 || !box || (e.altKey && id && trimmer?.slip(e, id))) return; // Alt on a kept part: a slip
		press(e, box, id && pick ? () => pick(lane.id, id) : undefined);
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
</script>

<div class="label" title={lane.label}><span class="sc-label" id="{uid}-label">{lane.label}</span></div>
<div class="track" data-axis data-lane={lane.id}>
	<div bind:this={box} class="items" role="listbox" aria-orientation="horizontal" aria-labelledby="{uid}-label" tabindex="-1"
		onpointerdown={down} onkeydown={keys} onfocusin={(e) => (active = (e.target as HTMLElement).dataset.item ?? active)}>
		{#each items as it (it.id)}
			{@const on = selected.includes(it.id)}
			<div class="item" class:sc-selected={on} role="option" aria-selected={on} aria-label={name(it)} tabindex={it.id === stop ? 0 : -1}
				title={previewed ? undefined : `${name(it)} · ${formatTime(size(it), 's', 1)}`} data-item={it.id} data-state={it.state}
				data-control-exempt="timeline item: a span of time, as wide as it lasts"
				style:left="min({pct(it.start)}, 100% - 0.25rem)" style:width="{(frac(it.start + size(it), span) - frac(it.start, span)) * 100}%">
				{#if item}{@render item(it, lane)}{:else}<span class="name" class:after-in={!!(it.trim && ontrim && size(it))} style:--in="{((it.trim?.in ?? 0) / (size(it) || 1)) * 100}%">{it.label ?? ''}</span>{/if}
				{#if it.trim && size(it)}
					<span class="cut" style:width="{(it.trim.in / size(it)) * 100}%"></span>
					<span class="cut end" style:width="{(1 - it.trim.out / size(it)) * 100}%"></span>
				{/if}
			</div>
		{/each}
	</div>
	{#if ontrim}<Trim bind:this={trimmer} {lane} items={trimmed} {span} {step} {host} {ontrim} />{/if}
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
	/* with grips: the name starts right of the in line's tab (timeline-trim.svelte: 1rem, a finger's 1.75rem) */
	.name.after-in { padding-left: calc(var(--in) + 1rem + var(--space-025)); }
	@media (pointer: coarse) {
		.name.after-in { padding-left: calc(var(--in) + 1.75rem + var(--space-025)); }
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
</style>
