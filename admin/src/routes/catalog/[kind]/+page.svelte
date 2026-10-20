<script lang="ts">
	import { crossAppClick } from '@scutoons/pwa';
	import type { Component } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Button, Tabs } from '@scutoons/ui';
	import type { CatalogKind } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { KINDS, type CatalogFormProps, type CatalogLists } from '@scutoons/catalog';
	import { ADMIN_KINDS, type AdminKind } from '$lib/catalog/admin-kinds.ts';
	import { CREATOR_URL } from '$lib/config.ts';
	import { errorText } from '$lib/format.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';
	import CatalogEditor from '$lib/components/catalog/catalog-editor.svelte';
	import { CodeTypeForm, MetaForm, PermissionForm, ToonForm } from '@scutoons/catalog';

	/**
	 * "Shop & codes" – the part of the catalog the admin keeps (agents/CATALOG.md part 3): toons, permissions, code types,
	 * meta; one tab per kind, each the same editor (list in order, form, history with undo). Items, recipes, albums,
	 * products and editions are edited in the creator (owner 2026-10-12, CATALOG-IN-CREATOR D6) – the note links there;
	 * the featured products stay on the Homepage page.
	 */
	const FORMS: Record<AdminKind, Component<CatalogFormProps, {}, 'data'>> = {
		toons: ToonForm,
		permissions: PermissionForm,
		'code-types': CodeTypeForm,
		meta: MetaForm
	};

	/** an old link to a moved kind (/catalog/items …) shows the note and the first tab */
	const kind = $derived<AdminKind>((ADMIN_KINDS as readonly string[]).includes(page.params.kind ?? '') ? (page.params.kind as AdminKind) : ADMIN_KINDS[0]);
	const def = $derived(KINDS[kind]);

	let lists = $state<CatalogLists>({});
	let error = $state<string | null>(null);

	async function refresh(k: CatalogKind) {
		const result = await admin.catalogList(k);
		lists[k] = result.items;
	}

	// every kind once (the code types' form lists the permissions)
	$effect(() => {
		error = null;
		Promise.all(ADMIN_KINDS.map((k) => refresh(k))).catch((e) => (error = errorText(e)));
	});

	const tabs = $derived(ADMIN_KINDS.map((k) => ({ id: k, label: KINDS[k].label() })));
</script>

<PageHeader type={m['nav.group_data']()} title={m['catalog.title']()} meta={m['catalog.meta']()} />

<!-- items, recipes, albums, products, editions: drafts, versions and review in the creator (agents/research/CATALOG-IN-CREATOR.md) -->
<p class="muted note" data-testid="catalog-in-creator">{m['catalog.in_creator']()} <a href="{CREATOR_URL}/catalog/items" target="_blank" rel="noopener" onclick={crossAppClick}>{m['catalog.in_creator_link']()}</a></p>

<Tabs {tabs} value={kind} label={m['catalog.title']()} onchange={(id) => goto(`/catalog/${id}`, { keepFocus: true, noScroll: true })}>
	{#snippet children()}
		{#if error}<p class="error-box" role="alert">{error}</p>{/if}
		{#key kind}
			<CatalogEditor {def} {lists} {refresh} form={FORMS[kind]} detailActions={kind === 'toons' ? toonActions : undefined} />
		{/key}
	{/snippet}
</Tabs>

{#snippet toonActions(id: string)}
	<Button size="sm" variant="secondary" href="{CREATOR_URL}/{encodeURIComponent(id)}" target="_blank" rel="noopener" data-testid="toon-open-creator" onclick={crossAppClick}>{m['catalog.toon_fill']()}</Button>
{/snippet}

<style>
	.note {
		margin: 0 0 var(--space-1);
	}
</style>
