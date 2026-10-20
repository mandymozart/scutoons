<script lang="ts">
	// A clip's inside on the timeline (#126; the studio's ClipStrip – owner 2026-10-20: "previews in the timeline, should be
	// not cover, but show the frame after frame, if only on is existing only on the left side of the clip"):
	// - a video (rendered / generated): a filmstrip – tiles at the video's aspect along the clip (the last one cut), each
	//   the frame at its left edge's time, read in the browser (a <video> seeked + a canvas – no ffmpeg); made again 0.3 s
	//   after the clip's size rests (zoom in → more tiles);
	// - a still / a take (only the shot's frame exists): the shot's schematic once at the left.
	// Sizes: a tile's width = its aspect × the strip's height (CSS) – px only to size the canvases.
	import { untrack } from 'svelte';
	import { shotSvg } from '@scutoons/content/storyboard';
	import { svgSource } from '$lib/editor/storyboard-shots.ts';
	import { filmstrip, isVideo, type ClipItem } from '$lib/editor/storyboard-timeline.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';

	let { sb, it }: { sb: StoryboardPage; it: ClipItem } = $props();
	let w = $state(0);
	let h = $state(0);
	/** the frames shown: { aspect, tiles (object URLs) } */
	let strip = $state<{ gen: number; aspect: number; tiles: string[] } | null>(null);
	const gen = $derived(isVideo(it.clip.source) ? (it.clip.gen ?? null) : null);
	const still = $derived.by(() => {
		if (gen !== null) return null;
		const e = sb.entries.find((x) => x.id === it.id);
		return e ? svgSource(shotSvg(e.shot, e.scene).svg) : null;
	});

	$effect(() => {
		const id = gen,
			width = w,
			height = h,
			length = it.length;
		if (id === null || !width || !height) return;
		let gone = false;
		const made: string[] = [];
		const timer = setTimeout(async () => {
			const url = await sb.tl.media(id);
			if (!url || gone) return;
			const v = document.createElement('video');
			v.muted = true;
			v.preload = 'auto';
			v.src = url;
			try {
				await new Promise<void>((ok, no) => ((v.onloadeddata = () => ok()), (v.onerror = () => no(new Error('video')))));
				const dpr = window.devicePixelRatio || 1;
				const times = filmstrip({ width: v.videoWidth, height: v.videoHeight, w: width, h: height, length });
				const th = Math.max(16, Math.round(height * dpr)),
					tw = Math.max(8, Math.round((th * v.videoWidth) / v.videoHeight));
				const c = document.createElement('canvas');
				c.width = tw;
				c.height = th;
				const g = c.getContext('2d')!;
				for (const t of times) {
					if (gone) return;
					await new Promise<void>((ok) => {
						v.onseeked = () => ok();
						v.currentTime = Math.min(t / 1000 + 0.001, Math.max(0, v.duration - 0.01));
					});
					g.drawImage(v, 0, 0, tw, th);
					const blob = await new Promise<Blob | null>((ok) => c.toBlob(ok, 'image/jpeg', 0.7));
					if (blob) made.push(URL.createObjectURL(blob));
				}
				if (!gone) strip = { gen: id, aspect: v.videoWidth / v.videoHeight, tiles: [...made] };
			} catch {
				// a file that can't be decoded here: no frames (the hover and the clip view say so)
			} finally {
				v.removeAttribute('src');
				v.load();
			}
		}, untrack(() => strip?.gen === id) ? 300 : 0);
		return () => {
			gone = true;
			clearTimeout(timer);
			const old = untrack(() => strip);
			setTimeout(() => old && untrack(() => old !== strip) && old.tiles.forEach((u) => URL.revokeObjectURL(u)), 1000);
		};
	});
	const shown = $derived(strip && strip.gen === gen ? strip : null);
</script>

<span class="strip" bind:clientWidth={w} bind:clientHeight={h} aria-hidden="true">
	{#if shown}
		{#each shown.tiles as src, i (i)}<img {src} alt="" style:aspect-ratio={shown.aspect} />{/each}
	{:else if still}
		<img src={still} alt="" class="still" />
	{/if}
</span>

<style>
	.strip {
		position: absolute;
		inset: 0;
		display: flex;
		overflow: hidden;
		pointer-events: none;
	}
	img {
		display: block;
		height: 100%;
		width: auto;
		flex: none;
		object-fit: cover;
	}
	.still {
		background: var(--paper);
	}
</style>
