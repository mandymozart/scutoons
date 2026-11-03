<script lang="ts">
	// The browser deck's controls over its 3D (#198, #188; #209, owner 2026-10-31: "the debug UI uses OSCT's style …
	// sliders become circular knobs, like Ableton's … the collapsed panel shows very basic statistics … using the
	// entire width"): collapsed, one full-width bar – Start (the sound needs a tap), the statistics, the fold; open,
	// every knob as a rotary dial in a dense grid, what the sticks drive now, MIDI learn, the status line. Dense, mono,
	// the shared tokens – OSCT's debug overlay as #209 describes it. Every control turns the deck's one state; the
	// being's 3D buttons, the sticks and MIDI turn the same values, so the dials follow them.
	import { Button, Icon } from '@scutoons/ui';
	import { MELODY, PARAMS, type QualityLevel, type SpaceQuality, type Spec } from '@scutoons/live-deck';
	import { panelKnobs } from '$lib/live-deck.ts';
	import DeckKnob from './deck-knob.svelte';
	import DeckQuality from './deck-quality.svelte';
	import DeckStats, { type Stats } from './deck-stats.svelte';
	import { m } from '$lib/paraglide/messages.js';

	let {
		values,
		params = PARAMS,
		started,
		starting,
		stats,
		status,
		alert = '',
		sticks,
		selected = '',
		learn = false,
		learning = '',
		midi = '',
		mapped,
		onstart,
		onset,
		onpick,
		onlearn,
		ontouch,
		sticksOn = true,
		onsticks,
		quality = null,
		onquality,
		onsettings,
		oncopy
	}: {
		values: Record<string, number>;
		/** the sliders: the fixed patch's PARAMS or the character's rig's knobs (#191) */
		params?: Record<string, Spec>;
		started: boolean;
		starting: boolean;
		stats: Stats;
		/** the detail line: 3D, sync, the hovered button */
		status: string;
		/** #208: the sound's dropout warning – flashes red for a few seconds after a dropout */
		alert?: string;
		/** what the sticks drive now: the selection's name and the two sticks' labels */
		sticks: string;
		/** the knob that owns the sticks (its path) */
		selected?: string;
		/** MIDI learn is on: a tap on a knob arms it */
		learn?: boolean;
		/** the knob MIDI learn waits for */
		learning?: string;
		/** MIDI's state: the inputs, or why there is none */
		midi?: string;
		mapped: (path: string) => string;
		onstart: () => void;
		onset: (path: string, value: number) => void;
		/** a knob tapped: select it (and arm MIDI learn) */
		onpick: (path: string) => void;
		onlearn: () => void;
		ontouch: (on: 0 | 1) => void;
		/** #225: the corner sticks are shown (off frees the knobs under them) */
		sticksOn?: boolean;
		onsticks: () => void;
		/** #220: the 3D's render quality (null = no 3D) and the choice of its level / Auto */
		quality?: SpaceQuality | null;
		onquality: (level: QualityLevel, auto: boolean) => void;
		/** #220: open the presets' settings sheet */
		onsettings: () => void;
		/** #356: copy the sound's measurement (the report); resolves true when it reached the clipboard */
		oncopy: () => Promise<boolean>;
	} = $props();

	let copied = $state(false);
	async function copy() {
		copied = await oncopy();
		if (copied) setTimeout(() => (copied = false), 2000);
	}

	// folded at first: the being fills the screen; the fold opens the knobs
	let open = $state(false);
	// the fixed patch's knobs + its page melody, or the rig's knobs (#191, #236)
	const all = $derived(panelKnobs(params, MELODY));
</script>

<section class="panel" aria-label={m['live_deck.controls']()}>
	{#if open}
		<div id="deck-controls" class="body">
			<div class="head">
				<span class="sel" data-testid="deck-selection"><b>{m['live_deck.selected']()}</b> {sticks}</span>
				<button type="button" class="learn" class:on={learn} aria-pressed={learn} title={m['live_deck.midi_learn_hint']()} onclick={onlearn} data-testid="deck-midi-learn">
					{m['live_deck.midi_learn']()}
				</button>
				<button type="button" class="learn" class:on={sticksOn} aria-pressed={sticksOn} title={m['live_deck.sticks_hint']()} onclick={onsticks} data-testid="deck-sticks-toggle">
					{m['live_deck.sticks']()}
				</button>
				<button type="button" class="learn" class:on={copied} title={m['live_deck.copy_stats_hint']()} onclick={copy} data-testid="deck-copy-stats">
					{copied ? m['live_deck.copied']() : m['live_deck.copy_stats']()}
				</button>
			</div>
			{#if quality}<DeckQuality {quality} {onquality} {onsettings} />{/if}
			{#if learn || midi}<p class="line">{learning ? m['live_deck.midi_waiting']({ knob: (all.find(([p]) => p === learning)?.[1].label ?? learning) }) : midi || m['live_deck.midi_learn_hint']()}</p>{/if}
			<div class="knobs">
				{#each all as [path, spec] (path)}
					<DeckKnob {path} {spec} value={values[path]} mapped={mapped(path)} selected={selected === path} learning={learning === path} {onset} {onpick} {ontouch} />
				{/each}
			</div>
			<p class="line" data-testid="deck-status" title={status}>{status}</p>
		</div>
	{/if}
	{#if alert}<p class="alert" role="alert" data-testid="deck-alert">{alert}</p>{/if}
	<div class="bar">
		{#if !started}
			<Button onclick={onstart} disabled={starting} data-testid="deck-start">{starting ? m['live_deck.starting']() : m['live_deck.start']()}</Button>
		{/if}
		<DeckStats {stats} />
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
</section>

<style>
	.panel {
		box-sizing: border-box;
		width: 100%;
		display: grid;
		/* minmax(0, …): the nowrap lines end in an ellipsis instead of widening the panel */
		grid-template-columns: minmax(0, 1fr);
		color: var(--paper);
		font-family: var(--font-mono);
		pointer-events: auto;
	}
	.bar,
	.body {
		background: color-mix(in srgb, var(--ink) 72%, transparent);
	}
	/* the full width, edge to edge (OSCT's collapsed bar) */
	/* before the sound starts a narrow screen puts the statistics under Start; then they have the whole width */
	.bar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05);
		padding: 0 var(--space-05) env(safe-area-inset-bottom) var(--space-1);
		min-height: var(--control-height);
		border-top: var(--border-width-regular) solid color-mix(in srgb, var(--paper) 15%, transparent);
	}
	.bar > :global(button:not(.fold)) {
		flex: none;
	}
	.bar > :global(.stats) {
		flex: 1 1 16rem;
	}
	.fold {
		flex: none;
		margin-left: auto;
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
		grid-template-columns: minmax(0, 1fr);
		gap: var(--space-025);
		padding: var(--space-05) var(--space-1);
		max-height: 50vh;
		overflow-y: auto;
		overscroll-behavior: contain;
	}
	.head {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		min-height: var(--control-height);
	}
	.sel {
		flex: 1;
		min-width: 0;
		font-size: 0.625rem;
		text-transform: uppercase;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	b {
		font-weight: var(--font-weight-regular);
		color: color-mix(in srgb, var(--paper) 45%, transparent);
	}
	/* a toggle with text: square (DESIGN "Round vs square"), at the control height */
	.learn {
		flex: none;
		height: var(--control-height);
		padding: 0 var(--space-075);
		border: var(--border-width-regular) solid color-mix(in srgb, var(--paper) 35%, transparent);
		border-radius: 0;
		background: none;
		color: inherit;
		font: inherit;
		font-size: 0.625rem;
		text-transform: uppercase;
		cursor: pointer;
	}
	.learn.on {
		border-color: var(--state-warning);
		color: var(--state-warning);
	}
	.knobs {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(3.75rem, 1fr));
		gap: var(--space-025) var(--space-05);
	}
	.line {
		margin: 0;
		font-size: 0.625rem;
		color: color-mix(in srgb, var(--paper) 60%, transparent);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		text-transform: uppercase;
	}
	/* #208: a live tool may not crackle unnoticed – the warning flashes */
	.alert {
		margin: 0;
		padding: 0 var(--space-1);
		font-size: 0.6875rem;
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
