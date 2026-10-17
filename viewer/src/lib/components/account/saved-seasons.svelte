<script lang="ts">
	/**
	 * Account page "Saved chapters": a season of which some chapters are saved here and more could be (PWA phase 3,
	 * lib/offline/save-season.svelte.ts, owner 2026-10-16 "alle Pakete angehen") – "Catpope · Season 1 · 2 of 5 saved ·
	 * Save the rest". The toons' season lists come from the library (the public toon request, cached); offline or without
	 * an answer the rows simply aren't there.
	 */
	import type { LoadedSequenceItem } from '@scutoons/content';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { fetchToon } from '$lib/api.ts';
	import { isListed, isReadable } from '$lib/draft-preview.svelte.ts';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { savedChapters } from '$lib/offline/saved-chapters.svelte.ts';
	import { seasonSaves } from '$lib/offline/save-season.svelte.ts';
	import { accessEnd } from '$lib/offline/auto-save.svelte.ts';
	import { seasonState } from '$lib/sw/saved-update-rules.ts';
	import { chapterName, type ToonSeason } from '$lib/utils/chapter-name.ts';

	const membership = getMembershipContext();
	const session = getSessionContext();

	/** the toons with saves: their title and seasons (asked once per toon) */
	let toons = $state<Record<string, { title: string; seasons: ToonSeason[] }>>({});
	const asked = new Set<string>();
	$effect(() => {
		for (const toon of new Set(savedChapters.list.map((s) => s.toon))) {
			if (asked.has(toon)) continue;
			asked.add(toon);
			fetchToon(toon, getLocale())
				.then((bundle) => {
					const seasons: ToonSeason[] = bundle.seasons
						.map((o) => ({ number: o.number, season: o.season, sequence: o.season?.sequence ?? [], chapters: o.chapters.filter((item) => isListed(item)) }))
						.sort((a, b) => a.number - b.number);
					toons = { ...toons, [toon]: { title: bundle.toon.title ?? toon, seasons } };
				})
				.catch(() => {});
		}
	});

	const rows = $derived(
		Object.entries(toons).flatMap(([toon, { title, seasons }]) =>
			seasons.flatMap((season) => {
				const chapters = season.chapters.map((c: LoadedSequenceItem) => ({
					number: c.number,
					title: c.title ?? '',
					label: chapterName(c.number, seasons),
					readable: isReadable(c) && (!!c.isPreview || membership.isValid)
				}));
				const saved = (n: number) => !!savedChapters.get(toon, n);
				const readable = chapters.filter((c) => c.readable);
				const count = readable.filter((c) => saved(c.number)).length;
				// only a season begun here and not finished – a running one shows its progress above
				if (!count || seasonState(chapters, saved) !== 'partial' || seasonSaves.job(toon, season.number)) return [];
				return [{ toon, title, season: season.number, chapters, count, of: readable.length }];
			})
		)
	);

	/** "Save the rest"; failed → what went wrong, "Try again" → once more */
	async function saveRest(row: (typeof rows)[number]): Promise<void> {
		const result = await seasonSaves
			.start({
				toon: row.toon,
				toonTitle: row.title,
				season: row.season,
				chapters: row.chapters,
				locale: getLocale(),
				accessUntil: accessEnd(membership, session)
			})
			.catch(() => 'failed' as const);
		if (result === 'failed' && (await seasonSaves.explain(row.toon, row.season))) return saveRest(row);
	}
</script>

{#each rows as row (row.toon + '/' + row.season)}
	<p class="season" data-testid="saved-season" data-toon={row.toon} data-season={row.season}>
		<span class="entry">
			{row.title} · {m['saved.season_name']({ number: row.season })}
			<small>{m['saved.season_count']({ saved: row.count, count: row.of })}</small>
		</span>
		<button
			type="button"
			class="btn-secondary"
			disabled={!!seasonSaves.sizing[`${row.toon}/${row.season}`]}
			onclick={() => saveRest(row)}
			data-testid="saved-season-rest">{m['saved.season_rest']()}</button
		>
	</p>
{/each}

<style>
	.season {
		margin: 0;
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.25rem 0;
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
	}
	.entry {
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}
	.btn-secondary {
		height: var(--control-height);
		padding: 0 1rem;
		flex: none;
	}
	small {
		font-size: 0.75rem;
		line-height: 1.3;
		color: var(--text-muted);
	}
</style>
