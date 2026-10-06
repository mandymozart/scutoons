<script lang="ts">
	import { onMount } from 'svelte';
	import { entities, type ToonLanguages } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { toonDoc, toonIndexDoc } from '$lib/editor/languages.ts';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import MetaPage from '$lib/components/page/meta-page.svelte';
	import CoverEditor from '$lib/components/cover/cover-editor.svelte';

	/**
	 * `/<toon>/season/<n>/cover` – a season's cover editor (owner 2026-10-02: covers are per season; the toon is shown
	 * with its avatar). Edits `season.cover` (art + lettering layout) in the language-neutral season document, saved /
	 * published with the bottom bar. The toon's title comes read-only from the toon entry (its texts, or the old
	 * per-language toon settings folded in on load – never saved here); the season's title
	 * from the season document itself (the same doc, so an unsaved title edit shows too).
	 */
	let { toon, number, languages }: { toon: string; number: number; languages: ToonLanguages } = $props();

	// svelte-ignore state_referenced_locally
	const season = toonDoc(entities.season(toon, number), languages);
	// svelte-ignore state_referenced_locally
	const settings = toonIndexDoc(toon, languages);
	const nav = getCreatorNav();

	/** a language's titles; a missing translation shows the source's (as the reader's API falls back) */
	function texts(locale: string) {
		const text = (doc: typeof settings, l: string) => String(doc.text(['title'], l) ?? '').trim();
		return {
			toon: text(settings, locale) || text(settings, languages.source),
			season: text(season, locale) || text(season, languages.source)
		};
	}

	onMount(async () => {
		await Promise.all([season.load(), settings.load().catch(() => {})]);
	});
</script>

<MetaPage {toon} title={m['creator.cover_editor']()} docs={[season]} help="cover" wide>
	{#if season.loading}
		<p>{m['creator.loading']()}</p>
	{:else if season.present.length === 0}
		<p class="sc-empty">{m['creator.cover_no_season']()} <a href="/{toon}">{nav.toonName(toon)}</a></p>
	{:else}
		<section class="block" data-testid="cover-page">
			<CoverEditor doc={season} {toon} languages={season.textLanguages} source={season.textSource} {texts} place="{toon}/s{number}/" />
		</section>
	{/if}
</MetaPage>

<style>
	.block {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		margin-bottom: var(--space-3);
	}
</style>
