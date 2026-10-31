<script lang="ts" module>
	export interface Stats {
		fps: number;
		/** the frame's work, average and worst over the last second (ms) */
		cpu: number;
		peak: number;
		/** the sound's dropouts in the last minute and its peak render load (%); null = no sound yet */
		dropouts: number | null;
		load: number;
		relay: string;
		/** "webgpu" / "webgl2" (+ the adapter), or why there is no 3D */
		backend: string;
	}
</script>

<script lang="ts">
	// The collapsed panel's statistics (#209, owner 2026-10-31: "very basic statistics (fps, CPU / frame, audio glitches
	// / dropouts, the relay, the backend), using the entire width, like OSCT's collapsed bar"): one dense mono line –
	// FPS, the frame's own work, the sound's dropouts per minute and its DSP load, the relay, the 3D backend. A number
	// out of the green turns amber / red. The abbreviations are the stage HUD's (language-neutral, like a meter's).
	let { stats }: { stats: Stats } = $props();
	const fpsLevel = $derived(stats.fps === 0 ? '' : stats.fps < 30 ? 'bad' : stats.fps < 50 ? 'warn' : '');
	const cpuLevel = $derived(stats.peak > 16 ? 'bad' : stats.peak > 10 ? 'warn' : '');
	const audioLevel = $derived(stats.dropouts === null ? '' : stats.dropouts > 0 ? 'bad' : stats.load > 80 ? 'warn' : '');
</script>

<p class="stats" data-testid="deck-stats" title="{stats.relay} · {stats.backend}">
	<span class={fpsLevel}><b>FPS</b> {stats.fps ? stats.fps.toFixed(0) : '–'}</span>
	<span class={cpuLevel}><b>CPU</b> {stats.cpu.toFixed(1)}/{stats.peak.toFixed(1)}MS</span>
	<span class={audioLevel}><b>AUDIO</b> {stats.dropouts === null ? 'OFF' : `${stats.dropouts}/MIN ${stats.load}%`}</span>
	<span class="relay"><b>RELAY</b> {stats.relay || '–'}</span>
	<span class="backend"><b>GPU</b> {stats.backend || '–'}</span>
</p>

<style>
	.stats {
		flex: 1;
		min-width: 0;
		margin: 0;
		display: flex;
		gap: var(--space-075);
		overflow: hidden;
		white-space: nowrap;
		font-family: var(--font-mono);
		font-size: 0.625rem;
		line-height: var(--control-height);
		text-transform: uppercase;
		color: color-mix(in srgb, var(--paper) 85%, transparent);
		font-variant-numeric: tabular-nums;
	}
	span {
		flex: none;
	}
	/* the relay and the backend take what is left and end in an ellipsis – the backend gives way first */
	.relay,
	.backend {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.relay {
		flex: 0 1 auto;
	}
	.backend {
		flex: 0 100 auto;
	}
	b {
		font-weight: var(--font-weight-regular);
		color: color-mix(in srgb, var(--paper) 45%, transparent);
	}
	.warn {
		color: var(--state-warning);
	}
	.bad {
		color: var(--state-error);
	}
</style>
