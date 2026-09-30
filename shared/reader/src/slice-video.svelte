<script lang="ts">
	import { onMount } from 'svelte';

	/**
	 * A slice video: muted, inline, looping by default; plays while in view and pauses outside. With
	 * `prefers-reduced-motion: reduce` only the poster is shown (an `<img>`, no video download).
	 */
	let {
		src,
		poster,
		width,
		height,
		alt = '',
		loop = true,
		autoplay = true,
		eager = false
	}: {
		/** absolute video URL */
		src: string;
		/** absolute poster image URL (the slice image) */
		poster: string;
		width: number;
		height: number;
		alt?: string;
		loop?: boolean;
		autoplay?: boolean;
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

	$effect(() => {
		const el = video;
		if (!el || !autoplay) return;
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
</script>

{#if reducedMotion}
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
</style>
