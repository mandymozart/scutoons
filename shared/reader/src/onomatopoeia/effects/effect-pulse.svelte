<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import type { Snippet } from 'svelte';

	let {
		duration = 1000,
		delay = 0,
		onComplete,
		children,
		fill = false
	}: {
		duration?: number;
		delay?: number;
		onComplete: () => void;
		children: Snippet;
		/** fill the parent (a text box revealed by an onomatopoeia event) instead of sizing to the content */
		fill?: boolean;
	} = $props();

	// initial value only: a delayed effect becomes visible in onMount
	let visible = $state(untrack(() => delay === 0));
	let playing = $state(false);

	onMount(() => {
		const delayTimer = setTimeout(() => {
			visible = true;
			playing = true;

			const durationTimer = setTimeout(() => {
				playing = false;
				onComplete();
			}, duration);

			return () => clearTimeout(durationTimer);
		}, delay);

		return () => clearTimeout(delayTimer);
	});
</script>

<div class="effect-pulse" class:fill class:visible class:playing style="--duration: {duration}ms">
	{@render children()}
</div>

<style>
	.effect-pulse {
		display: inline-block;
		visibility: hidden;
		opacity: 0;
		transition: 
			opacity 0.3s ease,
			visibility 0s linear 0.3s;
	}

	.effect-pulse.visible {
		visibility: visible;
		opacity: 1;
		transition: opacity 0.3s ease;
	}

	.effect-pulse.playing {
		animation: pulse var(--duration) ease-in-out;
	}

	@keyframes pulse {
		0%,
		100% {
			transform: scale(1);
			opacity: 1;
		}
		50% {
			transform: scale(1.2);
			opacity: 0.8;
		}
	}

	.fill {
		display: block;
		width: 100%;
		height: 100%;
	}
</style>
