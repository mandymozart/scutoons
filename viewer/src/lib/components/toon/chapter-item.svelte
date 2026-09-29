<script lang="ts">
	import { goto } from '$app/navigation';
	import { sanitizeNumber, isItemPublic } from '@scutoons/content';
	import { asset } from '$lib/utils/cdn.ts';
	import type { LoadedSequenceItem } from '@scutoons/content';
	import { getNavigationContext } from '$lib/contexts/navigation.svelte.ts';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import chapterIcon from '$lib/icons/chapter.svg';

	let {
		chapter,
		chapterNumber,
		toonId
	}: { chapter: LoadedSequenceItem; chapterNumber: number; toonId: string } = $props();

	const nav = getNavigationContext();
	const safeChapterNumber = sanitizeNumber(chapterNumber, 1, 1, 999);
	// `chapter.status` is the season's status of this locale (see the toon layout), never the chapter file's
	const isPublic = isItemPublic(chapter, import.meta.env.DEV);
	const status = chapter.status ?? 'published';

	function handleClick() {
		if (!isPublic) return;
		nav.closeOverlay();
		goto(localizeHref(`/${toonId}/chapter/${safeChapterNumber}`));
	}
</script>

<div
	class={`chapter-item ${status}`}
	onclick={handleClick}
	onkeydown={handleClick}
	role="link"
	aria-disabled={!isPublic}
	tabindex="0"
>
	<div class="chapter-image">
		<img
			class="chapter-thumbnail"
			src={asset(`${toonId}/${chapter.thumbnail || 'images/default-thumbnail.jpg'}`)}
			alt={`Kapitel ${chapter.number} Thumbnail`}
			loading="lazy"
		/>
	</div>
	<div class="chapter-info">
		<div class="chapter-title">{chapter.title}</div>
		<div class="chapter-release">
			{#if !isPublic}
				<span class="chapter-release-badge">Coming Soon</span>
			{/if}
			<span class="chapter-release-date">{chapter.release}</span>
		</div>
	</div>
	<div class="chapter-number">
		{isPublic ? ' ' : ''}#{chapter.number}
	</div>
</div>

<style>
	.chapter-item {
		display: grid;
		grid-template-columns: 6rem auto 3rem;
		align-items: center;
		gap: 1rem;
		cursor: pointer;
	}

	.chapter-item .chapter-info {
		display: grid;
		grid-template-columns: auto 7rem;
		align-items: center;
		gap: 0.5rem;
	}

	.chapter-item img {
		width: 100%;
		height: auto;
	}

	.chapter-item .chapter-thumbnail {
		width: 100%;
		height: auto;
	}

	.chapter-item .chapter-release {
		font-size: 0.75rem;
	}

	.chapter-item .chapter-release,
	.chapter-item .chapter-number {
		color: #6b6b6b;
	}

	.chapter-item .chapter-number {
		justify-content: space-between;
		display: flex;
		align-items: center;
	}

	.chapter-item:hover {
		background-color: #f0f0f0;
	}

	.chapter-item .chapter-release span {
		display: block;
	}

	.chapter-item.published .chapter-release span {
		display: none;
	}

	.chapter-item.coming-soon {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.chapter-item.coming-soon .chapter-release .chapter-release-badge {
		font-size: 0.5rem;
	}

	@media (max-width: 768px) {
		.chapters-header,
		.chapters-list {
			padding: 0 1rem;
		}

		.chapter-item {
			grid-template-columns: 5rem auto 3rem;
			gap: 0.5rem;
		}

		.chapter-item .chapter-info {
			display: block;
		}

		.chapter-item .chapter-release span {
			display: inline;
		}

		.chapter-item.coming-soon .chapter-release .chapter-release-badge {
			font-size: 0.75rem;
			margin-right: 0.5rem;
		}
	}
</style>
