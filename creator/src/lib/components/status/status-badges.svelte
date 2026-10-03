<script lang="ts" module>
	import { Status } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';

	/** A chapter status as a plain word ("published", "draft", "coming soon") in the UI language. */
	export function statusWord(status: string): string {
		if (status === Status.PUBLISHED) return m['creator.status_published']();
		if (status === Status.COMING_SOON) return m['creator.status_coming_soon']();
		if (status === Status.DRAFT) return m['creator.status_draft']();
		return status;
	}
</script>

<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { seasonStatusOf } from '$lib/editor/visibility.ts';

	/**
	 * A chapter's status per language as read-only badges (lists: toon page, season editor). The status is changed only
	 * on the chapter page (meta panel); the season's sequence is its one source. Owner 2026-10-02: the colour is the
	 * status – the chip shows only the language (a colour square for a one-language toon), the word is the tooltip.
	 */
	let { season, number }: { season: ContentDoc; number: number } = $props();
</script>

<span class="status-badges" data-testid="status-badges-{number}">
	{#each season.locales as l (l)}
		{@const status = seasonStatusOf(season, l, number)}
		<!-- owner 2026-10-03: only what is not published (draft, coming soon); published shows nothing -->
		{#if status && status !== Status.PUBLISHED}
			<span
				class="st st-{status}"
				class:square={season.locales.length < 2}
				title={m['creator.status_badge_hint']({ locale: l, status: statusWord(status) })}
				aria-label={m['creator.status_badge_hint']({ locale: l, status: statusWord(status) })}
				data-testid="status-{number}-{l}"
				data-status={status}
			>
				{#if season.locales.length > 1}{l}{/if}
			</span>
		{/if}
	{/each}
</span>

<style>
	/* filled badges (tokens.css --badge-*, DESIGN.md "Brutalist tools"): published · coming soon · draft */
	.status-badges {
		display: inline-flex;
		flex-wrap: wrap;
		gap: var(--space-025);
	}
	.st {
		padding: 0.1em 0.5em;
		background: var(--badge-draft);
		color: var(--badge-draft-text);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		white-space: nowrap;
	}
	.st.square {
		width: 0.75rem;
		height: 0.75rem;
		padding: 0;
		align-self: center;
	}
	.st-published {
		background: var(--badge-published);
		color: var(--badge-published-text);
	}
	.st-coming-soon {
		background: var(--badge-soon);
		color: var(--badge-soon-text);
	}
</style>
