<!-- An onomatopoeia event's effect on the text box it reveals (Dialog `play`): delay, then the effect for `duration`. -->
<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import type { Snippet } from 'svelte';

	let {
		duration = 1000,
		delay = 0,
		onComplete,
		children
	}: {
		duration?: number;
		delay?: number;
		onComplete: () => void;
		children: Snippet;
	} = $props();

	// initial value only: a delayed effect becomes visible in onMount
	let visible = $state(untrack(() => delay === 0));

	onMount(() => {
		const delayTimer = setTimeout(() => {
			visible = true;

			const durationTimer = setTimeout(() => {
				onComplete();
			}, duration);

			return () => clearTimeout(durationTimer);
		}, delay);

		return () => clearTimeout(delayTimer);
	});
</script>

<div class="effect-none" class:visible={visible}>
	{@render children()}
</div>

<style>
	/* fills the text box it reveals */
	.effect-none {
		display: block;
		width: 100%;
		height: 100%;
		visibility: hidden;
		opacity: 0;
		transition: 
			opacity 0.3s ease,
			visibility 0s linear 0.3s;
	}

	.effect-none.visible {
		visibility: visible;
		opacity: 1;
		transition: opacity 0.3s ease;
	}

</style>
