<script lang="ts">
	import { formatTime } from './playlist.ts';

	/**
	 * The record player's seek line (owner 2026-10-16, after res.radio's player: the time left, the line, the length right).
	 * Unlike res.radio's it seeks on `change` – a pointer released anywhere (also outside the line) and every arrow key
	 * seek; while dragging only the picture moves (no request per pixel for a vault file). The length comes from the
	 * file once known (`songLength`), not from guessed metadata. House look of the player overlay: a 2px line, the played
	 * part ink, a round thumb.
	 */
	let { time, length, label, onseek, testid }: { time: number; length: number; label: string; onseek: (seconds: number) => void; testid?: string } = $props();

	let dragging = $state(false);
	let value = $state(0);
	const shown = $derived(dragging ? value : Math.min(time, length || time));
</script>

<div class="seek-bar">
	<span class="time" data-testid={testid ? `${testid}-time` : undefined}>{formatTime(shown)}</span>
	<input
		class="seek"
		type="range"
		min="0"
		max={length || 0}
		step="any"
		value={shown}
		disabled={!length}
		style:--progress="{length > 0 ? (shown / length) * 100 : 0}%"
		aria-label={label}
		aria-valuetext="{formatTime(shown)} / {formatTime(length)}"
		oninput={(e) => {
			value = Number(e.currentTarget.value);
			dragging = true;
		}}
		onchange={(e) => {
			dragging = false;
			onseek(Number(e.currentTarget.value));
		}}
		onpointercancel={() => (dragging = false)}
		onblur={() => (dragging = false)}
		data-testid={testid}
	/>
	<span class="time">{formatTime(length)}</span>
</div>

<style>
	.seek-bar {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		min-width: 0;
	}

	.time {
		flex: none;
		font-size: 0.75rem;
		font-variant-numeric: tabular-nums;
	}

	.seek {
		-webkit-appearance: none;
		appearance: none;
		flex: 1;
		min-width: 0;
		height: 1rem;
		margin: 0;
		padding: 0;
		border: 0;
		border-radius: 0;
		background: transparent;
		cursor: pointer;
	}

	.seek:disabled {
		cursor: default;
	}

	.seek::-webkit-slider-runnable-track {
		height: var(--border-width-regular);
		background: linear-gradient(to right, var(--ink) var(--progress), var(--border-muted) var(--progress));
	}

	.seek::-moz-range-track {
		height: var(--border-width-regular);
		background: linear-gradient(to right, var(--ink) var(--progress), var(--border-muted) var(--progress));
	}

	.seek::-webkit-slider-thumb {
		-webkit-appearance: none;
		width: 0.75rem;
		height: 0.75rem;
		margin-top: calc(var(--border-width-regular) / 2 - 0.375rem);
		border: 0;
		border-radius: 50%;
		background: var(--ink);
	}

	.seek::-moz-range-thumb {
		width: 0.75rem;
		height: 0.75rem;
		border: 0;
		border-radius: 50%;
		background: var(--ink);
	}

	.seek:disabled::-webkit-slider-thumb {
		visibility: hidden;
	}

	.seek:disabled::-moz-range-thumb {
		visibility: hidden;
	}

	.seek:focus-visible {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: 0.125rem;
	}
</style>
