<script lang="ts">
	import type { Floats, StageZone } from './floats.svelte.ts';
	import { ZONE_CELLS, cellOf, cellRect, placeOn, zoneRects } from './stage.ts';

	/**
	 * The stage's drop zones while a tab or a panel is in the hand: they reach the screen's very edges, over the bar and the
	 * status bar (the page draws this over both). Each zone shows faintly, the one under the pointer lights up and the
	 * view's landing place is previewed with `label(tab, zone)`; past the top edge a line fills during the hold toward full
	 * screen. Short animations on the motion tokens (none with reduced motion).
	 */
	let { floats, label }: { floats: Floats; label: (tab: string, zone: StageZone) => string } = $props();

	const tab = $derived(floats.dragged);
	const zone = $derived(floats.zone);
	const rem = $derived(floats.metrics.rem || 16);
	const r = (px: number) => `${px / rem}rem`;
	const win = $derived({ x: 0, y: 0, w: floats.win.w, h: floats.win.h });
	/** where the view lands: its cell in the stage the drop would make, or the whole window for full screen */
	const lands = $derived.by(() => {
		if (!tab || !zone) return null;
		if (zone.full) return win;
		const cell = cellOf(placeOn(floats.stage, tab, zone.cell), tab);
		return cell ? cellRect(cell, win) : null;
	});
</script>

{#if tab && floats.win.w > 0}
	<div class="zones" aria-hidden="true" data-testid="stage-zones">
		{#each ZONE_CELLS as cell (cell)}
			{#each zoneRects(cell, floats.win, floats.zoneSize) as z, i (i)}
				<div class="arm" class:on={zone?.cell === cell} style:left={r(z.x)} style:top={r(z.y)} style:width={r(z.w)} style:height={r(z.h)} data-zone={cell}></div>
			{/each}
		{/each}
		{#if lands && zone}
			<div class="lands" class:full={zone.full} style:left={r(lands.x)} style:top={r(lands.y)} style:width={r(lands.w)} style:height={r(lands.h)} data-testid="stage-lands">
				<span class="label">{label(tab, zone)}</span>
			</div>
		{/if}
		{#if floats.hover?.past}
			<div class="hold" class:armed={floats.armed}><span></span></div>
		{/if}
	</div>
{/if}

<style>
	.zones {
		position: fixed;
		inset: 0;
		pointer-events: none;
		animation: zones-in var(--motion-fast) var(--motion-ease);
	}
	.arm {
		position: absolute;
		box-sizing: border-box;
		border: var(--editor-edge) dashed var(--kit-drop-rest-border);
		background: transparent;
		opacity: 0.7;
		transition:
			opacity var(--motion-fast) var(--motion-ease),
			background-color var(--motion-fast) var(--motion-ease),
			transform var(--motion-fast) var(--motion-ease);
	}
	/* the zone under the pointer lights up and swells a little */
	.arm.on {
		opacity: 1;
		border-style: solid;
		transform: scale(1.04);
		border-color: var(--kit-drop-on-border);
		background: var(--kit-drop-on-ground);
	}
	/* where the view lands: the accent's outline around its cell, the name and the place in a pill */
	.lands {
		position: absolute;
		box-sizing: border-box;
		display: grid;
		place-items: center;
		border: 0.125rem solid var(--kit-drop-on-border);
		border-radius: var(--kit-radius-box);
		background: var(--kit-drop-on-ground);
		animation: lands-in var(--motion-fast) var(--motion-ease);
		transition:
			left var(--motion-base) var(--motion-ease),
			top var(--motion-base) var(--motion-ease),
			width var(--motion-base) var(--motion-ease),
			height var(--motion-base) var(--motion-ease);
	}
	.lands.full {
		border-radius: 0;
	}
	.label {
		padding: 0.3rem 0.8rem;
		border: var(--editor-edge) solid var(--kit-drop-on-border);
		border-radius: 1rem;
		background: var(--tool-panel);
		color: var(--tool-text);
		text-shadow: var(--tool-halo);
		font-family: var(--editor-sans);
		font-size: var(--editor-small);
		white-space: nowrap;
	}
	/* past the top edge: the line fills during the hold, then full screen */
	.hold {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		height: 0.25rem;
		background: var(--kit-drop-on-ground);
	}
	.hold span {
		display: block;
		height: 100%;
		width: 0;
		background: var(--kit-list-row-insert-mark);
		animation: hold-fill 500ms linear forwards;
	}
	.hold.armed span {
		width: 100%;
		animation: none;
	}
	@keyframes zones-in {
		from {
			opacity: 0;
		}
	}
	@keyframes lands-in {
		from {
			opacity: 0;
			transform: scale(0.97);
		}
	}
	@keyframes hold-fill {
		to {
			width: 100%;
		}
	}
	/* reduced motion: the motion tokens are 0; the hold's line jumps full once armed */
	@media (prefers-reduced-motion: reduce) {
		.hold span {
			animation: none;
		}
	}
	:global(:root[data-motion='reduced']) .hold span {
		animation: none;
	}
</style>
