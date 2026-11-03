<script lang="ts" module>
	export interface Stats {
		fps: number;
		/** the frame's work, average and worst over the last second (ms) */
		cpu: number;
		peak: number;
		/** the sound's dropouts in the last minute and its render load per block (% of the budget): the median, the 99th
		 *  percentile and the longest of the last second (#356); null = no sound yet */
		dropouts: number | null;
		p50: number;
		p99: number;
		load: number;
		/** #356: the output limiter's deepest gain reduction of the last second (dB) – pumping can sound like a dropout */
		limit: number;
		/** the worker's adaptive lead in ms (#208: grows after a dropout, shrinks after a calm minute) */
		lead: number;
		relay: string;
		/** "webgpu" / "webgl2" (+ the adapter), or why there is no 3D */
		backend: string;
		/** #220: the 3D's level now and its render scale ("" = no 3D); `down` = the governor stepped below the level chosen */
		quality: string;
		down: boolean;
	}
</script>

<script lang="ts">
	// The collapsed panel's statistics (#209, owner 2026-10-31: "very basic statistics (fps, CPU / frame, audio glitches
	// / dropouts, the relay, the backend), using the entire width, like OSCT's collapsed bar"): one dense mono line –
	// FPS, the 3D's quality level and render scale (#220; amber while the governor holds it below the level chosen), the frame's own work, the sound's dropouts per minute and its DSP load per block – p50 / p99 / the longest (#356) –, the relay, the 3D backend. A number
	// out of the green turns amber / red; LIM is the output limiter's deepest gain reduction (#356). The abbreviations are the stage HUD's (language-neutral, like a meter's).
	let { stats }: { stats: Stats } = $props();
	const fpsLevel = $derived(stats.fps === 0 ? '' : stats.fps < 30 ? 'bad' : stats.fps < 50 ? 'warn' : '');
	const cpuLevel = $derived(stats.peak > 16 ? 'bad' : stats.peak > 10 ? 'warn' : '');
	// the lead absorbs a rare long block; a p99 near the budget does not stay absorbed
	const audioLevel = $derived(stats.dropouts === null ? '' : stats.dropouts > 0 ? 'bad' : stats.p99 > 50 || stats.load > 100 ? 'warn' : '');
	// a few dB are the limiter shaving peaks; past 6 dB it audibly holds the whole sound down
	const limitLevel = $derived(stats.limit > 6 ? 'warn' : '');
</script>

<p class="stats" data-testid="deck-stats" title="{stats.relay} · {stats.backend}">
	<span class={fpsLevel}><b>FPS</b> {stats.fps ? stats.fps.toFixed(0) : '–'}</span>
	{#if stats.quality}<span class:warn={stats.down} data-testid="deck-stats-quality"><b>Q</b> {stats.quality}</span>{/if}
	<span class={cpuLevel}><b>CPU</b> {stats.cpu.toFixed(1)}/{stats.peak.toFixed(1)}MS</span>
	<span class={audioLevel} title="dropouts/min · block p50/p99/max % of its budget · lead"><b>AUDIO</b> {stats.dropouts === null ? 'OFF' : `${stats.dropouts}/MIN ${stats.p50}/${stats.p99}/${stats.load}% +${stats.lead}MS`}</span>
	{#if stats.dropouts !== null}<span class={limitLevel} title="the output limiter's deepest gain reduction in the last second" data-testid="deck-stats-limit"><b>LIM</b> {stats.limit.toFixed(1)}DB</span>{/if}
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
