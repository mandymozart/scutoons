<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { ReachMark } from '$lib/components/chapter/reach-chip.svelte';
	import type { VariantMedia } from '@scutoons/media';
	import { Icon, SortGrip, Tabs, sortable } from '@scutoons/ui';
	import { getAt, previewText, scriptLabels, setSliceLayers, sliceLayers, type ChapterSliceRef, type Path, type SliceLayerKind } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { ChapterView, InspectorTab } from '$lib/editor/chapter-state.svelte.ts';
	import { boxChoices } from '$lib/editor/slice-edit.ts';
	import type { TextPreset } from '$lib/editor/text-presets.ts';
	import { moveAt } from '$lib/editor/move.ts';
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
	 * The open slice's inspector (COMPONENTS.md step 12): one tab per canvas layer – Layers (the slice's one stack of text
	 * boxes and placed items, BACKLOG 10.43: the list front first, drag to reorder; + text with presets, + item; the
	 * text / item inspector), audio, events – and "Slice" (media + the map places it unlocks). The tab and the selection are the chapter's view state (the canvas and the keyboard write
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
		eventMark,
		after
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
		/** right of the tabs: the region's own buttons (the chapter editor's maximize) */
		after?: Snippet;
	} = $props();

	const dialogs = $derived<any[]>(slice?.dialog ?? []);
	const items = $derived<any[]>(slice?.items ?? []);
	const selection = $derived(view.selection);

	interface LayerRow {
		kind: SliceLayerKind;
		index: number;
		/** the item's name / the start of the text (else the box's id) */
		name: string;
		/** tooltip: a text box's id */
		title?: string;
		/** the item's icon (a text box: the T glyph) */
		icon?: string;
		pickup: boolean;
	}
	/** the layer stack front first (the top row is drawn on top) */
	const layers = $derived.by<LayerRow[]>(() =>
		sliceLayers(slice)
			.reverse()
			.map(({ kind, index }): LayerRow => {
				if (kind === 'dialog') {
					const d = dialogs[index];
					const text = previewText(d?.text, [locale, doc.textSource]).trim().slice(0, 40);
					return { kind, index, name: text || d?.id || `#${index + 1}`, title: d?.id, pickup: false };
				}
				const it = items[index];
				const def = itemCatalog.catalog?.items.find((x) => x.id === it?.item);
				return { kind, index, name: itemName(def, it?.item ?? '?'), icon: def?.icon ? asset(def.icon) : undefined, pickup: it?.action === 'pickup' };
			})
	);
	/** a drag in the list (front first): every entry gets its `layer` (back = 0) – one edit, one undo step */
	function moveLayer(from: number, to: number) {
		const rows = layers.map(({ kind, index }) => ({ kind, index }));
		if (!moveAt(rows, from, to)) return;
		doc.edit((file) => setSliceLayers(getAt(file, slicePath), rows.reverse()));
	}
</script>

<div class="inspector">
	<!-- one icon each (owner 2026-10-03), the same as the canvas tools; background + map are one "Slice" tab -->
	<Tabs
		variant="grid"
		{after}
		label={m['creator.slice_tab']()}
		value={view.inspectorTab}
		onchange={(id) => view.setTab(id as InspectorTab)}
		tabs={[
			{ id: 'layers', icon: 'layers', label: m['creator.layers'](), count: layers.length, testid: 'tab-layers' },
			{ id: 'audio', icon: 'music', label: m['creator.audio'](), testid: 'tab-audio' },
			{ id: 'events', icon: 'zap', label: m['creator.events'](), count: slice?.events?.length ?? 0, testid: 'tab-events' },
			{ id: 'slice', icon: 'image', label: m['creator.slice_tab'](), testid: 'tab-background' }
		]}
	/>

	{#if view.inspectorTab === 'layers'}
		<!-- the slice's ONE layer stack (BACKLOG 10.43): text boxes and placed items, the top row in front; a drag changes what is drawn on top -->
		{#if layers.length}
			<ul class="layers" use:sortable={{ onmove: moveLayer }} data-testid="layers">
				{#each layers as row (`${row.kind}:${row.index}`)}
					{@const active = selection?.kind === row.kind && selection.index === row.index}
					<li data-sort-row class:active data-testid="layer-{row.kind}-{row.index}">
						<SortGrip label={m['creator.sort_move']({ name: row.name })} title={m['creator.sort_hint']()} />
						<button
							type="button"
							class="ghost row"
							class:active
							title={row.title}
							onclick={() => view.select({ kind: row.kind, index: row.index })}
							data-testid={row.kind === 'dialog' ? `dialog-${row.index}` : `slice-item-${row.index}`}
						>
							<span class="thumb">
								{#if row.icon}<img src={row.icon} alt="" draggable="false" />{:else}<Icon name="type" size="1rem" />{/if}
							</span>
							<span class="name">{row.name}</span>
						</button>
						{#if row.pickup}
							<span class="pickup" title={m['creator.slice_item_action_pickup']()}><Icon name="download" size="1rem" label={m['creator.slice_item_action_pickup']()} /></span>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
		<div class="adds">
			<button type="button" class="secondary" class:active={view.pickingPreset} aria-expanded={view.pickingPreset} onclick={() => (view.pickingPreset = !view.pickingPreset)} data-testid="add-text"><Icon name="plus" /> {m['creator.add_text_box']()}</button>
			<button type="button" class="secondary" onclick={() => view.pickItem()} data-testid="add-slice-item"><Icon name="plus" /> {m['creator.add_slice_item']()}</button>
		</div>
		{#if view.pickingPreset}
			<TextPresetPicker
				image={slice.image ? asset(`${toon}/${slice.image}`) : ''}
				ratio={slice.width > 0 && slice.height > 0 ? slice.height / slice.width : SLICE_RATIO}
				lang={locale}
				onpick={onaddtext}
			/>
		{/if}
		<ItemPickerSheet bind:open={view.pickingItem} title={m['creator.add_slice_item']()} onpick={(item) => onadditem(item.id)} />
		{#if selection?.kind === 'dialog' && dialogs[selection.index]}
			<hr />
			<DialogInspector {doc} path={[...slicePath, 'dialog', selection.index]} {onremove} />
		{:else if selection?.kind === 'item' && items[selection.index]}
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
</div>

<style>
	/* the region around it (border, width, scrolling, below the canvas under 1280px): the chapter editor */
	.inspector {
		padding: var(--space-1) var(--space-1) var(--space-3);
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
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
	/* the layer stack: one line per entry – grip, icon, name, the pickup mark */
	.layers {
		list-style: none;
		margin: 0;
		padding: 0;
		border: var(--border-width-regular) solid var(--ink);
	}
	.layers li {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		padding-inline: var(--space-05);
	}
	.layers li + li {
		border-top: var(--border-width-hairline) solid var(--ink);
	}
	.layers li.active {
		background: var(--ink);
		color: var(--paper);
	}
	.layers .row {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		justify-content: flex-start;
		gap: var(--space-05);
		height: calc(var(--control-height) - var(--border-width-hairline));
		padding: 0;
		border: 0;
		background: none;
		color: inherit;
		text-align: left;
	}
	.thumb {
		flex: none;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 1.75rem;
		height: 1.75rem;
		box-sizing: border-box;
		border: var(--border-width-hairline) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
	}
	.thumb img {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}
	.name {
		min-width: 0;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.pickup {
		flex: none;
		display: inline-flex;
	}
	.adds {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
	}
	hr {
		border: 0;
		border-top: var(--border-width-regular) solid var(--ink);
		width: 100%;
		margin: var(--space-05) 0;
	}
</style>
