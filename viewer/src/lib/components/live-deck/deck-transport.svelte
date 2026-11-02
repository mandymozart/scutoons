<script lang="ts">
	// The strip on top of /live/deck for a creator (#282, owner 2026-11-02: "tabs of the page itself … a transport strip
	// stays on top in every tab"): Start / Stop, a level meter, the tempo, the character chip and the moods, the frame's
	// statistics at the right – so the sound can be started, stopped and its mood switched while a tab other than Deck is
	// open. It only shows and sets what the page already has (the Deck object's level, mood and the audio context); the
	// engine keeps running whatever tab is open. Dense mono like deck-panel.svelte; on a phone one line: the button, a
	// small meter and a single "Name · mood ▾" chip.
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import CharacterChip from './character-chip.svelte';
	import type { Stats } from './deck-stats.svelte';

	type Character = { id: string; name?: Record<string, string> };

	let {
		started,
		starting,
		running,
		level,
		bpm,
		stats,
		character,
		characters,
		face,
		mood,
		moods,
		pinned,
		onplay,
		onpick,
		onmood,
		editing,
		onedit
	}: {
		started: boolean;
		starting: boolean;
		/** the audio context is running (false after Stop) */
		running: boolean;
		/** the master level 0..1 */
		level: number;
		bpm: number;
		stats: Stats;
		character: string;
		characters: Character[];
		face: string;
		/** the mood now (the ladder's, or the one held) */
		mood: string;
		/** the character's ladder */
		moods: string[];
		/** the mood held by a chip ('' = the ladder's own) */
		pinned: string;
		/** Start the sound / Stop it / Start it again */
		onplay: () => void;
		onpick: (id: string) => void;
		/** hold this mood ('' = let the ladder decide) */
		onmood: (mood: string) => void;
		/** edit (rehearsal) mode is on – the tab row and the designer (#282) */
		editing: boolean;
		onedit: (on: boolean) => void;
	} = $props();

	const playing = $derived(started && running);
	const label = $derived(starting ? m['live_deck.starting']() : playing ? m['live_deck.transport_stop']() : m['live_deck.transport_start']());
	const fpsLevel = $derived(stats.fps === 0 ? '' : stats.fps < 30 ? 'bad' : stats.fps < 50 ? 'warn' : '');
</script>

<div class="transport" data-testid="deck-transport">
	<button
		type="button"
		class="play"
		class:on={playing}
		disabled={starting}
		title={playing ? m['live_deck.transport_stop_hint']() : m['live_deck.transport_start_hint']()}
		aria-label={label}
		onclick={onplay}
		data-testid="deck-play"
	>
		<Icon name={playing ? 'square' : 'play'} size="1em" /><span class="word">{label}</span>
	</button>
	<div class="meter" role="meter" aria-label={m['live_deck.level']()} aria-valuemin="0" aria-valuemax="1" aria-valuenow={level} title={m['live_deck.level']()} data-testid="deck-meter">
		<i style:width="{Math.round(Math.min(1, level * 1.3) * 100)}%" class:hot={level > 0.85}></i>
	</div>
	<span class="bpm" title={m['live_deck.tempo']()}><b>{bpm}</b> BPM</span>
	{#if character}
		<CharacterChip current={character} {characters} {face} {mood} onpick={onpick} {moods} {pinned} {onmood} />
	{/if}
	<div class="moods" role="group" aria-label={m['live_deck.mood']()}>
		{#each moods as md (md)}
			<button type="button" class="mood" class:on={mood === md} class:held={pinned === md} aria-pressed={mood === md} title={pinned === md ? m['live_deck.mood_release']() : m['live_deck.mood_hold']({ mood: md })} onclick={() => onmood(pinned === md ? '' : md)}>
				{md}
			</button>
		{/each}
	</div>
	<p class="stats" data-testid="deck-transport-stats">
		<span class={fpsLevel}><b>FPS</b> {stats.fps ? stats.fps.toFixed(0) : '–'}</span>
		<span><b>CPU</b> {stats.cpu.toFixed(1)}/{stats.peak.toFixed(1)}MS</span>
	</p>
	<button type="button" class="edit" class:on={editing} aria-pressed={editing} title={m['live_deck.edit_hint']()} onclick={() => onedit(!editing)} data-testid="deck-edit">
		{m['live_deck.edit']()}
	</button>
</div>

<style>
	.transport {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		min-width: 0;
		padding: 0 calc(var(--control-height) + var(--space-1)) 0 var(--space-05);
		background: var(--ink);
		color: var(--paper);
		font-family: var(--font-mono);
		font-size: 0.625rem;
		text-transform: uppercase;
	}
	.play {
		flex: none;
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
		height: var(--control-height);
		padding: 0 var(--space-075);
		border: var(--border-width-regular) solid color-mix(in srgb, var(--paper) 35%, transparent);
		border-radius: 0;
		background: none;
		color: inherit;
		font: inherit;
		cursor: pointer;
	}
	/* playing: inverted, the live state (DESIGN "Selection") */
	.play.on {
		background: var(--paper);
		color: var(--ink);
		border-color: var(--paper);
	}
	.play:disabled {
		opacity: 0.5;
	}
	.meter {
		flex: 0 0 6rem;
		height: 0.75rem;
		border: 1px solid color-mix(in srgb, var(--paper) 35%, transparent);
	}
	.meter i {
		display: block;
		height: 100%;
		background: var(--paper);
	}
	.meter i.hot {
		background: var(--state-warning);
	}
	.bpm {
		flex: none;
		color: color-mix(in srgb, var(--paper) 60%, transparent);
		font-variant-numeric: tabular-nums;
	}
	.bpm b {
		font-weight: var(--font-weight-regular);
		color: var(--paper);
	}
	.moods {
		display: flex;
		min-width: 0;
		overflow-x: auto;
		scrollbar-width: none;
	}
	.mood {
		flex: none;
		height: var(--control-height);
		padding: 0 var(--space-075);
		border: var(--border-width-regular) solid color-mix(in srgb, var(--paper) 35%, transparent);
		margin-left: -2px;
		border-radius: 0;
		background: none;
		color: inherit;
		font: inherit;
		cursor: pointer;
	}
	/* the mood now: inverted; one a chip holds: an amber frame */
	.mood.on {
		background: var(--paper);
		color: var(--ink);
		border-color: var(--paper);
	}
	.mood.held {
		border-color: var(--state-warning);
	}
	/* a toggle with text: square, at the control height; on = inverted (DESIGN "Selection") */
	.edit {
		flex: none;
		height: var(--control-height);
		padding: 0 var(--space-075);
		border: var(--border-width-regular) solid color-mix(in srgb, var(--paper) 35%, transparent);
		border-radius: 0;
		background: none;
		color: inherit;
		font: inherit;
		cursor: pointer;
	}
	.edit.on {
		background: var(--paper);
		color: var(--ink);
		border-color: var(--paper);
	}
	.stats {
		flex: none;
		display: flex;
		gap: var(--space-075);
		margin: 0 0 0 auto;
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
		color: color-mix(in srgb, var(--paper) 85%, transparent);
	}
	.stats b {
		font-weight: var(--font-weight-regular);
		color: color-mix(in srgb, var(--paper) 45%, transparent);
	}
	.warn {
		color: var(--state-warning);
	}
	.bad {
		color: var(--state-error);
	}
	/* a phone: one line – the button (icon only, round), a small meter, the one chip */
	@media (max-width: 767px) {
		.word,
		.bpm,
		.moods,
		.stats {
			display: none;
		}
		.edit {
			margin-left: auto;
		}
		.play {
			width: var(--control-height);
			justify-content: center;
			padding: 0;
			border-radius: 50%;
		}
		.meter {
			flex: 0 0 3rem;
		}
	}
</style>
