<script lang="ts">
	import { Button, IconButton, Pill, Select } from '@scutoons/ui/editor';
	import { tick } from 'svelte';
	import { Icon, clickOutside } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { GRID_SIZES, type ChapterView } from '$lib/editor/chapter-state.svelte.ts';
	import type { Tool } from '$lib/components/slice/slice-canvas.svelte';
	import { exprContext } from '$lib/editor/expr-context.svelte.ts';
	import { SAMPLE_READERS, type SampleReaderId } from '$lib/editor/sample-readers.ts';

	/**
	 * The slice editor's tool strip, left of the canvas (UI-CONCEPTS E.4, owner 2026-10-07): two upright pills of icons (RULES H4).
	 * **View** – the grid (pressed = snap on, its size in the corner; hold, right-click, the number or → open the sizes),
	 * the zoom (opens zoom out · % · zoom in · fit · 1:1 – the Flow's square group), the strip's seams (a hairline across
	 * the art – TEXT-ACROSS-SLICES.md §4.1), and with texts holding `{…}` the
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

	const TOOLS: { value: Tool; icon: string; title: () => string }[] = [
		{ value: 'text', icon: 'type', title: () => m['creator.tool_text']() },
		{ value: 'item', icon: 'box', title: () => m['creator.tool_item']() },
		{ value: 'piece', icon: 'shapes', title: () => m['creator.tool_piece']() },
		{ value: 'sample', icon: 'music', title: () => m['creator.tool_sample']() },
		{ value: 'event', icon: 'zap', title: () => m['creator.tool_event']() }
	];

	type Flyout = 'sizes' | 'zoom' | 'sample';
	let open = $state<Flyout | null>(null);
	let root = $state<HTMLElement | null>(null);
	const percent = $derived(Math.round(view.shownZoom * 100));

	/** opens a flyout beside its button, its first control level with it, and moves the keyboard into it (its chosen entry, else its first control) */
	async function show(which: Flyout) {
		open = which;
		await tick();
		const box = root?.querySelector<HTMLElement>(`[data-flyout="${which}"]`);
		const opener = root?.querySelector<HTMLElement>(`[data-opens="${which}"]`);
		const first = box?.querySelector<HTMLElement>('button, select');
		if (root && box && opener && first) {
			const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
			const top = opener.getBoundingClientRect().top - root.getBoundingClientRect().top - (first.getBoundingClientRect().top - box.getBoundingClientRect().top);
			box.style.top = `${top / rem}rem`;
		}
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
	<Pill vertical label={m['creator.canvas_view']()} data-testid="tools-view">
		<IconButton
			label={m['creator.canvas_snap']()}
			class="grid-button"
			pressed={view.snap}
			aria-haspopup="true"
			aria-expanded={open === 'sizes'}
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
		</IconButton>
		<IconButton label={m['creator.canvas_zoom']({ percent })} icon="zoom-in" aria-haspopup="true" aria-expanded={open === 'zoom'} data-opens="zoom" onclick={() => toggle('zoom')} data-testid="canvas-zoom" />
		<!-- the strip's seams as a hairline across the art (TEXT-ACROSS-SLICES.md §4.1; the numbers always show) -->
		<IconButton label={m['creator.canvas_seams']()} icon="scissors" pressed={view.seams} onclick={() => (view.seams = !view.seams)} data-testid="seams-toggle" />
		{#if expressions}
			<IconButton
				label={m['creator.canvas_preview']()}
				icon="eye"
				title={m['creator.expr_canvas_preview']()}
				pressed={exprContext.sample !== 'written'}
				aria-haspopup="true"
				aria-expanded={open === 'sample'}
				data-opens="sample"
				onclick={() => toggle('sample')}
				data-testid="canvas-sample-open"
			/>
		{/if}
	</Pill>

	<Pill vertical label={m['creator.canvas_content']()} data-testid="tools-content">
		{#each TOOLS as t (t.value)}
			<IconButton label={t.title()} icon={t.icon} pressed={view.tool === t.value} onclick={() => view.setTool(t.value)} data-testid="tool-{t.value}" />
		{/each}
	</Pill>

	{#if open === 'sizes'}
		<Pill vertical label={m['creator.canvas_grid_sizes']()} class="flyout" data-flyout="sizes" data-testid="grid-sizes">
			{#each GRID_SIZES as g (g)}
				<IconButton label={String(g)} title={m['creator.canvas_grid_size']()} pressed={view.snap && view.gridSize === g} onclick={() => pickSize(g)} data-testid="grid-size-{g}">{g}</IconButton>
			{/each}
		</Pill>
	{:else if open === 'zoom'}
		<Pill label={m['creator.canvas_zoom']({ percent })} class="flyout" data-flyout="zoom" data-testid="canvas-zoom-flyout">
			<IconButton label={m['creator.flow_zoom_out']()} icon="zoom-out" disabled={view.shownZoom <= 0.5} onclick={() => view.zoomOut()} data-testid="canvas-zoom-out" />
			<output class="pct" aria-live="polite" data-testid="canvas-zoom-value">{percent} %</output>
			<IconButton label={m['creator.flow_zoom_in']()} icon="zoom-in" disabled={view.shownZoom >= 3} onclick={() => view.zoomIn()} data-testid="canvas-zoom-in" />
			<IconButton label={m['creator.canvas_zoom_fit']()} icon="maximize" pressed={view.zoom === null} onclick={() => (view.zoom = null)} data-testid="canvas-zoom-fit" />
			<Button variant="quiet" title={m['creator.canvas_zoom_1x']()} aria-label={m['creator.canvas_zoom_1x']()} pressed={view.zoom === 1} onclick={() => (view.zoom = 1)} data-testid="canvas-zoom-1x">1:1</Button>
		</Pill>
	{:else if open === 'sample'}
		<div class="flyout sample" data-flyout="sample">
			<Select quiet label={m['creator.expr_preview_as']()} title={m['creator.expr_canvas_preview']()} value={exprContext.sample} options={(['written', ...SAMPLE_READERS] as const).map((id) => ({ value: id, label: SAMPLE_WORDS[id]() }))} onchange={(v) => (exprContext.sample = v as typeof exprContext.sample)} data-testid="canvas-sample" />
		</div>
	{/if}
</div>

<style>
	.slice-tools {
		position: relative;
		z-index: 6;
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		align-self: flex-start;
	}
	/* the icon steps up-left to make room for the size */
	.slice-tools :global(.grid-button > svg) {
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
	.slice-tools :global(.grid-button:not(.pressed) > .corner) {
		opacity: 0.6;
	}
	/* beside the strip; show() sets its top level with its button */
	.slice-tools :global(.flyout) {
		position: absolute;
		left: calc(100% + var(--space-05));
		top: 0;
		z-index: 20;
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
	.sample :global(select) {
		width: auto;
	}
</style>
