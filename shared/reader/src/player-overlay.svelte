<script lang="ts" module>
	/** the overlay's texts (the app's language); English by default */
	export interface PlayerLabels {
		play: string;
		pause: string;
		replay: string;
		mute: string;
		unmute: string;
		/** the button over a player that autoplayed muted because the browser blocked sound */
		soundOn: string;
		fullscreen: string;
		exitFullscreen: string;
		seek: string;
		skip: string;
	}

	export const DEFAULT_PLAYER_LABELS: PlayerLabels = {
		play: 'Play',
		pause: 'Pause',
		replay: 'Watch again',
		mute: 'Mute',
		unmute: 'Unmute',
		soundOn: 'Sound on',
		fullscreen: 'Full screen',
		exitFullscreen: 'Exit full screen',
		seek: 'Position',
		skip: 'Skip'
	};
</script>

<script lang="ts">
	import { onDestroy, untrack } from 'svelte';
	import { SLICE_SKIP_EVENT } from './types.ts';

	/**
	 * The player video's own controls over the video (agents/DESIGN.md "Player video slice", owner 2026-10-04: like the
	 * Netflix / Prime players, instead of the browser's controls).
	 * - not started yet or paused: a big round ▶ in the middle; ended: ↺ (the video again from the start);
	 * - `full` (`SliceVideo.controls`, default on): a bar at the bottom – a seek line, play/pause + the time, mute,
	 *   full screen – that fades after `IDLE_MS` while it plays (and the mouse cursor with it); a mouse move, a tap, a key
	 *   or the keyboard focus brings it back. A mouse click on the video plays / pauses, a tap shows / hides the bar.
	 *   `skippable`: "Überspringen" bottom right while it runs, fading with the bar (a bubbling `sliceskip` event; the app decides where to go).
	 * - not `full`: only ▶ / ↺, a click or tap plays / pauses.
	 * - muted because the browser blocked sound (`soundBlocked`): a "Ton an" button that stays while the bar fades.
	 * Keys while the focus is in it: Space / K play-pause, M mute, F full screen, ← / → 5 s.
	 */
	let {
		video,
		box,
		full = true,
		skippable = false,
		labels,
		soundBlocked = $bindable(false)
	}: {
		video: HTMLVideoElement | null;
		/** the element that goes full screen (the slice's box, so the controls come along) */
		box: HTMLElement | null;
		/** the whole bar (`SliceVideo.controls`); false = only ▶ / ↺ */
		full?: boolean;
		/** show "Überspringen" (the reader; the app handles the `sliceskip` event) */
		skippable?: boolean;
		labels: PlayerLabels;
		soundBlocked?: boolean;
	} = $props();

	const IDLE_MS = 3000;
	const SEEK_STEP = 5;

	let paused = $state(true);
	let ended = $state(false);
	let muted = $state(false);
	/** played once (the first ▶ is the poster's; afterwards a pause shows the bar too) */
	let started = $state(false);
	let time = $state(0);
	let duration = $state(0);
	/** the reader is dragging the seek line: its position wins over the video's */
	let scrubbing = $state(false);
	/** something happened lately (a move, a tap, a key) */
	let active = $state(true);
	/** the keyboard focus is in the overlay (a focused control must stay visible) */
	let keyboardFocus = $state(false);
	let fullscreen = $state(false);
	let timer: ReturnType<typeof setTimeout> | undefined;

	const playing = $derived(!paused && !ended);
	const bigButton = $derived<'play' | 'replay' | null>(ended ? 'replay' : !started || paused ? 'play' : null);
	/** the bar is visible */
	const chrome = $derived(full && started && !ended && (paused || active || keyboardFocus || scrubbing));
	const canFullscreen =
		typeof document !== 'undefined' &&
		(document.fullscreenEnabled || (typeof HTMLVideoElement !== 'undefined' && 'webkitEnterFullscreen' in HTMLVideoElement.prototype));

	$effect(() => {
		const el = video;
		if (!el) return;
		const sync = () => {
			paused = el.paused;
			ended = el.ended;
			muted = el.muted;
			duration = Number.isFinite(el.duration) ? el.duration : 0;
			if (!scrubbing) time = el.currentTime;
			if (!el.paused || el.currentTime > 0) started = true;
		};
		const events = ['play', 'pause', 'ended', 'timeupdate', 'durationchange', 'loadedmetadata', 'volumechange', 'seeked', 'emptied'];
		for (const name of events) el.addEventListener(name, sync);
		sync();
		return () => {
			for (const name of events) el.removeEventListener(name, sync);
		};
	});

	// the end leaves full screen: what comes next (the app's follow-up bar, its countdown) is outside the box, and the next
	// chapter must not start (its music) behind a full-screen video
	$effect(() => {
		const el = video as (HTMLVideoElement & { webkitDisplayingFullscreen?: boolean; webkitExitFullscreen?: () => void }) | null;
		if (!el) return;
		const onEnded = () => {
			if (box && document.fullscreenElement === box) document.exitFullscreen().catch(() => {});
			else if (el.webkitDisplayingFullscreen) el.webkitExitFullscreen?.();
		};
		el.addEventListener('ended', onEnded);
		return () => el.removeEventListener('ended', onEnded);
	});

	$effect(() => {
		const onChange = () => (fullscreen = !!box && document.fullscreenElement === box);
		document.addEventListener('fullscreenchange', onChange);
		return () => document.removeEventListener('fullscreenchange', onChange);
	});

	// (re)started: the bar shows, then fades like after any other activity
	$effect(() => {
		if (playing) untrack(wake);
	});

	/** something happened: show the bar, fade it again after `IDLE_MS` */
	function wake() {
		active = true;
		clearTimeout(timer);
		timer = setTimeout(() => (active = false), IDLE_MS);
	}
	onDestroy(() => clearTimeout(timer));

	function play() {
		const el = video;
		if (!el) return;
		el.play().catch(() => {});
		// a click is a user gesture: the "Ton an" button stays only while it is still muted
		soundBlocked = soundBlocked && el.muted;
	}

	function togglePlay() {
		const el = video;
		if (!el) return;
		if (el.paused || el.ended) play();
		else el.pause();
		wake();
	}

	function replay() {
		const el = video;
		if (!el) return;
		el.currentTime = 0;
		play();
		wake();
	}

	function toggleMute() {
		const el = video;
		if (!el) return;
		el.muted = !el.muted;
		if (!el.muted) soundBlocked = false;
		wake();
	}

	function soundOn() {
		const el = video;
		soundBlocked = false;
		if (!el) return;
		// this click is a user gesture: unmuting keeps it playing
		el.muted = false;
		if (el.paused) el.play().catch(() => {});
		wake();
	}

	function toggleFullscreen() {
		const el = video;
		if (document.fullscreenElement) document.exitFullscreen().catch(() => {});
		else if (document.fullscreenEnabled && box) box.requestFullscreen().catch(() => {});
		// iPhone: only the video itself goes full screen (the system player)
		else (el as (HTMLVideoElement & { webkitEnterFullscreen?: () => void }) | null)?.webkitEnterFullscreen?.();
		wake();
	}

	function skip() {
		const el = video;
		if (!el) return;
		el.pause();
		el.dispatchEvent(new CustomEvent(SLICE_SKIP_EVENT, { bubbles: true }));
	}

	function seek(to: number) {
		const el = video;
		if (!el || !(duration > 0)) return;
		time = Math.max(0, Math.min(duration, to));
		el.currentTime = time;
	}

	/** a mouse click plays / pauses; a tap (touch, pen) shows / hides the bar – except before the start and without the bar */
	let pointerType = 'mouse';
	function onSurfaceClick() {
		if (ended) return;
		if (!full || !started || pointerType === 'mouse') {
			togglePlay();
			return;
		}
		if (chrome && playing) {
			clearTimeout(timer);
			active = false;
		} else wake();
	}

	function onKeydown(event: KeyboardEvent) {
		const target = event.target as HTMLElement;
		const onButton = target.tagName === 'BUTTON';
		const onSeek = target.tagName === 'INPUT';
		const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
		if ((key === ' ' || key === 'Enter') && onButton) return;
		if ((key === 'ArrowLeft' || key === 'ArrowRight') && onSeek) return wake();
		if (key === ' ' || key === 'k' || (key === 'Enter' && !onSeek)) {
			if (ended) replay();
			else togglePlay();
		} else if (key === 'm') toggleMute();
		else if (key === 'f' && full && canFullscreen) toggleFullscreen();
		else if (key === 'ArrowLeft' && full) seek(time - SEEK_STEP);
		else if (key === 'ArrowRight' && full) seek(time + SEEK_STEP);
		else return;
		event.preventDefault();
		wake();
	}

	function onFocusIn(event: FocusEvent) {
		keyboardFocus = (event.target as HTMLElement).matches(':focus-visible');
		if (keyboardFocus) wake();
	}

	/** 95 → "1:35", 3725 → "1:02:05" */
	function clock(seconds: number) {
		const s = Math.max(0, Math.floor(seconds));
		const h = Math.floor(s / 3600);
		const m = Math.floor((s % 3600) / 60);
		const rest = String(s % 60).padStart(2, '0');
		return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${rest}` : `${m}:${rest}`;
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="overlay"
	class:dimmed={bigButton !== null}
	class:idle={full && playing && !chrome}
	onpointermove={(event) => {
		if (event.pointerType === 'mouse') wake();
	}}
	onkeydown={onKeydown}
	onfocusin={onFocusIn}
	onfocusout={() => (keyboardFocus = false)}
	data-testid="player-overlay"
	data-state={ended ? 'ended' : playing ? 'playing' : started ? 'paused' : 'start'}
>
	<!-- its keys (Space / Enter / K …) are handled by the overlay's keydown -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		class="surface"
		role="button"
		tabindex="0"
		aria-label={playing ? labels.pause : labels.play}
		onpointerdown={(event) => (pointerType = event.pointerType)}
		onclick={onSurfaceClick}
	></div>

	{#if bigButton === 'play'}
		<button type="button" class="big" onclick={play} aria-label={labels.play} title={labels.play} data-testid="player-play">
			<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 2l10 6-10 6z" fill="currentColor" /></svg>
		</button>
	{:else if bigButton === 'replay'}
		<button type="button" class="big" onclick={replay} aria-label={labels.replay} title={labels.replay} data-testid="player-replay">
			<svg viewBox="0 0 16 16" aria-hidden="true">
				<path d="M2.5 8a5.5 5.5 0 1 0 1.6-3.9" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="square" />
				<path d="M3.5 1.5v3h3" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="square" />
			</svg>
		</button>
	{/if}

	{#if full && skippable && started && !ended}
		<button type="button" class="square skip fades" class:hidden={!chrome} onclick={skip} data-testid="player-skip">
			{labels.skip}
			<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M2 2l6 6-6 6zM9 2l6 6-6 6z" fill="currentColor" /></svg>
		</button>
	{/if}

	{#snippet soundOnButton()}
		<button type="button" class="square sound-on" onclick={soundOn} data-testid="player-sound-on">
			<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
				<path d="M1 6h3l4-3v10l-4-3H1z" fill="currentColor" />
				<path d="M10.5 5.5c1 .8 1.5 1.6 1.5 2.5s-.5 1.7-1.5 2.5M12.5 3.5c1.5 1.2 2.2 2.7 2.2 4.5s-.7 3.3-2.2 4.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="square" />
			</svg>
			{labels.soundOn}
		</button>
	{/snippet}

	{#if full && started && !ended}
		<div class="bar">
			<input
				class="seek fades"
				class:hidden={!chrome}
				type="range"
				min="0"
				max={duration || 0}
				step="any"
				value={time}
				style:--progress="{duration > 0 ? (time / duration) * 100 : 0}%"
				aria-label={labels.seek}
				aria-valuetext="{clock(time)} / {clock(duration)}"
				oninput={(event) => {
					scrubbing = true;
					seek(Number(event.currentTarget.value));
					wake();
				}}
				onchange={() => (scrubbing = false)}
				onpointerup={() => (scrubbing = false)}
				data-testid="player-seek"
			/>
			<div class="row">
				<button
					type="button"
					class="round fades"
					class:hidden={!chrome}
					onclick={togglePlay}
					aria-label={playing ? labels.pause : labels.play}
					title={playing ? labels.pause : labels.play}
					data-testid="player-toggle"
				>
					{#if playing}
						<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M3 2h3.5v12H3zM9.5 2H13v12H9.5z" fill="currentColor" /></svg>
					{:else}
						<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M4 2l10 6-10 6z" fill="currentColor" /></svg>
					{/if}
				</button>
				<span class="time fades" class:hidden={!chrome} data-testid="player-time">{clock(time)} / {clock(duration)}</span>
				<span class="grow"></span>
				{#if soundBlocked}
					{@render soundOnButton()}
				{:else}
					<button
						type="button"
						class="round fades"
						class:hidden={!chrome}
						onclick={toggleMute}
						aria-label={muted ? labels.unmute : labels.mute}
						aria-pressed={muted}
						title={muted ? labels.unmute : labels.mute}
						data-testid="player-mute"
					>
						{#if muted}
							<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
								<path d="M1 6h3l4-3v10l-4-3H1z" fill="currentColor" />
								<path d="M10.5 5.5l5 5M15.5 5.5l-5 5" stroke="currentColor" stroke-width="1.5" stroke-linecap="square" />
							</svg>
						{:else}
							<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
								<path d="M1 6h3l4-3v10l-4-3H1z" fill="currentColor" />
								<path d="M10.5 5.5c1 .8 1.5 1.6 1.5 2.5s-.5 1.7-1.5 2.5M12.5 3.5c1.5 1.2 2.2 2.7 2.2 4.5s-.7 3.3-2.2 4.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="square" />
							</svg>
						{/if}
					</button>
				{/if}
				{#if canFullscreen}
					<button
						type="button"
						class="round fades"
						class:hidden={!chrome}
						onclick={toggleFullscreen}
						aria-label={fullscreen ? labels.exitFullscreen : labels.fullscreen}
						title={fullscreen ? labels.exitFullscreen : labels.fullscreen}
						data-testid="player-fullscreen"
					>
						{#if fullscreen}
							<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M5 1v4H1M11 1v4h4M15 11h-4v4M1 11h4v4" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="square" /></svg>
						{:else}
							<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M1 5V1h4M11 1h4v4M15 11v4h-4M5 15H1v-4" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="square" /></svg>
						{/if}
					</button>
				{/if}
			</div>
		</div>
	{:else if soundBlocked && !ended}
		<div class="corner">{@render soundOnButton()}</div>
	{/if}
</div>

<style>
	/* the house look on the video: ink buttons with a 2px paper outline, IBM Plex Mono, no gradients */
	.overlay {
		position: absolute;
		inset: 0;
		z-index: 1;
		color: var(--paper, #fff);
		font-family: var(--font-mono, 'IBM Plex Mono', monospace);
		transition: background-color 0.2s;
	}
	/* not playing (▶ / ↺): a flat scrim so the button reads on any frame */
	.overlay.dimmed {
		background: rgb(0 0 0 / 0.35);
	}
	.surface {
		position: absolute;
		inset: 0;
		cursor: pointer;
		outline-offset: -4px;
	}
	.idle .surface {
		cursor: none;
	}
	button {
		margin: 0;
		font: inherit;
		cursor: pointer;
		touch-action: manipulation;
	}
	.big,
	.round {
		display: grid;
		place-items: center;
		box-sizing: border-box;
		padding: 0;
		border: 2px solid var(--paper, #fff);
		border-radius: 50%;
		background: var(--ink, #000);
		color: var(--paper, #fff);
	}
	.big {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		width: 4.5rem;
		height: 4.5rem;
	}
	.big svg {
		width: 1.75rem;
		height: 1.75rem;
	}
	.round {
		flex: none;
		width: var(--control-height, 2.25rem);
		height: var(--control-height, 2.25rem);
	}
	.big:hover,
	.big:focus-visible,
	.round:hover,
	.round:focus-visible,
	.square:hover,
	.square:focus-visible {
		background: var(--paper, #fff);
		color: var(--ink, #000);
	}
	/* text buttons ("Überspringen", "Ton an"): the primary style, square */
	.square {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		box-sizing: border-box;
		min-height: var(--control-height, 2.25rem);
		padding: 0 0.75rem;
		border: 2px solid var(--paper, #fff);
		border-radius: 0;
		background: var(--ink, #000);
		color: var(--paper, #fff);
		font-size: 0.75rem;
		line-height: 1;
		white-space: nowrap;
	}
	.bar {
		position: absolute;
		left: 0;
		right: 0;
		bottom: 0;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		padding: 0 1rem 1rem;
		pointer-events: none;
	}
	.bar > *,
	.row > * {
		pointer-events: auto;
	}
	.row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		pointer-events: none;
	}
	.grow {
		flex: 1;
	}
	.time {
		margin-left: 0.25rem;
		font-size: 0.75rem;
		font-variant-numeric: tabular-nums;
		/* readable on bright frames without a scrim */
		text-shadow: 0 0 0.25rem rgb(0 0 0 / 0.6);
		pointer-events: none;
	}
	/* the bar's parts and "Überspringen" fade while it plays; "Ton an" stays */
	.fades {
		transition: opacity 0.3s;
	}
	.fades.hidden {
		opacity: 0;
		pointer-events: none;
	}
	.skip {
		position: absolute;
		right: 1rem;
		/* above the bar: its row + the seek line + the gaps */
		bottom: calc(var(--control-height, 2.25rem) + 3rem);
	}
	.corner {
		position: absolute;
		right: 1rem;
		bottom: 1rem;
	}

	/* the seek line: 2px, the watched part in paper, a round thumb */
	.seek {
		-webkit-appearance: none;
		appearance: none;
		display: block;
		width: 100%;
		height: 1rem;
		margin: 0;
		padding: 0;
		border: 0;
		border-radius: 0;
		box-shadow: none;
		background: transparent;
		cursor: pointer;
	}
	.seek::-webkit-slider-runnable-track {
		height: 2px;
		background: linear-gradient(to right, var(--paper, #fff) var(--progress), rgb(255 255 255 / 0.4) var(--progress));
	}
	.seek::-moz-range-track {
		height: 2px;
		background: linear-gradient(to right, var(--paper, #fff) var(--progress), rgb(255 255 255 / 0.4) var(--progress));
	}
	.seek::-webkit-slider-thumb {
		-webkit-appearance: none;
		width: 0.75rem;
		height: 0.75rem;
		margin-top: calc(1px - 0.375rem);
		border: 0;
		border-radius: 50%;
		background: var(--paper, #fff);
	}
	.seek::-moz-range-thumb {
		width: 0.75rem;
		height: 0.75rem;
		border: 0;
		border-radius: 50%;
		background: var(--paper, #fff);
	}

	@media (prefers-reduced-motion: reduce) {
		.overlay,
		.fades {
			transition: none;
		}
	}
</style>
