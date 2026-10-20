<script lang="ts">
	// The clip under the timeline's playhead in the main view, over the 3D view (which stays mounted underneath) – #126,
	// the studio's ClipView (owner 2026-10-19: "in our case, it is shot/take/rendered/generated"): a rendered / generated
	// video at the playhead's time. Paused it shows the frame at the playhead (a seek); while Play runs the video plays and
	// is the transport's clock (TimelineState.video). No native controls: the timeline is the transport. A take or a still
	// is the 3D view's (nothing here).
	import { m } from '$lib/paraglide/messages.js';
	import { isVideo } from '$lib/editor/storyboard-timeline.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';

	let { sb }: { sb: StoryboardPage } = $props();
	const tl = $derived(sb.tl);
	let el = $state<HTMLVideoElement>();
	let failed = $state<number | null>(null);
	// the Variants board (#125) has the main view to itself
	const c = $derived(!tl.here.folded && !sb.board.open && tl.under && isVideo(tl.under.item.clip.source) ? tl.under : null);
	const gen = $derived(c?.item.clip.gen ?? null);
	const src = $derived(gen !== null ? (tl.urls[gen] ?? null) : null);
	/** where the media is (s) */
	const at = $derived(c ? (c.item.clip.from + c.off) / 1000 : 0);

	$effect(() => {
		if (gen !== null) void tl.media(gen);
	});
	$effect(() => {
		const v = el ?? null;
		tl.video = v;
		return () => {
			if (tl.video === v) tl.video = null;
		};
	});
	$effect(() => {
		if (!el || !src) return;
		if (tl.playing) el.play().catch(() => undefined);
		else el.pause();
	});
	$effect(() => {
		const t = at;
		if (!el || !src || tl.playing) return;
		if (Math.abs(el.currentTime - t) > 0.02) el.currentTime = t;
	});
	const what = $derived(c ? `${c.item.label}${gen !== null && tl.missing[gen] ? ` – ${tl.missing[gen]}` : failed === gen ? ` – ${m['storyboard.tl_cant_play']()}` : ''}` : '');
</script>

{#if c}
	<section class="clip-view" aria-label={m['storyboard.tl_clip']()} data-testid="clip-view">
		<p class="cap sc-caption" title={what}>{what}</p>
		{#if src}
			<!-- svelte-ignore a11y_media_has_caption -->
			<video bind:this={el} {src} data-gen={gen} preload="auto" playsinline muted onerror={() => (failed = gen)}></video>
		{/if}
	</section>
{/if}

<style>
	.clip-view {
		position: absolute;
		inset: 0;
		z-index: 1;
		display: flex;
		flex-direction: column;
		background: var(--ink);
	}
	.cap {
		margin: 0;
		padding: var(--space-025) var(--space-1);
		background: var(--paper);
		border-bottom: var(--border-width-regular) solid var(--ink);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	video {
		flex: 1;
		min-height: 0;
		width: 100%;
		object-fit: contain;
	}
</style>
