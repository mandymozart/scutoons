<script lang="ts">
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A coming-soon chapter's teaser (owner 2026-10-03: the key visual / key visual video "as a trailer for the episode"):
	 * the video muted on a loop with the key visual as poster, "Ton an" / "Ton aus" like the toon hero; only the image
	 * when there is no video. With reduced motion the video waits for "Ton an".
	 */
	let { toonId, keyvisual }: { toonId: string; keyvisual: { image?: string; video?: string } } = $props();

	const poster = $derived(keyvisual.image ? asset(`${toonId}/${keyvisual.image}`) : undefined);
	const src = $derived(keyvisual.video ? asset(`${toonId}/${keyvisual.video}`) : undefined);

	let video = $state<HTMLVideoElement>();
	let muted = $state(true);
	const reducedMotion = typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;

	$effect(() => {
		const el = video;
		if (!el) return;
		el.muted = muted;
		if (!reducedMotion || !muted) el.play().catch(() => {});
	});
</script>

<figure class="teaser" data-testid="chapter-teaser">
	{#if src}
		<!-- svelte-ignore a11y_media_has_caption -->
		<video bind:this={video} class="teaser-media" {poster} muted loop playsinline preload="metadata" data-testid="chapter-teaser-video">
			<source {src} type="video/mp4" />
		</video>
		<button type="button" class="teaser-sound" aria-pressed={!muted} onclick={() => (muted = !muted)} data-testid="chapter-teaser-sound">
			{muted ? m['hero.sound_on']() : m['hero.sound_off']()}
		</button>
	{:else if poster}
		<img class="teaser-media" src={poster} alt="" />
	{/if}
</figure>

<style>
	.teaser {
		position: relative;
		margin: 1.5rem auto 0;
		max-width: 500px;
	}
	.teaser-media {
		display: block;
		width: 100%;
		height: auto;
	}
	/* like the toon hero's "Ton an" (hero.svelte) */
	.teaser-sound {
		position: absolute;
		right: 0.75rem;
		bottom: 0.75rem;
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
	}
	.teaser-sound:hover,
	.teaser-sound[aria-pressed='true'] {
		border-color: var(--paper);
	}
	.teaser-sound:focus-visible {
		outline: var(--border-width-regular) solid var(--paper);
		outline-offset: 2px;
	}
</style>
