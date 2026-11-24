<script lang="ts">
	// Sound design's Monitor: the engine host on the core's empty rigging stage playing the character's rig – the being and
	// the sound (from the stage's Play button: the browser holds sound back until a gesture); the mood chip and the
	// performance at its foot. The deck's stage is black by day too: what lies on it takes the night colours. The page keeps
	// it (sound-workspace.svelte) and lends it to wherever the Monitor view is shown.
	import { EngineStage } from '@scutoons/engine-host';
	import { API_URL, CDN_SERVER } from '$lib/config.ts';
	import { m } from '$lib/paraglide/messages.js';
	import type { SoundReadout } from '$lib/editor/sound-readout.svelte.ts';
	import MoodChip from '$lib/components/sound/mood-chip.svelte';

	let { character, readout }: { character: string; readout: SoundReadout } = $props();

	// #425: `rigging` – the core's empty rigging stage: the being and its knobs, no room, set or scene behind it (owner 2026-11-08)
	// svelte-ignore state_referenced_locally
	const options = { api: API_URL, cdn: `${CDN_SERVER}/`, wasmBase: '/live/', character, nick: 'creator', rigs: true, rigging: true } as const;

	$effect(() => {
		const h = readout.host;
		return h ? readout.attach(h) : undefined;
	});

	const stats = $derived(readout.stats);
</script>

<div class="stage" data-theme="night" data-testid="sound-deck">
	<EngineStage {options} gate bind:host={readout.host} onmounted={(h) => (readout.moods = h.moods)} />
	<div class="foot">
		<MoodChip moods={readout.moods} portrait={(md) => readout.host?.deck?.face(md)?.path ?? ''} playing={readout.snap.mood} value={readout.mood} onpick={(v) => readout.pickMood(v)} />
		<span class="perf" data-testid="sound-status" title="{m['creator.sound_perf']()}: {stats.state} · {stats.perf}">{stats.state} · <span data-testid="sound-fps">{stats.perf}</span></span>
	</div>
</div>

<style>
	.stage {
		position: relative;
		width: 100%;
		height: 100%;
		min-height: 0;
		background: var(--tool-ground);
	}
	/* the mood chip and the performance at the picture's foot; the performance gives way */
	.foot {
		position: absolute;
		inset: auto var(--space-05) var(--space-05);
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		gap: var(--space-05);
		z-index: 2;
		pointer-events: none;
	}
	.foot > :global(:first-child) {
		pointer-events: auto;
	}
	.perf {
		min-width: 0;
		overflow: hidden;
		font-size: var(--type-nav-micro-size);
		color: var(--tool-muted);
		white-space: nowrap;
		text-overflow: ellipsis;
	}
</style>
