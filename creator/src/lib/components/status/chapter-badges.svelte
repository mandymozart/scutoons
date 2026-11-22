<script lang="ts">
	import { Status } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { seasonStatusOf } from '$lib/editor/visibility.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { statusWord } from '$lib/editor/status-word.ts';
	import SourceMark from '$lib/components/text/source-mark.svelte';

	/**
	 * A chapter's row in a list (toon page, season editor) says only what is up, as quiet words: its visibility in the
	 * season (draft / coming soon – content, not a save state) and a missing language (a problem, in the error colour),
	 * with the languages when only some are concerned. Saving and publishing show nowhere here: edits save on their
	 * own, the Publish pill's dot says what is not live.
	 */
	let { season, number, doc }: { season: ContentDoc; number: number; doc?: ContentDoc | null } = $props();

	interface Mark {
		word: string;
		missing: boolean;
		locales: string[];
	}

	const marks = $derived.by<Mark[]>(() => {
		const all = season.textLanguages;
		const out: Mark[] = [];
		const add = (word: string, missing: boolean, locale: string) => {
			// a language-neutral document's state concerns every language
			const locales = locale === doc?.primary && doc?.neutral ? all : [locale];
			const found = out.find((i) => i.word === word);
			if (found) found.locales.push(...locales);
			else out.push({ word, missing, locales: [...locales] });
		};
		for (const l of all) {
			const status = seasonStatusOf(season, l, number);
			if (status === Status.DRAFT || status === Status.COMING_SOON) add(statusWord(status), false, l);
		}
		if (doc && !doc.loading) for (const l of doc.locales) if (doc.source[l] === 'missing') add(m['creator.state_missing'](), true, l);
		return out;
	});
	/** the languages only when the mark doesn't concern all of them (the source marked with the star) */
	const which = (mark: Mark) => (season.textLanguages.length > 1 && mark.locales.length < season.textLanguages.length ? [...new Set(mark.locales)] : []);
</script>

{#if marks.length}
	<span class="chapter-marks" data-testid="chapter-badges-{number}">
		{#each marks as mark (mark.word)}
			<span class="mark" class:missing={mark.missing}
				>{mark.word}{#each which(mark) as l, i (l)}{i ? ', ' : ' · '}{l}{#if l === season.textSource}<SourceMark />{/if}{/each}</span
			>
		{/each}
	</span>
{/if}

<style>
	.chapter-marks {
		display: inline-flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: var(--space-05);
	}
	.mark {
		color: var(--text-muted);
		font-size: var(--type-caption-size);
	}
	.mark.missing {
		color: var(--error);
	}
</style>
