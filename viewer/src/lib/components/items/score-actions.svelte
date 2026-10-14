<script lang="ts">
	import { ageLook, type Entry } from '@scutoons/content';
	import { ScoreView } from '@scutoons/reader';
	import { getInventoryContext, localized } from '$lib/contexts/inventory.svelte.ts';
	import { scoreFile } from '$lib/api.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A score in the item detail (agents/research/SCORES.md §4, §5): "Open" shows it in the score view (its MusicXML
	 * through a signed URL – anyone who has it, guests too), "PDF" downloads the print file (members who own it). Lying
	 * in an ageing book (the DIY song book), its pages show the book's age.
	 */
	let { entry }: { entry: Entry } = $props();

	const inventory = getInventoryContext();
	const def = $derived(inventory.item(entry.item_id));
	const title = $derived(def?.score ? (def.name?.[def.score.language] ?? localized(def.name)) : '');
	const book = $derived(inventory.containerOf(entry));
	const age = $derived.by(() => {
		const ages = book ? inventory.item(book.item_id)?.ages : null;
		if (!book || !ages) return null;
		return { ...ageLook(ages, Date.parse(book.aged_since ?? book.acquired_at), Date.now()), seed: String(book.id) };
	});
	let src = $state<string | null>(null);
	let working = $state(false);
	let failed = $state(false);

	async function open() {
		working = true;
		failed = false;
		try {
			src = (await scoreFile(entry.item_id, 'musicxml')).url;
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
			window.open((await scoreFile(entry.item_id, 'pdf')).url, '_blank', 'noopener');
		} catch {
			failed = true;
		} finally {
			working = false;
		}
	}
</script>

{#if def?.score}
	<button class="btn-secondary" disabled={working} onclick={open} data-testid="score-open">{m['items.score_open']()}</button>
	{#if def.score.pdf && inventory.isAccount}
		<button class="btn-secondary" disabled={working} onclick={pdf} data-testid="score-pdf">{m['items.score_pdf']()}</button>
	{/if}
	{#if failed}<p class="note" role="alert">{m['items.score_failed']()}</p>{/if}
	{#if src}
		<ScoreView {src} {title} {age} labels={{ close: m['common.close'](), zoomIn: m['items.score_larger'](), zoomOut: m['items.score_smaller'](), previous: m['items.score_previous'](), next: m['items.score_next'](), page: m['items.score_page']({ page: '{page}', pages: '{pages}' }), pdf: 'PDF', loading: m['items.loading'](), failed: m['items.score_failed']() }} onclose={() => (src = null)} />
	{/if}
{/if}

<style>
	.note {
		width: 100%;
		margin: 0;
	}
</style>
