<script lang="ts">
	// The deck's tempo clock (#321, plan §3.7): where it comes from – deck-core's `ClockMode`: the show clock over the relay,
	// its own, its own as the show's leader, MIDI clock in (it leads the show – the venue's table-top clock) – the click,
	// the strobe on the beat, the MIDI clock out, and the clock's lines in the stage HUD's words (language-neutral, like a
	// meter's). The clock, the choice it is on and every rule of it are the core's (#370: the clock frame says them); this
	// only shows and forwards.
	import type { ClockState } from '@scutoons/live-deck';
	import { m } from '$lib/paraglide/messages.js';

	let {
		clock,
		modes,
		strobe,
		onchoose,
		onclick,
		onstrobe,
		onmidiout
	}: {
		clock: ClockState | null;
		/** the picker's choices – deck-core's `ClockMode::ALL` names; `clock.mode` is the index of the one it is on */
		modes: string[];
		strobe: boolean;
		onchoose: (mode: number) => void;
		onclick: () => void;
		onstrobe: () => void;
		/** MIDI clock out to every Web MIDI output (the core's schedule; none while on MIDI clock in) */
		onmidiout: () => void;
	} = $props();
</script>

<div class="clock" data-testid="deck-clock">
	<b title={m['live_deck.clock_hint']()}>{m['live_deck.clock']()}</b>
	<span class="seg" role="group" aria-label={m['live_deck.clock']()}>
		{#each modes as c, i (c)}
			<button type="button" class:on={clock?.mode === i} aria-pressed={clock?.mode === i} onclick={() => onchoose(i)} data-testid="deck-clock-{c}">{c}</button>
		{/each}
	</span>
	<button type="button" class="toggle" class:on={clock?.click} aria-pressed={!!clock?.click} onclick={onclick} data-testid="deck-clock-click">{m['live_deck.clock_click']()}</button>
	<button type="button" class="toggle" class:on={strobe} aria-pressed={strobe} onclick={onstrobe} data-testid="deck-clock-strobe">{m['live_deck.clock_strobe']()}</button>
	<button type="button" class="toggle" class:on={clock?.midiOut} aria-pressed={!!clock?.midiOut} onclick={onmidiout} title={m['live_deck.clock_midi_out_hint']()} data-testid="deck-clock-midi-out">{m['live_deck.clock_midi_out']()}</button>
	<span class="now" data-testid="deck-clock-line">{clock?.line ?? ''}</span>
</div>

<style>
	.clock {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		min-height: var(--control-height);
		font-size: 0.625rem;
		text-transform: uppercase;
		white-space: nowrap;
		overflow: hidden;
	}
	b {
		font-weight: var(--font-weight-regular);
		color: color-mix(in srgb, var(--paper) 45%, transparent);
	}
	/* grouped and text controls: square (DESIGN "Round vs square"), at the control height – the panel's toggles */
	button {
		flex: none;
		height: var(--control-height);
		padding: 0 var(--space-05);
		border: var(--border-width-regular) solid color-mix(in srgb, var(--paper) 35%, transparent);
		border-radius: 0;
		background: none;
		color: inherit;
		font: inherit;
		text-transform: uppercase;
		cursor: pointer;
	}
	button.on {
		background: var(--paper);
		color: var(--ink);
	}
	.seg {
		display: inline-flex;
	}
	.seg button + button {
		border-left: none;
	}
	.now {
		overflow: hidden;
		text-overflow: ellipsis;
		color: color-mix(in srgb, var(--paper) 70%, transparent);
	}
</style>
