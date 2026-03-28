<script lang="ts">
	import { onMount } from 'svelte';
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

	let visible = $state(delay === 0);

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
	.effect-none {
		display: inline-block;
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
