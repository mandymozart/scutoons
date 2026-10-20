<script>
	// Compare's transport (agents/research/VARIANTS-CANVAS.md §3.4 – Frame.io: ONE Play, ONE scrubber for every side; the
	// mockup's row under the sides): Play / Pause (round – Space while Compare has the focus, Compare.svelte), the scrubber
	// (no native control: a slider – a drag or a click puts every side at that moment; ← → 0.1 s, Shift 1 s, Home / End;
	// a drag while it plays pauses and plays on from where it let go), the time "0:02.0 / 0:05.0". The clock:
	// compare-clock.svelte.js – as long as the longest video; a shorter one holds its last frame.
	import { Icon, IconButton } from '@scutoons/ui';
	import { clockText } from '../compare.js';

	/** @type {{ clock: { t: number, playing: boolean, length: number, toggle: () => void, play: () => void, pause: () => void, seek: (ms: number) => void } }} */
	let { clock } = $props();
	const share = $derived(clock.length > 0 ? Math.min(1, clock.t / clock.length) : 0);
	let track = $state();
	let resume = false;
	let dragging = false;

	const at = (e) => {
		const r = track.getBoundingClientRect();
		return r.width > 0 ? Math.min(1, Math.max(0, (e.clientX - r.left) / r.width)) * clock.length : 0;
	};
	function down(e) {
		if (e.button !== 0 || !clock.length) return;
		e.preventDefault();
		track.focus({ preventScroll: true });
		track.setPointerCapture(e.pointerId);
		dragging = true;
		resume = clock.playing;
		if (resume) clock.pause();
		clock.seek(at(e));
	}
	function move(e) {
		if (dragging) clock.seek(at(e));
	}
	function up() {
		if (!dragging) return;
		dragging = false;
		if (resume) clock.play();
	}
	function keys(e) {
		const step = e.shiftKey ? 1000 : 100;
		const to = { ArrowLeft: clock.t - step, ArrowDown: clock.t - step, ArrowRight: clock.t + step, ArrowUp: clock.t + step, Home: 0, End: clock.length }[e.key];
		if (to === undefined) return;
		e.preventDefault();
		clock.seek(to);
	}
</script>

<div class="transport">
	<IconButton class="sc-round" label={clock.playing ? 'Pause (Space)' : 'Play – every side on one clock (Space)'} disabled={!clock.length} onclick={clock.toggle} data-testid="compare-play"><Icon name={clock.playing ? 'pause' : 'play'} /></IconButton>
	<div class="track" bind:this={track} role="slider" tabindex="0" aria-label="The time – every side at the same moment" aria-valuemin="0" aria-valuemax={Math.round(clock.length)} aria-valuenow={Math.round(clock.t)} aria-valuetext="{clockText(clock.t)} of {clockText(clock.length)}" title="Scrub every side – a shorter video holds its last frame (← → 0.1 s, Shift 1 s)" onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up} onkeydown={keys} data-testid="compare-scrub">
		<span class="bar"><i style:width="{share * 100}%"></i></span>
	</div>
	<span class="time">{clockText(clock.t)} / {clockText(clock.length)}</span>
</div>

<style>
	.transport {
		display: flex;
		flex: none;
		align-items: center;
		gap: var(--space-05);
		padding: var(--space-05) var(--space-1) var(--space-1);
	}
	/* a control high to grab, the bar drawn in its middle (the mockup's track: an ink frame, an ink fill) */
	.track {
		flex: 1;
		display: flex;
		align-items: center;
		min-width: 0;
		height: var(--control-height);
		cursor: pointer;
		touch-action: none;
	}
	.track:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: var(--focus-offset);
	}
	.bar {
		display: block;
		box-sizing: border-box;
		width: 100%;
		height: 0.75rem;
		border: var(--border-width-regular) solid var(--ink);
	}
	.bar > i {
		display: block;
		height: 100%;
		background: var(--ink);
	}
	.time {
		flex: none;
		font-size: var(--type-caption-size);
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}
</style>
