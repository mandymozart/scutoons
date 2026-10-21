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
	import { shownCharacterName, textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { CloseIcon, Icon, type PhoneBarItem } from '@scutoons/ui';
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
	import MetaPage from '$lib/components/page/meta-page.svelte';
	import CharacterEditor from '$lib/components/characters/character-editor.svelte';
	import CharacterCards from '$lib/components/characters/character-cards.svelte';
	import AreaTabs from '$lib/components/characters/character-area-tabs.svelte';
	import type { InspectorTab } from '$lib/components/characters/character-inspector.svelte';
	import MoodsPanel from '$lib/components/characters/moods-panel.svelte';
	import WantedImages from '$lib/components/characters/wanted-images.svelte';
	import { mapDoc, mapOf, mapPlaces, mapRegions } from '$lib/editor/map.ts';

	/**
	 * The global Characters page (research/CHARACTER-PAGE.md E + K3, owner 2026-10-20): the area's tabs Characters ·
	 * Moods · Wanted images; Characters = the registry (entity `characters`) as cards (search, toon filter, missing
	 * images) – a card opens the character's editor (CharacterEditor: face rows, stage, inspector). Casts of every toon
	 * are read for "appears in", the filters and the validation; people for "voiced by".
	 */
	const registry = registryDoc();
	const moods = moodsDoc();
	const requests = requestsDoc();
	const people = peopleDoc();
	// the map's region ids (origin / residence – agents/MAP.md §8.10); undefined until loaded (no false warnings)
	let regionIds = $state<string[] | undefined>(undefined);
	onMount(() => void mapRegions().then((list) => (regionIds = list.length ? list.map((r) => r.id) : undefined)));
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
	let tab = $state<string>(['moods', 'wanted'].includes(params.get('tab') ?? '') ? (params.get('tab') as string) : 'characters');
	let query = $state('');
	let toonFilter = $state<string>(params.get('toon') ?? '');
	let missingOnly = $state(false);
	let creating = $state(false);
	let newName = $state('');
	let nameInput = $state<HTMLInputElement>();

	onMount(async () => {
		await Promise.all([registry.load(), moods.load(), requests.load(), people.load(), worldMap.load()]);
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
					people: people.loading ? undefined : peopleListOf(people).map((p) => p.id),
					toons: toons.length ? toons.map((t) => t.id) : undefined,
					regions: regionIds,
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
		for (const [toon, doc] of Object.entries(castDocs)) {
			const cast = castOf(doc);
			if (cast) out.push(...validateCast(cast, { registry: file, languages: toons.find((t) => t.id === toon)?.languages.languages ?? ['de'], file: entities.cast(toon) }));
		}
		return out;
	}

	/** the toons a character is in: its own + the casts, each once */
	const toonsOf = (c: RegistryCharacter) => [...new Set([c.own_toon, ...castsOf(c.id, casts)].filter((t): t is string => !!t))];
	// the selected character is the deepest crumb (Creator / Characters / <name>)
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
		const nextTab = ['moods', 'wanted'].includes(params.get('tab') ?? '') ? (params.get('tab') as string) : 'characters';
		const nextId = params.get('id');
		untrack(() => {
			if (nextTab !== tab) tab = nextTab;
			// the crumb "Characters" (no id) goes back to the list
			if ((nextId ?? '') !== selected && (nextId !== null || nextTab === 'characters')) selected = nextId ?? '';
		});
	});
	// phones: an open character is the selection – its row in the bar (owner 2026-10-12: "das Zurück unten in der
	// Auswahl-Leiste", UI-CONCEPTS E.10; #44: one row of icon squares with ‹): the inspector's tabs open its sheet, the
	// grid square every mood, Delete; ‹ back to the cards
	let detail = $state<ReturnType<typeof CharacterEditor>>();
	let inspectorTab = $state<InspectorTab>('profile');
	let sheet = $state(false);
	let grid = $state(false);
	const selectedCharacter = $derived(tab === 'characters' ? characters.find((c) => c.id === selected) : undefined);
	function openTab(t: InspectorTab) {
		if (sheet && inspectorTab === t) sheet = false;
		else {
			inspectorTab = t;
			sheet = true;
		}
	}
	const phoneSelection = $derived(
		selectedCharacter
			? {
					label: shownCharacterName(selectedCharacter) || selectedCharacter.id,
					onback: () => ((selected = ''), (sheet = false)),
					items: [
						{ id: 'profile', icon: 'user', label: m['creator.characters_tab_profile'](), active: sheet && inspectorTab === 'profile', run: () => openTab('profile'), testid: 'phone-sel-profile' },
						{ id: 'model', icon: 'box', label: m['creator.characters_tab_model'](), active: sheet && inspectorTab === 'model', run: () => openTab('model'), testid: 'phone-sel-model' },
						{ id: 'wanted', icon: 'inbox', label: m['creator.characters_wanted'](), active: sheet && inspectorTab === 'wanted', run: () => openTab('wanted'), testid: 'phone-sel-wanted' },
						{ id: 'prompts', icon: 'sparkles', label: m['creator.prompts_title'](), active: sheet && inspectorTab === 'prompts', run: () => openTab('prompts'), testid: 'phone-sel-prompts' },
						{ id: 'grid', icon: 'grid', label: m['creator.characters_all_moods'](), active: grid, run: () => ((grid = !grid), (sheet = false)), testid: 'phone-sel-grid' },
						{ id: 'delete', icon: 'trash-2', label: m['creator.bar_delete'](), run: () => detail?.remove(), testid: 'phone-sel-delete' }
					] satisfies PhoneBarItem[]
				}
			: null
	);
	const openRequests = $derived((requestsOf(requests)?.requests ?? []).filter((r) => r.status === 'open').length);
</script>

<MetaPage toon="" title={m['creator.characters']()} {docs} {validate} wide fill={!!selectedCharacter} help="characters" {phoneSelection}>
	{#snippet actions()}
		<button type="button" class="secondary" onclick={startNew} data-testid="new-character"><Icon name="plus" /> {m['creator.characters_new']()}</button>
	{/snippet}
	<div class="area" class:open={!!selectedCharacter}>
		<div class="tabs-row"><AreaTabs {tab} counts={{ characters: characters.length, moods: vocabulary.moods.length, wanted: openRequests }} /></div>
		{#if tab === 'moods'}
			<div class="pad-top"><MoodsPanel {moods} registry={file} languages={allLanguages} /></div>
		{:else if tab === 'wanted'}
			<div class="pad-top"><WantedImages registry={file} moods={vocabulary} requests={requestsOf(requests)} locale={textLanguage.names} onopen={(id, p, l) => select(id, p, l)} map={mapOf(worldMap)} /></div>
		{:else if registry.loading}
			<p class="hint pad-top">{m['creator.loading']()}</p>
		{:else if selectedCharacter}
			{#key selected}
				<CharacterEditor
					bind:this={detail}
					{registry}
					moodsDoc={moods}
					requestsDoc={requests}
					moods={vocabulary}
					id={selected}
					bind:persona
					bind:look
					bind:tab={inspectorTab}
					bind:grid
					bind:sheet
					{toons}
					{casts}
					people={peopleListOf(people)}
					onselect={(id) => select(id)}
				/>
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
	/* an open character fills the page (MetaPage fill): the tabs on top, the editor the rest */
	.area.open {
		height: 100%;
	}
	/* the tabs run edge to edge: out of the page's padding (none while an open character fills the page) */
	.tabs-row {
		margin: -1rem -1rem 0;
	}
	.area.open .tabs-row {
		margin: 0;
	}
	.area.open > :global(.char-editor) {
		flex: 1;
		min-height: 0;
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
	/* phones: an open character is the whole page (the bar's ‹ goes back to the cards) */
	@media (max-width: 767px) {
		.area.open .tabs-row {
			display: none;
		}
	}
</style>
