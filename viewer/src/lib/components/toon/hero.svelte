<script lang="ts">
	import { onMount } from 'svelte';
	import { isItemPublic, type LoadedSequenceItem, type Season, type Toon } from '@scutoons/content';
	import { asset } from '$lib/utils/cdn.ts';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import StartButton from './start-button.svelte';

	/**
	 * The toon page's first screen on desktop (≥ 1024 px, agents/DESIGN.md "Toon page desktop"): the season's trailer loop
	 * (muted, looping, inline; only while the hero is on screen and the tab visible; the poster with reduced motion)
	 * under a flat 45 % scrim, the season line, the toon title + tagline, the start button (inverted) and "Trailer
	 * ansehen" → the trailer chapter (the prologue); "Ton an" bottom right opens it too (the loop itself stays muted).
	 * While the hero is under the fixed top icons, `<html class="on-hero">` turns them (and the logo) white.
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
		trailerChapter === null ? undefined : sequence.find((i) => i.number === trailerChapter && isItemPublic(i, import.meta.env.DEV))
	);
	const trailerHref = $derived(trailerItem ? localizeHref(`/${toonId}/chapter/${trailerItem.number}`) : null);
	const poster = $derived(asset(`${toonId}/${trailer?.poster || toon.cover?.square || ''}`));

	let hero = $state<HTMLElement | null>(null);
	let video = $state<HTMLVideoElement | null>(null);
	// read right away: a video rendered for one frame would already start downloading the loop
	let reducedMotion = $state(typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches);

	onMount(() => {
		const query = window.matchMedia('(prefers-reduced-motion: reduce)');
		reducedMotion = query.matches;
		const onChange = () => (reducedMotion = query.matches);
		query.addEventListener('change', onChange);
		return () => query.removeEventListener('change', onChange);
	});

	// the loop plays only while the hero is on screen and the tab is visible
	$effect(() => {
		const el = video;
		const node = hero;
		if (!el || !node) return;
		el.muted = true; // a property: the attribute alone doesn't count for autoplay everywhere
		let inView = false;
		const update = () => {
			if (inView && document.visibilityState === 'visible') el.play().catch(() => {});
			else el.pause();
		};
		const io = new IntersectionObserver(([entry]) => {
			inView = entry.isIntersecting;
			update();
		});
		io.observe(node);
		document.addEventListener('visibilitychange', update);
		return () => {
			io.disconnect();
			document.removeEventListener('visibilitychange', update);
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
	{#if trailer && !reducedMotion}
		<video bind:this={video} class="hero-media" {poster} muted loop playsinline preload="auto" aria-hidden="true" data-testid="hero-video">
			{#each trailer.sources ?? [] as source (source.src)}
				<source src={asset(`${toonId}/${source.src}`)} media={source.media || undefined} type="video/mp4" />
			{/each}
			<source src={asset(`${toonId}/${trailer.loop}`)} type="video/mp4" />
		</video>
	{:else}
		<img class="hero-media" src={poster} alt="" />
	{/if}
	<div class="scrim"></div>

	<div class="hero-text">
		<p class="hero-season">{m['hero.season']({ number: seasonNumber, title: season.title })}</p>
		<h1>{toon.title}</h1>
		{#if toon.description}<p class="hero-tagline">{toon.description}</p>{/if}
		<div class="hero-actions">
			<StartButton {toonId} season={seasonNumber} chapters={sequence} inverse testid="hero-start" />
			{#if trailerHref}
				<a class="hero-trailer" href={trailerHref} data-testid="hero-trailer">
					{m['hero.trailer']()}
					<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M3 2v12l11-6z" fill="currentColor" /></svg>
				</a>
			{/if}
		</div>
	</div>

	{#if trailerHref}
		<a class="hero-sound" href={trailerHref} title={m['hero.sound_on_title']()} data-testid="hero-sound">
			<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"
				><path d="M2 6h3l4-3v10l-4-3H2z" fill="currentColor" /><path d="M11 6l4 4M15 6l-4 4" stroke="currentColor" stroke-width="1.5" /></svg
			>
			{m['hero.sound_on']()}
		</a>
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
		width: min(31.6667rem, 60%);
		display: flex;
		flex-direction: column;
		gap: 0.8333rem;
	}
	.hero-text p,
	.hero-text h1 {
		margin: 0;
		color: inherit;
	}
	.hero-season {
		font-size: 0.5833rem;
		letter-spacing: 0.08em;
		text-transform: uppercase;
	}
	h1 {
		font-size: 3.3333rem;
		line-height: 1;
		font-weight: var(--font-weight-regular);
	}
	.hero-tagline {
		font-size: 0.8333rem;
		line-height: 1.5;
		max-width: 26.6667rem;
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
		padding: calc(0.75em - var(--border-width-regular)) calc(1.6em - var(--border-width-regular));
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
		padding: 0.4167rem 0.6667rem;
		border: var(--border-width-regular) solid rgb(255 255 255 / 0.6);
		color: var(--paper);
		font-size: 0.5833rem;
		line-height: 1;
		text-decoration: none;
		svg {
			width: 1em;
			height: 1em;
		}
	}
	.hero-sound:hover {
		border-color: var(--paper);
	}
</style>
