<script lang="ts">
	import { tick } from 'svelte';
	import { Icon, Segmented, clickOutside } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { GRID_SIZES, type ChapterView } from '$lib/editor/chapter-state.svelte.ts';
	import type { Tool } from '$lib/components/slice/slice-canvas.svelte';
	import { exprContext } from '$lib/editor/expr-context.svelte.ts';
	import { SAMPLE_READERS, type SampleReaderId } from '$lib/editor/sample-readers.ts';

	/**
	 * The slice editor's tool strip, left of the canvas (UI-CONCEPTS E.4, owner 2026-10-07): two square vertical groups.
	 * **View** – the grid (pressed = snap on, its size in the corner; hold, right-click, the number or → open the sizes),
	 * the zoom (opens zoom out · % · zoom in · fit · 1:1 – the Flow's square group), and with texts holding `{…}` the
	 * sample-reader preview (agents/EXPRESSIONS.md step 4). **Content** – the canvas tools, icon-only (owner 2026-10-03),
	 * each tied to its inspector tab: text · item · piece (collage – research/COLLAGE.md §3) · sound · event. One flyout at a time, beside its button; Esc / a click outside closes it.
	 */
	let { view, expressions = false }: { view: ChapterView; expressions?: boolean } = $props();

	const SAMPLE_WORDS: Record<SampleReaderId | 'written', () => string> = {
		written: () => m['creator.expr_sample_written'](),
		guest: () => m['creator.expr_sample_guest'](),
		halfway: () => m['creator.expr_sample_halfway'](),
		complete: () => m['creator.expr_sample_complete']()
	};

	type Flyout = 'sizes' | 'zoom' | 'sample';
	let open = $state<Flyout | null>(null);
	let root = $state<HTMLElement | null>(null);
	const percent = $derived(Math.round(view.shownZoom * 100));

	/** opens a flyout and moves the keyboard into it (its chosen entry, else its first control) */
	async function show(which: Flyout) {
		open = which;
		await tick();
		const box = root?.querySelector<HTMLElement>(`[data-flyout="${which}"]`);
		(box?.querySelector<HTMLElement>('[aria-checked="true"], [aria-pressed="true"]') ?? box?.querySelector<HTMLElement>('button, select'))?.focus();
	}
	function close(focusOpener = false) {
		const was = open;
		open = null;
		if (focusOpener && was) root?.querySelector<HTMLElement>(`[data-opens="${was}"]`)?.focus();
	}
	const toggle = (which: Flyout) => (open === which ? close() : void show(which));

	// the grid button: a click toggles snap; holding it (touch or mouse), a right-click, its corner number or → opens the sizes
	let holdTimer: ReturnType<typeof setTimeout> | undefined;
	let held = false;
	function holdStart(event: PointerEvent) {
		if (event.button !== 0) return;
		held = false;
		clearTimeout(holdTimer);
		holdTimer = setTimeout(() => {
			held = true;
			void show('sizes');
		}, 450);
	}
	const holdEnd = () => clearTimeout(holdTimer);
	function gridClick(event: MouseEvent) {
		if (held) {
			held = false;
			return;
		}
		if ((event.target as Element).closest('.corner')) return toggle('sizes');
		view.snap = !view.snap;
	}
	function gridMenu(event: MouseEvent) {
		event.preventDefault();
		holdEnd();
		void show('sizes');
	}
	function gridKey(event: KeyboardEvent) {
		if (event.key === 'ArrowRight') {
			event.preventDefault();
			void show('sizes');
		}
	}
	function pickSize(size: number) {
		view.gridSize = size;
		view.snap = true;
		close(true);
	}
	function onKeydown(event: KeyboardEvent) {
		if (open && (event.key === 'Escape' || (event.key === 'ArrowLeft' && !(event.target as Element).closest('select')))) {
			event.preventDefault();
			event.stopPropagation();
			close(true);
		}
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="slice-tools" bind:this={root} onkeydown={onKeydown} {@attach clickOutside(() => (open = null), { escape: false })} data-testid="slice-tools">
	<div class="sc-seg is-column" role="group" aria-label={m['creator.canvas_view']()} data-testid="tools-view">
		<button
			type="button"
			class="grid-button"
			aria-pressed={view.snap}
			aria-haspopup="true"
			aria-expanded={open === 'sizes'}
			aria-label={m['creator.canvas_snap']()}
			title={m['creator.canvas_snap_size']({ size: view.gridSize })}
			data-opens="sizes"
			onclick={gridClick}
			oncontextmenu={gridMenu}
			onpointerdown={holdStart}
			onpointerup={holdEnd}
			onpointerleave={holdEnd}
			onpointercancel={holdEnd}
			onkeydown={gridKey}
			data-testid="snap-toggle"
		>
			<Icon name="grid" />
			<span class="corner" data-testid="grid-size">{view.gridSize}</span>
		</button>
		<button
			type="button"
			aria-haspopup="true"
			aria-expanded={open === 'zoom'}
			aria-label={m['creator.canvas_zoom']({ percent })}
			title={m['creator.canvas_zoom']({ percent })}
			data-opens="zoom"
			onclick={() => toggle('zoom')}
			data-testid="canvas-zoom"
		>
			<Icon name="zoom-in" />
		</button>
		{#if expressions}
			<button
				type="button"
				aria-haspopup="true"
				aria-expanded={open === 'sample'}
				aria-pressed={exprContext.sample !== 'written'}
				aria-label={m['creator.canvas_preview']()}
				title={m['creator.expr_canvas_preview']()}
				data-opens="sample"
				onclick={() => toggle('sample')}
				data-testid="canvas-sample-open"
			>
				<Icon name="eye" />
			</button>
		{/if}
	</div>

	<Segmented
		vertical
		label={m['creator.canvas_content']()}
		value={view.tool}
		onchange={(t: Tool) => view.setTool(t)}
		testid="tools-content"
		options={[
			{ value: 'text', icon: 'type', title: m['creator.tool_text'](), testid: 'tool-text' },
			{ value: 'item', icon: 'box', title: m['creator.tool_item'](), testid: 'tool-item' },
			{ value: 'piece', icon: 'scissors', title: m['creator.tool_piece'](), testid: 'tool-piece' },
			{ value: 'sample', icon: 'music', title: m['creator.tool_sample'](), testid: 'tool-sample' },
			{ value: 'event', icon: 'zap', title: m['creator.tool_event'](), testid: 'tool-event' }
		]}
	/>

	{#if open === 'sizes'}
		<div class="flyout at-1 sc-seg is-column" role="radiogroup" aria-label={m['creator.canvas_grid_sizes']()} data-flyout="sizes" data-testid="grid-sizes">
			{#each GRID_SIZES as g (g)}
				<button type="button" role="radio" aria-checked={view.snap && view.gridSize === g} title={m['creator.canvas_grid_size']()} onclick={() => pickSize(g)} data-testid="grid-size-{g}">{g}</button>
			{/each}
		</div>
	{:else if open === 'zoom'}
		<!-- the Flow's zoom look: one square group (DESIGN.md "Round vs square") -->
		<div class="flyout at-2 sc-seg" role="group" aria-label={m['creator.canvas_zoom']({ percent })} data-flyout="zoom" data-testid="canvas-zoom-flyout">
			<button type="button" title={m['creator.flow_zoom_out']()} aria-label={m['creator.flow_zoom_out']()} disabled={view.shownZoom <= 0.5} onclick={() => view.zoomOut()} data-testid="canvas-zoom-out"><Icon name="zoom-out" /></button>
			<output class="pct" aria-live="polite" data-testid="canvas-zoom-value">{percent} %</output>
			<button type="button" title={m['creator.flow_zoom_in']()} aria-label={m['creator.flow_zoom_in']()} disabled={view.shownZoom >= 3} onclick={() => view.zoomIn()} data-testid="canvas-zoom-in"><Icon name="zoom-in" /></button>
			<button type="button" title={m['creator.canvas_zoom_fit']()} aria-label={m['creator.canvas_zoom_fit']()} aria-pressed={view.zoom === null} onclick={() => (view.zoom = null)} data-testid="canvas-zoom-fit"><Icon name="maximize" /></button>
			<button type="button" class="word" title={m['creator.canvas_zoom_1x']()} aria-label={m['creator.canvas_zoom_1x']()} aria-pressed={view.zoom === 1} onclick={() => (view.zoom = 1)} data-testid="canvas-zoom-1x">1:1</button>
		</div>
	{:else if open === 'sample'}
		<div class="flyout at-3 sample" data-flyout="sample">
			<select bind:value={exprContext.sample} aria-label={m['creator.expr_preview_as']()} title={m['creator.expr_canvas_preview']()} data-testid="canvas-sample">
				{#each ['written', ...SAMPLE_READERS] as const as id (id)}<option value={id}>{SAMPLE_WORDS[id]()}</option>{/each}
			</select>
		</div>
	{/if}
</div>

<style>
	.slice-tools {
		--strip: var(--control-height);
		position: relative;
		z-index: 6;
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		align-self: flex-start;
	}
	/* phones: touch targets ≥ 2.75rem (owner's phone draft) */
	@media (max-width: 767px) {
		.slice-tools {
			--strip: max(2.75rem, var(--control-height));
		}
	}
	.slice-tools :global(.sc-seg > button) {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		min-width: var(--strip);
		min-height: var(--strip); /* control-ok: --control-height, on a phone at least 2.75rem (touch) */
		padding: 0;
	}
	.slice-tools :global(.sc-seg > button[aria-expanded='true']) {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
	.grid-button {
		position: relative;
	}
	/* the icon steps up-left to make room for the size */
	.grid-button > :global(:first-child) {
		translate: -0.1875rem -0.1875rem;
	}
	/* the grid size in the button's corner – its own target: a click there opens the sizes */
	.corner {
		position: absolute;
		right: 0.0625rem;
		bottom: 0;
		padding: 0 0.125rem;
		font-size: var(--type-caption-size);
		line-height: 1;
		font-variant-numeric: tabular-nums;
	}
	.grid-button[aria-pressed='false'] .corner {
		opacity: 0.6;
	}
	/* beside its button: --strip per row above it, the group's frame included */
	.flyout {
		position: absolute;
		left: calc(100% + var(--space-05));
		top: calc(var(--row) * var(--strip));
		z-index: 20;
	}
	.flyout.at-1 {
		--row: 0;
	}
	.flyout.at-2 {
		--row: 1;
	}
	.flyout.at-3 {
		--row: 2;
	}
	.flyout > button.word {
		padding: 0 var(--space-05);
	}
	.pct {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: 4rem;
		padding: 0 var(--space-05);
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}
	.sample select {
		width: auto;
	}
</style>
