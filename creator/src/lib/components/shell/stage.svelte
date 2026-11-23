<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Button, Pill, measureDrop, slotRects, type Floats } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { isCanvas } from '$lib/shell/stage.ts';

	/**
	 * The views on the stage: the whole window, under the bar and the status bar, in the floats' layout (one · a half and
	 * two quarters · two quarters and a half · four quarters) with hairlines between the cells. A canvas fills its cell edge
	 * to edge, a page keeps its content clear of the bar where its cell touches it. A free cell shows the open item's own
	 * view under the stage. Each cell's name is its handle: dragged out, the view becomes a panel of its own (or lands in a
	 * panel, or in another cell). The panels float over all of it. `content(tab)` draws a view; `has(tab)`: it has content.
	 */
	let { floats, content, name, has }: { floats: Floats; content: Snippet<[tab: string]>; name: (tab: string) => string; has: (tab: string) => boolean } = $props();

	let W = $state(1440);
	let H = $state(900);
	const rem = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
	const r = (px: number) => `${px / rem()}rem`;

	const cells = $derived(slotRects(floats.stage, { x: 0, y: 0, w: W, h: H }).filter((c): c is typeof c & { view: string } => !!c.view && has(c.view)));

	const press = (e: PointerEvent, tab: string) => {
		if (e.button !== 0) return;
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
		floats.grabTab(tab, e.clientX, e.clientY);
	};
	const move = (e: PointerEvent) => {
		if (!floats.heldTab) return;
		const { target, marker } = measureDrop(floats, e.clientX, e.clientY);
		floats.dragTab(e.clientX, e.clientY, target, marker);
	};
</script>

<svelte:window bind:innerWidth={W} bind:innerHeight={H} />

{#each cells as c (c.view)}
	{@const top = c.rect.y < 1}
	{@const bottom = c.rect.y + c.rect.h > H - 1}
	<div
		class="cell"
		class:canvas={isCanvas(c.view)}
		class:page={!isCanvas(c.view)}
		class:top
		class:bottom
		class:clear={shell.clear}
		class:edge-l={c.rect.x > 1}
		class:edge-t={!top}
		style:left={r(c.rect.x)}
		style:top={r(c.rect.y)}
		style:width={r(c.rect.w)}
		style:height={r(c.rect.h)}
		data-stage-box={c.view}
		data-cell={c.cell}
		data-testid="on-stage"
		data-tab={c.view}
	>
		{#if isCanvas(c.view)}
			{@render content(c.view)}
		{:else}
			<div class="page-body"><div class="page-col">{@render content(c.view)}</div></div>
		{/if}
		<Pill class="cell-handle" data-control-exempt="a stage cell's handle: the kit's pill">
			<Button
				variant="quiet"
				class="drag"
				title={m['shell.stage_handle']({ tab: name(c.view) })}
				onpointerdown={(e: PointerEvent) => press(e, c.view)}
				onpointermove={move}
				onpointerup={() => floats.dropTab()}
				onpointercancel={() => floats.cancelTab()}
				data-testid="stage-handle">{name(c.view)}</Button
			>
		</Pill>
	</div>
{/each}

<style>
	/* a view on the stage: its cell of the whole window, under the bar too; the page stays mounted under it */
	.cell {
		position: fixed;
		z-index: 30;
		display: flex;
		flex-direction: column;
		box-sizing: border-box;
		overflow: hidden;
		background: var(--tool-ground);
		color: var(--tool-text);
		animation: cell-in var(--motion-base) var(--motion-ease);
	}
	/* the hairlines between cells */
	.cell.edge-l {
		border-left: var(--editor-edge) solid var(--tool-line);
	}
	.cell.edge-t {
		border-top: var(--editor-edge) solid var(--tool-line);
	}
	.cell.canvas > :global(.ws-tab) {
		flex: 1;
	}
	/* a canvas runs under the bar and the panels; its own controls read these to keep clear of them */
	.cell.canvas {
		--stage-left: 1.5rem;
		--stage-right: 1.5rem;
		--stage-top: 0rem;
		--stage-bottom: 0rem;
	}
	.cell.canvas.top.clear {
		--stage-top: var(--shell-top);
	}
	.cell.canvas.bottom {
		--stage-bottom: calc(var(--status-room) + var(--control-height) + 1rem);
	}
	.cell.page {
		overflow: auto;
	}
	.page-body {
		box-sizing: border-box;
		padding: calc(var(--control-height) + 1.5rem) 1.5rem 1.5rem;
	}
	.cell.page.top.clear .page-body {
		padding-top: calc(var(--shell-top) + var(--control-height) + 1.5rem);
	}
	.cell.page.bottom .page-body {
		padding-bottom: calc(var(--status-room) + 1.5rem);
	}
	.page-col {
		display: flex;
		flex-direction: column;
		max-width: 48rem;
		margin: 0 auto;
	}
	/* the cell's name, top middle (under the bar's row in a top cell): its handle out of the stage */
	:global(.kit-pill.cell-handle) {
		position: absolute;
		z-index: 2;
		top: 0.5rem;
		left: 50%;
		transform: translateX(-50%);
		max-width: calc(100% - 2rem);
		white-space: nowrap;
	}
	.cell.top.clear > :global(.kit-pill.cell-handle) {
		top: calc(var(--shell-top) + 0.25rem);
	}
	:global(.kit-pill.cell-handle .kit-button.drag) {
		cursor: grab;
		touch-action: none;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	@keyframes cell-in {
		from {
			opacity: 0;
			transform: scale(0.985);
		}
	}
</style>
