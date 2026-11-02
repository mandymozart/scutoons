<script lang="ts">
	// The deck half of the Sound design workspace (#282): the engine host of #298 (`<EngineStage gate>`) playing the
	// character's rig – the being, the knobs and the sound. The sound starts with the round Play button on the stage (or the
	// first click on it; the browser holds the first sound back until a gesture). The character's properties sit on the stage
	// as chips (the mood is the first; look, persona … follow); under it the signal strip (owner draft 2026-11-02). The
	// compiled module plays here with #290 (what the deck plays today is the Info panel's to say once the editor has one).
	import { EngineStage, emptySnapshot, type EngineHost, type EngineSnapshot } from '@scutoons/engine-host';
	import { API_URL, CDN_SERVER } from '$lib/config.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { inputMeters, knobColumns, outputMeters, reductionBar, sameColumns, signalRows, stripStatus, type KnobColumn, type KnobLike } from '$lib/editor/signal-strip.ts';
	import SignalStrip from '$lib/components/sound/signal-strip.svelte';

	let { character }: { character: string } = $props();

	// svelte-ignore state_referenced_locally
	const options = { api: API_URL, cdn: `${CDN_SERVER}/`, wasmBase: '/live/', character, nick: 'creator', rigs: true } as const;

	let host = $state<EngineHost | null>(null);
	let snap = $state<EngineSnapshot>(emptySnapshot());
	let moods = $state<string[]>(['neutral']);
	let mood = $state('auto');
	let rigKnobs: KnobLike[] = [];
	let columns = $state<KnobColumn[]>([]);
	let reduction = $state(0);

	$effect(() => {
		if (!host) return;
		const h = host;
		h.hooks = {
			rig: (played) => (rigKnobs = played?.knobs ?? []),
			// once per animation frame: the knobs' live values (the being's buttons, a mood's preset) and the limiter
			tick: () => {
				const d = h.deck;
				if (!d) return;
				const next = knobColumns(rigKnobs, d.value, getLocale());
				if (!sameColumns(next, columns)) columns = next;
				const gr = reductionBar(d.bands.reduction);
				if (Math.abs(gr - reduction) >= 0.01) reduction = gr;
			}
		};
		return h.subscribe((s) => (snap = s));
	});

	function pickMood(value: string) {
		mood = value;
		host?.setMood(value === 'auto' ? null : value);
	}

	// the host reports the master only (level); per-output peaks and the device-message tap come with the engine's report
	const peaks = $derived((snap as { outputs?: number[] }).outputs ?? [snap.level]);
	const outputs = $derived(outputMeters(peaks));
	const stats = $derived(stripStatus(snap));
	const signals = $derived(signalRows({}, 0));
	// the engine reports no audio inputs yet (their sources are still to be decided); an empty group shows nothing
	const inputs = inputMeters([]);
</script>

<div class="deck" data-testid="sound-deck">
	<div class="stage">
		<EngineStage {options} gate bind:host onmounted={(h) => (moods = h.moods)} />
		<div class="chips">
			<label class="chip" title={m['creator.sound_mood']()}>
				<span>{m['creator.sound_mood']()}</span>
				<select onchange={(e) => pickMood(e.currentTarget.value)} data-testid="sound-mood" aria-label={m['creator.sound_mood']()}>
					<option value="auto" selected={mood === 'auto'}>{m['creator.sound_mood_auto']()}</option>
					{#each moods as md (md)}<option value={md} selected={md === mood}>{md}</option>{/each}
				</select>
			</label>
		</div>
	</div>
	<SignalStrip {inputs} {outputs} {reduction} knobs={columns} {signals} state={stats.state} perf={stats.perf} />
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
		background: var(--ink);
	}
	.chips {
		position: absolute;
		left: var(--space-05);
		top: var(--space-05);
		display: flex;
		gap: 0.25rem;
		z-index: 2;
	}
	.chip {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0 0.375rem;
		color: var(--paper);
		background: color-mix(in srgb, var(--ink) 70%, transparent);
		border: 1px solid color-mix(in srgb, var(--paper) 45%, transparent);
		font-size: 0.75rem;
	}
	.chip span {
		color: color-mix(in srgb, var(--paper) 60%, transparent);
	}
	.chip select {
		all: unset;
		cursor: pointer;
		color: var(--paper);
		max-width: 9rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.chip select option {
		color: var(--ink);
		background: var(--paper);
	}
</style>
