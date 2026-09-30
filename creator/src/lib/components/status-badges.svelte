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
	 * on the chapter page (meta panel); the season's sequence is its one source.
	 */
	let { season, number, compact = false }: { season: ContentDoc; number: number; compact?: boolean } = $props();
</script>

<span class="status-badges" data-testid="status-badges-{number}">
	{#each season.locales as l (l)}
		{@const status = seasonStatusOf(season, l, number)}
		{#if status}
			<span class="st st-{status}" title={m['creator.status_badge_hint']({ locale: l, status: statusWord(status) })} data-testid="status-{number}-{l}">
				{#if season.locales.length > 1}<b>{l}</b>{/if}{compact ? '' : ` ${statusWord(status)}`}
			</span>
		{/if}
	{/each}
</span>

<style>
	.status-badges {
		display: inline-flex;
		flex-wrap: wrap;
		gap: 0.25rem;
	}
	.st {
		font-size: 10px;
		line-height: 1.5;
		padding: 0 0.4em;
		border: 1px solid var(--border-muted);
		white-space: nowrap;
	}
	.st b {
		font-weight: 700;
	}
	.st-published {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
	.st-coming-soon {
		border-style: dashed;
		border-color: var(--ink);
	}
</style>
