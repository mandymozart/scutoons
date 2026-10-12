<script lang="ts">
	import type { Component } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Button, Tabs } from '@scutoons/ui';
	import type { CatalogEntry, CatalogKind } from '@scutoons/api';
	import { admin, library } from '$lib/api.ts';
	import type { CharacterRegistry } from '@scutoons/content';
	import { EDITED_KINDS, KINDS, type CatalogFormProps, type CatalogLists, type EditedKind } from '@scutoons/catalog';
	import { CREATOR_URL } from '$lib/config.ts';
	import { errorText } from '$lib/format.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';
	import CatalogEditor from '$lib/components/catalog/catalog-editor.svelte';
	import { AlbumForm, CodeTypeForm, EditionForm, ItemForm, MetaForm, PermissionForm, ProductForm, RecipeForm, ToonForm } from '@scutoons/catalog';

	/**
	 * The catalog in the database (agents/CATALOG.md part 3): one tab per kind, each the same editor (list in order,
	 * form, history with undo). The featured products stay on the Homepage page (one place for them).
	 */
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

	/** edited in the creator now (owner 2026-10-12, D6 – read-only here, to be removed: FEATURES); the rest = "Shop & codes" */
	const IN_CREATOR: string[] = ['items', 'recipes', 'albums', 'products', 'editions'];
	const kind = $derived<EditedKind>((EDITED_KINDS as readonly string[]).includes(page.params.kind ?? '') ? (page.params.kind as EditedKind) : 'items');
	const def = $derived(KINDS[kind]);

	let lists = $state<CatalogLists>({});
	let error = $state<string | null>(null);

	async function refresh(k: CatalogKind) {
		const result = await admin.catalogList(k);
		lists[k] = result.items;
	}

	// the published characters, for the wear overlays' problems (not loaded = not checked; never blocks the page)
	let registry = $state<CharacterRegistry | null>(null);
	$effect(() => {
		library.fetchRegistry().then((r) => (registry = r), () => (registry = null));
	});

	// every kind once (the forms' selects need the others: items, editions, products, albums, toons, permissions)
	$effect(() => {
		error = null;
		Promise.all(EDITED_KINDS.map((k) => refresh(k))).catch((e) => (error = errorText(e)));
	});

	const tabs = $derived(EDITED_KINDS.map((k) => ({ id: k, label: KINDS[k].label() })));

</script>

<PageHeader type={m['nav.group_data']()} title={m['catalog.title']()} meta={m['catalog.meta']()} />

<Tabs {tabs} value={kind} label={m['catalog.title']()} onchange={(id) => goto(`/catalog/${id}`, { keepFocus: true, noScroll: true })}>
	{#snippet children()}
		{#if error}<p class="error-box" role="alert">{error}</p>{/if}
		{#if IN_CREATOR.includes(kind)}
			<!-- the move into the creator (agents/CATALOG-IN-CREATOR.md): drafts, versions, review there; physical changes need the rule here too -->
			<p class="muted note" data-testid="catalog-in-creator">{m['catalog.in_creator']()} <a href="{CREATOR_URL}/catalog/{kind}" target="_blank" rel="noopener">{m['catalog.in_creator_link']()}</a></p>
		{/if}
		{#if kind === 'products'}
			<p class="muted note">{m['catalog.featured_note']()} <a href="/homepage">{m['nav.homepage']()}</a></p>
		{/if}
		{#key kind}
			<CatalogEditor {def} {lists} {registry} {refresh} form={FORMS[kind]} readonly={IN_CREATOR.includes(kind)} detailActions={kind === 'toons' ? toonActions : undefined} />
		{/key}
	{/snippet}
</Tabs>

{#snippet toonActions(id: string)}
	<Button size="sm" variant="secondary" href="{CREATOR_URL}/{encodeURIComponent(id)}" target="_blank" rel="noopener" data-testid="toon-open-creator">{m['catalog.toon_fill']()}</Button>
{/snippet}

<style>
	.note {
		margin: 0 0 var(--space-1);
	}
</style>
