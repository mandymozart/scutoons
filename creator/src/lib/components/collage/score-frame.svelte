<script lang="ts">
	import type { Snippet } from 'svelte';
	import { SCREENS, screenScale, type Screen } from '$lib/editor/screens.ts';

	/**
	 * The score stage's screen (research/COLLAGE.md §4.4): a reader's phone (390 × 844) or desktop window (1440 × 810) at
	 * the device's own CSS size – so the score view inside breaks its lines and pages as it does for readers – scaled to
	 * the room with a transform, like the dock's Reader frame. The paper is inside the scaled group: pieces blend as they
	 * do for readers. `--stage-scale` lets handles inside keep their on-screen size.
	 */
	let { screen, children }: { screen: Screen; children: Snippet } = $props();

	let roomWidth = $state(0);
	let roomHeight = $state(0);
	const device = $derived(SCREENS[screen]);
	const scale = $derived(screenScale(device, { width: roomWidth, height: roomHeight }));
</script>

<div class="room" bind:clientWidth={roomWidth} bind:clientHeight={roomHeight}>
	<!-- the device's CSS px (a simulated screen, not a UI size – as the dock's Reader frame) -->
	<div class="device" style:width="{device.width * scale}px" style:height="{device.height * scale}px">
		<div
			class="screen"
			class:touch={screen === 'phone'}
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
	.screen {
		position: relative;
		transform-origin: 0 0;
		overflow: hidden;
	}
	/* a phone is a touch screen: its controls are the touch size (shared/tokens/tokens.css, pointer: coarse – 2.75rem =
	   the 44px floor), so the score view's header and footer – and its pages – are as tall as on the reader's phone */
	.screen.touch {
		--control-height: max(2.75rem, calc(2rem + 2 * var(--border-width-regular)));
	}
</style>
