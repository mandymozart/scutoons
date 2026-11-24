<script lang="ts">
	import { onMount, untrack, type Component } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { NEUTRAL_LOCALE, type Problem } from '@scutoons/content';
	import type { CatalogDoc, CatalogDocKind } from '@scutoons/api';
	import {
		AlbumForm,
		HomeSceneForm,
		AirliftCarrierForm,
		EditionForm,
		ItemForm,
		JsonField,
		KINDS,
		ProductForm,
		RecipeForm,
		catalogProblems,
		restOf,
		withRest,
		type CatalogFormProps,
		type Data
	} from '@scutoons/catalog';
	import { askDialog, confirmDialog, Icon } from '@scutoons/ui';
	import { IconButton, Pill, PillTabs } from '@scutoons/ui/editor';
	import { neutralDoc } from '$lib/editor/languages.ts';
	import { contentApi } from '$lib/api.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { CATALOG_LABELS, CATALOG_PAGE_KINDS, catalogKey, catalogLists, loadCatalogDoc, loadCatalogDocs } from '$lib/editor/catalog-docs.ts';
	import { shareUses } from '$lib/editor/library-hook.svelte.ts';
	import { contentName } from '$lib/editor/text-language.svelte.ts';
	import { loadToons } from '$lib/editor/characters.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { usePageNav } from '$lib/nav.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import MetaPage from '$lib/components/page/meta-page.svelte';

	/**
	 * The catalog in the creator (agents/CATALOG-IN-CREATOR.md): one tab per kind (items, recipes, albums, products,
	 * editions) and the open entry in its form (@scutoons/catalog, the admin's forms) – the entries are found in the
	 * Library (lap2 C9), no list of them here; without one asked for, the kind's first opens. Every entry is
	 * its own document (`item:<id>` …): Save stores a draft, History lists its versions, Publish is the one checklist –
	 * the server writes the catalog then, and decides who may (review rules; physical changes: the `physical` rule).
	 * Over 400 lines: one page of the list, the open form, the actions and the server's problems share one document session.
	 */
	let { kind }: { kind: CatalogDocKind } = $props();

	const FORMS: Record<CatalogDocKind, Component<CatalogFormProps, {}, 'data'>> = {
		items: ItemForm,
		recipes: RecipeForm,
		albums: AlbumForm,
		'home-scenes': HomeSceneForm,
		'airlift-carriers': AirliftCarrierForm,
		products: ProductForm,
		editions: EditionForm
	};
	const N = NEUTRAL_LOCALE;
	// svelte-ignore state_referenced_locally
	const def = KINDS[kind];
	// svelte-ignore state_referenced_locally
	const Form = FORMS[kind];

	let docs = $state<Partial<Record<CatalogDocKind, CatalogDoc[]>>>({});
	let loadError = $state<string | null>(null);
	let selected = $state<string | null>(null);
	let doc = $state<ContentDoc | null>(null);
	/** the last save's / publish's problems from the server (422 invalid_catalog) – shown at their fields */
	const serverProblems = $derived<string[]>(doc?.error ? [doc.error] : []);

	async function reload() {
		try {
			docs = await loadCatalogDocs();
			loadError = null;
		} catch (e) {
			loadError = e instanceof Error ? e.message : String(e);
		}
	}
	let toonList = $state<{ id: string; name: string }[]>([]);
	onMount(() => {
		void reload();
		void loadToons(getLocale()).then((list) => (toonList = list.map((t) => ({ id: t.id, name: t.name }))));
	});

	const entries = $derived(docs[kind] ?? []);

	// URL → the open entry (crumbs, back / forward; the leave guard asks before an edited entry is left)
	$effect(() => {
		const id = page.url.searchParams.get('id');
		const first = entries[0]?.id;
		untrack(() => {
			if (id && id !== selected) open(id);
			else if (!id && !selected && first) open(first, true);
		});
	});


	function open(id: string, replace = false) {
		selected = id;
		const next = neutralDoc(catalogKey(kind, id));
		doc = next;
		const known = (docs[kind] ?? []).some((e) => e.id === id);
		if (known || page.url.searchParams.get('id') === id) void loadCatalogDoc(next, kind, id);
		if (page.url.searchParams.get('id') !== id) void goto(`?id=${encodeURIComponent(id)}`, { replaceState: replace, keepFocus: true, noScroll: true });
	}

	/** a draft saved or published: the list (states, the other forms' selects) reads again */
	$effect(() => {
		if (!doc) return;
		void [doc.base[N], doc.published[N]];
		untrack(() => void reload());
	});

	const data = $derived<Data | null>(doc && !doc.loading ? ((doc.data[N] as Data | null) ?? null) : null);
	shareUses(() => (selected ? catalogKey(kind, selected) : null), () => data);
	/** the form's data: in place edits go straight into the document; a replaced object (Advanced JSON) too */
	const formData = {
		get: () => data ?? {},
		set: (next: Data) => {
			if (!doc) return;
			doc.edit((file: Data) => {
				for (const key of Object.keys(file)) delete file[key];
				Object.assign(file, next);
			}, 'form');
		}
	};
	const advanced = $derived(data && def.formKeys ? restOf(data, def.formKeys) : null);
	const lists = $derived(catalogLists(docs, toonList, selected && data ? { kind, id: selected, data } : undefined));
	const entry = $derived(entries.find((e) => e.id === selected) ?? null);

	/** the open entry's problems (validateCatalog over the lists as they stand – items, recipes, albums) */
	function validate(): Problem[] {
		if (!doc || !selected) return [];
		return catalogProblems(lists)
			.filter((p) => p.place?.kind === kind && p.place.id === selected)
			// keyed inside the entry (the admin's accepted_warnings keep working): the Problems panel's "Looks fine" stores it there
			.map(({ list: _list, place, ...p }) => ({ ...p, path: place?.path ?? p.path, file: doc!.entity }));
	}

	// ── Removing (owner 2026-10-12, D5 – the same as the map's places) ─────────────────────────────────────────────
	/**
	 * never published: the document goes (its id is free again); published: a version = the entry + removed: true – a
	 * draft like any other, published through the checklist (that takes the entry out of the catalog; physical: review)
	 */
	async function removeEntry() {
		if (!doc || !selected || !data) return;
		const neverPublished = !entry || (!entry.live && entry.published === null);
		const ok = await confirmDialog({
			title: m['creator.catalog_remove_title']({ id: selected }),
			message: neverPublished ? m['creator.catalog_remove_never_published']() : m['creator.catalog_remove_published'](),
			confirmLabel: m['creator.catalog_remove']()
		});
		if (!ok) return;
		if (neverPublished) {
			try {
				if (entry) await contentApi.saveBatch([], undefined, [catalogKey(kind, selected)]);
				selected = null;
				doc = null;
				// leave its URL first, then the list (the first entry opens)
				await goto(`/catalog/${kind}`, { replaceState: true });
				await reload();
			} catch (e) {
				loadError = e instanceof Error ? e.message : String(e);
			}
			return;
		}
		doc.edit((file: Data) => void (file.removed = true), 'remove');
	}
	function restoreEntry() {
		doc?.edit((file: Data) => void delete file.removed, 'remove');
	}

	async function create() {
		const id = await askDialog({
			title: m['creator.catalog_new']({ what: def.singular() }),
			message: m['creator.catalog_new_hint'](),
			label: 'id',
			placeholder: 'e.g. golden_spoon',
			confirmLabel: m['creator.catalog_create']()
		});
		const clean = id?.trim();
		if (!clean) return;
		if (!def.idPattern.test(clean)) return void (loadError = m['creator.catalog_id_invalid']({ id: clean }));
		// a removed entry's id stays taken (D5)
		if (entries.some((e) => e.id === clean)) return void (loadError = m['creator.catalog_id_taken']({ id: clean }));
		loadError = null;
		selected = clean;
		const next = neutralDoc(catalogKey(kind, clean));
		next.create(() => ({ ...(def.dataId ? { id: clean } : {}), ...def.blank() }));
		doc = next;
		void goto(`?id=${encodeURIComponent(clean)}`, { keepFocus: true, noScroll: true });
	}

	const title = (e: CatalogDoc) => contentName((e.content as Data).name ?? (e.content as Data).title, e.id) || def.title(e.content, e.id);

	usePageNav({ extra: () => (entry ? [{ href: `/catalog/${kind}?id=${encodeURIComponent(entry.id)}`, label: title(entry) }] : []) });
</script>

<MetaPage toon="" title={CATALOG_LABELS[kind]()} docs={doc ? [doc] : []} {validate} wide help="catalog">
	<!-- the open entry's group: its kind among the catalog's kinds -->
	<!-- the page's toolbar: the open entry's kind among the catalog's kinds, and the page's actions (RULES H4) -->
	<div class="toolbar">
		<Pill><PillTabs items={CATALOG_PAGE_KINDS.map((k) => ({ id: k, label: CATALOG_LABELS[k](), testid: `catalog-kind-${k}` }))} on={kind} onselect={(k) => k !== kind && void goto(`/catalog/${k}`)} label={m['creator.catalog']()} /></Pill>
		<Pill label={CATALOG_LABELS[kind]()} data-testid="page-actions">
			<IconButton label={m['creator.catalog_new']({ what: def.singular() })} icon="plus" onclick={create} data-testid="catalog-new" />
			{#if data && selected}
				{#if kind === 'items' && data.category === 'score'}
					<IconButton label={m['creator.catalog_collage']()} title={m['creator.catalog_collage_hint']()} icon="grid" onclick={() => goto(`/catalog/items/${encodeURIComponent(selected!)}/collage`)} data-testid="catalog-collage" />
				{/if}
				{#if data.removed}
					<IconButton label={m['creator.catalog_restore']()} icon="rotate-ccw" onclick={restoreEntry} data-testid="catalog-restore" />
				{:else}
					<IconButton label={m['creator.catalog_remove']()} icon="trash-2" onclick={removeEntry} data-testid="catalog-remove" />
				{/if}
			{/if}
		</Pill>
	</div>
	{#if loadError}<p class="error" role="alert">{loadError}</p>{/if}
	<div class="detail" data-testid="catalog-detail">
		{#if doc && data}
			{#if entry?.physical || (kind === 'items' && (data.kind === 'physical' || data.edition_id || data.product || data.max_world != null)) || kind === 'products' || kind === 'editions'}
				<p class="note" data-testid="catalog-physical-note"><Icon name="lock" /> {m['creator.catalog_physical_note']()}</p>
			{/if}
			{#if data.removed}
				<p class="note" data-testid="catalog-removed-note"><Icon name="trash-2" /> {m['creator.catalog_removed_note']()}</p>
			{/if}
			{#key selected}
				<Form bind:data={formData.get, formData.set} creating={!entry?.live} id={selected ?? ''} {lists} problems={serverProblems} />
				{#if advanced}
					<details class="advanced">
						<summary>{m['creator.catalog_advanced']()}</summary>
						<JsonField
							label={m['creator.catalog_advanced']()}
							value={advanced}
							onchange={(rest: Data) => formData.set(withRest(data, def.formKeys ?? [], rest))}
						/>
					</details>
				{/if}
			{/key}
		{:else if doc?.loading}
			<p class="hint">{m['creator.loading']()}</p>
		{/if}
	</div>
</MetaPage>

<style>
	.toolbar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05);
	}
	.detail {
		display: flex;
		flex-direction: column;
		padding-top: 0.75rem;
		gap: 0.75rem;
		min-width: 0;
	}
	.note {
		display: flex;
		gap: 0.4rem;
		align-items: center;
		margin: 0;
		color: var(--tool-muted);
	}
	.error {
		color: var(--tool-stop);
	}
</style>
