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
		lookImage,
		personaIds,
		pickText,
		findPersona,
		castSeasons,
		searchCharacters,
		validateAssetRequests,
		validateFile,
		validateLocales,
		validateCast,
		validateCharacters,
		validateMoods,
		type AssetRequests,
		type Problem,
		type RegistryCharacter,
		type ToonCast
	} from '@scutoons/content';
	import { shownCharacterName, textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { CloseIcon, Icon } from '@scutoons/ui';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import {
		addCharacter,
		castDoc,
		castWithStar,
		castOf,
		characterLanguages,
		edit,
		imageExists,
		loadToons,
		moodsDoc,
		moodsOf,
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
	import { library } from '$lib/editor/library.svelte.ts';
	import { useTabs } from '$lib/shell/shell.svelte.ts';
	import MetaPage from '$lib/components/page/meta-page.svelte';
	import CharacterEditor from '$lib/components/characters/character-editor.svelte';
	import CharacterCards from '$lib/components/characters/character-cards.svelte';
	import AreaTabs from '$lib/components/characters/character-area-tabs.svelte';
	import CharacterTree from '$lib/components/characters/character-tree.svelte';
	import { KanonDocs } from '$lib/editor/kanon-docs.svelte.ts';
	import MoodsPanel from '$lib/components/characters/moods-panel.svelte';
	import WantedImages from '$lib/components/characters/wanted-images.svelte';
	import { mapDoc, mapOf, mapPlaces } from '$lib/editor/map.ts';

	/**
	 * The Characters area: the registry (entity `characters`) as cards (search, toon filter, missing images) with the
	 * Wanted images beside it; a card opens the character's workspace (CharacterEditor) with the characters as the Tree.
	 * Every toon's cast is read for "appears in", the filters, the Library's cast facet and the validation – and edited in
	 * a character's Links (saved with the page). The mood words are the Moods panel's Vocabulary tab.
	 */
	const registry = registryDoc();
	const moods = moodsDoc();
	const requests = requestsDoc();
	// the Kanon texts of the characters, personas and looks (K3, #82): opened from their squares, saved with this page
	const kanon = new KanonDocs();
	onMount(() => void kanon.refresh());
	// the homes' places (research/CAVE.md D1): an unknown one warns – add it in the map builder
	let placeIds = $state<string[] | undefined>(undefined);
	onMount(() => void mapPlaces().then((list) => (placeIds = list.length ? list.map((p) => p.id) : undefined)));
	// the world map: its places without an image are listed in Wanted images (read-only here, never saved from this page)
	const worldMap = mapDoc();
	const docs = [registry, moods, requests];

	let toons = $state<ToonInfo[]>([]);
	let castDocs = $state<Record<string, ContentDoc>>({});
	const casts = $derived<Record<string, ToonCast | null>>(Object.fromEntries(Object.entries(castDocs).map(([t, d]) => [t, castOf(d)])));

	const params = page.url.searchParams;
	let selected = $state<string>(params.get('id') ?? '');
	// the History panel follows the selection (owner D5): every character is a document of its own
	$effect(() => {
		registry.selectedKey = selected ? `character:${selected}` : null;
	});
	let persona = $state<string>(params.get('persona') ?? REGULAR_PERSONA);
	let look = $state<string>(params.get('look') ?? '');
	let tab = $state<string>(params.get('tab') === 'wanted' ? 'wanted' : 'characters');
	let query = $state('');
	let toonFilter = $state<string>(params.get('toon') ?? '');
	let missingOnly = $state(false);
	let creating = $state(false);
	let newName = $state('');
	let nameInput = $state<HTMLInputElement>();

	onMount(async () => {
		await Promise.all([registry.load(), moods.load(), requests.load(), worldMap.load()]);
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
	// the Library's cast facet: every toon season's characters
	$effect(() => {
		const list = Object.entries(casts).flatMap(([toon, cast]) => castSeasons(cast).map((s) => ({ address: entities.season(toon, s), characters: (cast?.seasons[String(s)] ?? []).map((e) => e.character) })));
		untrack(() => library.setCasts(list));
		return () => untrack(() => library.setCasts([]));
	});
	/** a cast edited in Links keeps its toon's star in every season */
	function beforeSave() {
		for (const [toon, doc] of Object.entries(castDocs)) if (doc.dirty) castWithStar(doc, registry, toon, castSeasons(castOf(doc)));
	}

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
			// the address bar, not page.url: replaceState (shallow routing) leaves page.url at the last navigation
			const url = new URL(location.href);
			for (const [key, value] of [['id', id], ['persona', p === REGULAR_PERSONA ? '' : p], ['tab', t === 'characters' ? '' : t]] as const) {
				if (value) url.searchParams.set(key, value);
				else url.searchParams.delete(key);
			}
			if (url.href !== location.href) replaceState(url, {});
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
					toons: toons.length ? toons.map((t) => t.id) : undefined,
					places: placeIds,
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
					out.push({ level: 'error', file: registryFile, locale: null, path: ['characters', i, 'personas', pi, 'looks', li, 'images', DEFAULT_MOOD, AUTHORED_VIEW], message: `${shownCharacterName(c, issue.persona)} (${issue.look}): the neutral portrait is not on the CDN: ${issue.path}` });
				}
			});
		}
		if (moods.present.length) out.push(...validateMoods(vocabulary, allLanguages, entities.moods));
		const r = requestsOf(requests);
		if (r) out.push(...validateAssetRequests(r, file, entities.assetRequests));
		for (const doc of kanon.all()) {
			for (const l of doc.locales) if (doc.data[l] != null) out.push(...validateFile('kanon', doc.data[l], l, { file: doc.entity }));
			out.push(...validateLocales('kanon', doc.entity, doc.data, doc.primary));
		}
		for (const [toon, doc] of Object.entries(castDocs)) {
			const cast = castOf(doc);
			if (cast) out.push(...validateCast(cast, { registry: file, languages: toons.find((t) => t.id === toon)?.languages.languages ?? ['de'], file: entities.cast(toon) }));
		}
		return out;
	}

	/** the toons a character is in: its own + the casts, each once */
	const toonsOf = (c: RegistryCharacter) => [...new Set([c.own_toon, ...castsOf(c.id, casts)].filter((t): t is string => !!t))];
	// the selected character is the deepest crumb (Creator / Characters / <name>); an open one lists the others as the Tree
	usePageNav({
		get fill() {
			return !!selectedCharacter;
		},
		extra: () => {
			const c = tab === 'characters' ? characters.find((x) => x.id === selected) : undefined;
			return c ? [{ href: `/characters?id=${encodeURIComponent(c.id)}`, label: shownCharacterName(c) || c.id }] : [];
		}
	});
	// URL → state (the Universe links in the sidebar, crumbs, back / forward); the effect below mirrors state → URL
	$effect(() => {
		const params = page.url.searchParams;
		const nextTab = params.get('tab') === 'wanted' ? 'wanted' : 'characters';
		const nextId = params.get('id');
		untrack(() => {
			if (nextTab !== tab) tab = nextTab;
			// the crumb "Characters" (no id) goes back to the list
			if ((nextId ?? '') !== selected && (nextId !== null || nextTab === 'characters')) selected = nextId ?? '';
		});
	});
	const selectedCharacter = $derived(tab === 'characters' ? characters.find((c) => c.id === selected) : undefined);
	// the Moods panel's tabs: the mood words always, the moods once a character is open (its workspace lends them)
	useTabs(() => ({ tree: selectedCharacter ? tree : null, vocabulary: vocabularyTab, moods: selectedCharacter ? null : moodsHint }));
	const openRequests = $derived((requestsOf(requests)?.requests ?? []).filter((r) => r.status === 'open').length);
</script>

{#snippet tree()}
	<CharacterTree {characters} {selected} {imageIssues} onselect={(id) => select(id)} />
{/snippet}

{#snippet moodsHint()}
	<p class="sc-empty hint-pad">{m['creator.characters_moods_pick']()}</p>
{/snippet}

{#snippet vocabularyTab()}
	<MoodsPanel {moods} registry={file} languages={allLanguages} />
{/snippet}

<MetaPage toon="" title={m['creator.characters']()} {docs} along={() => [...kanon.all(), ...Object.values(castDocs)]} {validate} {beforeSave} wide fill={!!selectedCharacter} help="characters">
	{#snippet actions()}
		<button type="button" class="secondary" onclick={startNew} data-testid="new-character"><Icon name="plus" /> {m['creator.characters_new']()}</button>
	{/snippet}
	<div class="area" class:open={!!selectedCharacter}>
		{#if !selectedCharacter}<div class="tabs-row"><AreaTabs {tab} counts={{ characters: characters.length, wanted: openRequests }} /></div>{/if}
		{#if tab === 'wanted'}
			<div class="pad-top"><WantedImages registry={file} moods={vocabulary} requests={requestsOf(requests)} locale={textLanguage.names} onopen={(id, p, l) => select(id, p, l)} map={mapOf(worldMap)} /></div>
		{:else if registry.loading}
			<p class="hint pad-top">{m['creator.loading']()}</p>
		{:else if selectedCharacter}
			{#key selected}
				<CharacterEditor {registry} moodsDoc={moods} requestsDoc={requests} moods={vocabulary} id={selected} bind:persona bind:look {toons} {casts} {castDocs} {kanon} onselect={(id) => select(id)} />
			{/key}
		{:else}
			{#if creating}
				<span class="new">
					<input bind:this={nameInput} type="text" bind:value={newName} placeholder={m['creator.characters_name']()} onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); create(); } else if (e.key === 'Escape') creating = false; }} data-testid="new-character-name" />
					<button type="button" onclick={create} disabled={!newName.trim()} data-testid="new-character-create">{m['creator.characters_create']()}</button>
					<button type="button" class="ghost" onclick={() => (creating = false)} aria-label={m['creator.remove']()} title={m['creator.remove']()}><CloseIcon /></button>
				</span>
			{/if}
			<CharacterCards characters={visible} {toons} {toonsOf} issues={(c) => imageIssues(c).length} bind:query bind:toonFilter bind:missingOnly onselect={(id) => select(id)} />
		{/if}
	</div>
</MetaPage>

<style>
	.area {
		display: flex;
		flex-direction: column;
		min-height: 0;
	}
	/* an open character fills the page (MetaPage fill): its picture is the workspace's background */
	.area.open {
		height: 100%;
	}
	/* the tabs run edge to edge: out of the page's padding */
	.tabs-row {
		margin: -1rem -1rem 0;
	}
	.area.open > :global(.char-workspace) {
		flex: 1;
		min-height: 0;
	}
	.hint-pad {
		margin: 0;
		padding: 0 var(--editor-pad) var(--editor-pad);
	}
	.pad-top {
		padding-top: 0.75rem;
	}
	.new {
		display: flex;
		gap: 0.3rem;
		max-width: 40rem;
		padding-top: var(--space-1);
	}
</style>
