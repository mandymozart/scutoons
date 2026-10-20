<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { ReachMark } from '$lib/components/chapter/reach-chip.svelte';
	import type { VariantMedia } from '@scutoons/media';
	import { Icon, Tabs } from '@scutoons/ui';
	import { defaultMoodVocabulary, scriptLabels, type Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { toonUrl } from '$lib/utils/cdn.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { ChapterView, InspectorTab } from '$lib/editor/chapter-state.svelte.ts';
	import { boxChoices } from '$lib/editor/slice-edit.ts';
	import type { TextPreset } from '$lib/editor/text-presets.ts';
	import { SLICE_RATIO } from '$lib/editor/defaults.ts';
	import { layerRows } from '$lib/editor/collage-edit.ts';
	import type { CharacterChoices } from '$lib/components/characters/character-ref-field.svelte';
	import DialogInspector from '$lib/components/text/dialog-inspector.svelte';
	import TextPresetPicker from '$lib/components/text/text-preset-picker.svelte';
	import ItemInspector from '$lib/components/items/item-inspector.svelte';
	import PieceInspector from '$lib/components/collage/piece-inspector.svelte';
	import LayerList from '$lib/components/slice/layer-list.svelte';
	import SliceMedia from '$lib/components/slice/slice-media.svelte';
	import SliceAudio from '$lib/components/slice/slice-audio.svelte';
	import VnEvents from '$lib/components/events/vn-events.svelte';

	/**
	 * The open slice's inspector (COMPONENTS.md step 12): one tab per canvas layer – Layers (the slice's one stack of text
	 * boxes and placed items, BACKLOG 10.43, and the click / hover areas of events and samples, 10.54 – edited in their
	 * own tabs – with the collage pieces under it, research/COLLAGE.md §3: the list front first, drag to reorder; + text
	 * with presets, + item, + piece; the text / item / piece inspector), audio,
	 * events – and "Slice" (media + the map places it unlocks); first the Group tab – the open group's form and script (the
	 * strip editor shows the whole group). The tab and the selection are the chapter's view state (the canvas and the keyboard write
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
		presets,
		groupLabels,
		chapterAudio,
		librarySample,
		heldMusic,
		onaddtext,
		oncreatebox,
		onremove,
		eventMark,
		after,
		groupTab
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
		/** the custom collage presets' document (presets-doc.ts) – for the piece inspector */
		presets?: ContentDoc;
		groupLabels: string[];
		chapterAudio: any;
		/** a picked / uploaded sample file → its id in the chapter's sample library (added when new) */
		librarySample: (media: VariantMedia) => string;
		/** per track: the level it holds from an earlier slice (none = silent) */
		heldMusic: Record<string, { volume: number; label: string }>;
		onaddtext: (preset: TextPreset) => void;
		/** "+ New text box": a showTextBox event's box (`eventsPath` relative to the slice) */
		oncreatebox: (index: number, eventsPath?: Path) => void;
		onremove: () => void;
		/** the reachability mark of a slice event (never true, endless loop – @scutoons/content reach.ts) */
		eventMark?: (index: number) => ReachMark;
		/** right of the tabs: the region's own buttons (the chapter editor's maximize) */
		after?: Snippet;
		/** the Group tab's content: the group's form and script (the strip editor – TEXT-ACROSS-SLICES.md §4.1) */
		groupTab?: Snippet;
	} = $props();

	const dialogs = $derived<any[]>(slice?.dialog ?? []);
	const items = $derived<any[]>(slice?.items ?? []);
	const pieces = $derived<any[]>(slice?.pieces ?? []);
	const selection = $derived(view.selection);
	/** the Layers tab's count: the stack and the pieces */
	const layerCount = $derived(layerRows(slice).length);
	/** an area picked in the stack is edited in its own tab (Events / Audio) */
	const areaTab = $derived(selection?.kind === 'event' ? 'events' : selection?.kind === 'sample' ? 'audio' : null);
	/** the moods of a character picture (the picker, the piece inspector) */
	const moods = $derived(choices?.moods ?? defaultMoodVocabulary());
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
			...(groupTab ? [{ id: 'group', icon: 'film', label: m['creator.group_tab'](), testid: 'tab-group' }] : []),
			{ id: 'layers', icon: 'layers', label: m['creator.layers'](), count: layerCount, testid: 'tab-layers' },
			{ id: 'audio', icon: 'music', label: m['creator.audio'](), testid: 'tab-audio' },
			{ id: 'events', icon: 'zap', label: m['creator.events'](), count: slice?.events?.length ?? 0, testid: 'tab-events' },
			{ id: 'slice', icon: 'image', label: m['creator.slice_tab'](), testid: 'tab-background' }
		]}
	/>

	{#if view.inspectorTab === 'group' && groupTab}
		{@render groupTab()}
	{:else if view.inspectorTab === 'layers'}
		<!-- the slice's ONE layer stack (BACKLOG 10.43) + its collage pieces under it (COLLAGE.md §3), the top row in front -->
		<LayerList {view} {doc} {slice} {slicePath} {toon} {locale} registry={choices?.registry} {moods} />
		<div class="adds">
			<button type="button" class="secondary" class:active={view.pickingPreset} aria-expanded={view.pickingPreset} onclick={() => (view.pickingPreset = !view.pickingPreset)} data-testid="add-text"><Icon name="plus" /> {m['creator.add_text_box']()}</button>
			<button type="button" class="secondary" onclick={() => view.pickItem()} data-testid="add-slice-item"><Icon name="plus" /> {m['creator.add_slice_item']()}</button>
			<button type="button" class="secondary" onclick={() => view.pickPiece()} data-testid="add-slice-piece"><Icon name="plus" /> {m['creator.add_slice_piece']()}</button>
		</div>
		{#if view.pickingPreset}
			<TextPresetPicker
				image={toonUrl(toon, slice.image)}
				ratio={slice.width > 0 && slice.height > 0 ? slice.height / slice.width : SLICE_RATIO}
				lang={locale}
				onpick={onaddtext}
			/>
		{/if}
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
		{:else if selection?.kind === 'piece' && pieces[selection.index]}
			<hr />
			<PieceInspector {doc} path={[...slicePath, 'pieces', selection.index]} host="slice" registry={choices?.registry} {moods} {presets} {onremove} />
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
