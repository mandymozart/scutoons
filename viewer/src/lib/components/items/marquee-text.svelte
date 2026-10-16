<script lang="ts">
	import { MediaQuery } from 'svelte/reactivity';

	/**
	 * One line of text that doesn't fit (owner 2026-10-16, the player bar): an ellipsis, and – only when it overflows and
	 * never with reduced motion – a slow marquee: a pause (~2 s), it scrolls through, a pause, back.
	 */
	let { text, testid }: { text: string; testid?: string } = $props();

	const reduced = new MediaQuery('prefers-reduced-motion: reduce');
	const SPEED = 28; // px a second
	const PAUSE = 2000; // ms
	let box = $state<HTMLSpanElement>();
	let inner = $state<HTMLSpanElement>();
	let moving = $state(false);

	$effect(() => {
		const outer = box, line = inner;
		void text;
		if (!outer || !line || reduced.current) return;
		let anim: Animation | null = null;
		const start = () => {
			anim?.cancel();
			anim = null;
			const over = line.scrollWidth - outer.clientWidth;
			moving = over > 1;
			if (!moving) return;
			const run = (over / SPEED) * 1000, total = 2 * PAUSE + 2 * run;
			const at = (ms: number) => ms / total;
			const x = (px: number) => ({ transform: `translateX(${-px}px)` });
			anim = line.animate(
				[
					{ ...x(0), offset: 0 },
					{ ...x(0), offset: at(PAUSE) },
					{ ...x(over), offset: at(PAUSE + run) },
					{ ...x(over), offset: at(2 * PAUSE + run) },
					{ ...x(0), offset: 1 }
				],
				{ duration: total, iterations: Infinity, easing: 'linear' }
			);
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
