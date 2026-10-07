<!-- A showTextBox event's effect on the text box it reveals (Dialog `play`): delay, then the effect for `duration`. -->
<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import type { Snippet } from 'svelte';
	import type { FxSoundChoice } from '@scutoons/content';
	import { playRevealSound } from '../fx/sounds.ts';

	let {
		duration = 1000,
		delay = 0,
		onComplete,
		sound,
		children
	}: {
		duration?: number;
		delay?: number;
		onComplete: () => void;
		/** the event's sound when it appears (none = silent for this effect) */
		sound?: FxSoundChoice;
		children: Snippet;
	} = $props();

	// initial value only: a delayed effect becomes visible in onMount
	let visible = $state(untrack(() => delay === 0));
	let playing = $state(false);

	onMount(() => {
		const delayTimer = setTimeout(() => {
			visible = true;
			playRevealSound(sound);
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

<div class="effect-wobble" class:visible class:playing style="--duration: {duration}ms">
	{@render children()}
</div>

<style>
	/* fills the text box it reveals */
	.effect-wobble {
		display: block;
		width: 100%;
		height: 100%;
		visibility: hidden;
		opacity: 0;
		transition: 
			opacity 0.3s ease,
			visibility 0s linear 0.3s;
	}

	.effect-wobble.visible {
		visibility: visible;
		opacity: 1;
		transition: opacity 0.3s ease;
	}

	.effect-wobble.playing {
		animation: wobble var(--duration) ease-in-out;
	}

	@keyframes wobble {
		0%,
		100% {
			transform: rotate(0deg);
		}
		15% {
			transform: rotate(-5deg);
		}
		30% {
			transform: rotate(3deg);
		}
		45% {
			transform: rotate(-3deg);
		}
		60% {
			transform: rotate(2deg);
		}
		75% {
			transform: rotate(-1deg);
		}
	}

</style>
