<script lang="ts">
	import type { Snippet } from 'svelte';
	import { SCREENS, readerSizes, screenScale, type Screen } from '$lib/editor/screens.ts';

	/**
	 * The score stage's screen (research/COLLAGE.md §4.4): a reader's phone (390 × 844) or desktop window (1440 × 810) at
	 * the device's own CSS size – so the score view inside breaks its lines and pages as it does for readers – scaled to
	 * the room with a transform, like the dock's Reader frame. The paper is inside the scaled group: pieces blend as they
	 * do for readers. Inside, the tokens are the reader's on that device (`readerSizes`: its root, a phone's touch floor –
	 * not the creator's root), so the view's header and footer, and with them its pages, are as tall as a reader's.
	 * `--stage-scale` lets handles inside keep their on-screen size.
	 */
	let { screen, children }: { screen: Screen; children: Snippet } = $props();

	let roomWidth = $state(0);
	let roomHeight = $state(0);
	const device = $derived(SCREENS[screen]);
	const scale = $derived(screenScale(device, { width: roomWidth, height: roomHeight }));
	const sizes = $derived(Object.entries(readerSizes(device)).map(([name, value]) => `${name}: ${value}`).join('; '));
</script>

<div class="room" bind:clientWidth={roomWidth} bind:clientHeight={roomHeight}>
	<!-- the device's CSS px (a simulated screen, not a UI size – as the dock's Reader frame) -->
	<div class="device" style:width="{device.width * scale}px" style:height="{device.height * scale}px">
		<div
			class="screen"
			style={sizes}
			style:width="{device.width}px"
			style:height="{device.height}px"
			style:transform="scale({scale})"
			style:--stage-scale={scale || 1}
			data-screen={screen}
			data-testid="score-frame"
		>
			{@render children()}
		</div>
	</div>
</div>

<style>
	/* the room the screen is scaled into */
	.room {
		width: 100%;
		height: 100%;
		min-height: 0;
		display: flex;
		align-items: flex-start;
		justify-content: center;
		overflow: hidden;
	}
	/* the screen's edge drawn outside it (an outline takes no room from the scaled screen) */
	.device {
		flex: none;
		overflow: hidden;
		outline: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
	}
	/* scaled from its corner; its text = the reader's body text on that device (its root – the sizes set on this element) */
	.screen {
		position: relative;
		transform-origin: 0 0;
		overflow: hidden;
		font-size: var(--type-body-size);
	}
</style>
