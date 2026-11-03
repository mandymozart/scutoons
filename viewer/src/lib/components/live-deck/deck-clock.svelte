<script lang="ts" module>
	/** the picker's choices: follow the show clock, the deck's own clock, the own clock leading the show, MIDI clock in
	 *  (it leads the show – the venue's table-top clock, owner 2026-11-03) */
	export type ClockChoice = 'show' | 'own' | 'lead' | 'midi';
</script>

<script lang="ts">
	// The deck's tempo clock (#321, plan §3.7): where it comes from – the show clock over the relay, its own, its own as
	// the show's leader, MIDI clock in – the click, the strobe on the beat, and the clock's line in the stage HUD's words
	// (language-neutral, like a meter's). The clock and every rule of it are the core's (live-core clock.rs); this only
	// shows and forwards.
	import type { ClockState } from '@scutoons/live-deck';
	import { m } from '$lib/paraglide/messages.js';

	let {
		clock,
		choice,
		strobe,
		onchoose,
		onclick,
		onstrobe
	}: { clock: ClockState | null; choice: ClockChoice; strobe: boolean; onchoose: (c: ClockChoice) => void; onclick: () => void; onstrobe: () => void } = $props();
	const CHOICES: [ClockChoice, string][] = [
		['show', 'SHOW'],
		['own', 'OWN'],
		['lead', 'LEAD'],
		['midi', 'MIDI']
	];
	/** the clock's line: the tempo, then the source's state (the native HUD's words) */
	const line = $derived.by(() => {
		const c = clock;
		if (!c) return '';
		const bpm = `${c.bpm.toFixed(1)} BPM`;
		if (c.source === 'midi' && c.midiLost) return `NO MIDI CLOCK - HOLDS ${c.bpm.toFixed(0)}${c.leading ? ' LEADS' : ''}`;
		const lead = c.leading ? ` LEADS${c.locked ? '' : ' (SYNCING)'}` : '';
		if (c.source !== 'relay') return `${bpm}${lead}`;
		if (!c.anchor) return `${bpm} NO SHOW CLOCK YET`;
		if (!c.locked) return `${bpm} SHOW CLOCK SYNCING`;
		return `${bpm} SHOW ${c.errorMs >= 0 ? '+' : ''}${c.errorMs.toFixed(1)}MS RTT ${c.rttMs.toFixed(1)}MS`;
	});
</script>

<div class="clock" data-testid="deck-clock">
	<b title={m['live_deck.clock_hint']()}>{m['live_deck.clock']()}</b>
	<span class="seg" role="group" aria-label={m['live_deck.clock']()}>
		{#each CHOICES as [c, label] (c)}
			<button type="button" class:on={choice === c} aria-pressed={choice === c} onclick={() => onchoose(c)} data-testid="deck-clock-{c}">{label}</button>
		{/each}
	</span>
	<button type="button" class="toggle" class:on={clock?.click} aria-pressed={!!clock?.click} onclick={onclick} data-testid="deck-clock-click">{m['live_deck.clock_click']()}</button>
	<button type="button" class="toggle" class:on={strobe} aria-pressed={strobe} onclick={onstrobe} data-testid="deck-clock-strobe">{m['live_deck.clock_strobe']()}</button>
	<span class="now" data-testid="deck-clock-line">{line}</span>
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
