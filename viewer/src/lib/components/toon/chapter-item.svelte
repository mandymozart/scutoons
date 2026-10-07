<script lang="ts">
	import { goto } from '$app/navigation';
	import { sanitizeNumber, isPrologue, chapterProgress } from '@scutoons/content';
	import { getGameContext } from '$lib/contexts/game.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import type { LoadedSequenceItem } from '@scutoons/content';
	import { getNavigationContext } from '$lib/contexts/navigation.svelte.ts';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import chapterIcon from '$lib/icons/chapter.svg';
	import { m } from '$lib/paraglide/messages.js';
	import { chapterName, chapterShownNumber } from '$lib/utils/chapter-name.ts';
	import { isReadable } from '$lib/draft-preview.svelte.ts';

	let {
		chapter,
		chapterNumber,
		toonId
	}: { chapter: LoadedSequenceItem; chapterNumber: number; toonId: string } = $props();

	const nav = getNavigationContext();
	// chapter 0 is the season's prologue
	const safeChapterNumber = sanitizeNumber(chapterNumber, 1, 0, 999);
	// `chapter.status` is the season's status of this locale (see the toon layout), never the chapter file's
	const isPublic = isReadable(chapter);
	const status = chapter.status ?? 'published';

	// how far this chapter was read (guests: this browser, members: synced), agents/DESIGN.md "Chapter list progress"
	const game = getGameContext();
	const progress = $derived(isPublic ? chapterProgress(toonId, chapter.number, game.state.progress) : null);
	// only a bar (owner 2026-09-30): a finished chapter = a full ink bar, one being read = how far
	const percent = $derived(!progress ? 0 : progress.finished ? 100 : Math.max(1, Math.round(progress.position * 100)));
	const shown = $derived(!!progress && (progress.finished || progress.position > 0));

	// a coming-soon chapter with a key visual opens its page: the teaser (owner 2026-10-03)
	const hasTeaser = $derived.by(() => {
		const kv = chapter.keyvisual as { image?: string; video?: string } | string | undefined;
		return chapter.status === 'coming-soon' && !!(typeof kv === 'string' ? kv : kv?.image || kv?.video);
	});
	const opens = $derived(isPublic || hasTeaser);

	function handleClick() {
		if (!opens) return;
		nav.closeOverlay();
		goto(localizeHref(`/${toonId}/chapter/${safeChapterNumber}`));
	}
</script>

<div
	class={`chapter-item ${status}`}
	class:teaser={hasTeaser}
	onclick={handleClick}
	onkeydown={handleClick}
	role="link"
	aria-disabled={!opens}
	tabindex="0"
>
	<div class="chapter-image">
		<!-- no thumbnail = the empty frame (the old fallback images/default-thumbnail.jpg never existed on the CDN) -->
		{#if chapter.thumbnail}
			<img
				class="chapter-thumbnail"
				src={asset(`${toonId}/${chapter.thumbnail}`)}
				alt={`${chapterName(chapter.number)} Thumbnail`}
				loading="lazy"
			/>
		{/if}
	</div>
	<div class="chapter-info">
		<div class="chapter-title">
			{chapter.title}
		</div>
		<div class="chapter-release">
			{#if !isPublic}
				<span class="chapter-release-badge">Coming Soon</span>
			{/if}
			<span class="chapter-release-date">{chapter.release}</span>
		</div>
		{#if shown}
			<span class="chapter-progress" aria-hidden="true" data-testid="chapter-progress" data-percent={percent}
				><span style:width="{percent}%"></span></span
			>
		{/if}
	</div>
	<div class="chapter-number" data-testid="chapter-number">
		{#if isPrologue(chapter.number)}<span class="chapter-label">{m['chapter.prologue']()}</span>{:else}{isPublic ? ' ' : ''}#{chapterShownNumber(chapter.number) ?? chapter.number}{/if}
	</div>
</div>

<style>
	.chapter-item {
		display: grid;
		/* the number column: 3rem, wider only for a label ("Prolog") */
		grid-template-columns: 6rem 1fr auto;
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

	/* progress: a 2px rule under the title, the read part in ink (a finished chapter: full) */
	.chapter-progress {
		grid-column: 1 / -1;
		display: block;
		max-width: 8rem;
		height: var(--border-width-regular);
		margin-top: 0.25rem;
		background: var(--border-muted);
	}
	.chapter-progress span {
		display: block;
		height: 100%;
		background: var(--ink);
	}

	.chapter-item .chapter-release,
	.chapter-item .chapter-number {
		color: #6b6b6b;
	}

	.chapter-item .chapter-number {
		min-width: 3rem;
		justify-content: space-between;
		display: flex;
		align-items: center;
	}

	/* a word instead of "#n" (the prologue): caption size, so it fits the number column on a phone */
	.chapter-item .chapter-label {
		font-size: 0.75rem;
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
	/* a coming-soon chapter with a teaser opens it: greyed until hovered, a link pointer (owner 2026-10-03) */
	.chapter-item.coming-soon.teaser {
		cursor: pointer;
	}
	.chapter-item.coming-soon.teaser:hover,
	.chapter-item.coming-soon.teaser:focus-visible {
		opacity: 1;
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
			grid-template-columns: 5rem 1fr auto;
			gap: 0.5rem;
			/* the number column must not touch the screen edge ("Prologue") */
			padding-right: 0.5rem;
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
