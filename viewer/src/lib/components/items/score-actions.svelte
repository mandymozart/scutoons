<script lang="ts">
	import { ageLook, readerPart, type Entry } from '@scutoons/content';
	import type { ScoreView as ScoreViewType } from '@scutoons/reader/score';
	import { getInventoryContext, localized } from '$lib/contexts/inventory.svelte.ts';
	import { scoreFile } from '$lib/api.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A score in the item detail (agents/research/SCORES.md §4, §5): "Open" shows it in the score view (its MusicXML
	 * through a signed URL – anyone who has it, guests too), "PDF" downloads the print file (members who own it). For
	 * now both are the choir part when the score has one (owner 2026-10-14; parts switch on later with performance
	 * groups). Lying in an ageing book (the DIY song book), its pages show the book's age. The score view loads with the
	 * first Open (a dynamic import – it stays out of every toon page's eager code; research/TEXT-ACROSS-SLICES.md §9).
	 */
	let { entry }: { entry: Entry } = $props();

	const inventory = getInventoryContext();
	const def = $derived(inventory.item(entry.item_id));
	const title = $derived(def?.score ? (def.name?.[def.score.language] ?? localized(def.name)) : '');
	const part = $derived(readerPart(def?.score));
	const hasPdf = $derived(!!(part ? def?.score?.parts?.[part]?.pdf : def?.score?.pdf));
	const book = $derived(inventory.containerOf(entry));
	const age = $derived.by(() => {
		const ages = book ? inventory.item(book.item_id)?.ages : null;
		if (!book || !ages) return null;
		return { ...ageLook(ages, Date.parse(book.aged_since ?? book.acquired_at), Date.now()), seed: String(book.id) };
	});
	let src = $state<string | null>(null);
	let ScoreView = $state<typeof ScoreViewType | null>(null);
	let working = $state(false);
	let failed = $state(false);

	async function open() {
		working = true;
		failed = false;
		try {
			const [file, view] = await Promise.all([scoreFile(entry.item_id, 'musicxml', part), import('@scutoons/reader/score')]);
			ScoreView = view.ScoreView;
			src = file.url;
		} catch {
			failed = true;
		} finally {
			working = false;
		}
	}

	async function pdf() {
		working = true;
		failed = false;
		try {
			window.open((await scoreFile(entry.item_id, 'pdf', part)).url, '_blank', 'noopener');
		} catch {
			failed = true;
		} finally {
			working = false;
		}
	}
</script>

{#if def?.score}
	<button class="btn-secondary" disabled={working} onclick={open} data-testid="score-open">{m['items.score_open']()}</button>
	{#if hasPdf && inventory.isAccount}
		<button class="btn-secondary" disabled={working} onclick={pdf} data-testid="score-pdf">{m['items.score_pdf']()}</button>
	{/if}
	{#if failed}<p class="note" role="alert">{m['items.score_failed']()}</p>{/if}
	{#if src && ScoreView}
		<ScoreView {src} {title} {age} labels={{ close: m['common.close'](), zoomIn: m['items.score_larger'](), zoomOut: m['items.score_smaller'](), previous: m['items.score_previous'](), next: m['items.score_next'](), page: m['items.score_page']({ page: '{page}', pages: '{pages}' }), pdf: 'PDF', loading: m['items.loading'](), failed: m['items.score_failed'](), play: m['items.score_play'](), stop: m['items.score_stop'](), pad: m['items.score_pad']() }} onclose={() => (src = null)} />
	{/if}
{/if}

<style>
	.note {
		width: 100%;
		margin: 0;
	}
</style>
