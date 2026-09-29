// Characters: every character of every toon in one global registry, each with the same set of expressions (moods ×
// variants), plus a cast per toon. Language-neutral store files with translated texts inside:
//
//   scutoons/_/characters.json   CharacterRegistry – every character, global (a character may appear in several toons
//                                                    and may have its own toon, `own_toon`)
//   <toon>/_/cast.json           ToonCast          – which characters a toon uses, in order, with a role per toon
//
// The mood vocabulary is a constant of this package (MOODS, labels per locale in MOOD_LABELS). `resolveCast()` turns the
// registry + a cast into the legacy `characters.json` shape of one locale (`Character[]`, visual-novel.ts), so the
// viewer's VN renderer reads it unchanged; `<toon>/<locale>/characters.json` stays the static fallback.
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

export const moodLabel = (mood: string, locale: string): string =>
	MOOD_LABELS[mood as Mood] ? pickText(MOOD_LABELS[mood as Mood], locale, ['en']) : mood;

export const isMood = (value: unknown): value is Mood => typeof value === 'string' && (MOODS as readonly string[]).includes(value);

/** The three image variants of a character (VN: small avatar next to the message box, portrait, full figure). */
export const CHARACTER_IMAGE_VARIANTS = ['avatar', 'portrait', 'full'] as const;
export type CharacterImageVariant = (typeof CHARACTER_IMAGE_VARIANTS)[number];

/** Suggested groups (free text is allowed): lead → cast → extra; `support` is the old catpope `collection`. */
export const CHARACTER_GROUPS = ['lead', 'support', 'cast', 'extra'] as const;
export type CharacterGroup = (typeof CHARACTER_GROUPS)[number] | (string & {});

/** Avatar bodies of the reader's avatar (viewer `components/avatar/avatar.svelte`): id → CDN image. */
export const MANNEQUINS: { id: string; image: string }[] = [
	{ id: 'sphynx', image: 'catpope/avatar/mannequin.webp' },
	{ id: 'sphynx_nude', image: 'catpope/avatar/mannequin-nude.webp' },
	{ id: 'mouse', image: 'yellowmouse/avatar/mannequin.webp' }
];

// ── Files ─────────────────────────────────────────────────────────────────────────────────────────────────────

/** One character of the registry. Image values are CDN paths from the CDN root (`catpope/images/characters/…`). */
export interface RegistryCharacter {
	/** stable id, referenced by casts and by VN events (`joinScene` `name`) – `^[a-z0-9][a-z0-9-]*$` */
	id: string;
	/** display name per locale (the VN speaker plate) */
	name: LocaleText;
	description?: LocaleText;
	/** CSS color (VN avatar background, speaker accents) */
	color: string;
	/** `lead` | `support` | `cast` | `extra` | … */
	group: CharacterGroup;
	/** the toon this character is the protagonist of (e.g. the mouse → `yellowmouse`) */
	own_toon?: string;
	/** person id of the voice (scutoons/_/people.json) */
	voiced_by?: string;
	/** avatar body id (MANNEQUINS) */
	mannequin?: string;
	tags?: string[];
	/** legacy single image (last fallback of every mood/variant; placeholder e.g. `<toon>/avatar.webp`) */
	image?: string;
	/** mood → variant → CDN path */
	images: Partial<Record<string, CharacterImage>>;
}

/** `scutoons/_/characters.json` */
export interface CharacterRegistry {
	characters: RegistryCharacter[];
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
/** Per-toon cast file, locale `_`. */
export const CAST_FILE = 'cast.json';
/** The old per-locale file (`<toon>/<locale>/characters.json`, static fallback). */
export const LEGACY_CHARACTERS_FILE = 'characters.json';

export const CHARACTER_ID = /^[a-z0-9][a-z0-9-]*$/;

/** Folder (from the CDN root) of new character images. */
export const CHARACTER_IMAGE_ROOT = 'characters';

/**
 * Where a new image of a character lives by convention (the owner's image supplier uses this naming):
 * `characters/<id>/<id>_<variant>-<mood>.png` (from the CDN root). Existing explicit paths keep working.
 */
export function conventionImagePath(id: string, variant: string, mood: string): string {
	return `${CHARACTER_IMAGE_ROOT}/${id}/${id}_${variant}-${mood}.png`;
}

export const emptyCharacterRegistry = (): CharacterRegistry => ({ characters: [] });
export const emptyCast = (): ToonCast => ({ members: [] });

export function newCharacter(id: string, name: LocaleText, extra: Partial<RegistryCharacter> = {}): RegistryCharacter {
	return { id, name, color: 'black', group: 'cast', images: { [DEFAULT_MOOD]: {} }, ...extra };
}

// ── Lookup / resolving ────────────────────────────────────────────────────────────────────────────────────────

export function characterIndex(registry: CharacterRegistry | null | undefined): Map<string, RegistryCharacter> {
	const index = new Map<string, RegistryCharacter>();
	for (const c of registry?.characters ?? []) if (c?.id) index.set(c.id, c);
	return index;
}

/** The explicit path of one mood/variant (no fallback), '' when not set. */
export function explicitImage(character: RegistryCharacter | null | undefined, mood: string, variant: string): string {
	const value = (character?.images?.[mood] as Record<string, unknown> | undefined)?.[variant];
	return typeof value === 'string' ? value.trim() : '';
}

/**
 * The image shown for a mood/variant: the requested mood/variant → the neutral mood of the same variant → the legacy
 * `image`. '' when the character has none of them.
 */
export function characterImage(character: RegistryCharacter | null | undefined, mood: string, variant: string): string {
	return explicitImage(character, mood, variant) || explicitImage(character, DEFAULT_MOOD, variant) || (character?.image ?? '').trim();
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

/**
 * The legacy `characters.json` of a toon in one locale (`Character[]`, order of the cast): `name` = registry id,
 * `label` = name, `collection` = group, image paths relative to the toon folder (see `toonRelativePath`).
 * Cast members that are not in the registry are skipped.
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
		const c = index.get(member.character);
		if (!c) continue;
		const images: Record<string, CharacterImage> = {};
		for (const [mood, slots] of Object.entries(c.images ?? {})) {
			if (!slots) continue;
			const resolved: CharacterImage = {};
			for (const variant of CHARACTER_IMAGE_VARIANTS) {
				const path = explicitImage(c, mood, variant);
				if (path) resolved[variant] = toonRelativePath(path, toon);
			}
			if (Object.keys(resolved).length) images[mood] = resolved;
		}
		const description = pickText(c.description, locale, fallbacks);
		const character: Character = {
			name: c.id,
			label: pickText(c.name, locale, fallbacks) || c.id,
			image: toonRelativePath(c.image ?? '', toon),
			color: c.color,
			images,
			...(description ? { description } : {}),
			...(c.group ? { collection: c.group } : {})
		};
		out.push(character);
	}
	return out;
}

/** Every mood/variant cell of a character with its explicit path ('' = empty) and the convention path. */
export function expressionCells(character: RegistryCharacter) {
	return MOODS.flatMap((mood) =>
		CHARACTER_IMAGE_VARIANTS.map((variant) => ({
			mood,
			variant,
			path: explicitImage(character, mood, variant),
			convention: conventionImagePath(character.id, variant, mood)
		}))
	);
}

/** Fills every empty cell with its convention path (existing paths stay). Returns the number of cells filled. */
export function applyConventionPaths(character: RegistryCharacter, moods: readonly string[] = MOODS): number {
	let filled = 0;
	character.images ??= {};
	for (const mood of moods) {
		for (const variant of CHARACTER_IMAGE_VARIANTS) {
			if (explicitImage(character, mood, variant)) continue;
			const slots = (character.images[mood] ??= {}) as Record<string, string>;
			slots[variant] = conventionImagePath(character.id, variant, mood);
			filled++;
		}
	}
	return filled;
}

/** Sets (or clears, with '') one cell; empty moods are removed except `neutral`. */
export function setCharacterImage(character: RegistryCharacter, mood: string, variant: string, path: string) {
	character.images ??= {};
	const slots = (character.images[mood] ??= {}) as Record<string, string>;
	if (path.trim()) slots[variant] = path.trim();
	else delete slots[variant];
	if (!Object.keys(slots).length && mood !== DEFAULT_MOOD) delete character.images[mood];
}

/** Neutral variants without any image (no neutral path and no legacy image to fall back to). */
export function missingNeutral(character: RegistryCharacter): CharacterImageVariant[] {
	return CHARACTER_IMAGE_VARIANTS.filter((v) => !explicitImage(character, DEFAULT_MOOD, v));
}

/** Where a character is used: toon ids whose cast lists it. */
export function castsOf(id: string, casts: Record<string, ToonCast | null | undefined>): string[] {
	return Object.entries(casts)
		.filter(([, cast]) => cast?.members?.some((m) => m?.character === id))
		.map(([toon]) => toon)
		.sort();
}

/** Characters matching a query (id, names in any language, tags), best first. */
export function searchCharacters(characters: RegistryCharacter[], query: string, limit = 50): RegistryCharacter[] {
	const q = query.trim().toLowerCase();
	const all = characters.filter((c) => !!c?.id);
	if (!q) return all.slice(0, limit);
	const score = (c: RegistryCharacter) => {
		const names = [c.id, ...Object.values(c.name ?? {}), ...(c.tags ?? [])].filter((n) => typeof n === 'string').map((n) => n.toLowerCase());
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

export interface CharactersContext {
	/** languages the names must exist in: per character (its own toon / the toons whose cast lists it) or for all */
	languages: string[] | ((character: RegistryCharacter) => string[]);
	/** known person ids (voiced_by) */
	people?: string[];
	/** known toon ids (own_toon) */
	toons?: string[];
	file?: string;
}

/** scutoons/_/characters.json */
export function validateCharacters(registry: unknown, ctx: CharactersContext): Problem[] {
	const file = ctx.file ?? CHARACTERS_FILE;
	const problems: Problem[] = [];
	const add = (level: Level, path: Path, message: string) => problems.push({ level, file, locale: null, path, message });
	if (!isObj(registry) || !Array.isArray(registry.characters)) {
		add('error', [], 'characters file must be { characters: [...] }');
		return problems;
	}
	const seen = new Set<string>();
	registry.characters.forEach((c: any, i: number) => {
		const path: Path = ['characters', i];
		if (!isObj(c)) return add('error', path, `character ${i + 1} must be an object`);
		const who = isStr(c.id) && c.id ? c.id : `character ${i + 1}`;
		if (!isStr(c.id) || !CHARACTER_ID.test(c.id)) add('error', [...path, 'id'], `${who}: id must match ${CHARACTER_ID.source}`);
		else if (seen.has(c.id)) add('error', [...path, 'id'], `${who}: id is used twice`);
		else seen.add(c.id);

		const languages = typeof ctx.languages === 'function' ? ctx.languages(c as RegistryCharacter) : ctx.languages;
		if (!isObj(c.name)) add('error', [...path, 'name'], `${who}: name must be { <locale>: text }`);
		else {
			if (!Object.values(c.name).some((t) => isStr(t) && t.trim())) add('error', [...path, 'name'], `${who}: name is empty`);
			for (const l of languages) if (!(isStr(c.name[l]) && c.name[l].trim())) add('warning', [...path, 'name', l], `${who}: missing ${l} name`);
		}
		if (c.description !== undefined) {
			if (!isObj(c.description)) add('error', [...path, 'description'], `${who}: description must be { <locale>: text }`);
			else if (Object.values(c.description).some((t) => isStr(t) && t.trim())) {
				for (const l of languages) if (!(isStr(c.description[l]) && c.description[l].trim())) add('warning', [...path, 'description', l], `${who}: missing ${l} description`);
			}
		}
		if (!isStr(c.color) || !c.color.trim()) add('warning', [...path, 'color'], `${who}: no color`);
		if (!isStr(c.group) || !c.group.trim()) add('warning', [...path, 'group'], `${who}: no group`);
		if (c.own_toon !== undefined && (!isStr(c.own_toon) || (ctx.toons && !ctx.toons.includes(c.own_toon)))) add('warning', [...path, 'own_toon'], `${who}: unknown own toon "${c.own_toon}"`);
		if (c.voiced_by !== undefined && (!isStr(c.voiced_by) || (ctx.people && !ctx.people.includes(c.voiced_by)))) add('error', [...path, 'voiced_by'], `${who}: voiced by an unknown person "${c.voiced_by}"`);
		if (c.mannequin !== undefined && !MANNEQUINS.some((b) => b.id === c.mannequin)) add('warning', [...path, 'mannequin'], `${who}: unknown mannequin "${c.mannequin}"`);
		if (c.tags !== undefined && !(Array.isArray(c.tags) && c.tags.every(isStr))) add('error', [...path, 'tags'], `${who}: tags must be a list of texts`);
		if (c.image !== undefined && c.image !== '' && (!isStr(c.image) || !IMAGE_PATH.test(c.image))) add('warning', [...path, 'image'], `${who}: image must be a CDN image path`);

		if (!isObj(c.images)) add('error', [...path, 'images'], `${who}: images must be { <mood>: { avatar?, portrait?, full? } }`);
		else {
			for (const [mood, slots] of Object.entries(c.images)) {
				const mp: Path = [...path, 'images', mood];
				if (!isMood(mood)) add('warning', mp, `${who}: "${mood}" is not one of the ${MOODS.length} moods`);
				if (!isObj(slots)) {
					add('error', mp, `${who}: ${mood} must be { avatar?, portrait?, full? }`);
					continue;
				}
				for (const [variant, value] of Object.entries(slots)) {
					if (!(CHARACTER_IMAGE_VARIANTS as readonly string[]).includes(variant)) add('warning', [...mp, variant], `${who}: ${mood}: unknown variant "${variant}"`);
					else if (!isStr(value) || (value && !IMAGE_PATH.test(value))) add('warning', [...mp, variant], `${who}: ${mood} ${variant} must be a CDN image path`);
				}
			}
			const missing = missingNeutral(c as RegistryCharacter);
			if (missing.length) {
				const fallback = isStr(c.image) && c.image ? ` (falls back to ${c.image})` : '';
				add(fallback ? 'info' : 'warning', [...path, 'images', DEFAULT_MOOD], `${who}: no neutral ${missing.join(', ')} image${fallback}`);
			}
		}
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
	const add = (level: Level, path: Path, message: string) => problems.push({ level, file, locale: null, path, message });
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
		if (m.role !== undefined) {
			if (!isObj(m.role)) add('error', [...path, 'role'], `${who}: role must be { <locale>: text }`);
			else if (Object.values(m.role).some((t) => isStr(t) && t.trim())) {
				for (const l of ctx.languages) if (!(isStr(m.role[l]) && m.role[l].trim())) add('warning', [...path, 'role', l], `${who}: missing ${l} role`);
			}
		}
	});
	return problems;
}
