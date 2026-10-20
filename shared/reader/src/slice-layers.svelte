<script lang="ts">
	import type { ComponentProps, Snippet } from 'svelte';
	import { sliceLayers, type Dialog as DialogModel, type Sample, type SliceItem as SliceItemModel, type SliceLayerRef, type VNSliceEvent } from '@scutoons/content';
	import Dialog from './dialog/dialog.svelte';
	import SliceItem, { type ResolveSliceItem } from './items/slice-item.svelte';
	import type { DialogPlay, TextExpressions } from './types.ts';

	/**
	 * A slice's ONE layer stack (BACKLOG 10.43 + 10.54, @scutoons/content layers.ts): its text boxes, placed items and
	 * click / hover areas (events with a rect, samples with an area) in one paint order – `layer` on each entry, none =
	 * sample areas, items, text boxes, event areas. The reader's `Slice` and the creator's canvas draw it the same way.
	 * The areas are the app's: `area` draws one (with `events` / `samples` given, so they take their place in the
	 * order); without it they are left out (the app draws them itself, the old way).
	 *
	 * Text boxes: `lang` (hyphenation), `hidden` (a showTextBox event reveals it later), `play` (being revealed now),
	 * `expressions` (the reader's answers for `{…}`). Placed items: `resolve` gives the icon (no catalog yet = items not
	 * drawn), `itemHidden` what the reader took, `spent` a pickup they can't take (no hint, no nudge), `ontap` what a tap does.
	 */
	let {
		dialogs,
		items,
		lang,
		hidden,
		play,
		expressions,
		resolve,
		itemHidden,
		spent,
		ontap,
		onidle,
		events,
		samples,
		area
	}: {
		dialogs?: DialogModel[];
		items?: SliceItemModel[];
		lang?: string;
		hidden?: (dialog: DialogModel) => boolean;
		play?: (dialog: DialogModel) => DialogPlay | null | undefined;
		expressions?: TextExpressions;
		resolve?: ResolveSliceItem;
		itemHidden?: (item: SliceItemModel) => boolean;
		spent?: (item: SliceItemModel) => boolean;
		ontap?: ComponentProps<typeof SliceItem>['ontap'];
		/** a tappable item untouched on screen for a while (the app's nudge) */
		onidle?: ComponentProps<typeof SliceItem>['onidle'];
		/** the slice's events (its click / hover areas join the stack) */
		events?: VNSliceEvent[];
		/** the slice's samples (`audio.audio.samples`: those with a click / hover area join the stack) */
		samples?: Sample[];
		/** draws one area of the stack (an event area: `ref.events`; a sample: `samples[ref.index]`) – the app's */
		area?: Snippet<[SliceLayerRef]>;
	} = $props();

	const order = $derived(sliceLayers({ dialog: dialogs, items, events, audio: { audio: { samples } } }));
</script>

{#if order.length}
	<div class="slice-layers" {lang}>
		{#each order as ref (`${ref.kind}:${ref.index}`)}
			{#if ref.kind === 'dialog'}
				{@const dialog = dialogs![ref.index]}
				{#if dialog && !hidden?.(dialog)}
					<Dialog {dialog} index={ref.index} play={play?.(dialog)} {expressions} {lang} />
				{/if}
			{:else if ref.kind === 'item'}
				{@const item = items![ref.index]}
				{@const info = item?.rect && resolve ? resolve(item) : null}
				{#if info && !itemHidden?.(item)}
					<SliceItem {item} index={ref.index} {info} spent={!!spent?.(item)} {ontap} {onidle} />
				{/if}
			{:else if area}
				{@render area(ref)}
			{/if}
		{/each}
	</div>
{/if}

<style>
	/* the overlay plane: above every picture of the group, later slices' planes over earlier ones (DOM order); the
	   container of the boxes' cqw (= % of the slice width – TEXT-ACROSS-SLICES.md §7: on the slice itself it was a
	   stacking context per slice in older engines; here it lands on an element that is one anyway) */
	.slice-layers {
		position: absolute;
		inset: 0;
		z-index: 1;
		pointer-events: none;
		container-type: inline-size;
	}
</style>
