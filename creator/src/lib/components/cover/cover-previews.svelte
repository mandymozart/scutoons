<script lang="ts">
	import { CoverArt } from '@scutoons/reader';
	import type { CoverFormat, CoverLine, CoverTexts } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import SourceMark from '$lib/components/text/source-mark.svelte';

	/**
	 * The cover "as readers see it" (the reader's own CoverArt with the titles of the language): the header's chosen text
	 * language only (owner 2026-10-06) – the check for long translations, one language at a time.
	 */
	let {
		image,
		format,
		lines,
		off,
		languages,
		source,
		texts
	}: { image: string; format: CoverFormat; lines: CoverLine[]; off: boolean; languages: string[]; source: string; texts: (locale: string) => CoverTexts } = $props();
</script>

<section class="readers" data-testid="cover-readers">
	<h3 class="sc-heading">{m['creator.cover_readers']()}</h3>
	<div class="reader-grid">
		{#each languages as l (l)}
			{@const t = texts(l)}
			<figure class="reader" data-testid="cover-reader-{l}">
				<CoverArt src={image} {format} lettering={off ? false : { [format]: lines }} title={t.toon} subtitle={t.season} lang={l} />
				<figcaption class="hint">{l.toUpperCase()}{#if l === source}<SourceMark />{/if}</figcaption>
			</figure>
		{/each}
	</div>
</section>

<style>
	.readers h3 {
		margin: 0 0 var(--space-05);
	}
	.reader-grid {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
	}
	.reader {
		margin: 0;
		width: 14rem;
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
	}
	.hint {
		margin: 0;
	}
</style>
