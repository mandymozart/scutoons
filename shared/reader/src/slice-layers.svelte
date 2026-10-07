<script lang="ts">
	import type { Snippet } from 'svelte';
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
	 * drawn), `itemHidden` what the reader took, `ontap` what a tap does.
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
		ontap,
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
		ontap?: (item: SliceItemModel, index: number) => void;
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
					<SliceItem {item} index={ref.index} {info} {ontap} />
				{/if}
			{:else if area}
				{@render area(ref)}
			{/if}
		{/each}
	</div>
{/if}

<style>
	.slice-layers {
		position: absolute;
		inset: 0;
		z-index: 1;
		pointer-events: none;
	}
</style>
