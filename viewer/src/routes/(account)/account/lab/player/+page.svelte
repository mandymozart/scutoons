<script lang="ts">
	// Dev-only draft of the record player (owner 2026-10-14: "play the whole album – like a little playlist"): five short
	// local sounds as an "album" named like Awakening's songs (sides A and B) (so the card finds their credits in credits:catpope), each
	// "signed" a moment later like a vault track.
	import RecordPlayer from '$lib/components/items/record-player.svelte';
	import { asset } from '$lib/utils/cdn.ts';

	const isDev = import.meta.env.DEV;
	const tracks = [
		{ id: 'roar_little_kitty', src: 'catpope/s1/c1/s1c1-bells.128.mp3', title: 'Roar Little Kitty', duration: 475 },
		{ id: 'cloud_9', src: 'catpope/s1/c1/s1c1-choir-loud-loud-loud.128.mp3', title: 'Cloud 9', duration: 417 },
		{ id: 'not_my_city', src: 'catpope/s1/c1/s1c1-bells.96.mp3', title: 'Not My City', duration: 362, side: 'A' as const },
		{ id: 'no_he_said', src: 'catpope/s1/c1/s1c1-choir-loud-loud-loud.128.mp3', title: 'No He Said', duration: 499, side: 'B' as const },
		{ id: 'once_twice_thrice', src: 'catpope/s1/c1/s1c1-bells.128.mp3', title: 'Once Twice Thrice', duration: 528, side: 'B' as const }
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
				toon: 'catpope',
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
