<script lang="ts">
	import { Icon, Segmented } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { GRID_SIZES, type ChapterView } from '$lib/editor/chapter-state.svelte.ts';
	import type { Tool } from '$lib/components/slice/slice-canvas.svelte';

	/**
	 * Above the slice canvas (COMPONENTS.md step 12): snap on / off with its grid size, and the canvas tools – icon-only
	 * (owner 2026-10-03), each tied to its inspector tab.
	 */
	let { view }: { view: ChapterView } = $props();
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
</style>
