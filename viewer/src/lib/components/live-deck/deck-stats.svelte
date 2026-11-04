<script lang="ts" module>
	import type { MonitorItem } from '@scutoons/live-deck';

	export interface Stats {
		/** #364: deck-core's monitor – FPS, CPU, AUDIO, LIM: its words and its judgement (ok / warn / bad) */
		items: MonitorItem[];
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
	// / dropouts, the relay, the backend), using the entire width, like OSCT's collapsed bar"): one dense mono line. The
	// monitor's items are deck-core's (#364: FPS of drawn frames, the frame's work, the sound's glitches in 10 minutes and
	// its load per block – p50 / p99 / the longest – with the lead, the limiter), worded and judged there – amber / red as
	// the core says; the 3D's quality level and render scale (#220; amber while the governor holds it below the level
	// chosen) after the FPS, then the relay and the 3D backend. The abbreviations are the stage HUD's.
	let { stats }: { stats: Stats } = $props();
</script>

<p class="stats" data-testid="deck-stats" title="{stats.relay} · {stats.backend}">
	{#each stats.items as it, i (i)}
		{#if i === 1 && stats.quality}<span class:warn={stats.down} data-testid="deck-stats-quality"><b>Q</b> {stats.quality}</span>{/if}
		<span class={it.level} data-testid="deck-stats-{it.label.toLowerCase()}"><b>{it.label}</b> {it.value}</span>
	{/each}
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
