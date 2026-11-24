<script lang="ts" module>
	/** the handheld stage's words (each page brings its own: the creator's phone-words, the reader's paraglide) */
	export interface HandheldWords {
		title: string;
		start: string;
		startAr: string;
		rotationOnly: string;
		startHint: string;
		recenter: string;
		recenterHint: string;
		modeXr: string;
		modeOrientation: string;
		trackingLost: string;
		dragHint: string;
		stage: string;
	}

	/** the handheld's side the stage drives (@scutoons/content/storyboard `Handheld`), its state as plain values */
	export interface HandheldControls {
		down(e: PointerEvent): void;
		move(e: PointerEvent): void;
		up(e: PointerEvent): void;
		startXR(root: HTMLElement): void;
		startOrientation(): void;
	}
</script>

<script lang="ts">
	// The one handheld's phone screen: the stage a finger drags (rotation only), what the phone tracks, Start (AR where
	// the phone has it), Recenter – on ink, without the app's frame. The creator's storyboard phone page and the reader's
	// camera page wrap it: their own header items (`top`), status line and buttons (`actions`).
	import type { Snippet } from 'svelte';

	interface Props {
		words: HandheldWords;
		hand: HandheldControls;
		mode: 'xr' | 'orientation' | 'none';
		tracking: boolean;
		/** WebXR immersive-ar is there */
		xr: boolean;
		/** the link stands: the controls show */
		live: boolean;
		/** what the phone films (the shot, the camera's name) */
		big: string;
		/** a line over the stage (connecting, refused …); '' = none */
		status?: string;
		onrecenter?: () => void;
		top?: Snippet;
		actions?: Snippet;
		/** more under the stage's words (a question, a warning) */
		children?: Snippet;
	}
	const { words: w, hand, mode, tracking, xr, live, big, status = '', onrecenter, top, actions, children }: Props = $props();
	let root = $state<HTMLElement>();
	const tracked = $derived(mode === 'xr' ? (tracking ? w.modeXr : w.trackingLost) : mode === 'orientation' ? w.modeOrientation : '');
</script>

<main class="phone sc-tool" bind:this={root} data-testid="handheld-phone">
	<div class="stage" role="application" aria-label={w.stage} onpointerdown={hand.down} onpointermove={hand.move} onpointerup={hand.up} onpointercancel={hand.up}>
		<p class="big" data-testid="handheld-big">{big}</p>
		{#if tracked}<p class="small">{tracked}</p>{/if}
		{#if mode === 'orientation'}<p class="small">{w.dragHint}</p>{/if}
		{#if mode === 'none' && live}<p class="small">{w.startHint}</p>{/if}
		{@render children?.()}
	</div>
	<header class="bar">
		<span>{w.title}</span>
		{@render top?.()}
	</header>
	{#if status}<p class="status" role="status" data-testid="phone-status">{status}</p>{/if}
	{#if live}
		<footer class="bar bottom">
			{#if mode === 'none'}
				{#if xr}<button type="button" onclick={() => root && hand.startXR(root)} data-testid="phone-start-xr">{w.startAr}</button>{/if}
				<button type="button" class:secondary={xr} onclick={() => hand.startOrientation()} data-testid="phone-start">{xr ? w.rotationOnly : w.start}</button>
			{:else if onrecenter}
				<button type="button" class="secondary" title={w.recenterHint} onclick={onrecenter} data-testid="phone-recenter">{w.recenter}</button>
			{/if}
			{@render actions?.()}
		</footer>
	{/if}
</main>

<style>
	:global(body) {
		margin: 0;
		background: var(--ink);
	}
	/* over an app's chrome (the reader's menu 1003 and logo 1004) */
	.phone {
		position: fixed;
		inset: 0;
		z-index: 1005;
		background: var(--ink);
		color: var(--paper);
		overflow: hidden;
		touch-action: none;
		user-select: none;
		font-family: var(--font-sans);
	}
	.stage {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: var(--space-05);
		padding: var(--space-1);
		text-align: center;
	}
	.stage :global(p) {
		margin: 0;
	}
	.big {
		font-size: var(--type-title-size, 1.5rem);
	}
	.small {
		font-size: var(--type-caption-size);
		opacity: 0.8;
	}
	.bar {
		position: absolute;
		left: 0;
		right: 0;
		top: 0;
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
		align-items: center;
		padding: var(--space-05) var(--space-1);
		font-size: var(--type-caption-size);
		pointer-events: none;
	}
	.bottom {
		top: auto;
		bottom: 0;
		justify-content: center;
		padding-bottom: calc(var(--space-1) + env(safe-area-inset-bottom));
		pointer-events: auto;
	}
	/* on ink: the primary is paper, the secondary an outline */
	.bottom :global(button) {
		flex: 1 1 0;
		max-width: 12rem;
		background: var(--paper);
		color: var(--ink);
		border-color: var(--paper);
	}
	.bottom :global(button.secondary) {
		background: var(--ink);
		color: var(--paper);
		border: var(--border-width, 1px) solid var(--paper);
	}
	.bottom :global(button.rec-on) {
		background: var(--state-error);
		border-color: var(--state-error);
		color: var(--paper);
	}
	.status {
		position: absolute;
		left: var(--space-1);
		right: var(--space-1);
		top: 30%;
		margin: 0;
		text-align: center;
	}
</style>
