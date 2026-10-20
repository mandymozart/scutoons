<script>
	// A clip's inside on the studio's timeline (Timeline.svelte's item; owner 2026-10-20: "previews in the timeline, should
	// be not cover, but show the frame after frame, if only on is existing only on the left side of the clip"):
	// - a video (rendered / generated): a filmstrip – tiles at the video's aspect one after another along the clip (the
	//   last one cut), each the frame at its left edge's time. The frames come from ONE sprite (GET /api/thumbs, clips.mjs:
	//   ffmpeg, cached on disk); the server picks how many (a step that covers the clip), a tile shows the sprite's latest
	//   frame at or before its time – zoom in → more tiles, past a step a denser sprite (asked 0.3 s after the size rests);
	// - a still / a take (only the shot's frame exists): that frame once at the left, the rest the item's own background;
	// - no sprite (no ffmpeg, a broken file): the video's first frame at the left (the hover's), else the shot's frame.
	// Sizes: a tile's width = its aspect × the strip's height (CSS) – px only in the sprite request.
	import { untrack } from 'svelte';
	import { isVideo } from '../timeline-lanes.js';
	import { tl } from '../timeline.svelte.js';

	/** @type {{ it: { length: number, clip: { source: string, url?: string, probe?: boolean } }, frame: string }} */
	let { it, frame } = $props();
	let w = $state(0);
	let h = $state(0);
	/** the sprite on screen: { of (its video's url), url, n, tw, th } */
	let strip = $state(null);
	/** the video whose sprite failed (→ its first frame), the video that failed itself too (→ the shot's frame) */
	let failed = $state(null);
	let dead = $state(null);
	const video = $derived(isVideo(it.clip.source) ? it.clip.url : null);
	/** its length for the server when the file doesn't say (0: unknown yet – a video still probed) */
	const ms = $derived(it.clip.probe ? 0 : Math.round(it.length));
	const shown = $derived(video && strip?.of === video ? strip : null);
	/** the tiles that fit the clip: each one's sprite tile as a background position (%) */
	const tiles = $derived.by(() => {
		if (!shown || !w || !h) return [];
		const r = w / ((h * shown.tw) / shown.th);
		return Array.from({ length: Math.min(2000, Math.ceil(r)) }, (_, i) => {
			const j = Math.min(shown.n - 1, Math.floor((i * shown.n) / r + 1e-6));
			return shown.n > 1 ? (j / (shown.n - 1)) * 100 : 0;
		});
	});

	$effect(() => {
		const url = video;
		// the clips loaded again (a render, a generator, a save): ask again – a re-rendered take keeps its file name, the
		// server knows it by its mtime (an unchanged file: the cached sprite at once)
		void tl.clips;
		if (!url || !w || !h) return;
		const dpr = window.devicePixelRatio || 1;
		const q = new URLSearchParams({ url, w: String(Math.ceil(w * dpr)), h: String(Math.ceil(h * dpr)) });
		if (ms) q.set('ms', String(ms));
		const stop = new AbortController();
		const timer = setTimeout(() => load(url, q, stop.signal), untrack(() => strip?.of === url || failed === url) ? 300 : 0);
		return () => {
			clearTimeout(timer);
			stop.abort();
		};
	});
	async function load(url, q, signal) {
		try {
			const r = await fetch(`/api/thumbs?${q}`, { signal });
			const a = await r.json();
			if (!r.ok || a.error) throw new Error(a.error ?? r.statusText);
			// decoded before it replaces the one on screen (no blank between two sprites)
			if (a.url !== strip?.url) {
				const img = new Image();
				img.src = a.url;
				await img.decode();
			}
			if (!signal.aborted) strip = { of: url, ...a };
		} catch {
			if (!signal.aborted && strip?.of !== url) failed = url;
		}
	}
	// a frame that is still missing stays blank (the frames are rendered by "All frames" / Save + frame)
	const missing = (e) => e.currentTarget.classList.add('missing');
</script>

<div class="strip" bind:clientWidth={w} bind:clientHeight={h}>
	{#if shown}
		<div class="film" style:--sprite="url('{shown.url}')" style:--aspect="{shown.tw} / {shown.th}" style:--tiles={shown.n}>
			{#each tiles as x, i (i)}<span style:background-position-x="{x}%"></span>{/each}
		</div>
	{:else if video && failed === video && dead !== video}
		<video src="{video}#t=0.001" preload="metadata" muted playsinline onerror={() => (dead = video)}></video>
	{:else if !video || dead === video}
		<img src={frame} alt="" loading="lazy" draggable="false" onerror={missing} onload={(e) => e.currentTarget.classList.remove('missing')} />
	{/if}
</div>

<style>
	.strip {
		position: absolute;
		inset: 0;
		overflow: hidden;
		pointer-events: none;
	}
	.film {
		display: flex;
		height: 100%;
	}
	/* one tile: the sprite at n × its width, shifted to its frame; a paper hairline between two frames */
	.film > span {
		flex: none;
		box-sizing: border-box;
		height: 100%;
		aspect-ratio: var(--aspect);
		background: var(--sprite) no-repeat;
		background-size: calc(var(--tiles) * 100%) 100%;
	}
	.film > span + span {
		border-left: var(--border-width-hairline) solid var(--paper);
	}
	/* one frame at the left at its own aspect – never stretched over the clip */
	img,
	video {
		position: absolute;
		left: 0;
		top: 0;
		display: block;
		width: auto;
		max-width: none;
		height: 100%;
	}
	img:global(.missing) {
		visibility: hidden;
	}
</style>
