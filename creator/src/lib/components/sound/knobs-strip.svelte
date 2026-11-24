<script lang="ts">
	// Sound design's Knobs panel (#282): a column per exposed knob of the rig, where it stands now (the core's live value,
	// lib/editor/signal-strip.ts), its name under it; the knob's path is its hover.
	import type { KnobColumn } from '$lib/editor/signal-strip.ts';
	import { m } from '$lib/paraglide/messages.js';

	let { knobs }: { knobs: KnobColumn[] } = $props();

	const pct = (v: number) => `${Math.round(v * 1000) / 10}%`;
</script>

<div class="knobs" role="group" aria-label={m['creator.sound_knobs']()} data-testid="sound-knobs">
	{#each knobs as k (k.path)}
		<div class="cell" title={`${k.label} · ${k.path}`}>
			<div class="bar" role="meter" aria-label={k.label} aria-valuemin="0" aria-valuemax="1" aria-valuenow={k.position}>
				<u style:height={pct(k.position)}></u><i class="tick" style:bottom="calc({pct(k.position)} - 0.1rem)"></i>
			</div>
			<span class="lab">{k.label}</span>
		</div>
	{/each}
</div>

<style>
	.knobs {
		flex: 1;
		min-height: 0;
		display: flex;
		gap: 0.375rem;
		padding: var(--space-025) var(--space-05);
		overflow-x: auto;
		font-size: 0.75rem;
	}
	.cell {
		display: flex;
		flex-direction: column;
		align-items: center;
		min-height: 0;
	}
	.bar {
		flex: 1;
		width: 1rem;
		min-height: 0;
		position: relative;
		overflow: hidden;
		border: 1px solid var(--tool-text);
	}
	u {
		position: absolute;
		left: 0;
		right: 0;
		bottom: 0;
		background: var(--tool-control);
		text-decoration: none;
	}
	.tick {
		position: absolute;
		left: 0;
		right: 0;
		height: 0.1875rem;
		background: var(--tool-pressed);
	}
	.lab {
		writing-mode: vertical-rl;
		transform: rotate(180deg);
		height: 3.25rem;
		margin-top: 0.125rem;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		color: var(--tool-muted);
	}
</style>
