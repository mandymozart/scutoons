<script lang="ts">
	import ClampText from '$lib/components/clamp-text.svelte';
	import { onMount, untrack } from 'svelte';
	import { pickQualityVariant, type LoadedSequenceItem, type Season, type Toon } from '@scutoons/content';
	import { mediaQuality } from '$lib/media-quality.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import StartButton from './start-button.svelte';
	import { isReadable } from '$lib/draft-preview.svelte.ts';

	/**
	 * The toon page's first screen on desktop (≥ 1024 px, agents/DESIGN.md "Toon page desktop"): the season's trailer loop
	 * (muted by default, looping, inline; only while the hero is on screen and the tab visible; the poster with reduced
	 * motion) under a flat 45 % scrim, the season line, the toon title + tagline, the start button (inverted) and "Trailer
	 * ansehen" → the trailer chapter (the prologue). "Ton an" / "Ton aus" bottom right toggles the loop's own soundtrack in
	 * place (no navigation; with reduced motion "Ton an" is the explicit request to play the loop with sound). The video
	 * and the scrim are not clickable. While the hero is under the fixed top icons, `<html class="on-hero">` turns them
	 * (and the logo) white.
	 */
	let {
		toonId,
		toon,
		season,
		seasonNumber,
		sequence
	}: { toonId: string; toon: Toon; season: Season; seasonNumber: number; sequence: LoadedSequenceItem[] } = $props();

	const trailer = $derived(season.trailer);
	const trailerChapter = $derived(trailer ? (trailer.chapter ?? 0) : null);
	// the buttons only when the trailer chapter is readable (listed + published; drafts in dev)
	const trailerItem = $derived(
		trailerChapter === null ? undefined : sequence.find((i) => i.number === trailerChapter && isReadable(i))
	);
	const trailerHref = $derived(trailerItem ? localizeHref(`/${toonId}/chapter/${trailerItem.number}`) : null);
	// the hero sets the title itself, so the season's cover is the bare artwork here (no CoverArt lettering; older data:
	// the toon's cover); no image → just ink
	const posterPath = $derived(trailer?.poster || season.cover?.square || toon.cover?.square || '');
	const poster = $derived(posterPath ? asset(`${toonId}/${posterPath}`) : undefined);

	let hero = $state<HTMLElement | null>(null);
	let video = $state<HTMLVideoElement | null>(null);
	/** the trailer has quality variants (else the older media-query sources) */
	const trailerVariants = $derived(Object.values(trailer?.variants ?? {}).some((file) => !!file));

	// read right away: a video rendered for one frame would already start downloading the loop
	let reducedMotion = $state(typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches);

	onMount(() => {
		const query = window.matchMedia('(prefers-reduced-motion: reduce)');
		reducedMotion = query.matches;
		const onChange = () => (reducedMotion = query.matches);
		query.addEventListener('change', onChange);
		return () => query.removeEventListener('change', onChange);
	});

	// always muted on load (no remembered unmute); the sound button toggles it
	let muted = $state(true);
	// reduced motion: the loop only after an explicit "Ton an" (then with sound)
	let started = $state(false);
	const showVideo = $derived(!!trailer && (!reducedMotion || started));
	let inView = false;

	// the loop plays only while the hero is on screen and the tab is visible (with reduced motion only while the sound is on)
	function update() {
		const el = video;
		if (!el) return;
		if (inView && document.visibilityState === 'visible' && (!reducedMotion || !muted)) el.play().catch(() => {});
		else el.pause();
	}

	function toggleSound() {
		muted = !muted;
		if (!video) {
			// reduced motion: render the video now, its effect starts it (the click is the user activation)
			started = true;
			return;
		}
		video.muted = muted;
		update();
	}

	$effect(() => {
		const el = video;
		const node = hero;
		if (!el || !node) return;
		el.muted = untrack(() => muted); // a property: the attribute alone doesn't count for autoplay everywhere
		const io = new IntersectionObserver(([entry]) => {
			inView = entry.isIntersecting;
			update();
		});
		io.observe(node);
		document.addEventListener('visibilitychange', update);
		return () => {
			io.disconnect();
			document.removeEventListener('visibilitychange', update);
			inView = false;
			el.pause();
		};
	});

	// the fixed top icons + logo turn white while they sit on the hero
	$effect(() => {
		const node = hero;
		if (!node) return;
		const root = document.documentElement;
		const update = () => root.classList.toggle('on-hero', node.getBoundingClientRect().bottom > 64);
		update();
		window.addEventListener('scroll', update, { passive: true });
		window.addEventListener('resize', update);
		return () => {
			window.removeEventListener('scroll', update);
			window.removeEventListener('resize', update);
			root.classList.remove('on-hero');
		};
	});
</script>

<section class="hero" bind:this={hero} data-testid="hero">
	{#if trailer && showVideo}
		<video bind:this={video} class="hero-media" {poster} muted loop playsinline preload="auto" aria-hidden="true" data-testid="hero-video">
			{#if trailerVariants}
				<!-- the reader's media quality picks the encoding (like slice videos); the loop while it is not known yet -->
				<source src={asset(`${toonId}/${pickQualityVariant({ src: trailer.loop, variants: trailer.variants }, mediaQuality.level ?? 'low')}`)} type="video/mp4" />
			{:else}
				{#each trailer.sources ?? [] as source (source.src)}
					<source src={asset(`${toonId}/${source.src}`)} media={source.media || undefined} type="video/mp4" />
				{/each}
				<source src={asset(`${toonId}/${trailer.loop}`)} type="video/mp4" />
			{/if}
		</video>
	{:else if poster}
		<img class="hero-media" src={poster} alt="" />
	{/if}
	<div class="scrim"></div>

	<div class="hero-text">
		<h1>{toon.title}</h1>
		<!-- the season's description under the title (owner 2026-10-03: no "Staffel 1 · …" line, not the toon's text) -->
		{#if season.description || toon.description}<ClampText text={season.description || toon.description} lines={2} class="hero-tagline" />{/if}
		<div class="hero-actions">
			<StartButton {toonId} season={seasonNumber} chapters={sequence} inverse testid="hero-start" />
			{#if trailerHref}
				<a class="hero-trailer sc-control" href={trailerHref} data-testid="hero-trailer">
					{m['hero.trailer']()}
					<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M3 2v12l11-6z" fill="currentColor" /></svg>
				</a>
			{/if}
		</div>
	</div>

	{#if trailer}
		<button type="button" class="hero-sound" aria-pressed={!muted} onclick={toggleSound} data-testid="hero-sound">
			{#if muted}
				<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" data-icon="speaker"
					><path d="M2 6h3l4-3v10l-4-3H2z" fill="currentColor" /><path
						d="M11 5.5a3.5 3.5 0 0 1 0 5M12.5 3.5a6 6 0 0 1 0 9"
						fill="none"
						stroke="currentColor"
						stroke-width="1.5"
					/></svg
				>
				{m['hero.sound_on']()}
			{:else}
				<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" data-icon="speaker-muted"
					><path d="M2 6h3l4-3v10l-4-3H2z" fill="currentColor" /><path d="M11 6l4 4M15 6l-4 4" stroke="currentColor" stroke-width="1.5" /></svg
				>
				{m['hero.sound_off']()}
			{/if}
		</button>
	{/if}
</section>

<style>
	/* rem at the desktop root size (24 px): 0.5833rem ≈ 14 px, 0.8333rem = 20 px, 3.3333rem = 80 px */
	.hero {
		position: relative;
		height: min(100vh, 900px);
		background: var(--ink);
		color: var(--paper);
		overflow: hidden;
	}
	.hero-media {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	/* a flat scrim, no gradient */
	.scrim {
		position: absolute;
		inset: 0;
		background: rgb(0 0 0 / 0.45);
	}
	.hero-text {
		position: absolute;
		left: 2rem;
		bottom: 3rem;
		/* 3/4 of the hero, so the season's description flows wide (owner 2026-10-03) */
		width: 75%;
		display: flex;
		flex-direction: column;
		gap: 0.8333rem;
	}
	.hero-text :global(p),
	.hero-text h1 {
		margin: 0;
		color: inherit;
	}
	h1 {
		font-size: 3.3333rem;
		line-height: 1;
		font-weight: var(--font-weight-regular);
	}
	.hero-text :global(.hero-tagline) {
		font-size: 0.8333rem;
		line-height: 1.5;
		max-width: none;
	}
	.hero-actions {
		display: flex;
		align-items: flex-start;
		gap: 0.6667rem;
		margin-top: 0.5rem;
		font-size: 0.8333rem;
	}
	.hero-actions :global(.start) {
		margin-inline: 0;
	}
	.hero-trailer {
		display: inline-flex;
		align-items: center;
		gap: 0.5em;
		box-sizing: border-box;
		min-height: var(--control-height);
		padding: 0 calc(1.6em - var(--border-width-regular));
		border: var(--border-width-regular) solid var(--paper);
		color: var(--paper);
		line-height: var(--type-button-line);
		text-decoration: none;
		white-space: nowrap;
		svg {
			width: 0.9em;
			height: 0.9em;
		}
	}
	.hero-trailer:hover {
		background: rgb(255 255 255 / 0.12);
	}
	.hero-sound {
		position: absolute;
		right: 2rem;
		bottom: 3rem;
		display: inline-flex;
		align-items: center;
		gap: 0.4167rem;
		box-sizing: border-box;
		min-height: var(--control-height);
		padding: 0 0.6667rem;
		border: var(--border-width-regular) solid rgb(255 255 255 / 0.6);
		background: none;
		color: var(--paper);
		font: inherit;
		font-size: 0.5833rem;
		line-height: 1;
		cursor: pointer;
		svg {
			width: 1em;
			height: 1em;
		}
	}
	.hero-sound:hover,
	.hero-sound[aria-pressed='true'] {
		border-color: var(--paper);
	}
	.hero-sound:focus-visible {
		outline: var(--border-width-regular) solid var(--paper);
		outline-offset: 2px;
	}
</style>
