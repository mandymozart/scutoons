<script lang="ts">
	// Dev-only draft of the record player (owner 2026-10-14: "play the whole album – like a little playlist"): three short
	// local sounds as an "album", each "signed" a moment later like a vault track.
	import RecordPlayer from '$lib/components/items/record-player.svelte';
	import { asset } from '$lib/utils/cdn.ts';

	const isDev = import.meta.env.DEV;
	const tracks = [
		{ src: 'catpope/s1/c1/s1c1-bells.128.mp3', title: 'Bells' },
		{ src: 'catpope/s1/c1/s1c1-choir-loud-loud-loud.128.mp3', title: 'Choir' },
		{ src: 'catpope/s1/c1/s1c1-bells.96.mp3', title: 'Bells again' }
	];
	let open = $state(false);
</script>

{#if isDev}
	<div class="lab">
		<h1>Record player (draft)</h1>
		<button class="btn-secondary" onclick={() => (open = true)} data-testid="lab-player-open">Play the album</button>
	</div>
	{#if open}
		<RecordPlayer
			request={{
				tracks,
				title: 'Awakening',
				entry: null,
				cover: 'catpope/s1/s1-music-vinyl.webp',
				sign: (i) => new Promise((done) => setTimeout(() => done(asset(tracks[i].src)), 150))
			}}
			onclose={() => (open = false)}
		/>
	{/if}
{/if}

<style>
	.lab {
		padding: var(--space-1);
	}
</style>
