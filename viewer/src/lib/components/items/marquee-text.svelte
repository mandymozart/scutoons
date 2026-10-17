<script lang="ts">
	import { MediaQuery } from 'svelte/reactivity';
	import { marqueeRun } from './marquee.ts';

	/**
	 * One line of text that doesn't fit (owner 2026-10-16, the player bar): an ellipsis, and – only when it overflows and
	 * never with reduced motion – a slow marquee: a pause (~2 s), it scrolls through, a pause, back.
	 */
	let { text, testid }: { text: string; testid?: string } = $props();

	const reduced = new MediaQuery('prefers-reduced-motion: reduce');
	let box = $state<HTMLSpanElement>();
	let inner = $state<HTMLSpanElement>();
	let moving = $state(false);

	$effect(() => {
		const outer = box, line = inner;
		void text;
		if (!outer || !line || reduced.current) return;
		let anim: Animation | null = null;
		// `moving` is only written here, never read: the effect reading it (start() runs inside it once) made it depend on
		// its own write – the cleanup's `false` and the next run's `true` re-ran it without end once the line overflowed
		// (effect_update_depth_exceeded), and that error stopped every update of the page (owner 2026-10-17: the mini
		// player bar back over the inventory, then nothing could be pressed)
		const start = () => {
			anim?.cancel();
			anim = null;
			const run = marqueeRun(line.scrollWidth - outer.clientWidth);
			moving = !!run;
			if (run) anim = line.animate(run.keyframes, { duration: run.duration, iterations: Infinity, easing: 'linear' });
		};
		const ro = new ResizeObserver(start);
		ro.observe(outer);
		start();
		return () => {
			ro.disconnect();
			anim?.cancel();
			moving = false;
		};
	});
</script>

<span class="marquee" class:moving bind:this={box} title={text} data-testid={testid}><span class="line" bind:this={inner}>{text}</span></span>

<style>
	.marquee {
		display: block;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
	}

	.line {
		display: inline-block;
	}

	/* standing still: the ellipsis */
	.marquee:not(.moving) .line {
		display: block;
		overflow: hidden;
		text-overflow: ellipsis;
	}
</style>
