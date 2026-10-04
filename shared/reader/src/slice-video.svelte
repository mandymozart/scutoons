<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { parseFrameRatio, viewportObjectFit, viewportObjectPosition, type SliceFrame, type SliceViewport } from '@scutoons/content';
	import { SLICE_PLAYER_EVENT, type SlicePlayerDetail } from './types.ts';
	import PlayerOverlay, { DEFAULT_PLAYER_LABELS, type PlayerLabels } from './player-overlay.svelte';

	/**
	 * A slice video: muted, inline, looping by default; plays while in view and pauses outside. With
	 * `prefers-reduced-motion: reduce` only the poster is shown (an `<img>`, no video download).
	 *
	 * `player`: a video to watch instead (a music video): sound, our own controls over it (`PlayerOverlay`: ▶ / ↺, with
	 * `controls` the fading bar + "Überspringen" when `skippable`), no loop, only its metadata is loaded up front. It tells its ancestors about play / pause / end and how far it was watched with a
	 * bubbling `sliceplayer` event (`SlicePlayerDetail`): the app pauses its music and records progress.
	 * With `autoplayPlayer` (the reader; not the creator's preview) it starts once it is in view: with sound when the
	 * browser allows it (a click earlier in the same document, e.g. the in-app navigation from "Trailer ansehen"),
	 * otherwise muted with a "Sound on" button over it (`labels.soundOn`) that unmutes and keeps playing. Reduced motion:
	 * no autoplay, the poster and the ▶.
	 *
	 * `src` is the one file to load: the app picks it for the reader's media quality (`pickQualityVariant`); undefined =
	 * not decided yet (the poster only). A new `src` while it plays (the reader changed the quality) keeps the position and
	 * the play state. `viewport` = the slice fills the viewport, the video (and its poster) placed in it (`SliceViewport`);
	 * `frame` = in the column, a box of that ratio (the slice's width / height follow it) with the video placed in it
	 * (`SliceFrame`). The four fits: contain / width / height / cover (`viewportObjectFit` with the media's own ratio).
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
		frame,
		autoplayPlayer = false,
		playerLabels,
		skippable = false,
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
		/** a player: the whole bar (else only ▶ / ↺); a loop: the native controls. Default: on for a player, off for a loop */
		controls?: boolean;
		/** fill the viewport, placed like this (`SliceVideo.viewport`) */
		viewport?: SliceViewport;
		/** in the column: a box of the frame's ratio (`SliceVideo.frame`; ignored with `viewport`) */
		frame?: SliceFrame;
		/** the player starts by itself once it is in view (muted + "Sound on" when sound is blocked); needs `autoplay` */
		autoplayPlayer?: boolean;
		/** the player's texts in the app's language (English by default) */
		playerLabels?: Partial<PlayerLabels>;
		/** a player with controls shows "Überspringen" (the app handles the bubbling `sliceskip` event) */
		skippable?: boolean;
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
	const labels = $derived<PlayerLabels>({ ...DEFAULT_PLAYER_LABELS, ...playerLabels });
	/** the placement: full viewport, else a frame in the column, else none (the media's own ratio) */
	const placement = $derived<SliceViewport | SliceFrame | null>(viewport ?? (frame && parseFrameRatio(frame.ratio) ? frame : null));
	const position = $derived(placement?.position ?? 'center');
	/** the viewport's width without a scrollbar (100vw would include it and scroll sideways) */
	let viewportWidth = $state<number | null>(null);
	/** the box (viewport or frame) as measured */
	let box = $state<HTMLElement | null>(null);
	let boxRatio = $state<number | null>(null);
	/** the media's own ratio: the video's once its metadata is there, before that the poster's, else the slice's */
	let mediaRatio = $state<number | null>(null);
	let posterRatio = $state<number | null>(null);
	const ownRatio = $derived(mediaRatio ?? posterRatio ?? width / height);
	/** contain or cover in this box (fit width / height depend on the two ratios) */
	const objectFit = $derived(placement ? viewportObjectFit(placement.fit, ownRatio, boxRatio ?? (viewport && typeof window !== 'undefined' ? window.innerWidth / window.innerHeight : width / height)) : null);

	$effect(() => {
		const el = box;
		if (!el || !placement) return;
		const measure = () => {
			if (el.clientHeight > 0) boxRatio = el.clientWidth / el.clientHeight;
		};
		measure();
		const observer = new ResizeObserver(measure);
		observer.observe(el);
		return () => observer.disconnect();
	});

	// the poster's own ratio (a placed video only): the same image the <video> shows, from the cache
	$effect(() => {
		if (!placement || !poster) return;
		const img = new Image();
		img.onload = () => {
			if (img.naturalWidth > 0 && img.naturalHeight > 0) posterRatio = img.naturalWidth / img.naturalHeight;
		};
		img.src = poster;
	});

	// the video's own ratio
	$effect(() => {
		const el = video;
		if (!el || !placement) return;
		const read = () => {
			if (el.videoWidth > 0 && el.videoHeight > 0) mediaRatio = el.videoWidth / el.videoHeight;
		};
		read();
		el.addEventListener('loadedmetadata', read);
		return () => el.removeEventListener('loadedmetadata', read);
	});

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
	bind:this={box}
	class="video-box"
	class:viewport={!!viewport}
	class:framed={!viewport && !!placement}
	class:cover={objectFit === 'cover'}
	class:contain={objectFit === 'contain'}
	data-position={placement ? position : undefined}
	data-fit={placement ? (placement.fit ?? 'contain') : undefined}
	data-object-fit={objectFit ?? undefined}
	style:--ratio={ownRatio}
	style:--frame-ratio={!viewport && placement ? `${width} / ${height}` : undefined}
	style:--object-position={viewportObjectPosition(position)}
	style:--viewport-width={viewportWidth ? `${viewportWidth}px` : undefined}
	data-testid={viewport ? 'viewport-video' : placement ? 'framed-video' : undefined}
>
	<div class="frame">
		{#if player}
			<!-- a video to watch: the slice's size is the video's ratio, so nothing is cropped (in the column) -->
			<!-- (no captions exist for the music video; the overlay's controls replace the browser's) -->
			<!-- svelte-ignore a11y_media_has_caption -->
			<video
				bind:this={video}
				class="player"
				src={loaded}
				{poster}
				{width}
				{height}
				playsinline
				preload="metadata"
				style:aspect-ratio={`${width} / ${height}`}
				aria-label={alt || undefined}
				data-player
			></video>
			<PlayerOverlay {video} {box} full={showControls} {skippable} {labels} bind:soundBlocked />
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
	/* a frame in the column (SliceVideo.frame): the box has the frame's ratio (= the slice's width / height), the media
	   fills it as contain (bars, placed by object-position) or cover (cropped, the position = the part that stays) */
	.framed {
		aspect-ratio: var(--frame-ratio);
		background: var(--ink, #000);
		overflow: hidden;
	}
	.framed .frame {
		width: 100%;
		height: 100%;
	}
	.viewport img,
	.viewport video,
	.framed img,
	.framed video {
		width: 100%;
		height: 100%;
		aspect-ratio: auto !important;
		object-fit: contain;
		object-position: var(--object-position, 50% 50%);
		background: var(--ink, #000);
	}
	.viewport.cover img,
	.viewport.cover video,
	.framed.cover img,
	.framed.cover video {
		object-fit: cover;
	}

	/* full screen (the player's button): the box fills the screen, the whole video in it (the controls come along) */
	.video-box:fullscreen {
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--ink, #000);
	}
	.video-box:fullscreen .frame {
		width: min(100%, calc(100vh * var(--ratio)));
		height: auto;
		aspect-ratio: var(--ratio);
	}
	.video-box:fullscreen video {
		width: 100%;
		height: 100%;
		aspect-ratio: auto !important;
		object-fit: contain;
	}
</style>
