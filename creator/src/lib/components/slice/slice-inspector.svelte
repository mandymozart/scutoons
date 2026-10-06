<script lang="ts">
	import type { ReachMark } from '$lib/components/chapter/reach-chip.svelte';
	import type { VariantMedia } from '@scutoons/media';
	import { Icon, Tabs } from '@scutoons/ui';
	import { previewText, scriptLabels, type ChapterSliceRef, type Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { ChapterView, InspectorTab } from '$lib/editor/chapter-state.svelte.ts';
	import { boxChoices } from '$lib/editor/slice-edit.ts';
	import { matchTextPreset, type TextPreset } from '$lib/editor/text-presets.ts';
	import { SLICE_RATIO } from '$lib/editor/defaults.ts';
	import { itemCatalog, itemName } from '$lib/editor/catalog.svelte.ts';
	import type { CharacterChoices } from '$lib/components/characters/character-ref-field.svelte';
	import DialogInspector from '$lib/components/text/dialog-inspector.svelte';
	import TextPresetPicker from '$lib/components/text/text-preset-picker.svelte';
	import ItemInspector from '$lib/components/items/item-inspector.svelte';
	import ItemPickerSheet from '$lib/components/items/item-picker-sheet.svelte';
	import SliceMedia from '$lib/components/slice/slice-media.svelte';
	import SliceUnlocks from '$lib/components/slice/slice-unlocks.svelte';
	import SliceAudio from '$lib/components/slice/slice-audio.svelte';
	import VnEvents from '$lib/components/events/vn-events.svelte';

	/**
	 * The open slice's inspector (COMPONENTS.md step 12): one tab per canvas layer – texts (list, + text with presets, the
	 * text inspector), placed items (list, + item, the item inspector), audio, events – and "Slice" (media + the map
	 * places it unlocks). The tab and the selection are the chapter's view state (the canvas and the keyboard write
	 * them too).
	 */
	let {
		view,
		doc,
		slice,
		slicePath,
		group,
		toon,
		chapterNumber,
		seasonNumber,
		locale,
		characters,
		choices,
		groupLabels,
		chapterAudio,
		librarySample,
		heldMusic,
		worldMap,
		chapterSlices,
		onmaptouch,
		onaddtext,
		onadditem,
		oncreatebox,
		onremove,
		eventMark
	}: {
		view: ChapterView;
		doc: ContentDoc;
		slice: any;
		slicePath: Path;
		/** the slice's group (its script's named points) */
		group: any;
		toon: string;
		chapterNumber: number;
		seasonNumber: number;
		/** the language the canvas shows */
		locale: string;
		characters: { name: string; label?: string }[];
		choices: CharacterChoices | undefined;
		groupLabels: string[];
		chapterAudio: any;
		/** a picked / uploaded sample file → its id in the chapter's sample library (added when new) */
		librarySample: (media: VariantMedia) => string;
		/** per track: the level it holds from an earlier slice (none = silent) */
		heldMusic: Record<string, { volume: number; label: string }>;
		worldMap: ContentDoc;
		chapterSlices: ChapterSliceRef[];
		onmaptouch: () => void;
		onaddtext: (preset: TextPreset) => void;
		onadditem: (itemId: string) => void;
		/** "+ New text box": a showTextBox event's box (`eventsPath` relative to the slice) */
		oncreatebox: (index: number, eventsPath?: Path) => void;
		onremove: () => void;
		/** the reachability mark of a slice event (never true, endless loop – @scutoons/content reach.ts) */
		eventMark?: (index: number) => ReachMark;
	} = $props();

	const dialogs = $derived<any[]>(slice?.dialog ?? []);
	const items = $derived<any[]>(slice?.items ?? []);
	const selection = $derived(view.selection);
	/** a text's preset in the list ("custom" when its look was changed) */
	function presetLabel(d: Record<string, unknown>) {
		const id = matchTextPreset(d);
		return id ? m[`creator.preset_${id}`]() : m['creator.preset_custom']();
	}
</script>

<aside class="inspector">
	<!-- one icon each (owner 2026-10-03), the same as the canvas tools; background + map are one "Slice" tab -->
	<Tabs
		variant="grid"
		label={m['creator.slice_tab']()}
		value={view.inspectorTab}
		onchange={(id) => view.setTab(id as InspectorTab)}
		tabs={[
			{ id: 'text', icon: 'type', label: m['creator.text_boxes'](), count: dialogs.length, testid: 'tab-text' },
			{ id: 'items', icon: 'box', label: m['creator.slice_items'](), count: items.length, testid: 'tab-items' },
			{ id: 'audio', icon: 'music', label: m['creator.audio'](), testid: 'tab-audio' },
			{ id: 'events', icon: 'zap', label: m['creator.events'](), count: slice?.events?.length ?? 0, testid: 'tab-events' },
			{ id: 'slice', icon: 'image', label: m['creator.slice_tab'](), testid: 'tab-background' }
		]}
	/>

	{#if view.inspectorTab === 'text'}
		{#if dialogs.length}
		<ul class="boxes">
			{#each dialogs as d, di (di)}
				<li>
					<button class="ghost" class:active={selection?.kind === 'dialog' && selection.index === di} onclick={() => view.select({ kind: 'dialog', index: di })} data-testid="dialog-{di}">
						<span>{d.id ?? `#${di + 1}`}</span> <span class="hint">{presetLabel(d)}</span> {previewText(d.text, [locale, doc.textSource]).slice(0, 40)}
					</button>
				</li>
			{/each}
		</ul>
		{/if}
		<button class="secondary" class:active={view.pickingPreset} aria-expanded={view.pickingPreset} onclick={() => (view.pickingPreset = !view.pickingPreset)} data-testid="add-text"><Icon name="plus" /> {m['creator.add_text_box']()}</button>
		{#if view.pickingPreset}
			<TextPresetPicker
				image={slice.image ? asset(`${toon}/${slice.image}`) : ''}
				ratio={slice.width > 0 && slice.height > 0 ? slice.height / slice.width : SLICE_RATIO}
				lang={locale}
				onpick={onaddtext}
			/>
		{/if}
		{#if selection?.kind === 'dialog' && dialogs[selection.index]}
			<hr />
			<DialogInspector {doc} path={[...slicePath, 'dialog', selection.index]} {onremove} />
		{/if}
	{:else if view.inspectorTab === 'items'}
		{#if items.length}
		<ul class="boxes">
			{#each items as it, ii (ii)}
				<li>
					<button class="ghost" class:active={selection?.kind === 'item' && selection.index === ii} onclick={() => view.select({ kind: 'item', index: ii })} data-testid="slice-item-{ii}">
						<span>{itemName(itemCatalog.catalog?.items.find((x) => x.id === it?.item), it?.item ?? '?')}</span> <span class="hint">{it?.action === 'pickup' ? m['creator.slice_item_action_pickup']() : it?.action === 'events' ? m['creator.slice_item_action_events']() : ''}</span>
					</button>
				</li>
			{/each}
		</ul>
		{/if}
		<button type="button" class="secondary" onclick={() => view.pickItem()} data-testid="add-slice-item"><Icon name="plus" /> {m['creator.add_slice_item']()}</button>
		<ItemPickerSheet bind:open={view.pickingItem} title={m['creator.add_slice_item']()} onpick={(item) => onadditem(item.id)} />
		{#if selection?.kind === 'item' && items[selection.index]}
			<hr />
			<ItemInspector
				{doc}
				path={[...slicePath, 'items', selection.index]}
				{characters}
				{choices}
				dialogs={boxChoices([slice], [locale, doc.textSource])}
				oncreatebox={(i) => selection && oncreatebox(i, ['items', selection.index, 'events'])}
				{onremove}
			/>
		{/if}
	{:else if view.inspectorTab === 'slice'}
		<SliceMedia {doc} {slicePath} {toon} place="{toon}/s{seasonNumber}/c{chapterNumber}/" />
		<!-- the slice's map unlocks, in the same view (owner 2026-10-03) -->
		<section class="slice-map" data-testid="slice-map">
			<h3><Icon name="map-pin" /> {m['creator.map']()}</h3>
			<SliceUnlocks mapDoc={worldMap} {toon} chapter={chapterNumber} image={slice.image ?? ''} slices={chapterSlices} ontouch={onmaptouch} />
		</section>
	{:else if view.inspectorTab === 'audio'}
		<SliceAudio
			{doc}
			{slicePath}
			{chapterAudio}
			{librarySample}
			{heldMusic}
			{toon}
			place="{toon}/s{seasonNumber}/c{chapterNumber}/"
			selectedSample={selection?.kind === 'sample' ? selection.index : -1}
			onselectsample={(i) => (view.selection = i >= 0 ? { kind: 'sample', index: i } : null)}
		/>
	{:else if view.inspectorTab === 'events'}
		<VnEvents
			{doc}
			eventsPath={[...slicePath, 'events']}
			sliceEvents
			points={scriptLabels(group?.events)}
			{groupLabels}
			{characters}
			{choices}
			dialogs={boxChoices([slice], [locale, doc.textSource])}
			oncreatebox={(i) => oncreatebox(i)}
			markOf={eventMark}
			selected={selection?.kind === 'event' ? selection.index : -1}
			onselect={(i) => (view.selection = { kind: 'event', index: i })}
		/>
	{/if}
</aside>

<style>
	.inspector {
		overflow: auto;
		border-left: var(--border-width-regular) solid var(--ink);
		padding: var(--space-1) var(--space-1) var(--space-3);
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	/* narrower: the inspector goes below the canvas (the editor's columns) */
	@media (max-width: 1279px) {
		.inspector {
			border-left: 0;
			border-top: var(--border-width-regular) solid var(--ink);
		}
	}
	.slice-map {
		border-top: var(--border-width-regular) solid var(--ink);
		padding-top: var(--space-1);
	}
	.slice-map h3 {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		margin: 0 0 var(--space-05);
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.boxes {
		list-style: none;
		margin: 0;
		padding: 0;
		border: var(--border-width-regular) solid var(--ink);
	}
	.boxes li + li {
		border-top: var(--border-width-hairline) solid var(--ink);
	}
	/* one line per box, ellipsis (block, not flex: a text node can't ellipsize in flex) */
	.boxes button {
		display: block;
		width: 100%;
		padding-block: 0;
		line-height: calc(var(--control-height) - 2 * var(--border-width-regular));
		text-align: left;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.boxes button.active .hint {
		color: inherit;
	}
	hr {
		border: 0;
		border-top: var(--border-width-regular) solid var(--ink);
		width: 100%;
		margin: var(--space-05) 0;
	}
</style>
