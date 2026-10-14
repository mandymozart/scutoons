<script lang="ts">
	import { Language } from '@scutoons/content';
	import { FileSlot } from '@scutoons/media';
	import type { Data } from '../kinds.ts';
	import { m } from '../messages.ts';
	import CatalogField from './catalog-field.svelte';

	/**
	 * A score's files (agents/research/SCORES.md §1, §5): its one language (owner 2026-10-14: "a score is always in one
	 * language" – another language is another item), the MusicXML the score view renders and the PDF to print.
	 * Readers get them only as signed URLs (server ProtectedFiles).
	 */
	let { value, place, error = null, onchange }: { value: Data | null | undefined; place: string; error?: string | null; onchange: (score: Data) => void } = $props();

	const score = $derived<Data>(value ?? { language: 'en', musicxml: '' });
	const languages = Object.values(Language).map((l) => ({ value: l }));
	const set = (patch: Data) => onchange({ ...score, ...patch });
</script>

<fieldset class="score" data-testid="item-score">
	<legend>{m['catalog.score']()}</legend>
	<CatalogField label={m['catalog.score_language']()} type="select" value={score.language} options={languages} hint={m['catalog.score_language_hint']()} onchange={(v) => set({ language: v })} testid="item-score-language" />
	<FileSlot kind="document" value={score.musicxml || null} label={m['catalog.score_musicxml']()} {place} onchange={(path) => set({ musicxml: path ?? '' })} testid="item-score-musicxml" />
	<FileSlot kind="document" value={score.pdf ?? null} label={m['catalog.score_pdf']()} {place} onchange={(path) => set({ pdf: path ?? null })} testid="item-score-pdf" />
	<p class="sc-caption muted">{m['catalog.score_protected']()}</p>
	{#if error}<p class="sc-error">{error}</p>{/if}
</fieldset>

<style>
	.score {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		margin-bottom: var(--space-15);
	}
</style>
