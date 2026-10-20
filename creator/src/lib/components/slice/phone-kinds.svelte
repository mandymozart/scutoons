<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import type { ChapterView, InspectorTab } from '$lib/editor/chapter-state.svelte.ts';
	import { TEXT_PRESETS } from '$lib/editor/text-presets.ts';
	import type { Tool } from '$lib/components/slice/slice-canvas.svelte';

	/**
	 * The phone's ONE bottom row in the strip editor (agents/research/TEXT-ACROSS-SLICES.md §4.1, phone E / E3; owner
	 * 2026-10-20): the kinds – Group · Slice · Text · Item · Ornament · Sound · Event · Layers – or one kind's functions,
	 * ‹ first (back to the kinds). In a kind's row the kind itself is pressed: a tap on free canvas places one there; Text
	 * has its presets second (a row above: pick one, then tap); the last square opens the inspector sheet on the kind's
	 * tab. A selection shows its kind's row. Every button the same square, icon only (the name as tooltip / aria-label);
	 * undo / redo at the right, there only while there is something to undo / redo – their square kept (absent, not
	 * disabled). Group and Slice open their tab at once.
	 */
	let {
		view,
		canUndo,
		canRedo,
		onundo,
		onredo,
		oninspector
	}: {
		view: ChapterView;
		canUndo: boolean;
		canRedo: boolean;
		onundo: () => void;
		onredo: () => void;
		/** open the inspector sheet on a tab (again: the open one closes it) */
		oninspector: (tab: InspectorTab) => void;
	} = $props();

	type Kind = 'text' | 'item' | 'piece' | 'sample' | 'event' | 'layers';
	const KIND_TOOL: Partial<Record<Kind, Tool>> = { text: 'text', item: 'item', piece: 'piece', sample: 'sample', event: 'event' };
	const KIND_TAB: Record<Kind, InspectorTab> = { text: 'layers', item: 'layers', piece: 'layers', sample: 'audio', event: 'events', layers: 'layers' };
	const KIND_ICON: Record<Kind, string> = { text: 'type', item: 'package', piece: 'shapes', sample: 'music', event: 'zap', layers: 'layers' };
	const kindLabel = (k: Kind) =>
		({ text: m['creator.kind_text'](), item: m['creator.kind_item'](), piece: m['creator.kind_ornament'](), sample: m['creator.kind_sound'](), event: m['creator.kind_event'](), layers: m['creator.layers']() })[k];

	/** the kind whose row is open (null: the kinds) */
	let chosen = $state<Kind | null>(null);
	let presets = $state(false);
	/** a selection shows its kind's row */
	const kind = $derived<Kind | null>(
		view.selection ? ({ dialog: 'text', item: 'item', piece: 'piece', sample: 'sample', event: 'event' } as const)[view.selection.kind] : chosen
	);

	function open(k: Kind) {
		chosen = k;
		const tool = KIND_TOOL[k];
		if (tool) view.setTool(tool);
	}
	function back() {
		chosen = null;
		presets = false;
		view.selection = null;
	}
	const presetName: Record<string, () => string> = {
		narration: m['creator.preset_narration'],
		speech: m['creator.preset_speech'],
		thought: m['creator.preset_thought'],
		onomatopoeia: m['creator.preset_onomatopoeia'],
		story: m['creator.preset_story'],
		scene: m['creator.preset_scene'],
		lyrics: m['creator.preset_lyrics']
	};
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role -->
<nav class="phone-kinds" aria-label={m['creator.kinds']()} data-testid="phone-kinds">
	{#if presets && kind === 'text'}
		<!-- the text presets (the creator's real ones): pick one, then a tap on the canvas places it -->
		<div class="presets" role="radiogroup" aria-label={m['creator.text_presets']()} data-testid="phone-presets">
			{#each TEXT_PRESETS as p (p.id)}
				<button
					type="button"
					role="radio"
					aria-checked={view.textPreset === p.id}
					class="preset preset-{p.id}"
					onclick={() => {
						view.textPreset = p.id;
						view.setTool('text');
						presets = false;
					}}
					data-testid="phone-preset-{p.id}">{presetName[p.id]?.() ?? p.id}</button
				>
			{/each}
		</div>
	{/if}
	<div class="row">
		{#if kind === null}
			<button type="button" class="sq" title={m['creator.group_tab']()} aria-label={m['creator.group_tab']()} onclick={() => oninspector('group')} data-testid="kind-group"><Icon name="film" /></button>
			<button type="button" class="sq" title={m['creator.slice_tab']()} aria-label={m['creator.slice_tab']()} onclick={() => oninspector('slice')} data-testid="kind-slice"><Icon name="image" /></button>
			{#each ['text', 'item', 'piece', 'sample', 'event', 'layers'] as const as k (k)}
				{@const tool = KIND_TOOL[k]}
				<button type="button" class="sq" title={kindLabel(k)} aria-label={kindLabel(k)} aria-pressed={tool ? false : undefined} onclick={() => open(k)} data-testid={tool ? `tool-${tool}` : `kind-${k}`}><Icon name={KIND_ICON[k]} /></button>
			{/each}
		{:else}
			{@const tool = KIND_TOOL[kind]}
			<button type="button" class="sq" title={m['creator.bar_back']()} aria-label={m['creator.bar_back']()} onclick={back} data-testid="phone-kind-back"><Icon name="chevron-left" /></button>
			{#if tool}
				<button type="button" class="sq" title={kindLabel(kind)} aria-label={kindLabel(kind)} aria-pressed={view.tool === tool} onclick={() => view.setTool(tool)} data-testid="tool-{tool}"><Icon name={KIND_ICON[kind]} /></button>
			{:else}
				<button type="button" class="sq" title={kindLabel(kind)} aria-label={kindLabel(kind)} onclick={() => oninspector(KIND_TAB[kind])} data-testid="kind-{kind}-open"><Icon name={KIND_ICON[kind]} /></button>
			{/if}
			{#if kind === 'text'}
				<button type="button" class="sq" title={m['creator.text_presets']()} aria-label={m['creator.text_presets']()} aria-pressed={presets} onclick={() => (presets = !presets)} data-testid="phone-presets-toggle"><Icon name="grid" /></button>
			{/if}
			<button type="button" class="sq" title={m['creator.inspector']()} aria-label={m['creator.inspector']()} onclick={() => oninspector(KIND_TAB[kind])} data-testid="phone-inspector"><Icon name="sliders" /></button>
		{/if}
		<span class="grow"></span>
		<!-- absent, not disabled: the square stays, so nothing moves when they come or go -->
		<button type="button" class="sq" class:absent={!canUndo} title={m['creator.undo']()} aria-label={m['creator.undo']()} aria-hidden={!canUndo} tabindex={canUndo ? 0 : -1} onclick={onundo} data-testid="undo"><Icon name="rotate-ccw" /></button>
		<button type="button" class="sq" class:absent={!canRedo} title={m['creator.redo']()} aria-label={m['creator.redo']()} aria-hidden={!canRedo} tabindex={canRedo ? 0 : -1} onclick={onredo} data-testid="redo"><Icon name="rotate-cw" /></button>
	</div>
</nav>

<style>
	.phone-kinds {
		position: relative;
		width: 100%;
	}
	.row {
		display: flex;
		align-items: center;
		overflow-x: auto;
		scrollbar-width: none;
	}
	.grow {
		flex: 1;
	}
	/* every button the same square, borderless; pressed = inverted (DESIGN.md "Selection") */
	.sq {
		flex: none;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: var(--control-height);
		height: var(--control-height);
		min-width: 0;
		padding: 0;
		border: 0;
		background: transparent;
		color: var(--ink);
	}
	.sq[aria-pressed='true'] {
		background: var(--ink);
		color: var(--paper);
	}
	.sq.absent {
		visibility: hidden;
	}
	/* the presets: a row above the bar, scrolling sideways */
	.presets {
		position: absolute;
		left: 0;
		right: 0;
		bottom: 100%;
		display: flex;
		gap: var(--space-05);
		padding: var(--space-05);
		overflow-x: auto;
		background: var(--paper);
		border-top: var(--border-width-regular) solid var(--ink);
	}
	.preset {
		flex: none;
		min-height: var(--control-height);
		padding: 0 var(--space-1);
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
		white-space: nowrap;
	}
	.preset[aria-checked='true'] {
		background: var(--ink);
		color: var(--paper);
	}
</style>
