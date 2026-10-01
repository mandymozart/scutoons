<script lang="ts">
	import { onMount } from 'svelte';
	import { SLICE_PLAYER_EVENT, type SlicePlayerDetail } from './types.ts';

	/**
	 * A slice video: muted, inline, looping by default; plays while in view and pauses outside. With
	 * `prefers-reduced-motion: reduce` only the poster is shown (an `<img>`, no video download).
	 *
	 * `player`: a video to watch instead (a music video): sound, native controls (fullscreen through them), no loop, only
	 * its metadata is loaded up front. It tells its ancestors about play / pause / end and how far it was watched with a
	 * bubbling `sliceplayer` event (`SlicePlayerDetail`): the app pauses its music and records progress.
	 * With `autoplayPlayer` (the reader; not the creator's preview) it starts once it is in view: with sound when the
	 * browser allows it (a click earlier in the same document, e.g. the in-app navigation from "Trailer ansehen"),
	 * otherwise muted with a "Sound on" button over it (`soundOnLabel`) that unmutes and keeps playing. Reduced motion:
	 * no autoplay, the poster and the native play button.
	 */
	let {
		src,
		sources = [],
		poster,
		width,
		height,
		alt = '',
		loop = true,
		autoplay = true,
		player = false,
		autoplayPlayer = false,
		soundOnLabel = 'Sound on',
		eager = false
	}: {
		/** absolute video URL (the default, after `sources`) */
		src: string;
		/** other encodings (absolute URLs), tried first in this order: the first whose `media` matches is loaded */
		sources?: { src: string; media?: string }[];
		/** absolute poster image URL (the slice image) */
		poster: string;
		width: number;
		height: number;
		alt?: string;
		loop?: boolean;
		autoplay?: boolean;
		/** a video player (sound, controls) instead of the muted loop */
		player?: boolean;
		/** the player starts by itself once it is in view (muted + `soundOnLabel` when sound is blocked); needs `autoplay` */
		autoplayPlayer?: boolean;
		/** the label of the button that unmutes an autoplaying player (the app's language) */
		soundOnLabel?: string;
		/** one of the first slices: start buffering right away */
		eager?: boolean;
	} = $props();

	let video = $state<HTMLVideoElement | null>(null);
	// read synchronously: an autoplay (or the loop's download) must not start before the preference is known
	const motionQuery = typeof window !== 'undefined' && window.matchMedia ? window.matchMedia('(prefers-reduced-motion: reduce)') : null;
	let reducedMotion = $state(motionQuery?.matches ?? false);
	/** the player started muted because the browser blocked sound: show the "Sound on" button */
	let soundBlocked = $state(false);

	onMount(() => {
		const query = window.matchMedia('(prefers-reduced-motion: reduce)');
		reducedMotion = query.matches;
		const onChange = () => (reducedMotion = query.matches);
		query.addEventListener('change', onChange);
		return () => query.removeEventListener('change', onChange);
	});

	// the loop: plays while in view
	$effect(() => {
		const el = video;
		if (!el || player || !autoplay) return;
		// muted must be a property (the attribute alone doesn't count for autoplay in every browser)
		el.muted = true;
		const observer = new IntersectionObserver(
			([entry]) => {
				if (entry.isIntersecting) el.play().catch(() => {});
				else el.pause();
			},
			{ threshold: 0.25 }
		);
		observer.observe(el);
		return () => observer.disconnect();
	});

	// the player's autoplay: once, when at least half of it is in view
	$effect(() => {
		const el = video;
		if (!el || !player || !autoplayPlayer || !autoplay || reducedMotion) return;
		let alive = true;
		const observer = new IntersectionObserver(
			([entry]) => {
				if (!entry.isIntersecting) return;
				observer.disconnect();
				void start(el, () => alive);
			},
			{ threshold: 0.5 }
		);
		observer.observe(el);
		return () => {
			alive = false;
			observer.disconnect();
		};
	});

	/** with sound if the browser lets us, else muted (+ the button); a reader who already pressed play wins */
	async function start(el: HTMLVideoElement, alive: () => boolean) {
		if (!el.paused || el.ended) return;
		el.muted = false;
		try {
			await el.play();
			return;
		} catch (e) {
			// AbortError: unmounted / the source changed / paused meanwhile – nothing to do
			if (!alive() || (e as DOMException)?.name !== 'NotAllowedError') return;
		}
		el.muted = true;
		try {
			await el.play();
			if (alive()) soundBlocked = !!el.muted;
		} catch {
			// not even muted (data saver, a strict policy): the poster and the native play button stay
			el.muted = false;
		}
	}

	function soundOn() {
		const el = video;
		soundBlocked = false;
		if (!el) return;
		// this click is a user gesture: unmuting keeps it playing
		el.muted = false;
		if (el.paused) el.play().catch(() => {});
	}

	// unmuted another way (the native controls) → the button goes
	$effect(() => {
		const el = video;
		if (!el || !player) return;
		const onVolume = () => {
			if (!el.muted) soundBlocked = false;
		};
		el.addEventListener('volumechange', onVolume);
		return () => el.removeEventListener('volumechange', onVolume);
	});

	// the player: tells the app what it does
	$effect(() => {
		const el = video;
		if (!el || !player) return;
		let reported = -1;
		const progress = () => (el.ended ? 1 : el.duration > 0 ? Math.min(1, el.currentTime / el.duration) : 0);
		const emit = (state: SlicePlayerDetail['state'], playing: boolean) => {
			const detail: SlicePlayerDetail = { state, playing, progress: progress() };
			el.dispatchEvent(new CustomEvent<SlicePlayerDetail>(SLICE_PLAYER_EVENT, { bubbles: true, detail }));
		};
		const onPlay = () => emit('play', true);
		const onPause = () => {
			// `pause` also fires right before `ended`; the end reports itself
			if (!el.ended) emit('pause', false);
		};
		const onEnded = () => emit('ended', false);
		const onTime = () => {
			const percent = Math.floor(progress() * 100);
			if (percent === reported) return;
			reported = percent;
			emit('time', !el.paused);
		};
		el.addEventListener('play', onPlay);
		el.addEventListener('pause', onPause);
		el.addEventListener('ended', onEnded);
		el.addEventListener('timeupdate', onTime);
		return () => {
			el.removeEventListener('play', onPlay);
			el.removeEventListener('pause', onPause);
			el.removeEventListener('ended', onEnded);
			el.removeEventListener('timeupdate', onTime);
			// leaving while it plays: the app gets its music back
			if (!el.paused && !el.ended) emit('pause', false);
		};
	});
</script>

{#if player}
	<!-- a video to watch: the slice's size is the video's ratio, so nothing is cropped -->
	<div class="player-box">
	<video
		bind:this={video}
		class="player"
		{poster}
		{width}
		{height}
		controls
		playsinline
		preload="metadata"
		style:aspect-ratio={`${width} / ${height}`}
		aria-label={alt || undefined}
		data-player
	>
		{#each sources as source (source.src)}
			<source src={source.src} media={source.media || undefined} type="video/mp4" />
		{/each}
		<source {src} type="video/mp4" />
	</video>
	{#if soundBlocked}
		<button type="button" class="sound-on" onclick={soundOn} data-testid="player-sound-on">
			<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
				<path d="M2 6h3l4-3v10l-4-3H2z" fill="currentColor" />
				<path d="M11 5.5c1 .8 1.5 1.6 1.5 2.5s-.5 1.7-1.5 2.5M13 3.5c1.5 1.2 2.2 2.7 2.2 4.5s-.7 3.3-2.2 4.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="square" />
			</svg>
			{soundOnLabel}
		</button>
	{/if}
	</div>
{:else if reducedMotion}
	<img src={poster} {alt} {width} {height} loading={eager ? 'eager' : 'lazy'} decoding="async" />
{:else}
	<!-- the slice's size wins over the video's own ratio (object-fit: cover), so nothing shifts when it loads -->
	<video
		bind:this={video}
		{poster}
		{width}
		{height}
		{loop}
		muted
		playsinline
		disablepictureinpicture
		preload={eager ? 'auto' : 'none'}
		style:aspect-ratio={`${width} / ${height}`}
		aria-label={alt || undefined}
	>
		{#each sources as source (source.src)}
			<source src={source.src} media={source.media || undefined} type="video/mp4" />
		{/each}
		<source {src} type="video/mp4" />
	</video>
{/if}

<style>
	img,
	video {
		display: block;
		width: 100%;
		height: auto;
		object-fit: cover;
	}
	/* the player letterboxes instead (the slice already has the video's ratio) */
	video.player {
		object-fit: contain;
		background: var(--ink, #000);
	}
	.player-box {
		position: relative;
	}
	/* autoplay without sound: the primary button (ink / paper, square) in the middle of the video (the corners are under
	   the app's fixed logo and toggles, the bottom under the native controls) */
	.sound-on {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		margin: 0;
		padding: 0.5rem 0.75rem;
		border: 2px solid var(--paper, #fff);
		border-radius: 0;
		background: var(--ink, #000);
		color: var(--paper, #fff);
		font-family: var(--font-mono, 'IBM Plex Mono', monospace);
		font-size: 0.75rem;
		line-height: 1;
		cursor: pointer;
		touch-action: manipulation;
	}
	.sound-on:hover,
	.sound-on:focus-visible {
		background: var(--paper, #fff);
		color: var(--ink, #000);
	}
</style>
