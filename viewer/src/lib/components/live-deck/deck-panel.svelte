<script lang="ts">
	// The browser deck's controls over its 3D (#198; #188, owner 2026-10-31: "a compact controls panel … much smaller,
	// one thin bar with Start, a collapsed row of the most used knobs, and the rest on demand. The 3D being is the main
	// surface"): one thin bar – Start (the sound needs a tap), the three most used values as small sliders, the fold for
	// the rest (the two pads, every other slider) – and a one-line status under it. Every control turns the deck's one
	// state; the being's 3D buttons turn the same values, so the sliders and dots follow them.
	import { Button, Icon } from '@scutoons/ui';
	import { MELODY, PARAMS, toPos, toValue, type Spec } from '@scutoons/live-deck';
	import DeckPad from './deck-pad.svelte';
	import { m } from '$lib/paraglide/messages.js';

	let {
		values,
		started,
		starting,
		status,
		onstart,
		onset,
		ontouch
	}: {
		values: Record<string, number>;
		started: boolean;
		starting: boolean;
		/** the status line: relay, 3D, the sound's dropouts, the hovered button */
		status: string;
		onstart: () => void;
		onset: (path: string, value: number) => void;
		ontouch: (on: 0 | 1) => void;
	} = $props();

	// folded at first: the being fills the screen; the fold opens the rest
	let open = $state(false);
	const all: Record<string, Spec> = { ...MELODY, ...PARAMS };
	/** the bar's quick row: the voice, the drone, the melody's spread */
	const QUICK = ['voice.gain', 'drone.gain', 'melody.spread'];
	const pads = [
		{ x: 'voice.ratio', y: 'voice.index', label: () => m['live_deck.pad_voice']() },
		{ x: 'incense.cutoff', y: 'sway.rate', label: () => m['live_deck.pad_incense']() }
	];
	const onPads = new Set(pads.flatMap((p) => [p.x, p.y]));
	const sliders = Object.entries(all).filter(([p]) => !onPads.has(p) && !QUICK.includes(p));
	const shown = (v: number) => v.toFixed(v >= 10 ? 0 : 2);
</script>

<section class="panel" aria-label={m['live_deck.controls']()}>
	{#if open}
		<div id="deck-controls" class="body">
			<div class="pads">
				{#each pads as p (p.x)}
					<DeckPad x={p.x} y={p.y} label={p.label()} {values} {onset} {ontouch} />
				{/each}
			</div>
			<fieldset>
				{#each sliders as [path, spec] (path)}
					<label for="deck-{path}">{spec.label}</label>
					<input
						id="deck-{path}"
						type="range"
						min="0"
						max="1"
						step="0.001"
						value={toPos(spec, values[path])}
						oninput={(e) => onset(path, toValue(spec, Number(e.currentTarget.value)))}
					/>
					<output>{shown(values[path])}</output>
				{/each}
			</fieldset>
		</div>
	{/if}
	<div class="bar">
		{#if !started}
			<Button onclick={onstart} disabled={starting} data-testid="deck-start">{starting ? m['live_deck.starting']() : m['live_deck.start']()}</Button>
		{/if}
		<div class="quick">
			{#each QUICK as path (path)}
				<label class="knob" title={all[path].label}>
					<span>{all[path].label}</span>
					<input
						type="range"
						min="0"
						max="1"
						step="0.001"
						value={toPos(all[path], values[path])}
						oninput={(e) => onset(path, toValue(all[path], Number(e.currentTarget.value)))}
					/>
				</label>
			{/each}
		</div>
		<button
			type="button"
			class="fold sc-round sc-quiet"
			aria-expanded={open}
			aria-controls="deck-controls"
			title={m['live_deck.more']()}
			aria-label={m['live_deck.more']()}
			onclick={() => (open = !open)}
		>
			<Icon name={open ? 'chevrons-down' : 'chevrons-up'} />
		</button>
	</div>
	<p class="status" data-testid="deck-status" title={status}>{status}</p>
</section>

<style>
	.panel {
		box-sizing: border-box;
		width: 100%;
		max-width: 26rem;
		margin: 0 auto;
		display: grid;
		gap: var(--space-025);
		padding: var(--space-025) var(--space-05) calc(var(--space-025) + env(safe-area-inset-bottom));
		color: var(--paper);
		pointer-events: auto;
	}
	.bar,
	.body {
		background: color-mix(in srgb, var(--ink) 62%, transparent);
	}
	.bar {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		padding: 0 var(--space-05);
		min-height: var(--control-height);
	}
	.quick {
		flex: 1;
		min-width: 0;
		display: flex;
		gap: var(--space-05);
	}
	.knob {
		flex: 1;
		min-width: 0;
		display: grid;
		line-height: 1;
	}
	.knob span {
		font-size: 0.625rem;
		color: color-mix(in srgb, var(--paper) 65%, transparent);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.knob input {
		height: 1.25rem;
	}
	.fold {
		flex: none;
		display: inline-grid;
		place-items: center;
		width: var(--control-height);
		height: var(--control-height);
		padding: 0;
		border: 0;
		background: none;
		color: inherit;
		cursor: pointer;
	}
	.body {
		display: grid;
		gap: var(--space-075);
		padding: var(--space-05);
		max-height: 45vh;
		overflow-y: auto;
		overscroll-behavior: contain;
	}
	.pads {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 9rem));
		justify-content: center;
		gap: var(--space-1);
	}
	fieldset {
		border: 0;
		padding: 0;
		margin: 0;
		display: grid;
		grid-template-columns: 6.5rem minmax(0, 1fr) 3.5rem;
		gap: var(--space-05);
		align-items: center;
	}
	label {
		font-size: 0.875rem;
	}
	output {
		text-align: right;
		font-size: 0.8125rem;
		font-variant-numeric: tabular-nums;
		color: color-mix(in srgb, var(--paper) 70%, transparent);
	}
	input[type='range'] {
		width: 100%;
		margin: 0;
		accent-color: var(--primary-cyan);
		touch-action: pan-y;
	}
	fieldset input[type='range'] {
		height: var(--control-height);
	}
	.status {
		margin: 0;
		font-size: 0.6875rem;
		color: color-mix(in srgb, var(--paper) 60%, transparent);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		text-shadow: 0 0 0.25rem var(--ink);
	}
</style>
