// Characters editor: the platform-wide files of the characters model (see @scutoons/content characters.ts) –
// `scutoons/_/{characters,moods,asset-requests}.json` – and the per-toon `<toon>/_/cast.json`, as ContentDocs with
// the single locale `_`, saved / published through the ActionBar of the page that edits them (Characters page, Cast page,
// chapter editor when the picker adds a character to the season's cast).
import {
	ASSET_REQUESTS_FILE,
	AUTHORED_VIEW,
	CAST_FILE,
	CHARACTERS_FILE,
	CREDITS_SCOPE,
	MOODS_FILE,
	NEUTRAL_LOCALE,
	PEOPLE_FILE,
	REGULAR_PERSONA,
	conventionImagePath,
	defaultMoodVocabulary,
	emptyAssetRequests,
	emptyCast,
	emptyCharacterRegistry,
	findPersona,
	newCharacter,
	newLook,
	newPersona,
	slugId,
	toonLanguages,
	uniqueId,
	type AssetRequests,
	type CharacterPersona,
	type CharacterRegistry,
	type LocaleText,
	type MoodVocabulary,
	type PeopleRegistry,
	type RegistryCharacter,
	type ToonCast,
	type ToonIndex,
	type ToonLanguages
} from '@scutoons/content';
import { STATIC_API } from '$lib/config.ts';
import { asset } from '$lib/utils/cdn.ts';
import { ContentDoc } from './doc.svelte.ts';
import { loadToonEntry } from './languages.ts';

const N = NEUTRAL_LOCALE;

/** Language-neutral store file without a static fallback. */
const neutralDoc = (toon: string, path: string) => new ContentDoc(toon, path, [N], N, async () => null);

export const registryDoc = () => neutralDoc(CREDITS_SCOPE, CHARACTERS_FILE);
export const castDoc = (toon: string) => neutralDoc(toon, CAST_FILE);
export const peopleDoc = () => neutralDoc(CREDITS_SCOPE, PEOPLE_FILE);
export const moodsDoc = () => neutralDoc(CREDITS_SCOPE, MOODS_FILE);
export const requestsDoc = () => neutralDoc(CREDITS_SCOPE, ASSET_REQUESTS_FILE);

export const registryOf = (doc: ContentDoc | null | undefined) => (doc?.data[N] ?? null) as CharacterRegistry | null;
export const castOf = (doc: ContentDoc | null | undefined) => (doc?.data[N] ?? null) as ToonCast | null;
export const peopleListOf = (doc: ContentDoc | null | undefined) => ((doc?.data[N] ?? null) as PeopleRegistry | null)?.people ?? [];
/** the stored vocabulary, else the built-in seed (17 standard + dizzy, tripping) */
export const moodsOf = (doc: ContentDoc | null | undefined) => ((doc?.data[N] ?? null) as MoodVocabulary | null) ?? defaultMoodVocabulary();
export const requestsOf = (doc: ContentDoc | null | undefined) => (doc?.data[N] ?? null) as AssetRequests | null;

/** Starts a file when the store has none yet. */
export function ensure(doc: ContentDoc, make: () => unknown) {
	if (!doc.present.length) doc.create(make);
}
export const ensureRegistry = (doc: ContentDoc) => ensure(doc, emptyCharacterRegistry);
export const ensureCast = (doc: ContentDoc) => ensure(doc, emptyCast);
export const ensureMoods = (doc: ContentDoc) => ensure(doc, defaultMoodVocabulary);
export const ensureRequests = (doc: ContentDoc) => ensure(doc, emptyAssetRequests);

/** Edits the neutral file of a doc (one undo step; same `key` within ~1 s = one step). */
export function edit<T>(doc: ContentDoc, change: (file: T) => void, key?: string) {
	doc.structural((file) => change(file as T), key);
}

/** Edits one character of the registry by id. */
export function editCharacter(doc: ContentDoc, id: string, change: (character: RegistryCharacter) => void, key?: string) {
	edit<CharacterRegistry>(
		doc,
		(file) => {
			const c = file.characters.find((x) => x.id === id);
			if (c) change(c);
		},
		key
	);
}

/** Edits one persona of a character. */
export function editPersona(doc: ContentDoc, id: string, persona: string, change: (p: CharacterPersona, c: RegistryCharacter) => void, key?: string) {
	editCharacter(
		doc,
		id,
		(c) => {
			const p = findPersona(c, persona);
			if (p) change(p, c);
		},
		key
	);
}

const firstText = (languages: string[], text: string): LocaleText => Object.fromEntries(languages.map((l, i) => [l, i === 0 ? text.trim() : '']));

/** A new registry character (id from the name, unique, persona `regular` with an empty default look). Returns its id. */
export function addCharacter(doc: ContentDoc, name: string, languages: string[], extra: Partial<RegistryCharacter> = {}): string {
	ensureRegistry(doc);
	const registry = registryOf(doc)!;
	const id = uniqueId(slugId(name), registry.characters.map((c) => c.id));
	edit<CharacterRegistry>(doc, (file) => file.characters.push(newCharacter(id, firstText(languages, name), extra)));
	return id;
}

/** A new persona of a character (id from the label). Returns its id. */
export function addPersona(doc: ContentDoc, character: string, label: string, languages: string[]): string {
	const c = registryOf(doc)?.characters.find((x) => x.id === character);
	const id = uniqueId(slugId(label), [REGULAR_PERSONA, ...(c?.personas ?? []).map((p) => p.id)]);
	editCharacter(doc, character, (x) => x.personas.push(newPersona(id, firstText(languages, label))));
	return id;
}

/** A new look of a persona (id from the label, not the default). Returns its id. */
export function addLook(doc: ContentDoc, character: string, persona: string, label: string, languages: string[]): string {
	const p = findPersona(registryOf(doc)?.characters.find((x) => x.id === character), persona);
	const id = uniqueId(slugId(label), (p?.looks ?? []).map((l) => l.id));
	editPersona(doc, character, persona, (x) => x.looks.push(newLook(id, firstText(languages, label), !x.looks.length)));
	return id;
}

/**
 * Adds a character (and a persona of it) to a season's cast. Returns what changed (for an undo notice):
 * 'character' (new entry), 'persona' (persona added to an entry), or null (already there).
 */
export function addToSeasonCast(doc: ContentDoc, season: number, character: string, persona: string = REGULAR_PERSONA): 'character' | 'persona' | null {
	ensureCast(doc);
	const list = castOf(doc)?.seasons?.[String(season)] ?? [];
	const entry = list.find((e) => e.character === character);
	if (entry?.personas.includes(persona)) return null;
	edit<ToonCast>(doc, (file) => {
		const entries = (file.seasons[String(season)] ??= []);
		const existing = entries.find((e) => e.character === character);
		if (existing) existing.personas.push(persona);
		else entries.push({ character, personas: [persona] });
	});
	return entry ? 'persona' : 'character';
}

/** A new asset request (wanted image) for a cell; returns its id. */
export function addAssetRequest(
	doc: ContentDoc,
	cell: { character: string; persona: string; look: string; mood: string; view?: string },
	note: string,
	by: string | null
): string {
	ensureRequests(doc);
	const view = cell.view ?? AUTHORED_VIEW;
	const id = `${cell.character}-${cell.persona}-${cell.look}-${cell.mood}-${Date.now().toString(36)}`;
	edit<AssetRequests>(doc, (file) =>
		file.requests.push({
			id,
			...cell,
			view,
			path: conventionImagePath(cell.character, view, cell.mood, cell.look, cell.persona),
			...(note.trim() ? { note: note.trim() } : {}),
			requested_by: by,
			requested_at: new Date().toISOString(),
			status: 'open'
		})
	);
	return id;
}

// ── Toons (languages per toon) ────────────────────────────────────────────────────────────────────────────────

export interface ToonInfo {
	id: string;
	name: string;
	languages: ToonLanguages;
}

/** Every toon (viewer's toons.json) with its languages (newest index entry) – for filters and language lists. */
export async function loadToons(locale: string): Promise<ToonInfo[]> {
	let list: ToonIndex[] = [];
	try {
		const response = await fetch(`${STATIC_API}/toons.json`, { cache: 'no-cache' });
		if (response.ok) list = await response.json();
	} catch {
		/* none */
	}
	return Promise.all(
		(Array.isArray(list) ? list : []).map(async (entry) => {
			const current = (await loadToonEntry(entry.id).catch(() => null)) ?? entry;
			const languages = toonLanguages(current);
			const name = current.i18n?.[locale]?.name ?? current.i18n?.[languages.source]?.name ?? entry.id;
			return { id: entry.id, name, languages };
		})
	);
}

/**
 * The languages a character's texts need, source first: its own toon's, else those of the toons whose cast lists it,
 * else every toon language (de first).
 */
export function characterLanguages(character: RegistryCharacter | null | undefined, toons: ToonInfo[], castToons: string[]): ToonLanguages {
	const relevant = [character?.own_toon, ...castToons].filter((t): t is string => !!t);
	const infos = relevant.map((t) => toons.find((x) => x.id === t)).filter((x): x is ToonInfo => !!x);
	const pool = infos.length ? infos : toons;
	const source = infos[0]?.languages.source ?? 'de';
	const languages = [...new Set([source, ...pool.flatMap((t) => t.languages.languages)])];
	return { source, languages } as ToonLanguages;
}

// ── CDN check (✓ found / ✗ missing), cached per URL for the session ────────────────────────────────────────────

const found = new Map<string, Promise<boolean>>();

/** true when the CDN serves an image at `path` (from the CDN root). */
export function imageExists(path: string): Promise<boolean> {
	if (!path) return Promise.resolve(false);
	const url = asset(path);
	if (!url) return Promise.resolve(false);
	let pending = found.get(url);
	if (!pending) {
		pending = new Promise<boolean>((resolve) => {
			const img = new Image();
			img.onload = () => resolve(true);
			img.onerror = () => resolve(false);
			img.src = url;
		});
		found.set(url, pending);
	}
	return pending;
}

/** Forgets the cached results (after uploading images). */
export function recheckImages() {
	found.clear();
}
