<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import { page } from '$app/state';
	import { replaceState } from '$app/navigation';
	import {
		AUTHORED_VIEW,
		entities,
		DEFAULT_MOOD,
		REGULAR_PERSONA,
		castsOf,
		characterImage,
		characterName,
		lookImage,
		personaIds,
		pickText,
		findPersona,
		searchCharacters,
		validateAssetRequests,
		validateCast,
		validateCharacters,
		validateMoods,
		type AssetRequests,
		type Problem,
		type RegistryCharacter,
		type ToonCast
	} from '@scutoons/content';
	import { Tabs } from '@scutoons/ui';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import {
		addCharacter,
		castDoc,
		castOf,
		characterLanguages,
		edit,
		imageExists,
		loadToons,
		moodsDoc,
		moodsOf,
		peopleDoc,
		peopleListOf,
		registryDoc,
		registryOf,
		requestsDoc,
		requestsOf,
		type ToonInfo,
		isKnownMissing
	} from '$lib/editor/characters.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { usePageNav } from '$lib/nav.svelte.ts';
	import MetaPage from '../meta-page.svelte';
	import CharacterDetail from './character-detail.svelte';
	import MoodsPanel from './moods-panel.svelte';
	import WantedImages from './wanted-images.svelte';
	import WantedMapImages from '../map/wanted-map-images.svelte';

	/**
	 * The global Characters page: the registry (entity `characters`) as a searchable, filterable list (characters
	 * with persona chips) with a detail editor, the shared groups, the mood vocabulary and the wanted images (asset
	 * requests). Casts of every toon are read for "appears in", the filters and the validation; people for "voiced by".
	 */
	const registry = registryDoc();
	const moods = moodsDoc();
	const requests = requestsDoc();
	const people = peopleDoc();
	const docs = [registry, moods, requests];

	let toons = $state<ToonInfo[]>([]);
	let castDocs = $state<Record<string, ContentDoc>>({});
	const casts = $derived<Record<string, ToonCast | null>>(Object.fromEntries(Object.entries(castDocs).map(([t, d]) => [t, castOf(d)])));

	const params = page.url.searchParams;
	let selected = $state<string>(params.get('id') ?? '');
	let persona = $state<string>(params.get('persona') ?? REGULAR_PERSONA);
	let look = $state<string>(params.get('look') ?? '');
	let tab = $state<string>(['moods', 'wanted'].includes(params.get('tab') ?? '') ? (params.get('tab') as string) : 'characters');
	let query = $state('');
	let toonFilter = $state<string>(params.get('toon') ?? '');
	let missingOnly = $state(false);
	let creating = $state(false);
	let newName = $state('');
	let nameInput = $state<HTMLInputElement>();

	onMount(async () => {
		await Promise.all([registry.load(), moods.load(), requests.load(), people.load()]);
		toons = await loadToons(getLocale());
		const loaded: Record<string, ContentDoc> = {};
		await Promise.all(
			toons.map(async (t) => {
				const doc = castDoc(t.id);
				await doc.load();
				loaded[t.id] = doc;
			})
		);
		castDocs = loaded;
		closeArrivedRequests();
	});

	/** open requests whose file is now at the convention path are done (a draft edit of the wanted images) */
	async function closeArrivedRequests() {
		const open = (requestsOf(requests)?.requests ?? []).filter((r) => r.status === 'open');
		const arrived: string[] = [];
		for (const r of open) if (await imageExists(r.path)) arrived.push(r.id);
		if (!arrived.length) return;
		edit<AssetRequests>(requests, (f) =>
			f.requests.forEach((r) => {
				if (arrived.includes(r.id)) {
					r.status = 'done';
					r.done_at = new Date().toISOString();
				}
			})
		);
	}

	const file = $derived(registryOf(registry));
	const characters = $derived<RegistryCharacter[]>(file?.characters.filter((c) => !!c?.id) ?? []);
	const allLanguages = $derived([...new Set(['de', ...toons.flatMap((t) => t.languages.languages as string[])])]);
	const uiLocale = $derived(getLocale());
	const vocabulary = $derived(moodsOf(moods));

	// ── CDN status of every neutral portrait (list badge, "missing images" filter, problems) ────────────────
	const neutralFound = new SvelteMap<string, boolean>();
	$effect(() => {
		for (const c of characters)
			for (const p of c.personas ?? [])
				for (const l of p.looks ?? []) {
					const path = lookImage(l, DEFAULT_MOOD, AUTHORED_VIEW);
					if (path && !neutralFound.has(path)) imageExists(path).then((ok) => neutralFound.set(path, ok));
				}
	});
	/** looks without a neutral portrait, neutral portraits the CDN doesn't have */
	function imageIssues(c: RegistryCharacter) {
		const issues: { persona: string; look: string; path?: string }[] = [];
		for (const p of c.personas ?? [])
			for (const l of p.looks ?? []) {
				const path = lookImage(l, DEFAULT_MOOD, AUTHORED_VIEW);
				if (!path) issues.push({ persona: p.id, look: l.id });
				else if (neutralFound.get(path) === false) issues.push({ persona: p.id, look: l.id, path });
			}
		return issues;
	}

	// ── List ────────────────────────────────────────────────────────────────────────────────────────────
	const matches = $derived(new Set(searchCharacters(characters, query, 10000).map((c) => c.id)));
	const visible = $derived(
		characters.filter((c) => {
			if (!matches.has(c.id)) return false;
			if (toonFilter === '__none') {
				if (castsOf(c.id, casts).length || c.own_toon) return false;
			} else if (toonFilter && c.own_toon !== toonFilter && !castsOf(c.id, casts).includes(toonFilter)) return false;
			if (missingOnly && !imageIssues(c).length) return false;
			return true;
		})
	);

	function select(id: string, p: string = REGULAR_PERSONA, l = '') {
		selected = id;
		persona = p;
		look = l;
		tab = 'characters';
	}
	// the URL mirrors the selection (shareable, back to the same place)
	$effect(() => {
		const id = selected;
		const p = persona;
		const t = tab;
		try {
			const url = new URL(page.url);
			for (const [key, value] of [['id', id], ['persona', p === REGULAR_PERSONA ? '' : p], ['tab', t === 'characters' ? '' : t]] as const) {
				if (value) url.searchParams.set(key, value);
				else url.searchParams.delete(key);
			}
			if (url.href !== page.url.href) replaceState(url, {});
		} catch {
			/* before the router is ready */
		}
	});

	function startNew() {
		tab = 'characters';
		creating = true;
		newName = '';
		queueMicrotask(() => nameInput?.focus());
	}
	function create() {
		const name = newName.trim();
		if (!name) return;
		const own = toonFilter && toonFilter !== '__none' ? toons.find((t) => t.id === toonFilter) : undefined;
		const languages = own ? (own.languages.languages as string[]) : allLanguages;
		const id = addCharacter(registry, name, languages, own ? { own_toon: own.id } : {});
		creating = false;
		newName = '';
		select(id);
	}

	function validate(): Problem[] {
		const out: Problem[] = [];
		const registryFile = entities.characters;
		if (file) {
			out.push(
				...validateCharacters(file, {
					languages: (c) => characterLanguages(c, toons, castsOf(c.id, casts)).languages as string[],
					people: people.loading ? undefined : peopleListOf(people).map((p) => p.id),
					toons: toons.length ? toons.map((t) => t.id) : undefined,
					moods: vocabulary,
					file: registryFile
				})
			);
			characters.forEach((c) => {
				const i = file.characters.indexOf(c);
				for (const issue of imageIssues(c)) {
					if (!issue.path) continue;
					const pi = c.personas.findIndex((p) => p.id === issue.persona);
					const li = c.personas[pi]?.looks.findIndex((l) => l.id === issue.look) ?? -1;
					out.push({ level: 'error', file: registryFile, locale: null, path: ['characters', i, 'personas', pi, 'looks', li, 'images', DEFAULT_MOOD, AUTHORED_VIEW], message: `${characterName(c, uiLocale, issue.persona)} (${issue.look}): the neutral portrait is not on the CDN: ${issue.path}` });
				}
			});
		}
		if (moods.present.length) out.push(...validateMoods(vocabulary, allLanguages, entities.moods));
		const r = requestsOf(requests);
		if (r) out.push(...validateAssetRequests(r, file, entities.assetRequests));
		for (const [toon, doc] of Object.entries(castDocs)) {
			const cast = castOf(doc);
			if (cast) out.push(...validateCast(cast, { registry: file, languages: toons.find((t) => t.id === toon)?.languages.languages ?? ['de'], file: entities.cast(toon) }));
		}
		return out;
	}

	const thumb = (c: RegistryCharacter) => characterImage(c, {}, AUTHORED_VIEW, { isMissing: isKnownMissing });
	// the selected character is the deepest crumb (Creator / Characters / <name>)
	usePageNav({
		extra: () => {
			const c = tab === 'characters' ? characters.find((x) => x.id === selected) : undefined;
			return c ? [{ href: `/characters?id=${encodeURIComponent(c.id)}`, label: characterName(c, uiLocale) || c.id }] : [];
		}
	});
	// URL → state (the Universe links in the sidebar, crumbs, back / forward); the effect below mirrors state → URL
	$effect(() => {
		const params = page.url.searchParams;
		const nextTab = ['moods', 'wanted'].includes(params.get('tab') ?? '') ? (params.get('tab') as string) : 'characters';
		const nextId = params.get('id');
		untrack(() => {
			if (nextTab !== tab) tab = nextTab;
			if (nextId !== null && nextId !== selected) selected = nextId;
		});
	});
	const openRequests = $derived((requestsOf(requests)?.requests ?? []).filter((r) => r.status === 'open').length);
</script>

<MetaPage toon="" title={m['creator.characters']()} {docs} {validate} wide help="characters">
	{#snippet actions()}
		<button type="button" class="secondary" onclick={startNew} data-testid="new-character">+ {m['creator.characters_new']()}</button>
	{/snippet}
	<Tabs
		tabs={[
			{ id: 'characters', label: `${m['creator.characters']()} (${characters.length})` },
			{ id: 'moods', label: `${m['creator.characters_moods']()} (${vocabulary.moods.length})` },
			{ id: 'wanted', label: `${m['creator.characters_wanted']()}${openRequests ? ` (${openRequests} ✉)` : ''}` }
		]}
		bind:value={tab}
		label={m['creator.characters']()}
	>
		{#snippet children(active)}
			{#if active === 'moods'}
				<div class="pad-top"><MoodsPanel {moods} registry={file} languages={allLanguages} /></div>
			{:else if active === 'wanted'}
				<div class="pad-top"><WantedImages registry={file} moods={vocabulary} requests={requestsOf(requests)} locale={uiLocale} onopen={(id, p, l) => select(id, p, l)} /><WantedMapImages /></div>
			{:else}
				<div class="layout">
					<aside class="list" data-testid="character-list">
						<div class="filters">
							<input type="search" bind:value={query} placeholder={m['creator.characters_search']()} data-testid="character-search" />
							<select bind:value={toonFilter} aria-label={m['creator.characters_filter_toon']()} data-testid="filter-toon">
								<option value="">{m['creator.characters_all_toons']()}</option>
								{#each toons as t (t.id)}<option value={t.id}>{t.name}</option>{/each}
								<option value="__none">{m['creator.characters_no_toon']()}</option>
							</select>
							<label class="check"><input type="checkbox" bind:checked={missingOnly} data-testid="filter-missing" /> {m['creator.characters_missing_images']()}</label>
						</div>
						{#if creating}
							<span class="new">
								<input bind:this={nameInput} type="text" bind:value={newName} placeholder={m['creator.characters_name']()} onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); create(); } else if (e.key === 'Escape') creating = false; }} data-testid="new-character-name" />
								<button type="button" onclick={create} disabled={!newName.trim()} data-testid="new-character-create">{m['creator.characters_create']()}</button>
								<button type="button" class="ghost" onclick={() => (creating = false)}>✕</button>
							</span>
						{/if}
						{#if registry.loading}
							<p class="hint">{m['creator.loading']()}</p>
						{:else}
							<ul>
								{#each visible as c (c.id)}
									{@const issues = imageIssues(c)}
									{@const src = thumb(c)}
									<li>
										<button type="button" class="item" class:active={c.id === selected} onclick={() => select(c.id)} data-testid="character-{c.id}">
											<span class="avatar" style:background={c.color || 'transparent'}>{#if src}<img src={asset(src)} onerror={() => imageExists(src)} alt="" loading="lazy" />{/if}</span>
											<span class="text">
												<strong>{characterName(c, uiLocale)}</strong>
												<span class="hint">{c.id}{c.own_toon ? ` · ${c.own_toon}` : ''}{castsOf(c.id, casts).length ? ` · ${castsOf(c.id, casts).join(', ')}` : ''}</span>
												{#if personaIds(c).length > 1}
													<span class="chips">
														{#each personaIds(c) as p (p)}
															<span class="chip" class:regular={p === REGULAR_PERSONA}>{p === REGULAR_PERSONA ? m['creator.characters_regular']() : pickText(findPersona(c, p)?.label, uiLocale) || p}</span>
														{/each}
													</span>
												{/if}
											</span>
											{#if issues.length}<span class="bad" title={issues.map((x) => `${x.persona} · ${x.look}: ${x.path ?? 'no neutral portrait'}`).join('\n')}>✗</span>{/if}
										</button>
									</li>
								{:else}
									<li class="hint">{m['creator.characters_none_found']()}</li>
								{/each}
							</ul>
						{/if}
					</aside>
					<div class="detail">
						{#if !registry.loading}
							{#if !registry.present.length}
								<p class="hint">{m['creator.characters_empty_registry']()}</p>
							{/if}
							{#key selected}
								<CharacterDetail
									{registry}
									moodsDoc={moods}
									requestsDoc={requests}
									moods={vocabulary}
									id={selected}
									bind:persona
									bind:look
									{toons}
									{casts}
									people={peopleListOf(people)}
									onselect={(id) => select(id)}
								/>
							{/key}
						{/if}
					</div>
				</div>
			{/if}
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
	@media (max-width: 900px) {
		.layout {
			grid-template-columns: 1fr;
		}
	}
	.pad-top {
		padding-top: 0.75rem;
	}
	.list {
		position: sticky;
		top: 0.5rem;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		max-height: calc(100vh - 170px);
		overflow: auto;
	}
	.filters {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.3rem;
	}
	.filters input[type='search'] {
		grid-column: 1 / -1;
	}
	.check {
		display: flex;
		gap: 0.3rem;
		align-items: center;
		font-size: 11px;
	}
	.new {
		display: flex;
		gap: 0.3rem;
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
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
	.item.active {
		border: 2px solid var(--ink);
	}
	.avatar {
		width: 2.6rem;
		height: 2.6rem;
		flex: none;
		overflow: hidden;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.avatar img {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}
	.text {
		display: flex;
		flex-direction: column;
		min-width: 0;
		font-size: 12px;
	}
	.text .hint {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.2rem;
		margin-top: 0.15rem;
	}
	.chip {
		font-size: 10px;
		border: 1px solid var(--ink);
		padding: 0 0.3rem;
	}
	.chip.regular {
		border-color: var(--border-muted);
	}
	.bad {
		margin-left: auto;
		color: var(--error);
		font-weight: 700;
	}
</style>
