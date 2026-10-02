<script lang="ts">
	import { onMount } from 'svelte';
	import { entities, type ToonLanguages } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { toonDoc, toonIndexDoc } from '$lib/editor/languages.ts';
	import { knownImages } from '$lib/editor/images.ts';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import MetaPage from '../meta-page.svelte';
	import SharedField from '../shared-field.svelte';
	import CoverEditor from './cover-editor.svelte';

	/**
	 * `/<toon>/cover` – the cover editor page (owner 2026-10-02). Edits the cover the reader shows: the toon entry's
	 * (`toon:<toon>` → `cover`, language-neutral), saved / published with the bottom bar. The titles come read-only from the
	 * toon settings (`title`) and season 1 (`title`) – the toon page's season. Season covers (`season.cover`, not shown by
	 * the reader) would use the same `CoverEditor` with the season doc (agents/DATA.md "Cover lettering layout").
	 */
	let { toon, languages }: { toon: string; languages: ToonLanguages } = $props();

	// svelte-ignore state_referenced_locally
	const index = toonIndexDoc(toon);
	// svelte-ignore state_referenced_locally
	const settings = toonDoc(entities.toonSettings(toon), languages);
	// svelte-ignore state_referenced_locally
	const season = toonDoc(entities.season(toon, 1), languages);
	let images = $state<string[]>([]);
	const nav = getCreatorNav();

	/** a language's titles; a missing translation shows the source's (as the reader's API falls back) */
	function texts(locale: string) {
		const text = (doc: typeof settings, l: string) => String(doc.get(l, ['title']) ?? '').trim();
		return {
			toon: text(settings, locale) || text(settings, languages.source),
			season: text(season, locale) || text(season, languages.source)
		};
	}

	onMount(async () => {
		await Promise.all([index.load(), settings.load().catch(() => {}), season.load().catch(() => {})]);
		images = await knownImages(toon, settings.locales, [$state.snapshot(settings.data), $state.snapshot(index.data)]);
	});
</script>

<MetaPage {toon} title={m['creator.cover_editor']()} docs={[index]} help="cover" wide>
	{#if index.present.length === 0}
		<p class="warn">{m['creator.toon_entry_missing']()} <a href="/{toon}">{nav.toonName(toon)}</a></p>
	{:else}
		<section class="block" data-testid="cover-page">
			<CoverEditor doc={index} {toon} languages={[...languages.languages]} source={languages.source} {texts} />
		</section>
		<section class="block art" data-testid="cover-art-field">
			<h2 class="sc-heading">{m['creator.cover_art']()}</h2>
			<p class="hint">{m['creator.cover_art_hint']()}</p>
			<div class="art-field">
				<SharedField doc={index} path={['cover', 'square']} label="cover.square" kind="image" {toon} known={images} />
			</div>
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
	h2 {
		margin: 0;
	}
	.hint {
		margin: 0;
	}
	.art-field {
		max-width: 36rem;
	}
	.warn {
		color: var(--warn);
	}
</style>
