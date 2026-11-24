<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import { page } from '$app/state';
	import { goto, replaceState } from '$app/navigation';
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
	import { shownCharacterName } from '$lib/editor/text-language.svelte.ts';
	import { askDialog, Icon } from '@scutoons/ui';
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
	import { shelf } from '$lib/editor/library-shelf.svelte.ts';
	import MetaPage from '$lib/components/page/meta-page.svelte';
	import CharacterEditor from '$lib/components/characters/character-editor.svelte';
	import { KanonDocs } from '$lib/editor/kanon-docs.svelte.ts';
	import MoodsPanel from '$lib/components/characters/moods-panel.svelte';
	import { mapPlaces } from '$lib/editor/map.ts';

	/**
	 * A character's page: the registry (entity `characters`) with the open character as its workspace (CharacterEditor,
	 * opened from the Library's Items to open). No character open: Home with the
	 * Library's characters. Every toon's cast is read for "appears in", a character's Cast tab ("cast in") and the validation;
	 * a cast is edited in its toon's season. The mood words sit under its Expressions tab.
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
	/** a new character's toon when the page was opened for one (`?toon=`) */
	const toonFilter = params.get('toon') ?? '';

	onMount(async () => {
		await Promise.all([registry.load(), moods.load(), requests.load()]);
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

	function select(id: string, p: string = REGULAR_PERSONA, l = '') {
		selected = id;
		persona = p;
		look = l;
	}
	// the URL mirrors the selection (shareable, back to the same place)
	$effect(() => {
		const id = selected;
		const p = persona;
		try {
			// the address bar, not page.url: replaceState (shallow routing) leaves page.url at the last navigation
			const url = new URL(location.href);
			for (const [key, value] of [['id', id], ['persona', p === REGULAR_PERSONA ? '' : p], ['tab', '']] as const) {
				if (value) url.searchParams.set(key, value);
				else url.searchParams.delete(key);
			}
			if (url.href !== location.href) replaceState(url, {});
		} catch {
			/* before the router is ready */
		}
	});

	/** a new character: its name asked in the page, then it opens (its toon: the open character's own toon, if any) */
	async function startNew() {
		const name = (await askDialog({ title: m['creator.characters_new'](), label: m['creator.characters_name'](), confirmLabel: m['creator.characters_create'](), cancelLabel: m['creator.cancel']() }))?.trim();
		if (!name) return;
		const ownId = toonFilter || characters.find((c) => c.id === selected)?.own_toon;
		const own = ownId ? toons.find((t) => t.id === ownId) : undefined;
		const languages = own ? (own.languages.languages as string[]) : allLanguages;
		select(addCharacter(registry, name, languages, own ? { own_toon: own.id } : {}));
	}

	/** Home, the Library on the characters */
	function showCharacters() {
		shelf.choice = 'character';
		void goto('/', { replaceState: true });
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

	// the selected character is the deepest crumb (Creator / Characters / <name>)
	usePageNav({
		get fill() {
			return !!selectedCharacter;
		},
		extra: () => {
			const c = characters.find((x) => x.id === selected);
			return c ? [{ href: `/characters?id=${encodeURIComponent(c.id)}`, label: shownCharacterName(c) || c.id }] : [];
		}
	});
	// URL → state (the Universe links in the sidebar, crumbs, back / forward); the effect below mirrors state → URL
	$effect(() => {
		const params = page.url.searchParams;
		const nextId = params.get('id');
		untrack(() => {
			if ((nextId ?? '') !== selected) selected = nextId ?? '';
		});
	});
	const selectedCharacter = $derived(characters.find((c) => c.id === selected));
	// no character open (one removed, or an id that is gone): Home with the Library's characters
	$effect(() => {
		if (!registry.loading && registry.present.length && !selectedCharacter) untrack(() => showCharacters());
	});
</script>

{#snippet vocabularyTab()}
	<MoodsPanel {moods} registry={file} languages={allLanguages} />
{/snippet}

<MetaPage toon="" title={m['creator.characters']()} {docs} along={() => kanon.all()} {validate} wide fill={!!selectedCharacter} help="characters">
	{#snippet actions()}
		<button type="button" class="secondary" onclick={startNew} data-testid="new-character"><Icon name="plus" /> {m['creator.characters_new']()}</button>
	{/snippet}
	<div class="area" class:open={!!selectedCharacter}>
		{#if registry.loading}
			<p class="hint pad-top">{m['creator.loading']()}</p>
		{:else if selectedCharacter}
			{#key selected}
				<CharacterEditor {registry} moodsDoc={moods} requestsDoc={requests} moods={vocabulary} id={selected} bind:persona bind:look {toons} {casts} {kanon} vocabulary={vocabularyTab} onselect={(id) => select(id)} />
			{/key}
		{/if}
	</div>
</MetaPage>

<style>
	.area {
		display: flex;
		flex-direction: column;
		min-height: 0;
	}
	/* an open character fills the page (MetaPage fill): its own view is the workspace's stage */
	.area.open {
		height: 100%;
	}
	.area.open > :global(.char-workspace) {
		flex: 1;
		min-height: 0;
	}
	.pad-top {
		padding-top: 0.75rem;
	}
</style>
