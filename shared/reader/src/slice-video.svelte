<script lang="ts">
	import { onMount } from 'svelte';
	import { SLICE_PLAYER_EVENT, type SlicePlayerDetail } from './types.ts';

	/**
	 * A slice video: muted, inline, looping by default; plays while in view and pauses outside. With
	 * `prefers-reduced-motion: reduce` only the poster is shown (an `<img>`, no video download).
	 *
	 * `player`: a video to watch instead (a music video): sound, native controls (fullscreen through them), no loop, no
	 * autoplay, only its metadata is loaded up front. It tells its ancestors about play / pause / end and how far it was
	 * watched with a bubbling `sliceplayer` event (`SlicePlayerDetail`): the app pauses its music and records progress.
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
		/** one of the first slices: start buffering right away */
		eager?: boolean;
	} = $props();

	let video = $state<HTMLVideoElement | null>(null);
	let reducedMotion = $state(false);

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
</style>
