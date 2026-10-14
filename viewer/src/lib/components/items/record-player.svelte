<script lang="ts">
	import { Icon, IconButton, CloseIcon } from '@scutoons/ui';
	import type { AudioRequest } from '$lib/contexts/items-ui.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import { armAngle, nextTrack, previousTrack } from './playlist.ts';

	/**
	 * The audio player at the bottom (owner 2026-10-14: "play the whole album – like a little playlist, or an animation on
	 * the record player"): the album's songs one after another, the song's title and n / N, back / next, and a small
	 * turntable – the record (the album's cover as its label) turns while it plays, the tonearm sits on the outer groove at a
	 * song's start and moves in as it plays, lifting off between songs. After the last song the album starts again. A vault track gets
	 * its signed address only when it starts (`request.sign` – the addresses live ten minutes, an album runs longer).
	 * Reduced motion: the record stands still, the arm jumps.
	 */
	let { request, icon = null, onclose }: { request: AudioRequest; icon?: { src: string; name: string } | null; onclose: () => void } = $props();

	let index = $state(0);
	let src = $state<string | null>(null);
	let failed = $state(false);
	let playing = $state(false);
	/** between two songs: the arm is lifted for a moment */
	let changing = $state(false);
	let time = $state(0);
	let duration = $state(0);
	let audio = $state<HTMLAudioElement>();

	const count = $derived(request.tracks.length);
	const track = $derived(request.tracks[Math.min(index, count - 1)]);
	const cover = $derived(request.cover ? asset(request.cover) : (icon?.src ?? null));

	// a new album starts at its first song
	$effect(() => {
		void request;
		index = 0;
	});

	// the song's address: signed just before it plays
	$effect(() => {
		const i = index;
		const req = request;
		let cancelled = false;
		failed = false;
		src = null;
		const raw = req.tracks[i]?.src ?? '';
		(req.sign ? req.sign(i) : Promise.resolve(/^https?:/.test(raw) ? raw : asset(raw)))
			.then((url) => !cancelled && (src = url))
			.catch(() => !cancelled && (failed = true));
		return () => (cancelled = true);
	});

	let gap: ReturnType<typeof setTimeout> | null = null;
	function go(i: number) {
		if (gap) clearTimeout(gap);
		gap = null;
		changing = false;
		// the same song again (a one-song album looping): its address stays, so start it over by hand
		if (i === index && audio) {
			audio.currentTime = 0;
			void audio.play().catch(() => {});
		}
		index = i;
	}

	/** a song ended: lift the arm a moment, then the next one (after the last: the first again) */
	function ended() {
		const next = nextTrack(index, count);
		playing = false;
		if (next === null) return;
		changing = true;
		gap = setTimeout(() => go(next), 1200);
	}

	/** back: the song again a few seconds in, else the one before */
	function back() {
		const i = previousTrack(index, audio?.currentTime ?? 0);
		if (i === index && audio) audio.currentTime = 0;
		else go(i);
	}

	$effect(() => () => {
		if (gap) clearTimeout(gap);
	});

	const arm = $derived(armAngle(playing || changing, changing ? 0 : duration ? time / duration : 0));
</script>

<div class="record-player" data-testid="audio-player">
	<div class="deck" class:spinning={playing} aria-hidden="true">
		<div class="record">
			{#if cover}<img src={cover} alt="" />{/if}
		</div>
		<div class="arm" class:lifted={changing} style="--arm: {arm}deg"></div>
	</div>
	<div class="track">
		<div class="line">
			<span class="track-title" data-testid="audio-title">{m['items.now_playing']({ title: [track.title, request.title].filter(Boolean).join(' · ') })}</span>
			{#if count > 1}<span class="count" data-testid="audio-count">{index + 1} / {count}</span>{/if}
		</div>
		{#if failed}
			<p class="note" role="alert">{m['items.release_failed']()}</p>
		{:else}
			<!-- svelte-ignore a11y_media_has_caption -->
			<audio
				bind:this={audio}
				src={src ?? undefined}
				controls
				autoplay
				bind:currentTime={time}
				bind:duration
				onplay={() => (playing = true)}
				onpause={() => (playing = false)}
				onended={ended}
			></audio>
		{/if}
	</div>
	{#if count > 1}
		<div class="skip">
			<IconButton label={m['items.previous_track']()} onclick={back} data-testid="audio-prev"><Icon name="skip-back" /></IconButton>
			<IconButton label={m['items.next_track']()} onclick={() => go(nextTrack(index, count) ?? 0)} data-testid="audio-next"><Icon name="skip-forward" /></IconButton>
		</div>
	{/if}
	<button class="close" onclick={onclose} aria-label={m['items.close']()}><CloseIcon size="1rem" /></button>
</div>

<style>
	.record-player {
		position: fixed;
		z-index: 2040;
		left: var(--space-05);
		right: var(--space-05);
		bottom: var(--space-05);
		max-width: 34rem;
		margin: 0 auto;
		display: flex;
		align-items: center;
		gap: var(--space-05);
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--ink);
		padding: var(--space-025) var(--space-05);
	}

	/* the turntable: a record with the cover as its label, a tonearm from the upper right */
	.deck {
		position: relative;
		flex: none;
		width: 3.25rem;
		height: 3.25rem;
	}

	.record {
		position: absolute;
		inset: 0.125rem;
		border-radius: 50%;
		background: repeating-radial-gradient(circle, #1a1a1a 0 0.09rem, #2b2b2b 0.09rem 0.14rem);
		display: grid;
		place-items: center;
		animation: spin 1.8s linear infinite;
		animation-play-state: paused;
	}

	.spinning .record {
		animation-play-state: running;
	}

	.record img {
		width: 42%;
		height: 42%;
		border-radius: 50%;
		object-fit: cover;
	}

	.arm {
		position: absolute;
		top: -0.125rem;
		right: -0.125rem;
		width: 0.25rem;
		height: 2.1rem;
		background: linear-gradient(#c9c9c9, #8a8a8a);
		border: 0.0625rem solid var(--ink);
		border-radius: 0.125rem;
		transform-origin: 50% 0.125rem;
		transform: rotate(var(--arm)) translateY(0);
		transition: transform 0.6s ease;
	}

	.arm.lifted {
		transform: rotate(var(--arm)) translateY(-0.2rem) scale(1.06);
	}

	@keyframes spin {
		to {
			transform: rotate(1turn);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.record {
			animation: none;
		}
		.arm {
			transition: none;
		}
	}

	.track {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}

	.line {
		display: flex;
		gap: var(--space-05);
		font-size: 0.75rem;
	}

	.track-title {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.count {
		flex: none;
		font-variant-numeric: tabular-nums;
	}

	audio {
		width: 100%;
		min-width: 0;
	}

	.note {
		margin: 0;
		font-size: 0.75rem;
	}

	.skip {
		display: flex;
		flex: none;
	}

	.close {
		box-sizing: border-box;
		width: var(--control-height);
		height: var(--control-height);
		padding: 0;
		border-radius: 50%;
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--ink);
		display: inline-grid;
		place-items: center;
		flex: none;
	}

	.close:hover {
		background: var(--primary-color);
	}
</style>
