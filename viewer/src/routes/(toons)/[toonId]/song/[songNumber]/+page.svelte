<script lang="ts">
	import MembershipBlocker from '$lib/components/chapter/membership-blocker.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import { page } from '$app/state';
	import type { PageProps } from './$types.js';

	let { data }: PageProps = $props();
	let song = $state(data.song);
	$inspect(page.params);
	$effect(() => {
		song = data.song;
	});
	let videoId = $derived(String(song.video?.sources?.youtube?.id).trim());
	let hasVideo = $derived(/^[a-zA-Z0-9_-]{11}$/.test(String(song.video?.sources?.youtube?.id).trim()));
	let audioAutoplay = $derived(!hasVideo);
	let audioSrc = $derived(asset(`${page.params.toonId}/${song.src}`))
</script>

<MembershipBlocker songNumber={song.number} isPreview={song.isPreview}>
	<div class="song-view">
		{#if hasVideo}
			<div class="song-video">
				<iframe
					width="100%"
					height="315"
					title="Video"
					src={`https://www.youtube.com/embed/${videoId}?autoplay=1`}
					frameborder="0"
					allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
					allowfullscreen
				>
				</iframe>
			</div>
		{/if}
		<div class="song-header">
			<div class="song-title">{song.title}</div>
			<div class="song-description">{song.description}</div>
			{#if song.src}
				<div class="song-src">
					<audio controls autoplay={audioAutoplay}><source src={audioSrc} type="audio/mpeg" /></audio>
				</div>
			{/if}
		</div>
	</div>
</MembershipBlocker>

<style>

.song-view {
  max-width: 100%;
  padding: 0;
  min-height: calc(100vh - 5rem);
  display: flex;
  flex-direction: column;
  justify-content: space-between;
}
.song-header {
  font-size: 1rem;
  margin-bottom: 1rem;
  display: flex;
  flex-direction: column;
  align-items: center;
  flex: 1;
  justify-content: center;
}
.song-title {
	font-size: 1rem;
	margin-bottom: 1rem;
}
.song-description {
	font-size: 1rem;
	font-style: italic;
	border: 2px solid;
	padding: 1rem;
	margin: 0 1rem 1rem 1rem;
}
</style>
