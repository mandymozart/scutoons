<script lang="ts">
	import { Badge } from '@scutoons/ui';
	import { Status } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { seasonStatusOf } from '$lib/editor/visibility.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { statusWord } from './status-badges.svelte';

	/**
	 * A chapter's row in a list (toon page, season editor) says only what is up (owner 2026-10-02: "only show badges when
	 * something is up", "just the dot is a bit unclear"): filled badges with a word – visibility draft / coming soon (the
	 * season's sequence), not published, not saved, missing – with the languages when only some are concerned
	 * ("Draft · ko"). Everything published and saved: nothing. Any number of languages.
	 */
	let { season, number, doc }: { season: ContentDoc; number: number; doc?: ContentDoc | null } = $props();

	type Tone = 'draft' | 'soon' | 'info' | 'warning' | 'error';
	interface Issue {
		word: string;
		tone: Tone;
		locales: string[];
	}

	const issues = $derived.by<Issue[]>(() => {
		const all = season.locales;
		const out: Issue[] = [];
		const add = (word: string, tone: Tone, locale: string) => {
			const found = out.find((i) => i.word === word && i.tone === tone);
			if (found) found.locales.push(locale);
			else out.push({ word, tone, locales: [locale] });
		};
		for (const l of all) {
			const status = seasonStatusOf(season, l, number);
			if (status === Status.DRAFT) add(statusWord(status), 'draft', l);
			else if (status === Status.COMING_SOON) add(statusWord(status), 'soon', l);
		}
		if (doc && !doc.loading) {
			for (const l of doc.locales) {
				const source = doc.source[l];
				if (source === 'missing') add(m['creator.state_missing'](), 'error', l);
				else if (source === 'new' || doc.dirtyLocales.includes(l)) add(m['creator.not_saved'](), 'warning', l);
				else if (doc.published[l] == null || doc.base[l] !== doc.published[l]) add(m['creator.state_unpublished'](), 'info', l);
			}
		}
		return out;
	});
	/** the languages only when the issue doesn't concern all of them */
	const which = (issue: Issue) => (season.locales.length > 1 && issue.locales.length < season.locales.length ? ` · ${issue.locales.join(', ')}` : '');
</script>

{#if issues.length}
	<span class="chapter-badges" data-testid="chapter-badges-{number}">
		{#each issues as issue (issue.word + issue.tone)}
			<Badge tone={issue.tone}><span class="word">{issue.word}{which(issue)}</span></Badge>
		{/each}
	</span>
{/if}

<style>
	.chapter-badges {
		display: inline-flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: var(--space-025);
	}
	.word {
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
</style>
