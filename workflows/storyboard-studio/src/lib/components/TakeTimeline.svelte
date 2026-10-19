<script>
	// A take's trim (owner 2026-10-19: "Afterwards I only need to trim the camera movement"): the whole take as a bar,
	// the kept part between the in / out handles, the playhead; drag a handle = trim (the view shows that moment), drag /
	// click the bar = scrub, Play = from the playhead to out. Times in seconds of the whole take.
	import { Icon, IconButton } from '@scutoons/ui';

	/** @type {{ duration: number, tin: number, tout: number, playhead: number, playing: boolean, ontrim: (tin: number, tout: number) => void, onscrub: (t: number) => void, onplay: () => void, onpause: () => void }} */
	let { duration, tin, tout, playhead, playing, ontrim, onscrub, onplay, onpause } = $props();
	let bar = $state();
	const pct = (t) => `${(Math.max(0, Math.min(duration, t)) / Math.max(1e-6, duration)) * 100}%`;
	const MIN = 0.1; // the shortest take kept (s)

	function timeAt(e) {
		const r = bar.getBoundingClientRect();
		return Math.max(0, Math.min(duration, ((e.clientX - r.left) / r.width) * duration));
	}
	function drag(e, what) {
		e.preventDefault();
		e.stopPropagation();
		onpause();
		const move = (ev) => {
			const t = timeAt(ev);
			if (what === 'in') { const v = Math.min(t, tout - MIN); ontrim(v, tout); onscrub(v); }
			else if (what === 'out') { const v = Math.max(t, tin + MIN); ontrim(tin, v); onscrub(v); }
			else onscrub(t);
		};
		move(e);
		window.addEventListener('pointermove', move);
		window.addEventListener('pointerup', () => window.removeEventListener('pointermove', move), { once: true });
	}
	function key(e, what) {
		const step = e.shiftKey ? 1 : 1 / 24;
		const d = e.key === 'ArrowLeft' ? -step : e.key === 'ArrowRight' ? step : 0;
		if (!d) return;
		e.preventDefault();
		if (what === 'in') { const v = Math.max(0, Math.min(tin + d, tout - MIN)); ontrim(v, tout); onscrub(v); }
		else { const v = Math.min(duration, Math.max(tout + d, tin + MIN)); ontrim(tin, v); onscrub(v); }
	}
	const fmt = (t) => `${t.toFixed(2)} s`;
</script>

<div class="timeline">
	<div class="bar" bind:this={bar} role="slider" tabindex="0" aria-label="Playhead" aria-valuemin="0" aria-valuemax={duration} aria-valuenow={playhead} onpointerdown={(e) => drag(e, 'head')} onkeydown={(e) => { const d = e.key === 'ArrowLeft' ? -1 / 24 : e.key === 'ArrowRight' ? 1 / 24 : 0; if (d) { e.preventDefault(); onscrub(Math.max(0, Math.min(duration, playhead + d))); } }}>
		<span class="kept" style:left={pct(tin)} style:width={`calc(${pct(tout)} - ${pct(tin)})`}></span>
		<span class="head" style:left={pct(playhead)}></span>
		<span class="handle" role="slider" tabindex="0" aria-label="In" aria-valuemin="0" aria-valuemax={duration} aria-valuenow={tin} title="In – drag, or ← → (Shift = 1 s)" style:left={pct(tin)} onpointerdown={(e) => drag(e, 'in')} onkeydown={(e) => key(e, 'in')}></span>
		<span class="handle out" role="slider" tabindex="0" aria-label="Out" aria-valuemin="0" aria-valuemax={duration} aria-valuenow={tout} title="Out – drag, or ← → (Shift = 1 s)" style:left={pct(tout)} onpointerdown={(e) => drag(e, 'out')} onkeydown={(e) => key(e, 'out')}></span>
	</div>
	<div class="row">
		{#if playing}<IconButton label="Pause" class="sc-round" onclick={onpause}><Icon name="pause" /></IconButton>{:else}<IconButton label="Play the kept part" class="sc-round" onclick={onplay}><Icon name="play" /></IconButton>{/if}
		<span class="times">in {fmt(tin)} · out {fmt(tout)} · {fmt(tout - tin)} of {fmt(duration)} · at {fmt(playhead)}</span>
	</div>
</div>

<style>
	.timeline {
		display: grid;
		gap: var(--space-05);
	}
	.bar {
		position: relative;
		height: var(--control-height);
		background: var(--ink);
		cursor: ew-resize;
		touch-action: none;
	}
	.kept {
		position: absolute;
		top: 0;
		bottom: 0;
		background: var(--paper);
		opacity: 0.35;
		pointer-events: none;
	}
	.head {
		position: absolute;
		top: 0;
		bottom: 0;
		width: var(--border-width-focus);
		margin-left: calc(var(--border-width-focus) / -2);
		background: var(--state-warning);
		pointer-events: none;
	}
	.handle {
		position: absolute;
		top: 0;
		bottom: 0;
		width: 0.75rem;
		margin-left: -0.375rem;
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
		box-sizing: border-box;
		cursor: col-resize;
	}
	.row {
		display: flex;
		align-items: center;
		gap: var(--space-05);
	}
	.times {
		font-size: var(--type-caption-size);
		font-variant-numeric: tabular-nums;
	}
</style>
