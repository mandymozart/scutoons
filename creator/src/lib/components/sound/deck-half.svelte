<script lang="ts">
	// The deck half of the Sound design workspace: the engine host on the empty rigging stage playing the character's rig –
	// the being, the knobs and the sound (from the stage's Play button: the browser holds sound back until a gesture). The
	// character's properties sit on the stage as chips, the signal strip under it. The deck's stage is black by day too:
	// what lies on it takes the night colours.
	import { EngineStage, emptySnapshot, type EngineHost, type EngineSnapshot } from '@scutoons/engine-host';
	import type { GraphWiring } from '@scutoons/rig-designer';
	import { API_URL, CDN_SERVER } from '$lib/config.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { inputMeters, knobColumns, outputMeters, sameColumns, stripStatus, type KnobColumn, type KnobLike } from '$lib/editor/signal-strip.ts';
	import MoodChip from '$lib/components/sound/mood-chip.svelte';
	import SignalStrip from '$lib/components/sound/signal-strip.svelte';

	// `host` and `wiring` (the core's graph of what plays, anew after every rig turn) go up to the designer's Nodes tab
	let { character, host = $bindable(null), wiring = $bindable() }: { character: string; host?: EngineHost | null; wiring?: GraphWiring } = $props();

	// #425: `rigging` – the core's empty rigging stage: the being and its knobs, no room, set or scene behind it (owner 2026-11-08)
	// svelte-ignore state_referenced_locally
	const options = { api: API_URL, cdn: `${CDN_SERVER}/`, wasmBase: '/live/', character, nick: 'creator', rigs: true, rigging: true } as const;

	let snap = $state<EngineSnapshot>(emptySnapshot());
	let moods = $state<string[]>([]);
	let mood = $state('auto');
	let rigKnobs: KnobLike[] = [];
	let columns = $state<KnobColumn[]>([]);
	let reduction = $state(0);

	$effect(() => {
		if (!host) return;
		const h = host;
		wiring = h.deck?.wiring();
		h.hooks = {
			rig: (played) => ((rigKnobs = played?.knobs ?? []), (wiring = h.deck?.wiring())),
			// once per animation frame: the knobs' live values (the being's buttons, a mood's preset) and the limiter
			tick: () => {
				const d = h.deck;
				if (!d) return;
				const next = knobColumns(rigKnobs, d, getLocale());
				if (!sameColumns(next, columns)) columns = next;
				const gr = d.bands.limiterBar;
				if (Math.abs(gr - reduction) >= 0.01) reduction = gr;
			}
		};
		return h.subscribe((s) => (snap = s));
	});

	function pickMood(value: string) {
		mood = value;
		host?.setMood(value === 'auto' ? null : value);
	}

	// the engine host reports one reading per audio output (none while no sound runs: dim slots) and the scene's device messages –
	// the core's, held, placed on the dB scale and worded (#373)
	const outputs = $derived(outputMeters(snap.outputs));
	const stats = $derived(stripStatus(snap));
	const signals = $derived(snap.signals);
	// the engine reports no audio inputs yet (their sources are still to be decided); an empty group shows nothing
	const inputs = inputMeters([]);
</script>

<div class="deck" data-testid="sound-deck">
	<div class="stage" data-theme="night">
		<EngineStage {options} gate bind:host onmounted={(h) => (moods = h.moods)} />
		<div class="foot">
			<div class="chips">
				<MoodChip {moods} portrait={(md) => host?.deck?.face(md)?.path ?? ''} playing={snap.mood} value={mood} onpick={pickMood} />
			</div>
			<p class="perf" data-testid="sound-status" title="{m['creator.sound_perf']()}: {stats.state} · {stats.perf}">{stats.state} · <span data-testid="sound-fps">{stats.perf}</span></p>
		</div>
	</div>
	<SignalStrip {inputs} {outputs} {reduction} knobs={columns} {signals} />
</div>

<style>
	.deck {
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
		height: 100%;
	}
	.stage {
		flex: 1;
		min-height: 10rem;
		position: relative;
		background: var(--tool-ground);
	}
	/* the mood chip and the performance in one row at the picture's foot – the engine draws its build label at the top –
	   the performance gives way, never covers the chip */
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
	.chips {
		display: flex;
		flex: none;
		gap: 0.25rem;
		pointer-events: auto;
	}
	/* the performance, tiny: it is the visual and the audio together that it measures */
	.perf {
		min-width: 0;
		margin: 0;
		overflow: hidden;
		font-size: var(--type-nav-micro-size);
		color: var(--tool-muted);
		white-space: nowrap;
		text-overflow: ellipsis;
	}
</style>
