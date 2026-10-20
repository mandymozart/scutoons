<script lang="ts">
	// Compare's transport (#125; VARIANTS-CANVAS §3.4 – Frame.io: ONE Play, ONE scrubber for every side): Play / Pause
	// (round – Space while Compare has the focus), the scrubber (a slider – a drag or a click puts every side at that
	// moment; ← → 0.1 s, Shift 1 s, Home / End; a drag while it plays pauses and plays on from where it let go), the time.
	import { Icon, IconButton } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { clockText } from '$lib/editor/storyboard-compare.ts';
	import type { CompareClock } from '$lib/editor/storyboard-compare-clock.svelte.ts';

	let { clock }: { clock: CompareClock } = $props();
	const share = $derived(clock.length > 0 ? Math.min(1, clock.t / clock.length) : 0);
	let track = $state<HTMLDivElement>();
	let resume = false;
	let dragging = false;
	const at = (e: PointerEvent) => {
		const r = track!.getBoundingClientRect();
		return r.width > 0 ? Math.min(1, Math.max(0, (e.clientX - r.left) / r.width)) * clock.length : 0;
	};
	function down(e: PointerEvent) {
		if (e.button !== 0 || !clock.length || !track) return;
		e.preventDefault();
		track.focus({ preventScroll: true });
		track.setPointerCapture(e.pointerId);
		dragging = true;
		resume = clock.playing;
		if (resume) clock.pause();
		clock.seek(at(e));
	}
	function move(e: PointerEvent) {
		if (dragging) clock.seek(at(e));
	}
	function up() {
		if (!dragging) return;
		dragging = false;
		if (resume) clock.play();
	}
	function keys(e: KeyboardEvent) {
		const step = e.shiftKey ? 1000 : 100;
		const to = ({ ArrowLeft: clock.t - step, ArrowDown: clock.t - step, ArrowRight: clock.t + step, ArrowUp: clock.t + step, Home: 0, End: clock.length } as Record<string, number>)[e.key];
		if (to === undefined) return;
		e.preventDefault();
		clock.seek(to);
	}
</script>

<div class="transport">
	<IconButton class="sc-round" label={clock.playing ? m['storyboard.compare_pause']() : m['storyboard.compare_play']()} disabled={!clock.length} onclick={clock.toggle} data-testid="compare-play"><Icon name={clock.playing ? 'pause' : 'play'} /></IconButton>
	<div class="track" bind:this={track} role="slider" tabindex="0" aria-label={m['storyboard.compare_scrub']()} aria-valuemin={0} aria-valuemax={Math.round(clock.length)} aria-valuenow={Math.round(clock.t)} aria-valuetext="{clockText(clock.t)} / {clockText(clock.length)}" title={m['storyboard.compare_scrub']()} onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up} onkeydown={keys} data-testid="compare-scrub">
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
