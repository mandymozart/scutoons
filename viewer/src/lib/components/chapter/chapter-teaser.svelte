<script lang="ts">
	import { untrack, type Snippet } from 'svelte';
	import { Icon } from '@scutoons/ui';
	import { toonUrl } from '$lib/utils/cdn.ts';
	import { chromeHidesWhenIdle } from '$lib/chrome.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A coming-soon chapter's teaser (owner 2026-10-03: the key visual / key visual video "as a trailer for the episode";
	 * 2026-10-05: "fullscreen player … the text is overlayed. autoplay"; 2026-10-08: with sound from the start, plays once,
	 * a replay button in the centre): the video fills the window and starts with sound, the key visual as poster, the
	 * chapter text (`children`) on top; a white speaker / muted speaker bottom right toggles the sound (owner 2026-10-08), so does a tap
	 * anywhere on the playing video (2026-10-09, like Instagram / TikTok). When the browser refuses autoplay with
	 * sound (no recent click) it plays muted and the speaker brings the sound. At the end (and with reduced motion before
	 * it starts) a play / "Nochmal ansehen" button sits in the centre. Only the image when there is no video.
	 * Portrait windows crop to fill, landscape windows show the whole (portrait) key visual on black.
	 */
	let {
		toonId,
		keyvisual,
		children
	}: { toonId: string; keyvisual: { image?: string; video?: string }; children?: Snippet } = $props();

	const poster = $derived(keyvisual.image ? toonUrl(toonId, keyvisual.image) : undefined);
	const src = $derived(keyvisual.video ? toonUrl(toonId, keyvisual.video) : undefined);

	let video = $state<HTMLVideoElement>();
	let muted = $state(false);
	const reducedMotion =
		typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;
	/** stopped: before the start with reduced motion, after the end – the centre button shows */
	let stopped = $state(reducedMotion);
	let played = $state(false);

	// with sound if the browser lets us, else muted ("Ton an" brings the sound)
	function play(el: HTMLVideoElement) {
		stopped = false;
		el.muted = muted;
		el.play().catch(() => {
			if (el.muted) return;
			muted = el.muted = true;
			el.play().catch(() => {});
		});
	}

	function replay() {
		if (!video) return;
		video.currentTime = 0;
		play(video);
	}

	function toggleSound() {
		muted = !muted;
		if (video) video.muted = muted;
	}

	// while it plays the top row hides when the reader is idle (owner 2026-10-08); it ends → the row is back
	$effect(() => {
		if (video && !stopped) return chromeHidesWhenIdle();
	});

	$effect(() => {
		const el = video;
		if (!el) return;
		// only the element: toggling the sound must not restart the video
		untrack(() => !stopped && play(el));
		// opened in a background tab: the browser holds autoplay back – start once the page shows
		const onVisible = () => document.visibilityState === 'visible' && el.paused && !stopped && !el.ended && play(el);
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
			playsinline
			preload="auto"
			onplay={() => (played = true)}
			onended={() => (stopped = true)}
			data-testid="chapter-teaser-video"
		>
			<source {src} type="video/mp4" />
		</video>
	{:else if poster}
		<img class="teaser-media" src={poster} alt="" />
	{/if}
	{#if src}
		<!-- a tap anywhere on the playing video toggles the sound, like Instagram / TikTok (owner 2026-10-09); keyboard
		     and screen readers use the speaker button -->
		{#if !stopped}
			<button type="button" class="teaser-tap" tabindex="-1" aria-hidden="true" onclick={toggleSound} data-testid="chapter-teaser-tap"></button>
		{/if}
		<button
			type="button"
			class="teaser-sound"
			onclick={toggleSound}
			data-testid="chapter-teaser-sound"
		>
			<Icon name={muted ? 'volume-x' : 'volume-2'} size="1.5rem" strokeWidth={2} label={muted ? m['hero.sound_on']() : m['hero.sound_off']()} />
		</button>
		{#if stopped}
			<button type="button" class="teaser-replay" onclick={replay} data-testid="chapter-teaser-replay">
				{played ? m['player.replay']() : m['player.play']()}
			</button>
		{/if}
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
	/* the whole picture: a tap toggles the sound (under the text, which lets taps through, and the buttons) */
	.teaser-tap {
		position: absolute;
		inset: 0;
		padding: 0;
		border: none;
		background: none;
		cursor: pointer;
		-webkit-tap-highlight-color: transparent;
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
		pointer-events: none; /* a tap on the text toggles the sound too */
	}
	/* like the toon hero's "Ton an" (hero.svelte) */
	.teaser-sound {
		position: absolute;
		z-index: 1; /* above the text's scrim */
		right: 0.5rem;
		bottom: calc(0.5rem + env(safe-area-inset-bottom, 0px));
		display: grid;
		place-items: center;
		width: var(--control-height);
		height: var(--control-height);
		padding: 0;
		border: none;
		background: none;
		color: var(--paper);
		filter: drop-shadow(0 0 0.125rem rgb(0 0 0 / 0.6)); /* reads on a light frame too */
		cursor: pointer;
	}
	/* in the centre once the video has ended (or before it starts with reduced motion) */
	.teaser-replay {
		position: absolute;
		z-index: 1;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		box-sizing: border-box;
		min-height: var(--control-height);
		padding: 0 1rem;
		border: var(--border-width-regular) solid var(--paper);
		background: rgb(0 0 0 / 0.4);
		color: var(--paper);
		font: inherit;
		cursor: pointer;
	}
	.teaser-replay:hover {
		background: rgb(0 0 0 / 0.6);
	}
	.teaser-sound:focus-visible,
	.teaser-replay:focus-visible {
		outline: var(--border-width-regular) solid var(--paper);
		outline-offset: 0.125rem;
	}
</style>
