<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import type { SliceViewport } from '@scutoons/content';
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
	 *
	 * `src` is the one file to load: the app picks it for the reader's media quality (`pickQualityVariant`); undefined =
	 * not decided yet (the poster only). A new `src` while it plays (the reader changed the quality) keeps the position and
	 * the play state. `viewport` = the slice fills the viewport, the video (and its poster) placed in it (`SliceViewport`).
	 */
	let {
		src,
		poster,
		width,
		height,
		alt = '',
		loop = true,
		autoplay = true,
		player = false,
		controls,
		viewport,
		autoplayPlayer = false,
		soundOnLabel = 'Sound on',
		eager = false
	}: {
		/** absolute URL of the file to play (picked by quality); undefined = the poster until it is known */
		src?: string;
		/** absolute poster image URL (the slice image) */
		poster: string;
		width: number;
		height: number;
		alt?: string;
		loop?: boolean;
		autoplay?: boolean;
		/** a video player (sound, controls) instead of the muted loop */
		player?: boolean;
		/** the native controls; default: on for a player, off for a loop */
		controls?: boolean;
		/** fill the viewport, placed like this (`SliceVideo.viewport`) */
		viewport?: SliceViewport;
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
	/** the file the `<video>` has (follows `src`; a change is a quality switch) */
	let loaded = $state<string | undefined>(untrack(() => src));
	const hasSource = $derived(!!loaded);
	const showControls = $derived(controls ?? player);
	const fit = $derived(viewport ? (viewport.fit ?? 'contain') : null);
	const position = $derived(viewport?.position ?? 'center');
	/** the viewport's width without a scrollbar (100vw would include it and scroll sideways) */
	let viewportWidth = $state<number | null>(null);

	onMount(() => {
		const query = window.matchMedia('(prefers-reduced-motion: reduce)');
		reducedMotion = query.matches;
		const onChange = () => (reducedMotion = query.matches);
		query.addEventListener('change', onChange);
		return () => query.removeEventListener('change', onChange);
	});

	$effect(() => {
		if (!viewport) return;
		const measure = () => (viewportWidth = document.documentElement.clientWidth);
		measure();
		const observer = new ResizeObserver(measure);
		observer.observe(document.documentElement);
		return () => observer.disconnect();
	});

	// another file (the reader changed the quality): same position, same play/mute state
	$effect(() => {
		const el = video;
		const next = src;
		if (!next) return;
		untrack(() => {
			if (next === loaded) return;
			const previous = loaded;
			if (el && previous) {
				const time = el.currentTime;
				const playing = !el.paused && !el.ended;
				const muted = el.muted;
				el.addEventListener(
					'loadedmetadata',
					() => {
						if (time > 0) el.currentTime = el.duration > 0 ? Math.min(time, el.duration) : time;
						el.muted = muted;
						if (playing) el.play().catch(() => {});
					},
					{ once: true }
				);
			}
			loaded = next;
		});
	});

	// the loop: plays while in view
	$effect(() => {
		const el = video;
		if (!el || player || !autoplay || !hasSource) return;
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
		if (!el || !player || !autoplayPlayer || !autoplay || reducedMotion || !hasSource) return;
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

	/** a player without controls: a click / Space / Enter plays and pauses it */
	function toggle(event: Event) {
		const el = video;
		if (!el || showControls || !player) return;
		if (event instanceof KeyboardEvent) {
			if (event.key !== ' ' && event.key !== 'Enter') return;
			event.preventDefault();
		}
		if (el.paused || el.ended) {
			el.play().catch(() => {});
			soundBlocked = soundBlocked && el.muted;
		} else el.pause();
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
			// a quality switch starts the new file at 0 before it seeks back: not a step back
			if (el.seeking || el.readyState < 1) return;
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

<div
	class="video-box"
	class:viewport={!!viewport}
	class:cover={fit === 'cover'}
	class:contain={fit === 'contain'}
	data-position={viewport ? position : undefined}
	data-fit={fit ?? undefined}
	style:--ratio={width / height}
	style:--viewport-width={viewportWidth ? `${viewportWidth}px` : undefined}
	data-testid={viewport ? 'viewport-video' : undefined}
>
	<div class="frame">
		{#if player}
			<!-- a video to watch: the slice's size is the video's ratio, so nothing is cropped (in the column) -->
			<!-- (no captions exist for the music video; without controls it takes the focus to play/pause by keyboard) -->
			<!-- svelte-ignore a11y_media_has_caption a11y_no_noninteractive_tabindex -->
			<video
				bind:this={video}
				class="player"
				class:clickable={!showControls}
				src={loaded}
				{poster}
				{width}
				{height}
				controls={showControls}
				playsinline
				preload="metadata"
				style:aspect-ratio={`${width} / ${height}`}
				aria-label={alt || undefined}
				tabindex={showControls ? undefined : 0}
				onclick={toggle}
				onkeydown={toggle}
				data-player
			></video>
			{#if soundBlocked}
				<button type="button" class="sound-on" onclick={soundOn} data-testid="player-sound-on">
					<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
						<path d="M2 6h3l4-3v10l-4-3H2z" fill="currentColor" />
						<path d="M11 5.5c1 .8 1.5 1.6 1.5 2.5s-.5 1.7-1.5 2.5M13 3.5c1.5 1.2 2.2 2.7 2.2 4.5s-.7 3.3-2.2 4.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="square" />
					</svg>
					{soundOnLabel}
				</button>
			{/if}
		{:else if reducedMotion}
			<img src={poster} {alt} {width} {height} loading={eager ? 'eager' : 'lazy'} decoding="async" />
		{:else}
			<!-- the slice's size wins over the video's own ratio (object-fit: cover), so nothing shifts when it loads -->
			<video
				bind:this={video}
				src={loaded}
				{poster}
				{width}
				{height}
				{loop}
				controls={showControls}
				muted
				playsinline
				disablepictureinpicture
				preload={eager ? 'auto' : 'none'}
				style:aspect-ratio={`${width} / ${height}`}
				aria-label={alt || undefined}
			></video>
		{/if}
	</div>
</div>

<style>
	.video-box,
	.frame {
		position: relative;
	}
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
	video.clickable {
		cursor: pointer;
	}

	/* full viewport (SliceVideo.viewport): the whole width of the viewport (out of the reading column) and the screen's
	   height (svh: the toolbar-shown height on phones, so the box doesn't jump while scrolling) */
	.viewport {
		width: var(--viewport-width, 100vw);
		margin-left: calc(50% - var(--viewport-width, 100vw) / 2);
		height: 100vh;
		height: 100svh;
		background: var(--ink, #000);
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		overflow: hidden;
	}
	.viewport[data-position='top'] {
		justify-content: flex-start;
	}
	.viewport[data-position='bottom'] {
		justify-content: flex-end;
	}
	/* contain = fit width: the frame is the video's own box, as big as the viewport allows (the controls and the
	   "Sound on" button stay on the video, the bars are the rest of the box) */
	.viewport.contain .frame {
		width: min(100%, calc(100vh * var(--ratio)));
		width: min(100%, calc(100svh * var(--ratio)));
		aspect-ratio: var(--ratio);
	}
	/* cover = fit height: the frame fills the box, the video is cropped to it */
	.viewport.cover .frame {
		width: 100%;
		height: 100%;
	}
	.viewport img,
	.viewport video {
		width: 100%;
		height: 100%;
		aspect-ratio: auto !important;
		object-fit: contain;
		background: var(--ink, #000);
	}
	.viewport.cover img,
	.viewport.cover video {
		object-fit: cover;
		object-position: 50% 50%;
	}
	.viewport.cover[data-position='top'] img,
	.viewport.cover[data-position='top'] video {
		object-position: 50% 0%;
	}
	.viewport.cover[data-position='bottom'] img,
	.viewport.cover[data-position='bottom'] video {
		object-position: 50% 100%;
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
