<script lang="ts">
	import { goto } from '$app/navigation';
	import { sanitizeNumber, isItemPublic } from '$lib/utils/template.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import type { Song } from '$lib/models/content.ts';
	import { getContext, setContext } from 'svelte';
	import headphonesIcon from '$lib/../../static/icons/headphones.svg';

	let { song, songNumber, toonId }: { song: Song; songNumber: number; toonId: string } = $props();

	const safeSongNumber = sanitizeNumber(songNumber, 1, 1, 999);
	const isPublic = isItemPublic(song);
	const status = song.status ?? 'published';

	let navigate: (url: string, opts?: any) => void = getContext('navigation');
	function handleClick() {
		if (!isPublic) return;
		navigate(`/${toonId}/song/${safeSongNumber}`);
	}
</script>

<div
	class={`song-item ${status}`}
	onclick={handleClick}
	onkeydown={handleClick}
	role="link"
	tabindex="0"
>
	<div class="song-image">
		<img
			class="song-thumbnail"
			src={asset(`${toonId}/${song.thumbnail || 'images/default-thumbnail.jpg'}`)}
			alt={`${song.title} Thumbnail`}
			loading="lazy"
		/>
	</div>
	<div class="song-info">
		<div class="song-title">{song.title}</div>
		<div class="song-release">
			{#if !isPublic}
				<span class="song-release-badge">Coming Soon</span>
			{/if}
			<span class="song-release-date">{song.release}</span>
		</div>
	</div>
	<div class="song-number">
		<img class="song-icon" src={headphonesIcon} alt="Song" />
		{isPublic ? ' ' : ''}#{song.number}
	</div>
</div>

<style>
	.song-item {
		display: grid;
		grid-template-columns: 6rem auto 3rem;
		align-items: center;
		gap: 1rem;
		cursor: pointer;
	}

	.song-item .song-info {
		display: grid;
		grid-template-columns: auto 7rem;
		align-items: center;
		gap: 0.5rem;
	}

	.song-item img {
		width: 100%;
		height: auto;
	}

	.song-item .song-thumbnail {
		width: 100%;
		height: auto;
	}

	.song-item .song-icon {
		width: 1rem;
		height: auto;
	}

	.song-item .song-release {
		font-size: 0.75rem;
	}

	.song-item .song-release,
	.song-item .song-number {
		color: #6b6b6b;
	}

	.song-item .song-number {
		justify-content: space-between;
		display: flex;
		align-items: center;
	}

	.song-item:hover {
		background-color: #f0f0f0;
	}

	.song-item .song-release span {
		display: block;
	}

	.song-item.published .song-release span {
		display: none;
	}

	.song-item.coming-soon {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.song-item.coming-soon .song-release .song-release-badge {
		font-size: 0.5rem;
	}

	@media (max-width: 768px) {
		.songs-header,
		.songs-list {
			padding: 0 1rem;
		}

		.song-item {
			grid-template-columns: 5rem auto 3rem;
			gap: 0.5rem;
		}

		.song-item .song-info {
			display: block;
		}

		.song-item .song-release span {
			display: inline;
		}

		.song-item.coming-soon .song-release .song-release-badge {
			font-size: 0.75rem;
			margin-right: 0.5rem;
		}
	}
</style>
