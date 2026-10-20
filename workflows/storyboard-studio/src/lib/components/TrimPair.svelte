<script>
	// The in / out pair in the main view while a take is trimmed (research/TIMELINE.md §4 item 2 – Final Cut's two-up; the
	// prototype studio-layout/trim-prototype.html, way C): the kept part's first frame (in) and its last (out) side by
	// side, the moved one marked. The moved one is live – the 3D view as it shows now (trim() puts the playhead there, the
	// view shows the take's camera at that moment), copied every frame; where the view shows something else (the phone's
	// live pose holds it, "Back to the shot camera" and a key that left the playhead where it was) rendered at its moment
	// instead, once per change. The other is a still the 3D view renders at its moment when the gesture starts (a slip
	// moves both: the out follows, at most every 80 ms). Over the 3D view like ClipView; App shows it while `tl.pair` is
	// set (a gesture and a moment after its release – timeline.svelte.js).
	import { onMount } from 'svelte';
	import { formatTime } from '@scutoons/ui/timeline';
	import { hh, stillAt } from '../handheld/handheld.svelte.js';
	import { FRAME_MS } from '../timeline-lanes.js';
	import EditorSwitch from './EditorSwitch.svelte';

	/** @type {{ pair: { n: number, id: string, take: string, edge: 'in' | 'out' | 'slip', in: number, out: number, live: boolean }, step?: number }} */
	let { pair, step = FRAME_MS } = $props();
	let inCanvas = $state();
	let outCanvas = $state();
	/** the out frame: the last one kept (a frame before the out) */
	const last = $derived(Math.max(pair.in, pair.out - step));
	const fps = $derived({ fps: Math.round(1000 / step) });
	const what = $derived(`${pair.id} · take ${pair.take} · kept ${formatTime(pair.out - pair.in, 's')} – ${pair.edge === 'slip' ? 'slipped' : `the ${pair.edge} moved`}`);

	onMount(() => {
		let raf = 0;
		let [drawn, posed] = ['', ''];
		let at = 0;
		const frame = (now) => {
			raf = requestAnimationFrame(frame);
			if (!inCanvas || !outCanvas) return;
			// the live side: the in, or the out when only the out moves; the still: the other one (ms of the take)
			const outLive = pair.edge === 'out';
			const [tIn, tOut] = [pair.in, Math.max(pair.in, pair.out - step)];
			const [still, t] = outLive ? [inCanvas, tIn] : [outCanvas, tOut];
			const key = `${pair.n}|${t}`;
			// rendered when its gesture starts; a moving still (a slip) again at most every 80 ms – its last place always
			const fresh = !drawn.startsWith(`${pair.n}|`) || !pair.live || now - at >= 80;
			if (key !== drawn && fresh && stillAt(still, t / 1000)) [drawn, at] = [key, now];
			const [live, tl] = outLive ? [outCanvas, tOut] : [inCanvas, tIn];
			// the 3D view shows the take at the live side's moment (hh.playhead: s of the take, set by its pose): a copy
			if (hh.held && !hh.live && Math.abs(hh.playhead * 1000 - tl) < 1) {
				if (stillAt(live)) posed = '';
			} else if (posed !== `${pair.n}|${tl}` && stillAt(live, tl / 1000)) posed = `${pair.n}|${tl}`;
		};
		raf = requestAnimationFrame(frame);
		return () => cancelAnimationFrame(raf);
	});
</script>

<section class="trim-pair" aria-label="The kept part's in and out">
	<div class="tools"><EditorSwitch /><span class="cap" title={what}>{what}</span></div>
	<div class="stage">
		<figure class:moved={pair.edge !== 'out'}>
			<figcaption>In · {formatTime(pair.in, fps)}</figcaption>
			<canvas bind:this={inCanvas}></canvas>
		</figure>
		<figure class:moved={pair.edge !== 'in'}>
			<figcaption>Out · {formatTime(last, fps)}</figcaption>
			<canvas bind:this={outCanvas}></canvas>
		</figure>
	</div>
</section>

<style>
	/* over the 3D view (App's main), the same bar on top as ClipView */
	.trim-pair {
		position: absolute;
		inset: 0;
		z-index: 1;
		display: flex;
		flex-direction: column;
		min-height: 0;
		background: var(--paper);
	}
	.tools {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		min-height: var(--control-height);
		padding: var(--space-025) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	.cap {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--type-caption-size);
	}
	/* the two frames side by side on the stage's ink, each fitted whole into its half */
	.stage {
		flex: 1;
		min-height: 0;
		display: flex;
		gap: var(--space-1);
		padding: var(--space-1);
		background: var(--ink);
		color: var(--paper);
	}
	figure {
		flex: 1 1 0;
		min-width: 0;
		min-height: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-05);
	}
	/* the time code over each frame; the moved one inverted (paper on ink) */
	figcaption {
		flex: none;
		padding: 0 var(--space-05);
		border: var(--border-width-regular) solid var(--paper);
		font-size: var(--type-caption-size);
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}
	.moved figcaption {
		background: var(--paper);
		color: var(--ink);
	}
	canvas {
		flex: 1 1 0;
		min-height: 0;
		width: 100%;
		object-fit: contain;
	}
</style>
