<script lang="ts">
	import type { Component } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Button, CloseIcon, Tabs } from '@scutoons/ui';
	import type { CatalogEntry, CatalogKind } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { invalidateCatalog } from '$lib/catalog.ts';
	import { importItemImages, type ImportReport } from '$lib/catalog/import.ts';
	import { EDITED_KINDS, KINDS, type CatalogFormProps, type CatalogLists, type EditedKind } from '$lib/catalog/kinds.ts';
	import { CREATOR_URL } from '$lib/config.ts';
	import { errorText } from '$lib/format.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';
	import CatalogEditor from '$lib/components/catalog/catalog-editor.svelte';
	import AlbumForm from '$lib/components/catalog/album-form.svelte';
	import CodeTypeForm from '$lib/components/catalog/code-type-form.svelte';
	import EditionForm from '$lib/components/catalog/edition-form.svelte';
	import ItemForm from '$lib/components/catalog/item-form.svelte';
	import MetaForm from '$lib/components/catalog/meta-form.svelte';
	import PermissionForm from '$lib/components/catalog/permission-form.svelte';
	import ProductForm from '$lib/components/catalog/product-form.svelte';
	import RecipeForm from '$lib/components/catalog/recipe-form.svelte';
	import ToonForm from '$lib/components/catalog/toon-form.svelte';

	/**
	 * The catalog in the database (agents/CATALOG.md part 3): one tab per kind, each the same editor (list in order,
	 * form, history with undo). The featured products stay on the Homepage page (one place for them).
	 */
	const toaster = getToaster();
	const FORMS: Record<EditedKind, Component<CatalogFormProps, {}, 'data'>> = {
		items: ItemForm,
		recipes: RecipeForm,
		albums: AlbumForm,
		products: ProductForm,
		editions: EditionForm,
		toons: ToonForm,
		permissions: PermissionForm,
		'code-types': CodeTypeForm,
		meta: MetaForm
	};

	const kind = $derived<EditedKind>((EDITED_KINDS as readonly string[]).includes(page.params.kind ?? '') ? (page.params.kind as EditedKind) : 'items');
	const def = $derived(KINDS[kind]);

	let lists = $state<CatalogLists>({});
	let fallback = $state(false);
	let error = $state<string | null>(null);

	async function refresh(k: CatalogKind) {
		const result = await admin.catalogList(k);
		lists[k] = result.items;
		fallback = result.fallback;
	}

	// every kind once (the forms' selects need the others: items, editions, products, albums, toons, permissions)
	$effect(() => {
		error = null;
		Promise.all(EDITED_KINDS.map((k) => refresh(k))).catch((e) => (error = errorText(e)));
	});

	const tabs = $derived(EDITED_KINDS.map((k) => ({ id: k, label: KINDS[k].label() })));

	// ── items: "Import images …" ────────────────────────────────────────────────────────────────────────────────
	let importInput = $state<HTMLInputElement>();
	let importing = $state<{ done: number; total: number } | null>(null);
	let report = $state<ImportReport | null>(null);
	/** bumped after the import wrote items: the open item is read again */
	let editorReload = $state(0);

	async function importImages(event: Event) {
		const files = [...((event.currentTarget as HTMLInputElement).files ?? [])];
		if (importInput) importInput.value = '';
		if (!files.length) return;
		report = null;
		importing = { done: 0, total: files.length };
		try {
			report = await importItemImages(files, lists.items ?? [], (done, total) => (importing = { done, total }));
			invalidateCatalog();
			await refresh('items');
			editorReload++;
			toaster.addToast({ message: summary(report), type: report.problems.length ? 'warning' : 'success' });
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		} finally {
			importing = null;
		}
	}
	const summary = (r: ImportReport) => m['catalog.import_summary']({ uploaded: r.uploaded, replaced: r.replaced, unchanged: r.unchanged });
	const problemText = (p: ImportReport['problems'][number]) =>
		p.problem === 'unknown_item' ? m['catalog.import_unknown_item']({ file: p.file }) : p.problem === 'unknown_variant' ? m['catalog.import_unknown_variant']({ file: p.file, item: p.item ?? '' }) : `${p.file}: ${p.message ?? ''}`;
</script>

<PageHeader type={m['nav.group_data']()} title={m['catalog.title']()} meta={m['catalog.meta']()} />

<Tabs {tabs} value={kind} label={m['catalog.title']()} onchange={(id) => goto(`/catalog/${id}`, { keepFocus: true, noScroll: true })}>
	{#snippet children()}
		{#if error}<p class="error-box" role="alert">{error}</p>{/if}
		{#if kind === 'products'}
			<p class="muted note">{m['catalog.featured_note']()} <a href="/homepage">{m['nav.homepage']()}</a></p>
		{/if}
		{#if kind === 'items' && report}
			<div class="report" role="status" data-testid="import-report">
				<p>
					{summary(report)}{#if report.updated.length}{' · '}{m['catalog.import_updated']({ items: report.updated.join(', ') })}{/if}
				</p>
				{#if report.problems.length}
					<ul class="problems">{#each report.problems as p, i (i)}<li>{problemText(p)}</li>{/each}</ul>
				{/if}
				<button type="button" class="sc-round" title={m['common.close']()} aria-label={m['common.close']()} onclick={() => (report = null)}><CloseIcon /></button>
			</div>
		{/if}
		{#key kind}
			<CatalogEditor {def} {lists} {fallback} {refresh} reload={editorReload} form={FORMS[kind]} barActions={kind === 'items' ? itemActions : undefined} detailActions={kind === 'toons' ? toonActions : undefined} />
		{/key}
	{/snippet}
</Tabs>

{#snippet itemActions()}
	<input bind:this={importInput} type="file" accept="image/png,image/jpeg,image/webp" multiple hidden onchange={importImages} data-testid="import-images-input" />
	<Button size="sm" variant="secondary" disabled={!!importing || !lists.items} onclick={() => importInput?.click()} title={m['catalog.import_hint']()} data-testid="import-images">
		{importing ? m['catalog.importing']({ done: importing.done, total: importing.total }) : m['catalog.import_images']()}
	</Button>
{/snippet}

{#snippet toonActions(id: string)}
	<Button size="sm" variant="secondary" href="{CREATOR_URL}/{encodeURIComponent(id)}" target="_blank" rel="noopener" data-testid="toon-open-creator">{m['catalog.toon_fill']()}</Button>
{/snippet}

<style>
	.note {
		margin: 0 0 var(--space-1);
	}
	.report {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		gap: var(--space-05) var(--space-1);
		align-items: start;
		margin-bottom: var(--space-1);
		padding: var(--space-075);
		border: var(--border-width-regular) solid var(--ink);
	}
	.report p {
		margin: 0;
	}
	.report .problems {
		grid-column: 1;
		margin: 0;
		padding-left: 1.2em;
		color: var(--state-error);
	}
	.report .sc-round {
		grid-column: 2;
		grid-row: 1;
	}
</style>
