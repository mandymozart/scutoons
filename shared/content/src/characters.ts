// Characters: every character of every toon in one global registry, each with the same set of expressions (moods ×
// variants, per look), plus a cast per toon. Language-neutral store files with translated texts inside:
//
//   scutoons/_/characters.json   CharacterRegistry – every character, global (a character may appear in several toons
//                                                    and may have its own toon, `own_toon`)
//   scutoons/_/groups.json       GroupVocabulary   – shared affiliations across toons (e.g. the biker gang); characters
//                                                    reference them in `affiliations` (with a role, since when)
//   <toon>/_/cast.json           ToonCast          – which characters a toon uses, in order, with a role per toon
//
// A character may join the cast of any toon, also when it has its own toon (the mouse in catpope's cast).
//
// Personas: a different appearance of the same person (Cat Pope after he joins a biker gang) is its own registry
// character with `persona_of: <base id>` + `persona` (label, introduced, note). It has its own images and cast membership
// and can meet its base in one scene; name / color / voiced_by / own_toon are inherited unless set (`effectiveCharacter`),
// the shown name is "<name> · <persona label>". Credits and people stay with the base identity.
//
// Looks: the same character in different gear (with / without helmet), each with a full mood set. `images` is keyed by
// look: `images.<look>.<mood>.<variant>`; characters without `looks` have the one look `default`. Resolution (VN
// `joinScene` `look` + `mood`): look+mood → look neutral → default look+mood → default neutral → the first look with
// images → the legacy `image` (`characterImage`).
//
// The mood vocabulary is a constant of this package (MOODS, labels per locale in MOOD_LABELS; optional EXTRA_MOODS a
// character may have in addition, e.g. `dizzy`). `resolveCast()` turns the registry + a cast into the legacy
// `characters.json` shape of one locale (`Character[]`, visual-novel.ts; `images` = the default look, `looks` = all),
// so the viewer's VN renderer keeps reading it; `<toon>/<locale>/characters.json` stays the static fallback.
import type { LocaleText } from './credits.ts';
import { pickText } from './credits.ts';
import type { Character, CharacterImage } from './visual-novel.ts';
import type { Path } from './paths.ts';
import type { Level, Problem } from './validate.ts';

// ── Vocabulary ────────────────────────────────────────────────────────────────────────────────────────────────

/** The 17 moods every character can have, in display order (the expression grid's rows). */
export const MOODS = [
	'neutral',
	'happy',
	'laughing',
	'sad',
	'crying',
	'angry',
	'furious',
	'surprised',
	'scared',
	'disgusted',
	'confused',
	'thinking',
	'smug',
	'embarrassed',
	'denial',
	'bargaining',
	'acceptance'
] as const;
export type Mood = (typeof MOODS)[number];

/** The mood every renderer falls back to. */
export const DEFAULT_MOOD: Mood = 'neutral';

/** Label of each mood per locale (languages without a label show the English one). */
export const MOOD_LABELS: Record<Mood, LocaleText> = {
	neutral: { de: 'neutral', en: 'neutral' },
	happy: { de: 'fröhlich', en: 'happy' },
	laughing: { de: 'lachend', en: 'laughing' },
	sad: { de: 'traurig', en: 'sad' },
	crying: { de: 'weinend', en: 'crying' },
	angry: { de: 'verärgert', en: 'angry' },
	furious: { de: 'wütend', en: 'furious' },
	surprised: { de: 'überrascht', en: 'surprised' },
	scared: { de: 'verängstigt', en: 'scared' },
	disgusted: { de: 'angeekelt', en: 'disgusted' },
	confused: { de: 'verwirrt', en: 'confused' },
	thinking: { de: 'nachdenklich', en: 'thinking' },
	smug: { de: 'selbstgefällig', en: 'smug' },
	embarrassed: { de: 'verlegen', en: 'embarrassed' },
	denial: { de: 'Leugnen', en: 'denial' },
	bargaining: { de: 'Verhandeln', en: 'bargaining' },
	acceptance: { de: 'Akzeptanz', en: 'acceptance' }
};

/** Optional moods beyond the shared 17 that single characters have (shown below the base moods). */
export const EXTRA_MOODS = ['dizzy', 'tripping'] as const;
export type ExtraMood = (typeof EXTRA_MOODS)[number];

export const EXTRA_MOOD_LABELS: Record<ExtraMood, LocaleText> = {
	dizzy: { de: 'benommen', en: 'dizzy' },
	tripping: { de: 'auf Trip', en: 'tripping' }
};

export const moodLabel = (mood: string, locale: string): string => {
	const labels = MOOD_LABELS[mood as Mood] ?? EXTRA_MOOD_LABELS[mood as ExtraMood];
	return labels ? pickText(labels, locale, ['en']) : mood;
};

/** One of the shared 17 moods. */
export const isMood = (value: unknown): value is Mood => typeof value === 'string' && (MOODS as readonly string[]).includes(value);
/** A shared or an optional (extra) mood. */
export const isKnownMood = (value: unknown): value is Mood | ExtraMood =>
	isMood(value) || (typeof value === 'string' && (EXTRA_MOODS as readonly string[]).includes(value));
export const MOOD_ID = /^[a-z][a-z0-9-]*$/;

/** The three image variants ("views") of a character (VN: avatar next to the message box, portrait, full figure). */
export const CHARACTER_IMAGE_VARIANTS = ['avatar', 'portrait', 'full'] as const;
export type CharacterImageVariant = (typeof CHARACTER_IMAGE_VARIANTS)[number];

/** The look of characters without `looks` (and the look whose convention path has no look segment). */
export const DEFAULT_LOOK = 'default';

/** Suggested cast groups (free text is allowed): lead → cast → extra; `support` is the old catpope `collection`. */
export const CHARACTER_GROUPS = ['lead', 'support', 'cast', 'extra'] as const;
export type CharacterGroup = (typeof CHARACTER_GROUPS)[number] | (string & {});

/** Avatar bodies of the reader's avatar (viewer `components/avatar/avatar.svelte`): id → CDN image. */
export const MANNEQUINS: { id: string; image: string }[] = [
	{ id: 'sphynx', image: 'catpope/avatar/mannequin.webp' },
	{ id: 'sphynx_nude', image: 'catpope/avatar/mannequin-nude.webp' },
	{ id: 'mouse', image: 'yellowmouse/avatar/mannequin.webp' }
];

// ── Files ─────────────────────────────────────────────────────────────────────────────────────────────────────

/** A point in a toon's story (a free reference for the editor and the story bible). */
export interface StoryPoint {
	toon: string;
	season?: number;
	chapter?: number;
}

/** What makes a registry character a persona of its base (`persona_of`). */
export interface Persona {
	/** appended to the name: "Cat Pope · Biker" */
	label: LocaleText;
	/** where the persona first appears (none = it has always been there) */
	introduced?: StoryPoint;
	note?: LocaleText;
}

/** The same character in different gear (with / without helmet), each with its own mood set. */
export interface CharacterLook {
	/** `^[a-z0-9][a-z0-9-]*$`; `default` = no look segment in the convention path */
	id: string;
	label: LocaleText;
	/** exactly one look of a character is the default (VN refs without `look`) */
	default?: boolean;
}

/** Character artwork that is not an expression (key visuals, turnarounds …). */
export interface CharacterArtwork {
	/** `^[a-z0-9][a-z0-9-]*$`, unique per character */
	id: string;
	/** CDN path from the CDN root */
	path: string;
	label?: LocaleText;
}

/** Membership of a character in a shared group. */
export interface Affiliation {
	/** group id (GroupVocabulary) */
	group: string;
	/** e.g. "Anführerin" / "Leader" */
	role?: LocaleText;
	since?: StoryPoint;
}

/** mood → variant → CDN path */
export type MoodImages = Partial<Record<string, CharacterImage>>;

/** One character of the registry. Image values are CDN paths from the CDN root (`catpope/images/characters/…`). */
export interface RegistryCharacter {
	/** stable id, referenced by casts and by VN events (`joinScene` `name`) – `^[a-z0-9][a-z0-9-]*$` */
	id: string;
	/** display name per locale (the VN speaker plate); a persona without one uses its base's name */
	name?: LocaleText;
	description?: LocaleText;
	/** CSS color (VN avatar background, speaker accents); a persona inherits its base's when empty */
	color?: string;
	/** cast group: `lead` | `support` | `cast` | `extra` | … */
	group: CharacterGroup;
	/** the toon this character is the protagonist of (e.g. the mouse → `yellowmouse`) */
	own_toon?: string;
	/** person id of the voice (scutoons/_/people.json) */
	voiced_by?: string;
	/** avatar body id (MANNEQUINS) */
	mannequin?: string;
	tags?: string[];
	/** legacy single image (last fallback of every look/mood/variant; placeholder e.g. `<toon>/avatar.webp`) */
	image?: string;
	/** the character's looks; none = the one look `default` */
	looks?: CharacterLook[];
	/** look → mood → variant → CDN path */
	images: Partial<Record<string, MoodImages>>;
	/** key visuals and other artwork */
	artwork?: CharacterArtwork[];
	/** base character id: this character is a persona (another appearance) of it – one level only */
	persona_of?: string;
	persona?: Persona;
	/** groups the character belongs to (scutoons/_/groups.json) */
	affiliations?: Affiliation[];
}

/** `scutoons/_/characters.json` */
export interface CharacterRegistry {
	characters: RegistryCharacter[];
}

/** A shared group / affiliation across toons (the biker gang, a family, a band …). */
export interface AffiliationGroup {
	/** `^[a-z0-9][a-z0-9-]*$` */
	id: string;
	name: LocaleText;
	description?: LocaleText;
	color?: string;
}

/** `scutoons/_/groups.json` */
export interface GroupVocabulary {
	groups: AffiliationGroup[];
}

export interface CastMember {
	/** registry id */
	character: string;
	/** the character's role in this toon, per locale (e.g. "Erzähler" / "Narrator") */
	role?: LocaleText;
	/** display order (ascending) */
	order: number;
}

/** `<toon>/_/cast.json` */
export interface ToonCast {
	members: CastMember[];
}

// ── Store paths ───────────────────────────────────────────────────────────────────────────────────────────────

/** Registry file in the pseudo-toon `scutoons` (CREDITS_SCOPE), locale `_`. */
export const CHARACTERS_FILE = 'characters.json';
/** Shared affiliation groups (pseudo-toon `scutoons`, locale `_`). */
export const GROUPS_FILE = 'groups.json';
/** Per-toon cast file, locale `_`. */
export const CAST_FILE = 'cast.json';
/** The old per-locale file (`<toon>/<locale>/characters.json`, static fallback). */
export const LEGACY_CHARACTERS_FILE = 'characters.json';

export const CHARACTER_ID = /^[a-z0-9][a-z0-9-]*$/;

/** Folder (from the CDN root) of new character images. */
export const CHARACTER_IMAGE_ROOT = 'characters';

/**
 * Where a new image of a character lives by convention (the owner's image supplier uses this naming):
 * `characters/<id>/<id>_<variant>-<mood>.png` for the default look, `characters/<id>/<id>_<look>_<variant>-<mood>.png`
 * for the others (from the CDN root). Existing explicit paths keep working.
 */
export function conventionImagePath(id: string, variant: string, mood: string, look: string = DEFAULT_LOOK): string {
	const lookPart = look && look !== DEFAULT_LOOK ? `_${look}` : '';
	return `${CHARACTER_IMAGE_ROOT}/${id}/${id}${lookPart}_${variant}-${mood}.png`;
}

export const emptyCharacterRegistry = (): CharacterRegistry => ({ characters: [] });
export const emptyCast = (): ToonCast => ({ members: [] });
export const emptyGroups = (): GroupVocabulary => ({ groups: [] });

export function newCharacter(id: string, name: LocaleText, extra: Partial<RegistryCharacter> = {}): RegistryCharacter {
	return { id, name, color: 'black', group: 'cast', images: { [DEFAULT_LOOK]: { [DEFAULT_MOOD]: {} } }, ...extra };
}

// ── Looks ─────────────────────────────────────────────────────────────────────────────────────────────────────

/** The look ids of a character in order: its `looks`, else `default`; image keys without a declared look follow. */
export function lookIds(character: Pick<RegistryCharacter, 'looks' | 'images'> | null | undefined): string[] {
	const declared = (character?.looks ?? []).map((l) => l?.id).filter((id): id is string => typeof id === 'string' && !!id);
	const ids = declared.length ? declared : [DEFAULT_LOOK];
	for (const key of Object.keys(character?.images ?? {})) if (!ids.includes(key)) ids.push(key);
	return ids;
}

/** The default look id: the look marked `default`, else the first look, else `default`. */
export function defaultLookId(character: Pick<RegistryCharacter, 'looks'> | null | undefined): string {
	const looks = character?.looks ?? [];
	return looks.find((l) => l?.default)?.id ?? looks[0]?.id ?? DEFAULT_LOOK;
}

export function lookLabel(character: Pick<RegistryCharacter, 'looks'> | null | undefined, look: string, locale: string): string {
	const found = character?.looks?.find((l) => l?.id === look);
	return (found && pickText(found.label, locale, ['en', 'de'])) || look;
}

/** The moods of one look's grid: the shared 17, then every other mood that look has images for (extras first). */
export function characterMoods(character: Pick<RegistryCharacter, 'images'> | null | undefined, look: string = DEFAULT_LOOK): string[] {
	const own = Object.keys(character?.images?.[look] ?? {}).filter((m) => !isMood(m));
	const extras = [
		...(EXTRA_MOODS as readonly string[]).filter((m) => own.includes(m)),
		...own.filter((m) => !(EXTRA_MOODS as readonly string[]).includes(m)).sort()
	];
	return [...MOODS, ...extras];
}

// ── Lookup / resolving ────────────────────────────────────────────────────────────────────────────────────────

export function characterIndex(registry: CharacterRegistry | null | undefined): Map<string, RegistryCharacter> {
	const index = new Map<string, RegistryCharacter>();
	for (const c of registry?.characters ?? []) if (c?.id) index.set(c.id, c);
	return index;
}

const hasText = (text: LocaleText | undefined) => !!text && Object.values(text).some((t) => typeof t === 'string' && t.trim());

/** The base of a persona (null for base characters and unknown bases). */
export function personaBase(character: RegistryCharacter | null | undefined, index: Map<string, RegistryCharacter>): RegistryCharacter | null {
	if (!character?.persona_of || character.persona_of === character.id) return null;
	return index.get(character.persona_of) ?? null;
}

/** The personas of a base character, in registry order. */
export function personasOf(id: string, registry: CharacterRegistry | null | undefined): RegistryCharacter[] {
	return (registry?.characters ?? []).filter((c) => c?.persona_of === id && c.id !== id);
}

/**
 * A character with what a persona inherits from its base filled in: name, color, voiced_by, own_toon (the persona's
 * own values win). Base characters are returned as they are.
 */
export function effectiveCharacter(character: RegistryCharacter, index: Map<string, RegistryCharacter>): RegistryCharacter {
	const base = personaBase(character, index);
	if (!base) return character;
	const voicedBy = character.voiced_by ?? base.voiced_by;
	const ownToon = character.own_toon ?? base.own_toon;
	return {
		...character,
		name: hasText(character.name) ? character.name : base.name,
		color: character.color?.trim() ? character.color : base.color,
		...(voicedBy ? { voiced_by: voicedBy } : {}),
		...(ownToon ? { own_toon: ownToon } : {})
	};
}

/** The shown name in a locale: "Katzenpapst", a persona "Katzenpapst · Biker". */
export function characterName(
	character: RegistryCharacter | null | undefined,
	index: Map<string, RegistryCharacter>,
	locale: string,
	fallbacks: string[] = ['en', 'de']
): string {
	if (!character) return '';
	const effective = effectiveCharacter(character, index);
	const name = pickText(effective.name, locale, fallbacks) || character.id;
	const label = character.persona_of ? pickText(character.persona?.label, locale, fallbacks) : '';
	return label ? `${name} · ${label}` : name;
}

/** The explicit path of one look/mood/variant (no fallback), '' when not set. */
export function explicitImage(character: Pick<RegistryCharacter, 'images'> | null | undefined, look: string, mood: string, variant: string): string {
	const value = (character?.images?.[look]?.[mood] as Record<string, unknown> | undefined)?.[variant];
	return typeof value === 'string' ? value.trim() : '';
}

/** true when a look has any image */
export function lookHasImages(character: Pick<RegistryCharacter, 'images'> | null | undefined, look: string): boolean {
	return Object.values(character?.images?.[look] ?? {}).some((slots) => Object.values(slots ?? {}).some((p) => typeof p === 'string' && p.trim()));
}

/**
 * The image shown for a look/mood/variant (`look` empty = the default look): look+mood → look neutral → default
 * look+mood → default neutral → the first look with images (mood, then neutral) → the legacy `image`; '' when none.
 */
export function characterImage(character: RegistryCharacter | null | undefined, mood: string, variant: string, look?: string): string {
	if (!character) return '';
	const fallbackLook = defaultLookId(character);
	const requested = look || fallbackLook;
	const moods = mood && mood !== DEFAULT_MOOD ? [mood, DEFAULT_MOOD] : [DEFAULT_MOOD];
	for (const l of [requested, fallbackLook]) for (const m of moods) {
		const path = explicitImage(character, l, m, variant);
		if (path) return path;
	}
	for (const l of lookIds(character)) {
		if (!lookHasImages(character, l)) continue;
		for (const m of moods) {
			const path = explicitImage(character, l, m, variant);
			if (path) return path;
		}
	}
	return (character.image ?? '').trim();
}

/**
 * The path a VN renderer shows for a legacy/resolved `Character` (characters.json shape): look+mood → look neutral →
 * default look (`images`) mood → neutral → the first look with images → `image`. '' when the character has none.
 */
export function legacyCharacterImage(character: Character | null | undefined, mood: string | undefined, variant: string, look?: string): string {
	if (!character) return '';
	const moods = mood && mood !== DEFAULT_MOOD ? [mood, DEFAULT_MOOD] : [DEFAULT_MOOD];
	const pick = (set: Record<string, CharacterImage> | undefined) => {
		for (const m of moods) {
			const path = (set?.[m] as Record<string, string> | undefined)?.[variant];
			if (typeof path === 'string' && path.trim()) return path.trim();
		}
		return '';
	};
	const requested = look && look !== character.defaultLook ? character.looks?.[look] : undefined;
	return (
		pick(requested) ||
		pick(character.images) ||
		Object.values(character.looks ?? {}).map(pick).find(Boolean) ||
		(character.image ?? '').trim()
	);
}

/** Members of a cast in display order (stable for equal `order`). */
export function sortedMembers(cast: ToonCast | null | undefined): CastMember[] {
	return (cast?.members ?? [])
		.map((m, i) => [m, i] as const)
		.filter(([m]) => !!m && typeof m.character === 'string')
		.sort((a, b) => (Number(a[0].order) || 0) - (Number(b[0].order) || 0) || a[1] - b[1])
		.map(([m]) => m);
}

/**
 * A registry path (from the CDN root) as the legacy characters.json of `toon` has it: relative to the toon folder when
 * it lies inside it (`catpope/images/x.svg` → `images/x.svg`, byte-identical to the old files), otherwise from the CDN
 * root with a leading `/` (the VN renderer reads a leading `/` as "from the CDN root").
 */
export function toonRelativePath(path: string, toon: string): string {
	if (!path) return '';
	const clean = path.replace(/^\/+/, '');
	return clean.startsWith(`${toon}/`) ? clean.slice(toon.length + 1) : `/${clean}`;
}

export interface ResolveCastOptions {
	/** locales tried when a text is missing in `locale` (e.g. the toon's source language) */
	fallbacks?: string[];
}

/** One look's images in the legacy shape (explicit paths only, toon-relative). */
function legacyImages(character: RegistryCharacter, look: string, toon: string): Record<string, CharacterImage> {
	const images: Record<string, CharacterImage> = {};
	for (const mood of Object.keys(character.images?.[look] ?? {})) {
		const resolved: CharacterImage = {};
		for (const variant of CHARACTER_IMAGE_VARIANTS) {
			const path = explicitImage(character, look, mood, variant);
			if (path) resolved[variant] = toonRelativePath(path, toon);
		}
		if (Object.keys(resolved).length) images[mood] = resolved;
	}
	return images;
}

/**
 * The legacy `characters.json` of a toon in one locale (`Character[]`, order of the cast): `name` = registry id,
 * `label` = the shown name (personas "Name · Label"), `collection` = group, `images` = the default look (or the first
 * look with images when the default has none), `looks` + `defaultLook` when the character has several looks; image
 * paths relative to the toon folder (see `toonRelativePath`). Cast members that are not in the registry are skipped.
 */
export function resolveCast(
	registry: CharacterRegistry | null | undefined,
	cast: ToonCast | null | undefined,
	toon: string,
	locale: string,
	options: ResolveCastOptions = {}
): Character[] {
	const fallbacks = options.fallbacks ?? ['en', 'de'];
	const index = characterIndex(registry);
	const out: Character[] = [];
	for (const member of sortedMembers(cast)) {
		const own = index.get(member.character);
		if (!own) continue;
		const c = effectiveCharacter(own, index);
		const ids = lookIds(c);
		const main = defaultLookId(c);
		const shown = lookHasImages(c, main) ? main : (ids.find((l) => lookHasImages(c, l)) ?? main);
		const description = pickText(c.description, locale, fallbacks);
		const character: Character = {
			name: c.id,
			label: characterName(own, index, locale, fallbacks),
			image: toonRelativePath(c.image ?? '', toon),
			color: c.color ?? '',
			images: legacyImages(c, shown, toon),
			...(description ? { description } : {}),
			...(c.group ? { collection: c.group } : {})
		};
		if (ids.length > 1) {
			character.looks = Object.fromEntries(ids.map((l) => [l, legacyImages(c, l, toon)]));
			character.defaultLook = main;
		}
		out.push(character);
	}
	return out;
}

/** Every mood/variant cell of one look with its explicit path ('' = empty) and the convention path. */
export function expressionCells(character: RegistryCharacter, look: string = defaultLookId(character)) {
	return characterMoods(character, look).flatMap((mood) =>
		CHARACTER_IMAGE_VARIANTS.map((variant) => ({
			mood,
			variant,
			path: explicitImage(character, look, mood, variant),
			convention: conventionImagePath(character.id, variant, mood, look)
		}))
	);
}

/** Sets (or clears, with '') one cell; empty moods are removed except `neutral`. */
export function setCharacterImage(character: RegistryCharacter, look: string, mood: string, variant: string, path: string) {
	character.images ??= {};
	const moods = (character.images[look] ??= {}) as Record<string, Record<string, string>>;
	const slots = (moods[mood] ??= {});
	if (path.trim()) slots[variant] = path.trim();
	else delete slots[variant];
	if (!Object.keys(slots).length && mood !== DEFAULT_MOOD) delete moods[mood];
}

/** Fills the given empty cells of a look with their convention path. Returns the number of cells filled. */
export function applyConventionPaths(character: RegistryCharacter, look: string, cells: { mood: string; variant: string }[]): number {
	let filled = 0;
	for (const { mood, variant } of cells) {
		if (explicitImage(character, look, mood, variant)) continue;
		setCharacterImage(character, look, mood, variant, conventionImagePath(character.id, variant, mood, look));
		filled++;
	}
	return filled;
}

/** Neutral variants of a look without an image path. */
export function missingNeutral(character: RegistryCharacter, look: string = defaultLookId(character)): CharacterImageVariant[] {
	return CHARACTER_IMAGE_VARIANTS.filter((v) => !explicitImage(character, look, DEFAULT_MOOD, v));
}

/** Where a character is used: toon ids whose cast lists it. */
export function castsOf(id: string, casts: Record<string, ToonCast | null | undefined>): string[] {
	return Object.entries(casts)
		.filter(([, cast]) => cast?.members?.some((m) => m?.character === id))
		.map(([toon]) => toon)
		.sort();
}

/** The members of a group: characters with an affiliation to it (registry order). */
export function groupMembers(groupId: string, registry: CharacterRegistry | null | undefined): { character: RegistryCharacter; affiliation: Affiliation }[] {
	const out: { character: RegistryCharacter; affiliation: Affiliation }[] = [];
	for (const c of registry?.characters ?? []) {
		const affiliation = c?.affiliations?.find((a) => a?.group === groupId);
		if (affiliation) out.push({ character: c, affiliation });
	}
	return out;
}

/** Characters matching a query (id, names in any language, persona label, tags), best first. */
export function searchCharacters(characters: RegistryCharacter[], query: string, limit = 50): RegistryCharacter[] {
	const q = query.trim().toLowerCase();
	const all = characters.filter((c) => !!c?.id);
	if (!q) return all.slice(0, limit);
	const score = (c: RegistryCharacter) => {
		const names = [c.id, ...Object.values(c.name ?? {}), ...Object.values(c.persona?.label ?? {}), ...(c.tags ?? [])]
			.filter((n) => typeof n === 'string')
			.map((n) => n.toLowerCase());
		if (names.some((n) => n === q)) return 0;
		if (names.some((n) => n.startsWith(q))) return 1;
		if (names.some((n) => n.split(/[\s-]+/).some((w) => w.startsWith(q)))) return 2;
		if (names.some((n) => n.includes(q))) return 3;
		return 9;
	};
	return all
		.map((c) => [c, score(c)] as const)
		.filter(([, s]) => s < 9)
		.sort((a, b) => a[1] - b[1] || a[0].id.localeCompare(b[0].id))
		.slice(0, limit)
		.map(([c]) => c);
}

// ── Validation ────────────────────────────────────────────────────────────────────────────────────────────────

const isObj = (v: unknown): v is Record<string, any> => !!v && typeof v === 'object' && !Array.isArray(v);
const isStr = (v: unknown): v is string => typeof v === 'string';
const IMAGE_PATH = /\.(jpe?g|png|webp|gif|svg)$/i;

type Add = (level: Level, path: Path, message: string) => void;

function checkText(add: Add, value: unknown, path: Path, what: string, languages: string[]) {
	if (value === undefined) return;
	if (!isObj(value)) return add('error', path, `${what} must be { <locale>: text }`);
	if (!Object.values(value).some((t) => isStr(t) && t.trim())) return;
	for (const l of languages) if (!(isStr(value[l]) && value[l].trim())) add('warning', [...path, l], `${what}: missing ${l}`);
}

function checkStoryPoint(add: Add, value: unknown, path: Path, what: string, toons?: string[]) {
	if (!isObj(value) || !isStr(value.toon)) return add('error', path, `${what} must be { toon, season?, chapter? }`);
	if (toons && !toons.includes(value.toon)) add('warning', [...path, 'toon'], `${what}: unknown toon "${value.toon}"`);
	for (const key of ['season', 'chapter'] as const) {
		if (value[key] !== undefined && !(Number.isInteger(value[key]) && value[key] > 0)) add('error', [...path, key], `${what}: ${key} must be a positive number`);
	}
}

export interface CharactersContext {
	/** languages the texts must exist in: per character (its own toon / the toons whose cast lists it) or for all */
	languages: string[] | ((character: RegistryCharacter) => string[]);
	/** known person ids (voiced_by) */
	people?: string[];
	/** known toon ids (own_toon, introduced, since) */
	toons?: string[];
	/** known group ids (affiliations) */
	groups?: string[];
	file?: string;
}

/** scutoons/_/characters.json */
export function validateCharacters(registry: unknown, ctx: CharactersContext): Problem[] {
	const file = ctx.file ?? CHARACTERS_FILE;
	const problems: Problem[] = [];
	const add: Add = (level, path, message) => problems.push({ level, file, locale: null, path, message });
	if (!isObj(registry) || !Array.isArray(registry.characters)) {
		add('error', [], 'characters file must be { characters: [...] }');
		return problems;
	}
	const seen = new Set<string>();
	const index = characterIndex(registry as CharacterRegistry);
	registry.characters.forEach((c: any, i: number) => {
		const path: Path = ['characters', i];
		if (!isObj(c)) return add('error', path, `character ${i + 1} must be an object`);
		const who = isStr(c.id) && c.id ? c.id : `character ${i + 1}`;
		if (!isStr(c.id) || !CHARACTER_ID.test(c.id)) add('error', [...path, 'id'], `${who}: id must match ${CHARACTER_ID.source}`);
		else if (seen.has(c.id)) add('error', [...path, 'id'], `${who}: id is used twice`);
		else seen.add(c.id);

		// personas: an existing base, one level only, no cycles
		const isPersona = c.persona_of !== undefined;
		if (isPersona) {
			const base = isStr(c.persona_of) ? index.get(c.persona_of) : undefined;
			if (!isStr(c.persona_of) || c.persona_of === c.id) add('error', [...path, 'persona_of'], `${who}: a character can't be a persona of itself`);
			else if (!base) add('error', [...path, 'persona_of'], `${who}: persona of an unknown character "${c.persona_of}"`);
			else if (base.persona_of !== undefined) {
				add(
					'error',
					[...path, 'persona_of'],
					base.persona_of === c.id
						? `${who}: personas form a cycle (${c.id} ↔ ${base.id})`
						: `${who}: "${base.id}" is itself a persona of "${base.persona_of}" – personas are one level deep, use "${base.persona_of}"`
				);
			}
			if (!isObj(c.persona)) add('warning', [...path, 'persona'], `${who}: persona without a label (e.g. "Biker")`);
		} else if (c.persona !== undefined) add('warning', [...path, 'persona'], `${who}: persona details without persona_of`);

		const effective = isStr(c.id) && index.get(c.id) === c ? effectiveCharacter(c as RegistryCharacter, index) : (c as RegistryCharacter);
		const languages = typeof ctx.languages === 'function' ? ctx.languages(effective) : ctx.languages;
		if (c.name === undefined && isPersona) {
			/* inherits the base's name */
		} else if (!isObj(c.name)) add('error', [...path, 'name'], `${who}: name must be { <locale>: text }`);
		else if (!Object.values(c.name).some((t) => isStr(t) && t.trim())) {
			if (!isPersona) add('error', [...path, 'name'], `${who}: name is empty`);
		} else checkText(add, c.name, [...path, 'name'], `${who}: name`, languages);
		checkText(add, c.description, [...path, 'description'], `${who}: description`, languages);
		if (isObj(c.persona)) {
			const pp: Path = [...path, 'persona'];
			if (!isObj(c.persona.label) || !Object.values(c.persona.label).some((t) => isStr(t) && t.trim())) add('warning', [...pp, 'label'], `${who}: persona label is empty`);
			else checkText(add, c.persona.label, [...pp, 'label'], `${who}: persona label`, languages);
			if (c.persona.introduced !== undefined) checkStoryPoint(add, c.persona.introduced, [...pp, 'introduced'], `${who}: introduced`, ctx.toons);
			checkText(add, c.persona.note, [...pp, 'note'], `${who}: persona note`, languages);
		}

		if (!isStr(effective.color) || !effective.color.trim()) add('warning', [...path, 'color'], `${who}: no color`);
		if (!isStr(c.group) || !c.group.trim()) add('warning', [...path, 'group'], `${who}: no cast group`);
		if (c.own_toon !== undefined && (!isStr(c.own_toon) || (ctx.toons && !ctx.toons.includes(c.own_toon)))) add('warning', [...path, 'own_toon'], `${who}: unknown own toon "${c.own_toon}"`);
		if (c.voiced_by !== undefined && (!isStr(c.voiced_by) || (ctx.people && !ctx.people.includes(c.voiced_by)))) add('error', [...path, 'voiced_by'], `${who}: voiced by an unknown person "${c.voiced_by}"`);
		if (c.mannequin !== undefined && !MANNEQUINS.some((b) => b.id === c.mannequin)) add('warning', [...path, 'mannequin'], `${who}: unknown mannequin "${c.mannequin}"`);
		if (c.tags !== undefined && !(Array.isArray(c.tags) && c.tags.every(isStr))) add('error', [...path, 'tags'], `${who}: tags must be a list of texts`);
		if (c.image !== undefined && c.image !== '' && (!isStr(c.image) || !IMAGE_PATH.test(c.image))) add('warning', [...path, 'image'], `${who}: image must be a CDN image path`);

		// looks: unique ids, exactly one default
		let declared: string[] = [DEFAULT_LOOK];
		if (c.looks !== undefined) {
			if (!Array.isArray(c.looks)) add('error', [...path, 'looks'], `${who}: looks must be a list`);
			else {
				declared = [];
				c.looks.forEach((l: any, j: number) => {
					const lp: Path = [...path, 'looks', j];
					if (!isObj(l) || !isStr(l.id) || !CHARACTER_ID.test(l.id)) return add('error', lp, `${who}: look ${j + 1} needs an id matching ${CHARACTER_ID.source}`);
					if (declared.includes(l.id)) add('error', [...lp, 'id'], `${who}: look "${l.id}" is used twice`);
					declared.push(l.id);
					if (!isObj(l.label) || !Object.values(l.label).some((t) => isStr(t) && t.trim())) add('warning', [...lp, 'label'], `${who}: look "${l.id}" has no label`);
					else checkText(add, l.label, [...lp, 'label'], `${who}: look "${l.id}" label`, languages);
				});
				const defaults = c.looks.filter((l: any) => isObj(l) && l.default === true).length;
				if (c.looks.length && defaults !== 1) add('error', [...path, 'looks'], `${who}: exactly one look must be the default (${defaults} are)`);
			}
		}

		if (!isObj(c.images)) add('error', [...path, 'images'], `${who}: images must be { <look>: { <mood>: { avatar?, portrait?, full? } } }`);
		else {
			for (const [look, moods] of Object.entries(c.images)) {
				const lp: Path = [...path, 'images', look];
				if (!declared.includes(look)) add('error', lp, `${who}: images of an undeclared look "${look}"${c.looks === undefined ? ` (without looks only "${DEFAULT_LOOK}")` : ''}`);
				if (!isObj(moods)) {
					add('error', lp, `${who}: images.${look} must be { <mood>: { avatar?, portrait?, full? } }`);
					continue;
				}
				for (const [mood, slots] of Object.entries(moods)) {
					const mp: Path = [...lp, mood];
					if (!MOOD_ID.test(mood)) add('error', mp, `${who}: mood "${mood}" must match ${MOOD_ID.source}`);
					else if (!isKnownMood(mood)) add('info', mp, `${who}: extra mood "${mood}" (not in the shared vocabulary – no label)`);
					if (!isObj(slots)) {
						add('error', mp, `${who}: ${look}/${mood} must be { avatar?, portrait?, full? }`);
						continue;
					}
					for (const [variant, value] of Object.entries(slots)) {
						if (!(CHARACTER_IMAGE_VARIANTS as readonly string[]).includes(variant)) add('warning', [...mp, variant], `${who}: ${look}/${mood}: unknown variant "${variant}"`);
						else if (!isStr(value) || (value && !IMAGE_PATH.test(value))) add('warning', [...mp, variant], `${who}: ${look}/${mood} ${variant} must be a CDN image path`);
					}
				}
			}
			// neutral images per look: none at all = warning; some variants missing = info (the renderer falls back)
			for (const look of declared) {
				const missing = missingNeutral(c as RegistryCharacter, look);
				const np: Path = [...path, 'images', look, DEFAULT_MOOD];
				const where = declared.length > 1 || look !== DEFAULT_LOOK ? ` in look "${look}"` : '';
				if (missing.length === CHARACTER_IMAGE_VARIANTS.length) {
					const fallback = isStr(c.image) && c.image ? ` (falls back to ${c.image})` : '';
					add('warning', np, `${who}: no neutral image${where}${fallback}`);
				} else if (missing.length) add('info', np, `${who}: no neutral ${missing.join(', ')} image${where}`);
			}
		}

		if (c.affiliations !== undefined) {
			if (!Array.isArray(c.affiliations)) add('error', [...path, 'affiliations'], `${who}: affiliations must be a list`);
			else {
				const groups = new Set<string>();
				c.affiliations.forEach((a: any, j: number) => {
					const ap: Path = [...path, 'affiliations', j];
					if (!isObj(a) || !isStr(a.group)) return add('error', ap, `${who}: affiliation ${j + 1} must be { group, role?, since? }`);
					if (ctx.groups && !ctx.groups.includes(a.group)) add('error', [...ap, 'group'], `${who}: unknown group "${a.group}"`);
					if (groups.has(a.group)) add('warning', [...ap, 'group'], `${who}: in group "${a.group}" twice`);
					groups.add(a.group);
					checkText(add, a.role, [...ap, 'role'], `${who}: ${a.group} role`, languages);
					if (a.since !== undefined) checkStoryPoint(add, a.since, [...ap, 'since'], `${who}: ${a.group} since`, ctx.toons);
				});
			}
		}
		if (c.artwork !== undefined) {
			if (!Array.isArray(c.artwork)) add('error', [...path, 'artwork'], `${who}: artwork must be a list`);
			else {
				const ids = new Set<string>();
				c.artwork.forEach((a: any, j: number) => {
					const ap: Path = [...path, 'artwork', j];
					if (!isObj(a)) return add('error', ap, `${who}: artwork ${j + 1} must be { id, path, label? }`);
					if (!isStr(a.id) || !CHARACTER_ID.test(a.id)) add('error', [...ap, 'id'], `${who}: artwork ${j + 1}: id must match ${CHARACTER_ID.source}`);
					else if (ids.has(a.id)) add('error', [...ap, 'id'], `${who}: artwork id "${a.id}" is used twice`);
					else ids.add(a.id);
					if (!isStr(a.path) || !IMAGE_PATH.test(a.path)) add('warning', [...ap, 'path'], `${who}: artwork ${a.id ?? j + 1} must be a CDN image path`);
					checkText(add, a.label, [...ap, 'label'], `${who}: artwork ${a.id ?? j + 1} label`, languages);
				});
			}
		}
	});
	return problems;
}

/** scutoons/_/groups.json (`languages`: every language any toon uses, missing names are warnings) */
export function validateGroups(vocabulary: unknown, languages: string[] = [], file = GROUPS_FILE): Problem[] {
	const problems: Problem[] = [];
	const add: Add = (level, path, message) => problems.push({ level, file, locale: null, path, message });
	if (!isObj(vocabulary) || !Array.isArray(vocabulary.groups)) {
		add('error', [], 'groups file must be { groups: [...] }');
		return problems;
	}
	const seen = new Set<string>();
	vocabulary.groups.forEach((g: any, i: number) => {
		const path: Path = ['groups', i];
		if (!isObj(g)) return add('error', path, `group ${i + 1} must be an object`);
		const who = isStr(g.id) && g.id ? g.id : `group ${i + 1}`;
		if (!isStr(g.id) || !CHARACTER_ID.test(g.id)) add('error', [...path, 'id'], `${who}: id must match ${CHARACTER_ID.source}`);
		else if (seen.has(g.id)) add('error', [...path, 'id'], `${who}: id is used twice`);
		else seen.add(g.id);
		if (!isObj(g.name) || !Object.values(g.name).some((t) => isStr(t) && t.trim())) add('error', [...path, 'name'], `${who}: name is empty`);
		else checkText(add, g.name, [...path, 'name'], `${who}: name`, languages);
		checkText(add, g.description, [...path, 'description'], `${who}: description`, languages);
		if (g.color !== undefined && !isStr(g.color)) add('error', [...path, 'color'], `${who}: color must be a text`);
	});
	return problems;
}

export interface CastContext {
	registry?: CharacterRegistry | null;
	languages: string[];
	file?: string;
}

/** <toon>/_/cast.json against the registry */
export function validateCast(cast: unknown, ctx: CastContext): Problem[] {
	const file = ctx.file ?? CAST_FILE;
	const problems: Problem[] = [];
	const add: Add = (level, path, message) => problems.push({ level, file, locale: null, path, message });
	if (!isObj(cast) || !Array.isArray(cast.members)) {
		add('error', [], 'cast file must be { members: [...] }');
		return problems;
	}
	const index = ctx.registry ? characterIndex(ctx.registry) : null;
	const seen = new Set<string>();
	cast.members.forEach((m: any, i: number) => {
		const path: Path = ['members', i];
		if (!isObj(m)) return add('error', path, `member ${i + 1} must be { character, role?, order }`);
		const who = isStr(m.character) ? m.character : `member ${i + 1}`;
		if (!isStr(m.character) || !m.character) add('error', [...path, 'character'], `member ${i + 1}: character is missing`);
		else if (index && !index.has(m.character)) add('error', [...path, 'character'], `cast member "${m.character}" is not in the character registry`);
		if (isStr(m.character)) {
			if (seen.has(m.character)) add('warning', [...path, 'character'], `${who} is in the cast twice`);
			seen.add(m.character);
		}
		if (typeof m.order !== 'number' || !Number.isFinite(m.order)) add('warning', [...path, 'order'], `${who}: order must be a number`);
		checkText(add, m.role, [...path, 'role'], `${who}: role`, ctx.languages);
	});
	return problems;
}
