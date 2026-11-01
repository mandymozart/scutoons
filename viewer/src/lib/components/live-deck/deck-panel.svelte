<script lang="ts">
	// The browser deck's controls over its 3D (#198; #188, owner 2026-10-31: "a compact controls panel … much smaller,
	// one thin bar with Start, a collapsed row of the most used knobs, and the rest on demand. The 3D being is the main
	// surface"): one thin bar – Start (the sound needs a tap), the three most used values as small sliders, the fold for
	// the rest (the two pads, every other slider) – and a one-line status under it. Every control turns the deck's one
	// state; the being's 3D buttons turn the same values, so the sliders and dots follow them.
	import { Button, Icon } from '@scutoons/ui';
	import { MELODY, PARAMS, toPos, toValue, type Spec } from '@scutoons/live-deck';
	import { panelLayout } from '$lib/live-deck.ts';
	import DeckPad from './deck-pad.svelte';
	import { m } from '$lib/paraglide/messages.js';

	let {
		values,
		params = PARAMS,
		started,
		starting,
		status,
		alert = '',
		onstart,
		onset,
		ontouch
	}: {
		values: Record<string, number>;
		/** the sliders: the fixed patch's PARAMS or the character's rig's knobs (#191) */
		params?: Record<string, Spec>;
		started: boolean;
		starting: boolean;
		/** the status line: relay, 3D, the sound's dropouts and load, the hovered button */
		status: string;
		/** #208: the sound's dropout warning – flashes red over the status for a few seconds after a dropout */
		alert?: string;
		onstart: () => void;
		onset: (path: string, value: number) => void;
		ontouch: (on: 0 | 1) => void;
	} = $props();

	// folded at first: the being fills the screen; the fold opens the rest
	let open = $state(false);
	// the fixed patch: the voice, the drone, the melody's spread in the bar + two pads; a rig: its gains + the fold
	const layout = $derived(panelLayout(params, MELODY));
	const all = $derived(layout.all);
	const QUICK = $derived(layout.quick);
	const pads = $derived(layout.pads);
	const sliders = $derived(layout.sliders);
	const padLabel = (k: 'voice' | 'incense') => (k === 'voice' ? m['live_deck.pad_voice']() : m['live_deck.pad_incense']());
	const shown = (v: number) => v.toFixed(v >= 10 ? 0 : 2);
</script>

<section class="panel" aria-label={m['live_deck.controls']()}>
	{#if open}
		<div id="deck-controls" class="body">
			{#if pads.length}
				<div class="pads">
					{#each pads as p (p.x)}
						<DeckPad x={p.x} y={p.y} label={padLabel(p.label)} {values} {onset} {ontouch} />
					{/each}
				</div>
			{/if}
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
	{#if alert}<p class="alert" role="alert" data-testid="deck-alert">{alert}</p>{/if}
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
	/* #208: a live tool may not crackle unnoticed – the warning flashes */
	.alert {
		margin: 0;
		padding: 0 var(--space-05);
		font-size: 0.75rem;
		color: var(--paper);
		background: var(--state-error);
		animation: flash 0.5s steps(2, jump-none) infinite;
	}
	@keyframes flash {
		50% {
			background: transparent;
			color: var(--state-error);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.alert {
			animation: none;
		}
	}
</style>
