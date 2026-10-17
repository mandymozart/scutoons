<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { ReachMark } from '$lib/components/chapter/reach-chip.svelte';
	import type { VariantMedia } from '@scutoons/media';
	import { Icon, SortGrip, Tabs, sortable } from '@scutoons/ui';
	import { getAt, previewText, scriptLabels, setSliceLayers, sliceLayers, type Path, type SliceLayerKind, type SliceLayerRef } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { asset, toonUrl } from '$lib/utils/cdn.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { ChapterView, InspectorTab } from '$lib/editor/chapter-state.svelte.ts';
	import { boxChoices } from '$lib/editor/slice-edit.ts';
	import { triggerWord } from '$lib/editor/vn-forms.ts';
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
	import SliceAudio from '$lib/components/slice/slice-audio.svelte';
	import VnEvents from '$lib/components/events/vn-events.svelte';

	/**
	 * The open slice's inspector (COMPONENTS.md step 12): one tab per canvas layer – Layers (the slice's one stack of text
	 * boxes and placed items, BACKLOG 10.43, and the click / hover areas of events and samples, 10.54 – edited in their
	 * own tabs: the list front first, drag to reorder; + text with presets, + item; the text / item inspector), audio,
	 * events – and "Slice" (media + the map places it unlocks). The tab and the selection are the chapter's view state (the canvas and the keyboard write
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

	/** collage pieces are not rows here yet (research/COLLAGE.md §8 step 4; `setSliceLayers` skips them anyway) */
	type RowKind = Exclude<SliceLayerKind, 'piece'>;
	interface LayerRow {
		kind: RowKind;
		index: number;
		/** the item's name / the start of the text (else the box's id) */
		name: string;
		/** tooltip: a text box's id */
		title?: string;
		/** the item's icon */
		icon?: string;
		/** the Feather glyph without an icon: a text box `type`, an event area `zap`, a sample area `music` (as their tabs) */
		glyph: string;
		pickup: boolean;
		/** an event area: all of its events (they move together) */
		events?: number[];
	}
	/** the layer stack front first (the top row is drawn on top) */
	const layers = $derived.by<LayerRow[]>(() =>
		sliceLayers(slice)
			.filter((ref): ref is SliceLayerRef & { kind: RowKind } => ref.kind !== 'piece')
			.reverse()
			.map(({ kind, index, events }): LayerRow => {
				if (kind === 'dialog') {
					const d = dialogs[index];
					const text = previewText(d?.text, [locale, doc.textSource]).trim().slice(0, 40);
					return { kind, index, name: text || d?.id || `#${index + 1}`, title: d?.id, glyph: 'type', pickup: false };
				}
				if (kind === 'event') {
					// an area: its events' types (the first, + how many more) and its trigger, as the Events tab writes them
					const list = (events ?? [index]).map((i) => slice.events?.[i]);
					const more = list.length > 1 ? ` +${list.length - 1}` : '';
					const name = `${list[0]?.type ?? '?'}${more} · ${triggerWord(list[0])}`;
					return { kind, index, name, title: list.map((e) => e?.type).join(', '), glyph: 'zap', pickup: false, events };
				}
				if (kind === 'sample') {
					const sample = slice.audio?.audio?.samples?.[index];
					return { kind, index, name: sample?.id ? `${sample.id} · ${triggerWord(sample)}` : triggerWord(sample), glyph: 'music', pickup: false };
				}
				const it = items[index];
				const def = itemCatalog.catalog?.items.find((x) => x.id === it?.item);
				return { kind, index, name: itemName(def, it?.item ?? '?'), icon: def?.icon ? asset(def.icon) : undefined, glyph: 'box', pickup: it?.action === 'pickup' };
			})
	);
	/** the row of the canvas selection (an event area: any of its events) */
	const isSelected = (row: LayerRow) => selection?.kind === row.kind && (row.events ?? [row.index]).includes(selection.index);
	/** an area picked in the stack is edited in its own tab (Events / Audio) */
	const areaTab = $derived(selection?.kind === 'event' ? 'events' : selection?.kind === 'sample' ? 'audio' : null);
	/** a drag in the list (front first): every entry gets its `layer` (back = 0) – one edit, one undo step */
	function moveLayer(from: number, to: number) {
		const rows = layers.map(({ kind, index, events }) => ({ kind, index, events }));
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
					<!-- the canvas selection: the canvas + this inspector are ONE region ('slice', the chapter editor) -->
					<li data-sort-row class:sc-selected={isSelected(row)} data-testid="layer-{row.kind}-{row.index}">
						<SortGrip label={m['creator.sort_move']({ name: row.name })} title={m['creator.sort_hint']()} />
						<button
							type="button"
							class="ghost row"
							title={row.title}
							onclick={() => view.select({ kind: row.kind, index: row.index })}
							data-testid={row.kind === 'dialog' ? `dialog-${row.index}` : row.kind === 'item' ? `slice-item-${row.index}` : `layer-${row.kind}-row-${row.index}`}
						>
							<span class="thumb">
								{#if row.icon}<img src={row.icon} alt="" draggable="false" />{:else}<Icon name={row.glyph} size="1rem" />{/if}
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
				image={toonUrl(toon, slice.image)}
				ratio={slice.width > 0 && slice.height > 0 ? slice.height / slice.width : SLICE_RATIO}
				lang={locale}
				onpick={onaddtext}
			/>
		{/if}
		<ItemPickerSheet bind:open={view.pickingItem} title={m['creator.add_slice_item']()} onpick={(item) => onadditem(item.id)} />
		{#if areaTab}
			<!-- an area of the stack is edited in its own tab (owner 2026-10-07, BACKLOG 10.54) -->
			<hr />
			<div class="adds">
				<button type="button" class="secondary" onclick={() => areaTab && view.setTab(areaTab)} data-testid="layer-edit-area">
					<Icon name={areaTab === 'events' ? 'zap' : 'music'} />
					{areaTab === 'events' ? m['creator.events']() : m['creator.audio']()}
				</button>
			</div>
		{:else if selection?.kind === 'dialog' && dialogs[selection.index]}
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
			region={null}
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
