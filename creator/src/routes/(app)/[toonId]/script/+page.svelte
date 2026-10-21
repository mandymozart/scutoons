<script lang="ts">
	import { page } from '$app/state';
	import { entities, parseEntityKey } from '@scutoons/content';
	import type { ContentIndexEntity } from '@scutoons/api';
	import { contentIndex } from '$lib/api.ts';
	import { fileLabel } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A toon's scripts (research/KANON.md §5, #81): the toon's Kanon text, a script per season and per chapter – each a
	 * link to its editor, with its state (published / draft / not written). Seasons and chapters come from the toon's
	 * stored documents (one request: the content index).
	 */
	const toon = $derived(page.params.toonId as string);
	const index = $derived(contentIndex(toon).then((r) => r.entities));

	interface Row {
		href: string;
		label: string;
		/** the stored texts of this entity (every language) */
		stored: ContentIndexEntity[];
	}

	/** the entity's stored versions, any language (`season-script:catpope:1` ⇐ `…:1:de`, `…:1:en`) */
	const storedOf = (all: ContentIndexEntity[], entity: string) => all.filter((e) => e.key.startsWith(`${entity}:`));
	const numbers = (all: ContentIndexEntity[], kind: string) =>
		[...new Set(all.map((e) => parseEntityKey(e.key)).filter((r) => r?.kind === kind && r.number !== undefined).map((r) => r!.number!))].sort((a, b) => a - b);

	function rows(all: ContentIndexEntity[]): { kanon: Row; seasons: Row[]; chapters: Row[] } {
		const kanon = entities.kanon(`toon.${toon}`);
		return {
			kanon: { href: `/kanon/toon.${toon}`, label: fileLabel(kanon), stored: storedOf(all, kanon) },
			seasons: [...new Set([...numbers(all, 'season'), ...numbers(all, 'season-script')])].sort((a, b) => a - b).map((n) => {
				const e = entities.seasonScript(toon, n);
				return { href: `/${toon}/season/${n}/script`, label: fileLabel(e), stored: storedOf(all, e) };
			}),
			chapters: [...new Set([...numbers(all, 'chapter'), ...numbers(all, 'chapter-script')])].sort((a, b) => a - b).map((n) => {
				const e = entities.chapterScript(toon, n);
				return { href: `/${toon}/chapter/${n}/script`, label: fileLabel(e), stored: storedOf(all, e) };
			})
		};
	}
	/** published when any language is; a draft when one has an unpublished version; else not written */
	function state(r: Row): { cls: string; label: string } {
		if (!r.stored.length) return { cls: 'is-off', label: m['creator.scripts_not_written']() };
		if (r.stored.some((e) => e.latest !== e.published)) return { cls: 'is-draft', label: m['creator.status_draft']() };
		return { cls: 'is-published', label: m['creator.status_published']() };
	}
</script>

<div class="scripts" data-testid="scripts-list">
	<p class="sc-caption intro">{m['creator.scripts_intro']()}</p>
	{#await index}
		<p>{m['creator.loading']()}</p>
	{:then all}
		{@const r = rows(all)}
		{#snippet line(row: Row)}
			{@const s = state(row)}
			<li><a href={row.href} data-testid="scripts-link">{row.label}</a> <span class="sc-chip {s.cls}">{s.label}</span></li>
		{/snippet}
		<section class="sc-section">
			<h2 class="sc-heading">{m['creator.kanon']()}</h2>
			<ul>{@render line(r.kanon)}</ul>
		</section>
		{#if r.seasons.length}
			<section class="sc-section">
				<h2 class="sc-heading">{m['creator.seasons']()}</h2>
				<ul>{#each r.seasons as row (row.href)}{@render line(row)}{/each}</ul>
			</section>
		{/if}
		{#if r.chapters.length}
			<section class="sc-section">
				<h2 class="sc-heading">{m['creator.chapters']()}</h2>
				<ul>{#each r.chapters as row (row.href)}{@render line(row)}{/each}</ul>
			</section>
		{/if}
	{:catch error}
		<p class="error">{error instanceof Error ? error.message : String(error)}</p>
	{/await}
</div>

<style>
	/* the dividers between the sections run edge to edge (RULES "UI rules") */
	.scripts {
		padding: 1rem;
	}
	.intro {
		margin: 0 0 1rem;
		max-width: 40rem;
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		gap: 0.5rem;
	}
	li {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
	a {
		color: var(--ink);
	}
	.error {
		color: var(--error);
	}
</style>
