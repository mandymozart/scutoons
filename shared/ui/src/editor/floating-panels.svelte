<script lang="ts">
	import type { Snippet } from 'svelte';
	import FloatingPanel from './floating-panel.svelte';
	import type { Floats } from './floats.svelte.ts';

	/**
	 * The floating panels of one workspace over its stage (`children`, the background): measures the room, lays every
	 * panel out (`floats`) and draws the tab in the hand and where it would land in a panel (the stage's drop zones:
	 * StageZones). `tab(id)` renders a tab's content, `toolbar(panel)` an inline panel's row.
	 */
	let {
		floats,
		tab,
		toolbar,
		children,
		labels = {}
	}: {
		floats: Floats;
		tab: Snippet<[tab: string]>;
		toolbar?: Snippet<[panel: string]>;
		children?: Snippet;
		labels?: { close?: string; move?: string };
	} = $props();

	let layer = $state<HTMLDivElement>();

	$effect(() => {
		const el = layer;
		if (!el) return;
		const measure = () => {
			const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
			const control = parseFloat(getComputedStyle(el).getPropertyValue('--control-height')) * rem || 2.15 * rem;
			const r = el.getBoundingClientRect();
			const gap = rem;
			floats.metrics = { rem, gap, head: control + 0.375 * rem + 2, button: control, char: 0.62 * rem };
			floats.area = { x: r.left + gap, y: r.top + gap, w: Math.max(0, r.width - 2 * gap), h: Math.max(0, r.height - 2 * gap) };
			floats.win = { w: window.innerWidth, h: window.innerHeight };
		};
		measure();
		const observer = new ResizeObserver(measure);
		observer.observe(el);
		window.addEventListener('resize', measure);
		return () => {
			observer.disconnect();
			window.removeEventListener('resize', measure);
		};
	});

	const held = $derived(floats.heldTab?.moved ? floats.heldTab : null);
	const thrown = $derived(floats.thrownTab);
	const rem = (px: number) => `${px / (floats.metrics.rem || 16)}rem`;
	/** The layer's top left in client px */
	const origin = $derived({ left: floats.area.x - floats.metrics.gap, top: floats.area.y - floats.metrics.gap });
</script>

<svelte:window
	onkeydown={(e) => {
		if (e.key === 'Escape' && floats.heldTab) floats.cancelTab();
	}}
/>

<div class="floats" bind:this={layer}>
	<div class="stage">{@render children?.()}</div>
	{#if floats.area.w > 0}
		{#each floats.specs as spec (spec.id)}
			<FloatingPanel
				{floats}
				id={spec.id}
				closeLabel={labels.close}
				moveLabel={labels.move}
			>
				{#snippet children(active)}
					{#if spec.inline}
						{@render toolbar?.(spec.id)}
					{:else if active}
						{@render tab(active)}
					{/if}
				{/snippet}
			</FloatingPanel>
		{/each}
	{/if}
	{#if held}
		{#if held.marker}
			<div
				class="marker"
				style:left="{held.marker.x - origin.left}px"
				style:top="{held.marker.y - origin.top}px"
				style:height="{held.marker.h}px"
			></div>
		{/if}
		<div class="ghost" class:out={!held.target} class:staging={!!floats.zone} style:left="{held.at[0] - origin.left}px" style:top="{held.at[1] - origin.top}px">
			{floats.tab(held.tab)?.name ?? held.tab}
		</div>
	{/if}
	{#if thrown}
		<!-- a tab flung off the screen: its ghost flies on out -->
		<div
			class="ghost thrown"
			data-testid="thrown-tab"
			style:left="{thrown.rect.x - origin.left}px"
			style:top="{thrown.rect.y - origin.top}px"
			style:--throw-x={rem(thrown.x)}
			style:--throw-y={rem(thrown.y)}
		>
			{floats.tab(thrown.id)?.name ?? thrown.id}
		</div>
	{/if}
</div>

<style>
	.floats {
		position: relative;
		width: 100%;
		height: 100%;
		overflow: hidden;
		background: var(--tool-ground);
	}
	.stage {
		position: absolute;
		inset: 0;
	}
	.marker {
		position: absolute;
		z-index: 1000;
		width: 2px;
		background: var(--kit-list-row-insert-mark);
		box-shadow: 0 0 0.375rem var(--kit-list-row-insert-mark);
		pointer-events: none;
	}
	.ghost {
		position: absolute;
		z-index: 1001;
		padding: 0.25rem 0.75rem;
		border-radius: 1rem;
		background: var(--tool-pressed);
		color: var(--tool-pressed-text);
		font-family: var(--editor-sans);
		font-size: var(--editor-small);
		white-space: nowrap;
		box-shadow: none;
		transform: translate(0.75rem, 0.5rem) rotate(-3deg);
		pointer-events: none;
	}
	.ghost.thrown {
		animation: throw-out var(--motion-slow) var(--motion-ease-in) forwards;
	}
	@keyframes throw-out {
		to {
			transform: translate(calc(0.75rem + var(--throw-x)), calc(0.5rem + var(--throw-y))) rotate(-3deg);
			opacity: 0;
		}
	}
	.ghost.out {
		outline: 1px dashed var(--kit-drop-on-border);
		outline-offset: 0.25rem;
	}
	/* over a drop zone the stage's preview says where it lands: the ghost steps back */
	.ghost.staging {
		opacity: 0;
	}
</style>
