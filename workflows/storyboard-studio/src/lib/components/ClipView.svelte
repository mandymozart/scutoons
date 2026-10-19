<script>
	// The clip under the playhead in the main area, over the 3D view (which stays mounted underneath – WebGL – App):
	// a rendered / generated video at the playhead's time (owner 2026-10-19: "video over take, but i think it is a choice
	// what to see … in our case, it is shot/take/rendered/generated"), or – for a still / a take of a shot that is not the
	// current one (a video mode does not change the current shot; a drag across other shots) – that shot's frame. Paused it
	// shows the frame at the playhead (a seek); while Play runs the video plays and is the transport's clock
	// (timeline.svelte.js media). No native controls: the timeline is the transport.
	import { ch, studio } from '../studio.svelte.js';
	import { isVideo } from '../timeline-lanes.js';
	import { media } from '../timeline.svelte.js';

	/** @type {{ clip: { item: any, off: number }, playing: boolean }} */
	let { clip, playing } = $props();
	let el = $state();
	/** the url whose file could not be played (missing, a codec the browser lacks) – the clock then runs by the frame */
	let failed = $state('');
	const c = $derived(clip.item.clip);
	const url = $derived(isVideo(c.source) ? c.url : null);
	/** where the media is (s): its start + the playhead's place in the clip */
	const at = $derived((c.from + clip.off) / 1000);
	const what = $derived(url ? `${clip.item.id} · ${c.source} · ${c.file ?? ''}${failed === url ? ' – the file can\'t be played' : ''}` : `${clip.item.id} · ${c.source} – its shot is not the current one (a click on the clip edits it)`);

	$effect(() => {
		const v = el;
		media.video = v ?? null;
		return () => { if (media.video === v) media.video = null; };
	});
	// Play: the video runs (a new clip starts when it can); paused: it holds
	$effect(() => {
		if (!el || !url) return;
		if (playing) el.play().catch(() => { /* the transport's clock goes on by the frame (clockAt) */ });
		else el.pause();
	});
	// paused: the frame at the playhead (a seek; before its metadata it is the start position)
	$effect(() => {
		const t = at;
		if (!el || !url || playing) return;
		if (Math.abs(el.currentTime - t) > 0.02) el.currentTime = t;
	});
</script>

<section class="clip-view" aria-label="Clip">
	<div class="tools"><span class="cap" title={what}>{what}</span></div>
	<div class="stage">
		{#if url}
			<!-- svelte-ignore a11y_media_has_caption -->
			<video bind:this={el} src={url} data-url={url} preload="auto" playsinline onerror={() => (failed = url)}></video>
		{:else}
			<img src={`/out/${ch()}/frames/${clip.item.id}.png?v=${studio.version}`} alt={clip.item.id} />
		{/if}
	</div>
</section>

<style>
	/* over the 3D view (App's main), the same bar on top */
	.clip-view {
		position: absolute;
		inset: 0;
		z-index: 1;
		display: flex;
		flex-direction: column;
		min-height: 0;
		background: var(--paper);
	}
	.tools {
		display: flex;
		align-items: center;
		min-height: var(--control-height);
		padding: var(--space-025) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	.cap {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--type-caption-size);
	}
	.stage {
		position: relative;
		flex: 1;
		min-height: 0;
		background: var(--ink);
		overflow: hidden;
	}
	/* fitted into the stage, whole */
	video,
	img {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		object-fit: contain;
	}
</style>
