<script lang="ts">
	// The conductor's linked reader (#529, agents/research/SCENE-EVENTS.md §6.3): the row – where the reader starts
	// (reader-place-picker), the link from the core (relay-web `reader_link`, conductor-reader.ts), Copy link, a QR for a
	// phone, Open here (a new tab) – and below it the linked readers now (the core's, linked-readers).
	import { onMount } from 'svelte';
	import { Button, QrCode } from '@scutoons/ui';
	import { loadRelay } from '@scutoons/live-deck';
	import { page } from '$app/state';
	import { m } from '$lib/paraglide/messages.js';
	import { WASM_BASE, codeOf } from '$lib/live-deck.ts';
	import { readerLink, type LinkModule, type ReaderPlace } from '$lib/conductor-reader.ts';
	import type { ReadersView } from '$lib/conductor-readers.svelte.ts';
	import ReaderPlacePicker from './reader-place-picker.svelte';
	import LinkedReaders from './linked-readers.svelte';

	let { readers }: { readers: ReadersView } = $props();

	let place = $state<ReaderPlace>({ toon: '', chapter: null, label: '' });
	let core = $state.raw<LinkModule | null>(null);
	let problem = $state('');
	let copied = $state('');
	const code = $derived(codeOf(page.url.searchParams, page.params.code));
	const link = $derived(core ? readerLink(core, page.url.origin, code, place) : '');

	onMount(() => {
		loadRelay(WASM_BASE).then(
			(mod) => (core = mod as unknown as LinkModule),
			(e) => (problem = m['conductor_reader.failed']({ why: String(e) }))
		);
	});

	async function copy() {
		try {
			await navigator.clipboard.writeText(link);
			copied = link;
		} catch (e) {
			problem = m['conductor_reader.failed']({ why: String(e) });
		}
	}
</script>

<section class="row" aria-labelledby="conductor-reader-title" data-testid="conductor-reader">
	<h2 id="conductor-reader-title">{m['conductor_reader.title']()}</h2>
	<ReaderPlacePicker bind:place />
	{#if problem}<p class="note" role="alert">{problem}</p>{/if}
	{#if link}
		<div class="hand">
			<QrCode text={link} label={m['conductor_reader.qr']({ url: link })} testid="conductor-reader-qr" />
			<div class="out">
				<output class="link" data-testid="conductor-reader-link">{link}</output>
				<div class="actions">
					<Button variant="secondary" size="sm" onclick={copy} data-testid="conductor-reader-copy">{copied === link ? m['conductor_reader.copied']() : m['conductor_reader.copy']()}</Button>
					<Button variant="secondary" size="sm" href={link} target="_blank" rel="noreferrer" data-testid="conductor-reader-open">{m['conductor_reader.open']()}</Button>
				</div>
			</div>
		</div>
	{/if}
</section>
<LinkedReaders readers={readers.now} />

<style>
	.row {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		padding: var(--space-15);
		border: var(--border-width-regular) solid var(--ink);
	}
	h2 {
		margin: 0;
		font-size: var(--type-caption-size);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.hand {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-start;
		gap: var(--space-1);
	}
	.out {
		flex: 1;
		min-width: 12rem;
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
	.link {
		overflow-wrap: anywhere;
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
	}
	.note {
		margin: 0;
		padding-left: var(--space-05);
		border-left: 0.5rem solid var(--badge-warning);
		font-size: var(--type-caption-size);
	}
</style>
