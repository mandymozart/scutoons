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
	// The collapsed panel's statistics (#209; #464, owner 2026-11-10: "It can be much more compact"): one dense mono line
	// of what is read at a glance. The monitor's items are deck-core's (#364), worded and judged there – amber / red as the
	// core says; the 3D's quality only while the governor holds it below the level chosen (#220); then the relay's line.
	// The other numbers are in the panel's copy of the sound report, the backend in its status line.
	let { stats }: { stats: Stats } = $props();
</script>

<p class="stats" data-testid="deck-stats" title="{stats.relay} · {stats.backend}">
	{#each stats.items as it, i (i)}
		{#if i === 1 && stats.down}<span class="warn" data-testid="deck-stats-quality"><b>Q</b> {stats.quality}</span>{/if}
		<span class={it.level} data-testid="deck-stats-{it.label.toLowerCase()}"><b>{it.label}</b> {it.value}</span>
	{/each}
	<span class="relay">{stats.relay}</span>
</p>

<style>
	.stats {
		flex: 1;
		min-width: 0;
		margin: 0;
		display: flex;
		gap: var(--space-05);
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
	/* the relay takes what is left and ends in an ellipsis */
	.relay {
		flex: 0 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
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
