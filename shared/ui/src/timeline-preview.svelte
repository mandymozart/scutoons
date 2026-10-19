<script lang="ts">
	/**
	 * The Timeline's hover preview (timeline.svelte – read its header; owner 2026-10-19: "next to the cursor we need to see
	 * the first frame as a preview"): the app's `preview` snippet for the hovered item, in a box in the top layer (a manual
	 * popover – no region clips it, nothing docked stacks over it; without the popover API a fixed box), placed from the
	 * pointer in % of the window (`previewPlace`, timeline.ts): on the item's top / bottom edge – never over the item –,
	 * sliding along the timeline's visible part, inside the window. Never in the way: no pointer events, hidden from
	 * assistive tech (the item's name is its option's label). Its sizes are the app's content's, in rem.
	 */
	import type { Snippet } from 'svelte';
	import type { Ms, TimelineHover, TimelineItem, TimelineLane } from './timeline.ts';

	let { lanes, hover, preview }: {
		lanes: TimelineLane[];
		hover: TimelineHover;
		preview: Snippet<[TimelineItem, TimelineLane, Ms]>;
	} = $props();

	/** looked up in the lanes as they are now (an item gone under a still pointer takes its preview along) */
	const shown = $derived.by(() => {
		const lane = lanes.find((l) => l.id === hover.lane);
		const it = lane?.items.find((i) => i.id === hover.item);
		return lane && it && hover.place ? { lane, it, place: hover.place } : null;
	});
	/** into the top layer (already shown, or no popover API: it stays a fixed box) */
	const lift = (el: HTMLElement) => {
		try {
			el.showPopover();
		} catch {
			/* see above */
		}
	};
</script>

{#if shown}
	{@const p = shown.place}
	<div class="sc-timeline-preview" popover="manual" aria-hidden="true" {@attach lift}
		style:left="{p.x * 100}%" style:top="{p.y * 100}%" style:translate="{-p.shift * 100}% {p.above ? 'calc(-100% - var(--space-05))' : 'var(--space-05)'}"
		style:max-width="{p.width * 100}%" style:max-height="calc({p.room * 100}% - 2 * var(--space-05))">
		{@render preview(shown.it, shown.lane, hover.t)}
	</div>
{/if}

<style>
	/* left / top / translate inline (from the pointer, % of the window); the app draws the inside */
	.sc-timeline-preview {
		position: fixed;
		inset: auto;
		z-index: 10000;
		box-sizing: border-box;
		margin: 0;
		padding: var(--space-025);
		overflow: hidden;
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--ink);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-variant-numeric: tabular-nums;
		pointer-events: none;
	}
</style>
