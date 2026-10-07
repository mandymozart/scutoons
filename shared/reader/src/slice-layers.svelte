<script lang="ts">
	import { sliceLayers, type Dialog as DialogModel, type SliceItem as SliceItemModel } from '@scutoons/content';
	import Dialog from './dialog/dialog.svelte';
	import SliceItem, { type ResolveSliceItem } from './items/slice-item.svelte';
	import type { DialogPlay, TextExpressions } from './types.ts';

	/**
	 * A slice's ONE layer stack (BACKLOG 10.43, @scutoons/content layers.ts): its text boxes and placed items in one
	 * paint order – `layer` on each entry, none = items under the text boxes. The reader's `Slice` and the creator's
	 * canvas draw it the same way.
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
		ontap
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
	} = $props();

	const order = $derived(sliceLayers({ dialog: dialogs, items }));
</script>

{#if order.length}
	<div class="slice-layers" {lang}>
		{#each order as ref (`${ref.kind}:${ref.index}`)}
			{#if ref.kind === 'dialog'}
				{@const dialog = dialogs![ref.index]}
				{#if dialog && !hidden?.(dialog)}
					<Dialog {dialog} index={ref.index} play={play?.(dialog)} {expressions} {lang} />
				{/if}
			{:else}
				{@const item = items![ref.index]}
				{@const info = item?.rect && resolve ? resolve(item) : null}
				{#if info && !itemHidden?.(item)}
					<SliceItem {item} index={ref.index} {info} {ontap} />
				{/if}
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
