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

<div class="effect-shiver" class:fill class:visible class:playing style="--duration: {duration}ms">
	{@render children()}
</div>

<style>
	.effect-shiver {
		display: inline-block;
		visibility: hidden;
		opacity: 0;
		transition: 
			opacity 0.3s ease,
			visibility 0s linear 0.3s;
	}

	.effect-shiver.visible {
		visibility: visible;
		opacity: 1;
		transition: opacity 0.3s ease;
	}

	.effect-shiver.playing {
		animation: shiver var(--duration) ease-in-out;
	}

	@keyframes shiver {
		0%,
		100% {
			transform: translateX(0);
		}
		10%,
		30%,
		50%,
		70%,
		90% {
			transform: translateX(-2px) rotate(-1deg);
		}
		20%,
		40%,
		60%,
		80% {
			transform: translateX(2px) rotate(1deg);
		}
	}

	.fill {
		display: block;
		width: 100%;
		height: 100%;
	}
</style>
