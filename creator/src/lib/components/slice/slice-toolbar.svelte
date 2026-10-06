<script lang="ts">
	import { Icon, Segmented } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { GRID_SIZES, type ChapterView } from '$lib/editor/chapter-state.svelte.ts';
	import type { Tool } from '$lib/components/slice/slice-canvas.svelte';
	import { exprContext } from '$lib/editor/expr-context.svelte.ts';
	import { SAMPLE_READERS, type SampleReaderId } from '$lib/editor/sample-readers.ts';

	/**
	 * Above the slice canvas (COMPONENTS.md step 12): snap on / off with its grid size, and the canvas tools – icon-only
	 * (owner 2026-10-03), each tied to its inspector tab. `expressions`: the slice's text boxes hold `{…}` – a small select
	 * shows them as a sample reader sees them (agents/EXPRESSIONS.md step 4; default "new guest") or as written.
	 */
	let { view, expressions = false }: { view: ChapterView; expressions?: boolean } = $props();

	const SAMPLE_WORDS: Record<SampleReaderId | 'written', () => string> = {
		written: () => m['creator.expr_sample_written'](),
		guest: () => m['creator.expr_sample_guest'](),
		halfway: () => m['creator.expr_sample_halfway'](),
		complete: () => m['creator.expr_sample_complete']()
	};
</script>

<div class="canvas-tools">
	<span class="sc-seg" role="group" aria-label={m['creator.canvas_snap']()}>
		<button class="ghost" class:active={view.snap} aria-pressed={view.snap} onclick={() => (view.snap = !view.snap)} title={m['creator.canvas_snap_hint']()} data-testid="snap-toggle"><Icon name="grid" /> {m['creator.canvas_snap']()}</button>
		<select bind:value={view.gridSize} disabled={!view.snap} aria-label={m['creator.canvas_grid_size']()} title={m['creator.canvas_grid_size']()} data-testid="grid-size">
			{#each GRID_SIZES as g (g)}<option value={g}>{g} px</option>{/each}
		</select>
	</span>
	<Segmented
		label={m['creator.tool']()}
		value={view.tool}
		onchange={(t: Tool) => view.setTool(t)}
		options={[
			{ value: 'text', icon: 'type', title: m['creator.tool_text'](), testid: 'tool-text' },
			{ value: 'item', icon: 'box', title: m['creator.tool_item'](), testid: 'tool-item' },
			{ value: 'sample', icon: 'music', title: m['creator.tool_sample'](), testid: 'tool-sample' },
			{ value: 'event', icon: 'zap', title: m['creator.tool_event'](), testid: 'tool-event' }
		]}
	/>
	{#if expressions}
		<select class="sample" bind:value={exprContext.sample} aria-label={m['creator.expr_preview_as']()} title={m['creator.expr_canvas_preview']()} data-testid="canvas-sample">
			{#each ['written', ...SAMPLE_READERS] as const as id (id)}<option value={id}>{SAMPLE_WORDS[id]()}</option>{/each}
		</select>
	{/if}
</div>

<style>
	.canvas-tools {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05) var(--space-1);
		align-items: center;
		position: sticky;
		top: calc(-1 * var(--space-1));
		background: var(--paper);
		z-index: 5;
		padding: var(--space-05) 0;
	}
	.sample {
		width: auto;
	}
</style>
