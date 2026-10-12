<script lang="ts">
	import { onMount, untrack, type Component } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { NEUTRAL_LOCALE, type Problem } from '@scutoons/content';
	import type { CatalogDoc, CatalogDocKind } from '@scutoons/api';
	import {
		AlbumForm,
		EditionForm,
		ItemForm,
		JsonField,
		KINDS,
		ProductForm,
		RecipeForm,
		catalogProblems,
		importItemImages,
		importRecipeImages,
		type CatalogWrite,
		restOf,
		withRest,
		type CatalogFormProps,
		type Data
	} from '@scutoons/catalog';
	import { askDialog, confirmDialog, Badge, Icon, Tabs, region } from '@scutoons/ui';
	import { neutralDoc } from '$lib/editor/languages.ts';
	import { contentApi, putDraft } from '$lib/api.ts';
	import { toast } from '$lib/toasts.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { CATALOG_LABELS, CATALOG_PAGE_KINDS, catalogKey, catalogLists, loadCatalogDocs } from '$lib/editor/catalog-docs.ts';
	import { contentName } from '$lib/editor/text-language.svelte.ts';
	import { loadToons } from '$lib/editor/characters.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { usePageNav } from '$lib/nav.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import MetaPage from '$lib/components/page/meta-page.svelte';

	/**
	 * The catalog in the creator (agents/CATALOG-IN-CREATOR.md): one tab per kind (items, recipes, albums, products,
	 * editions), the entries as a list, the open one in its form (@scutoons/catalog, the admin's forms). Every entry is
	 * its own document (`item:<id>` …): Save stores a draft, History lists its versions, Publish is the one checklist –
	 * the server writes the catalog then, and decides who may (review rules; physical changes: the `physical` rule).
	 */
	let { kind }: { kind: CatalogDocKind } = $props();

	const FORMS: Record<CatalogDocKind, Component<CatalogFormProps, {}, 'data'>> = {
		items: ItemForm,
		recipes: RecipeForm,
		albums: AlbumForm,
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
	let query = $state('');
	let toonFilter = $state('');
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
	const toons = $derived([...new Set(entries.map((e) => (e.content as Data).toon).filter((t): t is string => typeof t === 'string'))].sort());
	const visible = $derived(
		entries.filter((e) => {
			const data = e.content as Data;
			if (e.removed && !e.draft) return false;
			if (toonFilter === '__none' ? data.toon != null : toonFilter && data.toon !== toonFilter) return false;
			const q = query.trim().toLowerCase();
			return !q || e.id.toLowerCase().includes(q) || def.title(data, e.id).toLowerCase().includes(q);
		})
	);

	// URL → the open entry (crumbs, back / forward; the leave guard asks before an edited entry is left)
	$effect(() => {
		const id = page.url.searchParams.get('id');
		const first = entries[0]?.id;
		untrack(() => {
			if (id && id !== selected) open(id);
			else if (!id && !selected && first) open(first, true);
		});
	});

	/** the first entry opened by itself (no ?id yet) – on phones the list stays until one is tapped */
	let autoOpened = $state(false);
	/** phones (owner 2026-10-12, the list lay over the form): an entry the reader opened replaces the list – the crumb goes back */
	const picked = $derived(!!page.url.searchParams.get('id') && !autoOpened);

	function open(id: string, replace = false) {
		selected = id;
		autoOpened = replace;
		const next = neutralDoc(catalogKey(kind, id));
		doc = next;
		const known = (docs[kind] ?? []).some((e) => e.id === id);
		if (known || page.url.searchParams.get('id') === id) void next.load();
		if (page.url.searchParams.get('id') !== id) void goto(`?id=${encodeURIComponent(id)}`, { replaceState: replace, keepFocus: true, noScroll: true });
	}

	/** a draft saved or published: the list (states, the other forms' selects) reads again */
	$effect(() => {
		if (!doc) return;
		void [doc.base[N], doc.published[N]];
		untrack(() => void reload());
	});

	const data = $derived<Data | null>(doc && !doc.loading ? ((doc.data[N] as Data | null) ?? null) : null);
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

	// ── Import images … (moved from the admin): `<id>[_<variant>].<ext>` → the pictures, a draft per changed entry ──
	let importInput = $state<HTMLInputElement>();
	let importing = $state<string | null>(null);
	const write: CatalogWrite = (k, id, next, note) => putDraft(catalogKey(k, id), next, undefined, note);
	async function importImages(event: Event) {
		const files = [...((event.currentTarget as HTMLInputElement).files ?? [])];
		if (importInput) importInput.value = '';
		if (!files.length) return;
		const step = (done: number, total: number) => (importing = m['creator.catalog_importing']({ done, total }));
		try {
			step(0, files.length);
			const report = kind === 'recipes' ? await importRecipeImages(files, lists.recipes ?? [], lists.items ?? [], write, step) : await importItemImages(files, lists.items ?? [], write, step);
			const unknown = report.problems.map((p) => p.file);
			loadError = unknown.length ? m['creator.catalog_import_unknown']({ files: unknown.join(', ') }) : null;
			toast(m['creator.catalog_import_done']({ uploaded: report.uploaded, replaced: report.replaced, unchanged: report.unchanged, drafts: report.updated.length }), { type: unknown.length ? 'info' : 'success' });
			await reload();
			if (doc && selected && report.updated.includes(selected)) await doc.load();
		} catch (e) {
			loadError = e instanceof Error ? e.message : String(e);
		} finally {
			importing = null;
		}
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
	const icon = (e: CatalogDoc): string | null => {
		const d = e.content as Data;
		return typeof d.icon === 'string' ? d.icon : typeof d.image === 'string' ? d.image : typeof d.cover === 'string' ? d.cover : null;
	};

	let tab = $state<string>(untrack(() => kind));
	$effect(() => {
		if (tab !== kind) void goto(`/catalog/${tab}`);
	});
	usePageNav({ extra: () => (entry ? [{ href: `/catalog/${kind}?id=${encodeURIComponent(entry.id)}`, label: title(entry) }] : []) });
</script>

<MetaPage toon="" title={CATALOG_LABELS[kind]()} docs={doc ? [doc] : []} {validate} wide help="catalog">
	{#snippet actions()}
		<button type="button" class="secondary" onclick={create} data-testid="catalog-new"><Icon name="plus" /> {m['creator.catalog_new']({ what: def.singular() })}</button>
		{#if kind === 'items' || kind === 'recipes'}
			<input bind:this={importInput} type="file" accept="image/png,image/jpeg,image/webp" multiple hidden onchange={importImages} data-testid="catalog-import-input" />
			<button type="button" class="secondary" disabled={!!importing} onclick={() => importInput?.click()} title={m['creator.catalog_import_hint']()} data-testid="catalog-import">{importing ?? m['creator.catalog_import']()}</button>
		{/if}
		{#if data && selected}
			{#if data.removed}
				<button type="button" class="secondary" onclick={restoreEntry} data-testid="catalog-restore">{m['creator.catalog_restore']()}</button>
			{:else}
				<button type="button" class="secondary" onclick={removeEntry} data-testid="catalog-remove">{m['creator.catalog_remove']()}</button>
			{/if}
		{/if}
	{/snippet}
	<Tabs tabs={CATALOG_PAGE_KINDS.map((k) => ({ id: k, label: CATALOG_LABELS[k]() }))} bind:value={tab} label={m['creator.catalog']()}>
		{#snippet children()}
			{#if loadError}<p class="error" role="alert">{loadError}</p>{/if}
			<div class="layout" class:picked>
				<aside class="list" data-testid="catalog-list">
					<div class="filters">
						<input type="search" bind:value={query} placeholder={m['creator.catalog_search']()} data-testid="catalog-search" />
						{#if toons.length}
							<select bind:value={toonFilter} aria-label={m['creator.catalog_filter_toon']()} data-testid="catalog-filter-toon">
								<option value="">{m['creator.characters_all_toons']()}</option>
								{#each toons as t (t)}<option value={t}>{t}</option>{/each}
								<option value="__none">{m['creator.catalog_every_toon']()}</option>
							</select>
						{/if}
					</div>
					<!-- a selection region (DESIGN.md "Selection"): the open entry is sc-selected -->
					<ul use:region={'catalog'}>
						{#each visible as e (e.id)}
							{@const src = icon(e)}
							<li>
								<button type="button" class="item" data-control-exempt="two-line list row with picture" class:sc-selected={e.id === selected} onclick={() => open(e.id)} data-testid="catalog-{e.id}">
									<span class="thumb">{#if src}<img src={asset(src)} alt="" loading="lazy" />{/if}</span>
									<span class="text">
										<span>{title(e)}</span>
										<span class="hint">{e.id}{(e.content as Data).toon ? ` · ${(e.content as Data).toon}` : ''}</span>
									</span>
									<span class="marks">
										{#if e.removed}<Badge tone="error">{m['creator.catalog_state_removed']()}</Badge>{:else if !e.live}<Badge tone="soon">{m['creator.catalog_state_new']()}</Badge>{:else if e.draft}<Badge tone="draft">{m['creator.catalog_state_draft']()}</Badge>{/if}
										{#if e.physical}<span title={m['creator.catalog_physical_hint']()}><Badge tone="info">{m['creator.catalog_physical']()}</Badge></span>{/if}
									</span>
								</button>
							</li>
						{/each}
					</ul>
				</aside>
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
			</div>
		{/snippet}
	</Tabs>
</MetaPage>

<style>
	.layout {
		display: grid;
		grid-template-columns: 20rem minmax(0, 1fr);
		gap: 1.25rem;
		align-items: start;
		padding-top: 0.75rem;
	}
	/* one column: the list in the flow (sticky it lay over the form) */
	@media (max-width: 900px) {
		.layout {
			grid-template-columns: 1fr;
		}
		.list {
			position: static;
			max-height: none;
		}
	}
	/* phones: the list or the opened entry (like the characters) */
	@media (max-width: 767px) {
		.layout.picked .list,
		.layout:not(.picked) .detail {
			display: none;
		}
	}
	.list {
		position: sticky;
		top: 0.5rem;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		max-height: calc(100vh - 10rem);
		overflow: auto;
	}
	.filters {
		display: grid;
		grid-template-columns: 1fr;
		gap: 0.3rem;
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}
	.item {
		all: unset;
		box-sizing: border-box;
		width: 100%;
		display: flex;
		gap: 0.5rem;
		align-items: center;
		padding: 0.3rem;
		border: 1px solid var(--border-subtle);
		cursor: pointer;
	}
	.item:hover {
		border-color: var(--ink);
	}
	.thumb {
		width: 2.6rem;
		height: 2.6rem;
		flex: none;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.thumb img {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}
	.text {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.text .hint {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.marks {
		margin-left: auto;
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 0.15rem;
	}
	.detail {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		min-width: 0;
	}
	.note {
		display: flex;
		gap: 0.4rem;
		align-items: center;
		margin: 0;
		color: var(--muted, inherit);
	}
	.error {
		color: var(--error);
	}
</style>
