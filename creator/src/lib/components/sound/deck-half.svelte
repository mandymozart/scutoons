<script lang="ts">
	// The deck half of the Sound design workspace (#282): the engine host of #298 (`<EngineStage gate>`) playing the
	// character's rig – the being, the knobs and the sound – with the mood select, the status line and the levels. The sound
	// starts with the round Play button on the stage (or the first click on it; the browser holds the first sound back until a
	// gesture). The compiled module plays here with #290 – until then the muted line says so.
	import { EngineStage, emptySnapshot, meterPercent, statusLine, type EngineHost, type EngineSnapshot } from '@scutoons/engine-host';
	import { API_URL, CDN_SERVER } from '$lib/config.ts';
	import { m } from '$lib/paraglide/messages.js';

	let { character }: { character: string } = $props();

	// svelte-ignore state_referenced_locally
	const options = { api: API_URL, cdn: `${CDN_SERVER}/`, wasmBase: '/live/', character, nick: 'creator', rigs: true } as const;

	let host = $state<EngineHost | null>(null);
	let snap = $state<EngineSnapshot>(emptySnapshot());
	let moods = $state<string[]>(['neutral']);
	let mood = $state('auto');
	let knobs = $state(0);

	$effect(() => {
		if (!host) return;
		const h = host;
		h.hooks = { rig: (played) => (knobs = played?.knobs.length ?? 0) };
		return h.subscribe((s) => (snap = s));
	});

	function pickMood(value: string) {
		mood = value;
		host?.setMood(value === 'auto' ? null : value);
	}

	const meters = $derived([
		['level', snap.level],
		['bass', snap.bass],
		['mid', snap.mid],
		['high', snap.high]
	] as const);
</script>

<div class="deck" data-testid="sound-deck">
	<div class="stage"><EngineStage {options} gate bind:host onmounted={(h) => (moods = h.moods)} /></div>

	<div class="bar">
		<label class="mood">
			<span class="muted">{m['creator.sound_mood']()}</span>
			<select value={mood} onchange={(e) => pickMood(e.currentTarget.value)} data-testid="sound-mood">
				<option value="auto">{m['creator.sound_mood_auto']()}</option>
				{#each moods as md (md)}<option value={md}>{md}</option>{/each}
			</select>
		</label>
		<p class="status" data-testid="sound-status">{statusLine(snap)}</p>
	</div>

	<div class="meters" data-testid="sound-levels">
		{#each meters as [name, v] (name)}
			<div class="meter" title={name}>
				<span class="muted">{name}</span>
				<div class="track"><div class="fill" style:width="{meterPercent(v)}%"></div></div>
			</div>
		{/each}
		<p class="muted fps" data-testid="sound-fps">{snap.fps.toFixed(0)} fps · {knobs} {m['creator.sound_knobs']()}</p>
	</div>

	<p class="muted note">{m['creator.sound_play_note']()}</p>
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
		min-height: 12rem;
		background: var(--ink);
	}
	.bar,
	.meters,
	.note {
		flex: none;
		padding: var(--space-05) var(--space-1);
		border-top: var(--border-width-regular) solid var(--ink);
	}
	.bar {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		flex-wrap: wrap;
	}
	.mood {
		display: flex;
		align-items: center;
		gap: var(--space-05);
	}
	.status {
		margin: 0;
		min-width: 0;
		flex: 1;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}
	.muted {
		color: var(--text-muted);
	}
	.meters {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr)) auto;
		align-items: center;
		gap: var(--space-1);
	}
	.meter {
		display: grid;
		grid-template-columns: auto 1fr;
		align-items: center;
		gap: var(--space-05);
	}
	.track {
		height: 0.5rem;
		background: var(--surface-panel);
	}
	.fill {
		height: 100%;
		background: var(--ink);
	}
	.fps,
	.note {
		margin: 0;
		font-variant-numeric: tabular-nums;
	}
	.note {
		padding-top: var(--space-05);
	}
	@media (max-width: 56rem) {
		.meters {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}
</style>
