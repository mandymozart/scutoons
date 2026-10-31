<script lang="ts">
	// The browser deck's controls over its 3D (#198): a collapsible panel at the bottom – Start (the sound needs a tap),
	// the two pads, the sliders. Every control turns the deck's one state; the being's 3D buttons turn the same values,
	// so the sliders and dots follow them.
	import { Button, Icon } from '@scutoons/ui';
	import { MELODY, PARAMS, toPos, toValue } from '@scutoons/live-deck';
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
		/** the status line: relay, 3D, the hovered button */
		status: string;
		onstart: () => void;
		onset: (path: string, value: number) => void;
		ontouch: (on: 0 | 1) => void;
	} = $props();

	// folded at first: the being fills the screen; a tap opens the controls
	let open = $state(false);
	const pads = [
		{ x: 'voice.ratio', y: 'voice.index', label: () => m['live_deck.pad_voice']() },
		{ x: 'incense.cutoff', y: 'sway.rate', label: () => m['live_deck.pad_incense']() }
	];
	const onPads = new Set(pads.flatMap((p) => [p.x, p.y]));
	const sliders = Object.entries({ ...MELODY, ...PARAMS }).filter(([p]) => !onPads.has(p));
	const shown = (v: number) => v.toFixed(v >= 10 ? 0 : 2);
</script>

<section class="panel" class:open aria-label={m['live_deck.controls']()}>
	<div class="bar">
		<button type="button" class="fold sc-quiet" aria-expanded={open} aria-controls="deck-controls" onclick={() => (open = !open)}>
			<Icon name={open ? 'chevrons-down' : 'chevrons-up'} />
			<span>{m['live_deck.controls']()}</span>
		</button>
		{#if !started}
			<Button onclick={onstart} disabled={starting} data-testid="deck-start">{starting ? m['live_deck.starting']() : m['live_deck.start']()}</Button>
		{/if}
	</div>
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
	<p class="status" data-testid="deck-status">{status}</p>
</section>

<style>
	.panel {
		box-sizing: border-box;
		width: 100%;
		max-width: 32rem;
		margin: 0 auto;
		display: grid;
		gap: var(--space-05);
		padding: var(--space-05) var(--space-1) calc(var(--space-05) + env(safe-area-inset-bottom));
		background: color-mix(in srgb, var(--ink) 72%, transparent);
		color: var(--paper);
		pointer-events: auto;
	}
	.bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-05);
	}
	.fold {
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
		height: var(--control-height);
		padding: 0;
		border: 0;
		background: none;
		color: inherit;
		font: inherit;
		cursor: pointer;
	}
	.body {
		display: grid;
		gap: var(--space-075);
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
		height: var(--control-height);
		margin: 0;
		accent-color: var(--primary-cyan);
		touch-action: pan-y;
	}
	.status {
		margin: 0;
		font-size: 0.75rem;
		color: color-mix(in srgb, var(--paper) 60%, transparent);
		overflow-wrap: anywhere;
	}
</style>
