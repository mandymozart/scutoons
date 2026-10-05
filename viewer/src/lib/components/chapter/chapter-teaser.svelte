<script lang="ts">
	import type { Snippet } from 'svelte';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A coming-soon chapter's teaser (owner 2026-10-03: the key visual / key visual video "as a trailer for the episode";
	 * 2026-10-05: "fullscreen player in a loop … the text is overlayed. autoplay"): the video fills the window, muted on
	 * a loop from the start, the key visual as poster, the chapter text (`children`) on top; "Ton an" / "Ton aus" like
	 * the toon hero. Only the image when there is no video. With reduced motion the video waits for "Ton an".
	 * Portrait windows crop to fill, landscape windows show the whole (portrait) key visual on black.
	 */
	let {
		toonId,
		keyvisual,
		children
	}: { toonId: string; keyvisual: { image?: string; video?: string }; children?: Snippet } = $props();

	const poster = $derived(keyvisual.image ? asset(`${toonId}/${keyvisual.image}`) : undefined);
	const src = $derived(keyvisual.video ? asset(`${toonId}/${keyvisual.video}`) : undefined);

	let video = $state<HTMLVideoElement>();
	let muted = $state(true);
	const reducedMotion =
		typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;

	$effect(() => {
		const el = video;
		if (!el) return;
		el.muted = muted;
		if (reducedMotion && muted) return;
		const play = () => el.play().catch(() => {});
		play();
		// opened in a background tab: the browser holds autoplay back – start once the page shows
		const onVisible = () => document.visibilityState === 'visible' && el.paused && play();
		document.addEventListener('visibilitychange', onVisible);
		return () => document.removeEventListener('visibilitychange', onVisible);
	});
</script>

<figure class="teaser" data-testid="chapter-teaser">
	{#if src}
		<!-- svelte-ignore a11y_media_has_caption -->
		<video
			bind:this={video}
			class="teaser-media"
			{poster}
			muted
			loop
			playsinline
			autoplay={!reducedMotion}
			preload="auto"
			data-testid="chapter-teaser-video"
		>
			<source {src} type="video/mp4" />
		</video>
	{:else if poster}
		<img class="teaser-media" src={poster} alt="" />
	{/if}
	{#if src}
		<button
			type="button"
			class="teaser-sound"
			aria-pressed={!muted}
			onclick={() => (muted = !muted)}
			data-testid="chapter-teaser-sound"
		>
			{muted ? m['hero.sound_on']() : m['hero.sound_off']()}
		</button>
	{/if}
	<figcaption class="teaser-text">
		{@render children?.()}
	</figcaption>
</figure>

<style>
	.teaser {
		position: fixed;
		inset: 0;
		margin: 0;
		background: #000;
		color: var(--paper);
	}
	.teaser-media {
		display: block;
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	@media (orientation: landscape) {
		.teaser-media {
			object-fit: contain;
		}
	}
	/* the chapter text over the picture's lower part, on a scrim so it reads on any frame */
	.teaser-text {
		position: absolute;
		left: 0;
		right: 0;
		bottom: 0;
		padding: 4rem 1rem calc(4.5rem + env(safe-area-inset-bottom, 0px));
		background: linear-gradient(to top, rgb(0 0 0 / 0.7), rgb(0 0 0 / 0));
		text-align: center;
	}
	/* like the toon hero's "Ton an" (hero.svelte) */
	.teaser-sound {
		position: absolute;
		z-index: 1; /* above the text's scrim */
		right: 0.75rem;
		bottom: calc(0.75rem + env(safe-area-inset-bottom, 0px));
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
		outline-offset: 0.125rem;
	}
</style>
