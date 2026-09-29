// Characters: every character of the SCUTOONS universe in one global registry, and a cast per toon and season.
// Language-neutral store files with translated texts inside:
//
//   scutoons/_/characters.json   CharacterRegistry – every character, global (a character may appear in several toons;
//                                                    `own_toon` is optional – many characters have none)
//   scutoons/_/groups.json       GroupVocabulary   – shared affiliations across toons (e.g. the biker gang)
//   scutoons/_/moods.json        MoodVocabulary    – every mood id with its label per locale (open-ended; the 17
//                                                    `standard` ones are a checklist, not a limit)
//   <toon>/_/cast.json           ToonCast          – per season: which characters, with which personas
//   scutoons/_/asset-requests.json AssetRequests   – wanted images (a note for the image supplier), open → done
//
// Hierarchy (owner, 2026-09-29): Character → Persona → Look → Mood × View.
//   Character – the being. Identity-level: name, description, color, voiced_by, affiliations (default), tags, artwork.
//   Persona   – a different identity or role of the same being (Cat Pope's `biker`). Nested in the character; every
//               character has the persona `regular`. May override the affiliations. Changes who they are in the story.
//   Look      – the appearance right now: outfit, gear, accessories; can change within a scene (helmet on/off), one
//               default look per persona. Rule of thumb: if it could come off in the next panel, it's a look.
//               `transitions` (per persona) = an image shown while a look changes (helmet → default).
//   Mood      – the expression. Open-ended vocabulary (moods.json); only `neutral` is required (the fallback anchor).
//   View      – the framing: avatar / portrait / full. The data field is called `variant` (backwards compatibility);
//               every UI label says "View" / "Ansicht". Only `portrait` is authored now: `avatar` is a derived 192px
//               downscale of the portrait, `full` is unused. Completeness counts portraits only.
//   A-pose    – `character.apose.looks` (character level, not per persona): the reader's avatar body in its own looks
//               (`swim` default, `naked`), each one image + wear anchors per avatar slot. Never a fallback; outfitting
//               happens only in the reader's inventory.
//
// Image fallback: `resolveExpression` (one implementation for the viewer VN and the creator). `resolveCast()` turns the
// registry + a cast into the legacy `characters.json` shape of one locale (`Character[]`, visual-novel.ts: `images` =
// regular persona, default look – what older readers show – plus `personas` with every persona's looks), so the viewer
// VN renderer keeps working; `<toon>/<locale>/characters.json` stays the static fallback.
import type { LocaleText } from './credits.ts';
import { pickText } from './credits.ts';
import type { Character, CharacterImage, ResolvedPersona } from './visual-novel.ts';
import type { Path } from './paths.ts';
import type { Level, Problem } from './validate.ts';
import { AVATAR_SLOTS, type AvatarSlot } from './items.ts';

// ── Vocabulary ────────────────────────────────────────────────────────────────────────────────────────────────

/** The standard moods (a checklist, not a limit – the vocabulary in moods.json is open-ended). */
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

/** Moods of the seed that are not standard (the vocabulary grows in the creator). */
export const EXTRA_MOODS = ['dizzy', 'tripping'] as const;
export type ExtraMood = (typeof EXTRA_MOODS)[number];

export const EXTRA_MOOD_LABELS: Record<ExtraMood, LocaleText> = {
	dizzy: { de: 'benommen', en: 'dizzy' },
	tripping: { de: 'auf Trip', en: 'tripping' }
};

/** One mood of the vocabulary. */
export interface MoodDef {
	/** `^[a-z][a-z0-9-]*$` – the key in `look.images` */
	id: string;
	label: LocaleText;
	/** part of the standard set (the creator's checklist; never required except `neutral`) */
	standard?: boolean;
	/** display order (ascending; the file order when equal) */
	order?: number;
}

/** `scutoons/_/moods.json` */
export interface MoodVocabulary {
	moods: MoodDef[];
}

/** The seed vocabulary (and the fallback while the store has none): the 17 standard moods, then dizzy, tripping. */
export function defaultMoodVocabulary(): MoodVocabulary {
	return {
		moods: [
			...MOODS.map((id, i) => ({ id, label: { ...MOOD_LABELS[id] }, standard: true, order: i + 1 })),
			...EXTRA_MOODS.map((id, i) => ({ id, label: { ...EXTRA_MOOD_LABELS[id] }, order: MOODS.length + i + 1 }))
		]
	};
}

/** The vocabulary's moods in display order. */
export function sortedMoods(vocabulary: MoodVocabulary | null | undefined): MoodDef[] {
	return (vocabulary?.moods ?? [])
		.map((m, i) => [m, i] as const)
		.filter(([m]) => !!m?.id)
		.sort((a, b) => (a[0].order ?? Infinity) - (b[0].order ?? Infinity) || a[1] - b[1])
		.map(([m]) => m);
}

/** A mood's label in a locale (the vocabulary, else the built-in labels, else the id). */
export function moodLabel(mood: string, locale: string, vocabulary?: MoodVocabulary | null): string {
	const labels = vocabulary?.moods?.find((m) => m?.id === mood)?.label ?? MOOD_LABELS[mood as Mood] ?? EXTRA_MOOD_LABELS[mood as ExtraMood];
	return (labels && pickText(labels, locale, ['en', 'de'])) || mood;
}

/** One of the standard moods (built-in list). */
export const isMood = (value: unknown): value is Mood => typeof value === 'string' && (MOODS as readonly string[]).includes(value);
/** In the vocabulary (default: the seed vocabulary). */
export const isKnownMood = (value: unknown, vocabulary: MoodVocabulary | null = null): boolean =>
	typeof value === 'string' && (vocabulary ?? defaultMoodVocabulary()).moods.some((m) => m?.id === value);
export const MOOD_ID = /^[a-z][a-z0-9-]*$/;

/**
 * The three views (framings) of a character. The data field is called `variant` for backwards compatibility – in the UI
 * it is always "View" / "Ansicht", never "variant" (which would be confused with looks).
 */
export const CHARACTER_IMAGE_VARIANTS = ['avatar', 'portrait', 'full'] as const;
export type CharacterImageVariant = (typeof CHARACTER_IMAGE_VARIANTS)[number];
/** The only view that is authored (the others fall back to it; `avatar` is a derived 192px downscale of it). */
export const AUTHORED_VIEW: CharacterImageVariant = 'portrait';
/** Derived from the portrait (generated, never flagged missing). */
export const DERIVED_VIEWS: CharacterImageVariant[] = ['avatar'];
/** Not used for now (hidden in the creator, never flagged missing). */
export const UNUSED_VIEWS: CharacterImageVariant[] = ['full'];
/** The A-pose view: the character's avatar body (`character.apose.looks`), one image per A-pose look. */
export const APOSE_VIEW = 'apose';

/** Every character's base persona. */
export const REGULAR_PERSONA = 'regular';
/** The look of a persona without other looks (and the look whose convention path has no look segment). */
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

/** mood → view → CDN path */
export type MoodImages = Partial<Record<string, CharacterImage>>;

/** Where an avatar item sits on an A-pose image: centre and size in px of that image. */
export interface AposeAnchor {
	x: number;
	y: number;
	size: number;
}

/** One A-pose look of a character (full body, neutral): an avatar body with the wear anchors of the avatar slots. */
export interface AposeLook {
	/** `swim` (default) | `naked` | … */
	id: string;
	label?: LocaleText;
	/** exactly one A-pose look is the default */
	default?: boolean;
	/** CDN path from the CDN root (e.g. `catpope/avatar/mannequin.webp`, 768 × 1152); missing = still to be delivered */
	path?: string;
	/** avatar slot (AVATAR_SLOTS: head, face, neck, torso, hands, legs, feet, back, accessory) → anchor in image px */
	anchors?: Partial<Record<AvatarSlot, AposeAnchor>>;
}

/** The A-pose of a character (the reader's avatar body) – separate from the persona / story looks. */
export interface CharacterApose {
	looks: AposeLook[];
}

/** The standard A-pose looks. */
export const APOSE_LOOKS: { id: string; label: LocaleText }[] = [
	{ id: 'swim', label: { de: 'Badehose', en: 'swim pants' } },
	{ id: 'naked', label: { de: 'nackt', en: 'naked' } }
];

/** `characters/<character>/<character>_apose-<look>.png` (convention for a new A-pose image) */
export function aposeConventionPath(character: string, look: string): string {
	return `${CHARACTER_IMAGE_ROOT}/${character}/${character}_apose-${look}.png`;
}

/** An appearance of a persona (with / without helmet): its own mood set. */
export interface CharacterLook {
	/** `^[a-z0-9][a-z0-9-]*$`; `default` = no look segment in the convention path */
	id: string;
	label?: LocaleText;
	/** exactly one look of a persona is the default (VN refs without `look`) */
	default?: boolean;
	/** mood → view → CDN path (from the CDN root) */
	images: MoodImages;
}

/** An image shown while a persona's look changes (the mouse taking her helmet off). */
export interface LookTransition {
	from: string;
	to: string;
	/** CDN path from the CDN root */
	image: string;
	label?: LocaleText;
}

/** Membership of a character (or of one of its personas) in a shared group. */
export interface Affiliation {
	/** group id (GroupVocabulary) */
	group: string;
	/** e.g. "Anführerin" / "Leader" */
	role?: LocaleText;
	since?: StoryPoint;
}

/** A different identity or role of the same being. `regular` = the base persona every character has. */
export interface CharacterPersona {
	/** `regular` or `^[a-z0-9][a-z0-9-]*$` (e.g. `biker`, `human`) */
	id: string;
	/** appended to the name: "Cat Pope · Biker" (the regular persona needs none) */
	label?: LocaleText;
	/** where the persona first appears (none = it has always been there) */
	introduced?: StoryPoint;
	description?: LocaleText;
	/** replaces the character's affiliations for this persona (e.g. only the biker is in the gang) */
	affiliations?: Affiliation[];
	looks: CharacterLook[];
	transitions?: LookTransition[];
}

/** Character artwork that is not an expression (key visuals, turnarounds …). */
export interface CharacterArtwork {
	/** `^[a-z0-9][a-z0-9-]*$`, unique per character */
	id: string;
	/** CDN path from the CDN root */
	path: string;
	label?: LocaleText;
}

/** One character of the registry. Image values are CDN paths from the CDN root (`catpope/images/characters/…`). */
export interface RegistryCharacter {
	/** stable id, referenced by casts and by VN events (`joinScene` `name`) – `^[a-z0-9][a-z0-9-]*$` */
	id: string;
	/** display name per locale (the VN speaker plate) */
	name: LocaleText;
	description?: LocaleText;
	/** CSS color (VN avatar background, speaker accents) */
	color: string;
	/** cast group: `lead` | `support` | `cast` | `extra` | … */
	group: CharacterGroup;
	/** the toon this character is the protagonist of (optional – most characters have none) */
	own_toon?: string;
	/** person id of the voice (scutoons/_/people.json) */
	voiced_by?: string;
	/** avatar body id (MANNEQUINS) */
	mannequin?: string;
	tags?: string[];
	/** legacy single image (the last fallback; placeholder e.g. `<toon>/avatar.webp`) */
	image?: string;
	/** default groups of every persona (a persona's own `affiliations` replace them) */
	affiliations?: Affiliation[];
	artwork?: CharacterArtwork[];
	/** `regular` first */
	personas: CharacterPersona[];
	/** the reader's avatar body (character level – the persona doesn't matter) */
	apose?: CharacterApose;
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

/** A character in a season's cast, with the personas that appear in that season. */
export interface CastEntry {
	/** registry id */
	character: string;
	/** persona ids (`regular` …) */
	personas: string[];
	/** the character's role in this toon, per locale (e.g. "Erzähler" / "Narrator") */
	role?: LocaleText;
}

/** `<toon>/_/cast.json` – per season (key = season number), in display order */
export interface ToonCast {
	seasons: Record<string, CastEntry[]>;
}

// ── Store paths ───────────────────────────────────────────────────────────────────────────────────────────────

/** Registry file in the pseudo-toon `scutoons` (CREDITS_SCOPE), locale `_`. */
export const CHARACTERS_FILE = 'characters.json';
/** Shared affiliation groups (pseudo-toon `scutoons`, locale `_`). */
export const GROUPS_FILE = 'groups.json';
/** The mood vocabulary (pseudo-toon `scutoons`, locale `_`). */
export const MOODS_FILE = 'moods.json';
/** Wanted images (pseudo-toon `scutoons`, locale `_`). */
export const ASSET_REQUESTS_FILE = 'asset-requests.json';

/** A wanted image: one cell of a look (portrait only for now), with the convention path it should get. */
export interface AssetRequest {
	id: string;
	character: string;
	persona: string;
	look: string;
	mood: string;
	view: string;
	/** where the file should go (convention path, from the CDN root) */
	path: string;
	note?: string;
	/** email / name of who asked */
	requested_by?: string | null;
	/** ISO date-time */
	requested_at: string;
	status: 'open' | 'done';
	done_at?: string;
}

/** `scutoons/_/asset-requests.json` */
export interface AssetRequests {
	requests: AssetRequest[];
}
/** Per-toon cast file, locale `_`. */
export const CAST_FILE = 'cast.json';
/** The old per-locale file (`<toon>/<locale>/characters.json`, static fallback). */
export const LEGACY_CHARACTERS_FILE = 'characters.json';

export const CHARACTER_ID = /^[a-z0-9][a-z0-9-]*$/;

/** Folder (from the CDN root) of new character images. */
export const CHARACTER_IMAGE_ROOT = 'characters';

/** `<character>` for the regular persona, else `<character>-<persona>` (folder and file prefix of the convention). */
export function conventionName(character: string, persona: string = REGULAR_PERSONA): string {
	return persona && persona !== REGULAR_PERSONA ? `${character}-${persona}` : character;
}

/**
 * Where a new image lives by convention (the owner's image supplier uses this naming), from the CDN root:
 * `characters/<character>[-<persona>]/<character>[-<persona>][_<look>]_<view>-<mood>.png` – the regular persona and the
 * default look are left out. Stored paths are always explicit (existing files keep their paths).
 */
export function conventionImagePath(character: string, view: string, mood: string, look: string = DEFAULT_LOOK, persona: string = REGULAR_PERSONA): string {
	const name = conventionName(character, persona);
	const lookPart = look && look !== DEFAULT_LOOK ? `_${look}` : '';
	return `${CHARACTER_IMAGE_ROOT}/${name}/${name}${lookPart}_${view}-${mood}.png`;
}

/** `characters/<character>[-<persona>]/<character>[-<persona>]_transition-<from>-to-<to>.png` */
export function conventionTransitionPath(character: string, persona: string, from: string, to: string): string {
	const name = conventionName(character, persona);
	return `${CHARACTER_IMAGE_ROOT}/${name}/${name}_transition-${from}-to-${to}.png`;
}

export const emptyCharacterRegistry = (): CharacterRegistry => ({ characters: [] });
export const emptyCast = (): ToonCast => ({ seasons: {} });
export const emptyGroups = (): GroupVocabulary => ({ groups: [] });
export const emptyAssetRequests = (): AssetRequests => ({ requests: [] });

/** The cell an asset request is about: `character/persona/look/mood/view`. */
export const cellKey = (r: Pick<AssetRequest, 'character' | 'persona' | 'look' | 'mood' | 'view'>) => `${r.character}/${r.persona}/${r.look}/${r.mood}/${r.view}`;

/** A missing image of a look: the standard moods without a portrait (only `portrait` is authored). */
export interface WantedImage {
	character: string;
	persona: string;
	look: string;
	mood: string;
	view: string;
	/** the convention path (from the CDN root) */
	path: string;
	/** the open request for this cell, if any */
	request?: AssetRequest;
}

/**
 * Every wanted image of the registry (or of one character): per persona and look, the standard moods of the vocabulary
 * (`standard: true`) plus every mood with an open request that have no portrait; ordered character → persona → look →
 * vocabulary order. `exists(path)` (optional) drops cells whose file is already at the convention path.
 */
export function wantedImages(
	registry: CharacterRegistry | null | undefined,
	options: { moods?: MoodVocabulary | null; requests?: AssetRequests | null; character?: string; exists?: (path: string) => boolean } = {}
): WantedImage[] {
	const vocabulary = sortedMoods(options.moods ?? defaultMoodVocabulary());
	const standard = vocabulary.filter((m) => m.standard).map((m) => m.id);
	const order = vocabulary.map((m) => m.id);
	const open = (options.requests?.requests ?? []).filter((r) => r?.status === 'open');
	const out: WantedImage[] = [];
	for (const c of registry?.characters ?? []) {
		if (!c?.id || (options.character && c.id !== options.character)) continue;
		// A-pose looks without an image (character level; persona `regular`, mood `neutral`, view `apose`)
		for (const look of c.apose?.looks ?? []) {
			if (!look?.id || look.path?.trim()) continue;
			const path = aposeConventionPath(c.id, look.id);
			if (options.exists?.(path)) continue;
			const request = open.find((r) => r.character === c.id && r.view === APOSE_VIEW && r.look === look.id);
			out.push({ character: c.id, persona: REGULAR_PERSONA, look: look.id, mood: DEFAULT_MOOD, view: APOSE_VIEW, path, ...(request ? { request } : {}) });
		}
		for (const personaId of personaIds(c)) {
			const persona = findPersona(c, personaId)!;
			for (const look of persona.looks ?? []) {
				if (!look?.id) continue;
				const requested = open.filter((r) => r.character === c.id && r.persona === personaId && r.look === look.id);
				const moods = [...new Set([...standard, ...requested.map((r) => r.mood)])].sort((a, b) => {
					const ia = order.indexOf(a);
					const ib = order.indexOf(b);
					return (ia < 0 ? Infinity : ia) - (ib < 0 ? Infinity : ib);
				});
				for (const mood of moods) {
					if (lookImage(look, mood, AUTHORED_VIEW)) continue;
					const path = conventionImagePath(c.id, AUTHORED_VIEW, mood, look.id, personaId);
					if (options.exists?.(path)) continue;
					const request = requested.find((r) => r.mood === mood && r.view === AUTHORED_VIEW);
					out.push({ character: c.id, persona: personaId, look: look.id, mood, view: AUTHORED_VIEW, path, ...(request ? { request } : {}) });
				}

			}
		}
	}
	return out;
}

/** The file names per line, grouped by character / persona / look (a brief for the image supplier). */
export function wantedImagesText(wanted: WantedImage[]): string {
	const lines: string[] = [];
	let group = '';
	for (const w of wanted) {
		const head = `${w.character} · ${w.persona} · ${w.look}`;
		if (head !== group) {
			if (lines.length) lines.push('');
			lines.push(head);
			group = head;
		}
		lines.push(`  ${w.path.split('/').pop()}  (${w.mood}${w.request?.note ? ` – ${w.request.note}` : ''})`);
	}
	return lines.join('\n');
}

/** CSV: character,persona,look,mood,view,file,path,note */
export function wantedImagesCsv(wanted: WantedImage[]): string {
	const q = (v: string) => (/[",\n]/.test(v) ? `"${v.replace(/"/g, '""')}"` : v);
	const rows = wanted.map((w) => [w.character, w.persona, w.look, w.mood, w.view, w.path.split('/').pop() ?? '', w.path, w.request?.note ?? ''].map(q).join(','));
	return ['character,persona,look,mood,view,file,path,note', ...rows].join('\n');
}

/**
 * Marks open requests done whose cell now has an image path (or whose file exists at the convention path, when
 * `exists` says so). Returns the number of requests closed.
 */
export function closeFulfilledRequests(requests: AssetRequests, registry: CharacterRegistry | null | undefined, now: string, exists?: (path: string) => boolean): number {
	const index = characterIndex(registry);
	let closed = 0;
	for (const r of requests.requests ?? []) {
		if (r?.status !== 'open') continue;
		const c = index.get(r.character);
		const done =
			r.view === APOSE_VIEW
				? !!c?.apose?.looks?.find((l) => l?.id === r.look)?.path?.trim()
				: !!lookImage(findLook(findPersona(c, r.persona), r.look), r.mood, r.view);
		if (done || exists?.(r.path)) {
			r.status = 'done';
			r.done_at = now;
			closed++;
		}
	}
	return closed;
}

export function newLook(id: string = DEFAULT_LOOK, label?: LocaleText, isDefault = true): CharacterLook {
	return { id, ...(label ? { label } : {}), ...(isDefault ? { default: true } : {}), images: {} };
}
export function newPersona(id: string, label?: LocaleText): CharacterPersona {
	return { id, ...(label ? { label } : {}), looks: [newLook()] };
}
export function newCharacter(id: string, name: LocaleText, extra: Partial<RegistryCharacter> = {}): RegistryCharacter {
	return { id, name, color: 'black', group: 'cast', personas: [newPersona(REGULAR_PERSONA)], ...extra };
}

// ── Lookup ────────────────────────────────────────────────────────────────────────────────────────────────────

export function characterIndex(registry: CharacterRegistry | null | undefined): Map<string, RegistryCharacter> {
	const index = new Map<string, RegistryCharacter>();
	for (const c of registry?.characters ?? []) if (c?.id) index.set(c.id, c);
	return index;
}

/** The personas in order (`regular` first). */
export function personaIds(character: Pick<RegistryCharacter, 'personas'> | null | undefined): string[] {
	const ids = (character?.personas ?? []).map((p) => p?.id).filter((id): id is string => typeof id === 'string' && !!id);
	return [...ids.filter((id) => id === REGULAR_PERSONA), ...ids.filter((id) => id !== REGULAR_PERSONA)];
}

export function findPersona(character: Pick<RegistryCharacter, 'personas'> | null | undefined, persona?: string): CharacterPersona | undefined {
	const id = persona || REGULAR_PERSONA;
	return character?.personas?.find((p) => p?.id === id);
}

export function findLook(persona: CharacterPersona | null | undefined, look?: string): CharacterLook | undefined {
	return look ? persona?.looks?.find((l) => l?.id === look) : undefined;
}

/** The default look id of a persona: the look marked `default`, else the first look, else `default`. */
export function defaultLookId(persona: Pick<CharacterPersona, 'looks'> | null | undefined): string {
	const looks = persona?.looks ?? [];
	return looks.find((l) => l?.default)?.id ?? looks[0]?.id ?? DEFAULT_LOOK;
}

export function personaLabel(character: RegistryCharacter | null | undefined, persona: string | undefined, locale: string, fallbacks: string[] = ['en', 'de']): string {
	if (!persona || persona === REGULAR_PERSONA) return '';
	return pickText(findPersona(character, persona)?.label, locale, fallbacks) || persona;
}

export function lookLabel(persona: CharacterPersona | null | undefined, look: string, locale: string): string {
	return pickText(persona?.looks?.find((l) => l?.id === look)?.label, locale, ['en', 'de']) || look;
}

/** The shown name in a locale: "Katzenpapst", a persona "Katzenpapst · Biker". */
export function characterName(character: RegistryCharacter | null | undefined, locale: string, persona?: string, fallbacks: string[] = ['en', 'de']): string {
	if (!character) return '';
	const name = pickText(character.name, locale, fallbacks) || character.id;
	const label = personaLabel(character, persona, locale, fallbacks);
	return label ? `${name} · ${label}` : name;
}

/** The affiliations of a persona: its own (they replace the character's) or the character's. */
export function personaAffiliations(character: RegistryCharacter, persona?: string): Affiliation[] {
	return findPersona(character, persona)?.affiliations ?? character.affiliations ?? [];
}

/**
 * The moods of one look's grid: the moods it has (vocabulary order, unknown ones after), then – with `all` – every other
 * mood of the vocabulary (empty rows to fill). `neutral` is always first.
 */
export function lookMoods(look: Pick<CharacterLook, 'images'> | null | undefined, vocabulary?: MoodVocabulary | null, all = false): string[] {
	const order = sortedMoods(vocabulary ?? defaultMoodVocabulary()).map((m) => m.id);
	const own = Object.keys(look?.images ?? {}).filter((m) => Object.values(look?.images?.[m] ?? {}).some((p) => typeof p === 'string' && p.trim()));
	const known = order.filter((m) => own.includes(m) || m === DEFAULT_MOOD || all);
	const unknown = own.filter((m) => !order.includes(m)).sort();
	return [DEFAULT_MOOD, ...[...known, ...unknown].filter((m) => m !== DEFAULT_MOOD)];
}

/** Moods a look has an authored (portrait) image for. */
export function lookPortraitMoods(look: Pick<CharacterLook, 'images'> | null | undefined): string[] {
	return Object.keys(look?.images ?? {}).filter((m) => !!lookImage(look, m, AUTHORED_VIEW));
}

/** The explicit path of one mood/view of a look (no fallback), '' when not set. */
export function lookImage(look: Pick<CharacterLook, 'images'> | null | undefined, mood: string, view: string): string {
	const value = (look?.images?.[mood] as Record<string, unknown> | undefined)?.[view];
	return typeof value === 'string' ? value.trim() : '';
}

export function lookHasImages(look: Pick<CharacterLook, 'images'> | null | undefined): boolean {
	return Object.values(look?.images ?? {}).some((slots) => Object.values(slots ?? {}).some((p) => typeof p === 'string' && p.trim()));
}

// ── The fallback rule ─────────────────────────────────────────────────────────────────────────────────────────

/**
 * THE fallback rule of character images (owner, 2026-09-29), regressive, one step at a time:
 *   1. persona + look + mood
 *   2. persona + look + neutral                (a missing mood never changes the look)
 *   3. persona + default look + mood
 *   4. persona + default look + neutral
 *   5. regular persona + default look + mood
 *   6. regular persona + default look + neutral
 *   7. the legacy single `image`
 * Within every step the requested view comes first, then the same mood's portrait (only `portrait` is authored: an
 * avatar or full view without its own file shows the portrait before any mood/look/persona step).
 * `lookSet(persona, look)` returns a look's mood → view → path map; `defaultLook(persona)` its default look id.
 */
export function resolveExpression(
	lookSet: (persona: string, look: string) => Record<string, CharacterImage | undefined> | undefined,
	defaultLook: (persona: string) => string,
	ref: { persona?: string; look?: string; mood?: string },
	view: string,
	legacyImage?: string
): string {
	const persona = ref.persona || REGULAR_PERSONA;
	const mood = ref.mood || DEFAULT_MOOD;
	const personaDefault = defaultLook(persona);
	const look = ref.look || personaDefault;
	const regularDefault = defaultLook(REGULAR_PERSONA);
	// the A-pose never falls back (a look without one is no avatar body)
	if (view === APOSE_VIEW) return '';
	const views = view === AUTHORED_VIEW ? [view] : [view, AUTHORED_VIEW];
	const steps: [string, string, string][] = [
		[persona, look, mood],
		[persona, look, DEFAULT_MOOD],
		[persona, personaDefault, mood],
		[persona, personaDefault, DEFAULT_MOOD],
		[REGULAR_PERSONA, regularDefault, mood],
		[REGULAR_PERSONA, regularDefault, DEFAULT_MOOD]
	];
	for (const [p, l, m] of steps) {
		const set = lookSet(p, l);
		for (const v of views) {
			const path = (set?.[m] as Record<string, unknown> | undefined)?.[v];
			if (typeof path === 'string' && path.trim()) return path.trim();
		}
	}
	return (legacyImage ?? '').trim();
}

export interface ExpressionRef {
	persona?: string;
	look?: string;
	mood?: string;
}

/** The image of a registry character (`resolveExpression`). */
export function characterImage(character: RegistryCharacter | null | undefined, ref: ExpressionRef, view: string): string {
	if (!character) return '';
	if (view === APOSE_VIEW) {
		const looks = character.apose?.looks ?? [];
		const look = looks.find((l) => l?.id === ref.look) ?? looks.find((l) => l?.default) ?? looks[0];
		return look?.path?.trim() ?? '';
	}
	return resolveExpression(
		(p, l) => findLook(findPersona(character, p), l)?.images,
		(p) => defaultLookId(findPersona(character, p)),
		ref,
		view,
		character.image
	);
}

/** The image a VN renderer shows for a resolved `Character` (characters.json shape) – same rule (`resolveExpression`). */
export function legacyCharacterImage(character: Character | null | undefined, ref: ExpressionRef, view: string): string {
	if (!character) return '';
	const persona = (id: string) => character.personas?.[id];
	return resolveExpression(
		(p, l) => {
			const resolved = persona(p);
			if (resolved) return resolved.looks[l];
			// an older characters.json without personas: its `images` are the regular persona's default look
			return p === REGULAR_PERSONA && l === DEFAULT_LOOK ? character.images : undefined;
		},
		(p) => persona(p)?.defaultLook ?? DEFAULT_LOOK,
		ref,
		view,
		character.image
	);
}

/** The shown name of a resolved `Character` for a persona ("Cat Pope · Biker"). */
export function legacyCharacterLabel(character: Character | null | undefined, persona?: string): string {
	if (!character) return '';
	return (persona && character.personas?.[persona]?.label) || character.label;
}

/** The transition image of a resolved `Character` when a persona's look changes from `from` to `to` ('' = none). */
export function legacyTransition(character: Character | null | undefined, persona: string | undefined, from: string | undefined, to: string | undefined): string {
	const p = character?.personas?.[persona || REGULAR_PERSONA];
	if (!p) return '';
	const fromLook = from || p.defaultLook;
	const toLook = to || p.defaultLook;
	if (fromLook === toLook) return '';
	return p.transitions?.find((t) => t.from === fromLook && t.to === toLook)?.image ?? '';
}

/** For the VN validation (ValidateContext.expressions): character → persona → default look + look → portrait moods. */
export function expressionIndex(registry: CharacterRegistry | null | undefined): Record<string, Record<string, { defaultLook: string; looks: Record<string, string[]> }>> {
	const out: Record<string, Record<string, { defaultLook: string; looks: Record<string, string[]> }>> = {};
	for (const c of registry?.characters ?? []) {
		if (!c?.id) continue;
		out[c.id] = {};
		for (const p of c.personas ?? []) {
			if (!p?.id) continue;
			out[c.id][p.id] = {
				defaultLook: defaultLookId(p),
				looks: Object.fromEntries((p.looks ?? []).filter((l) => l?.id).map((l) => [l.id, lookPortraitMoods(l)]))
			};
		}
	}
	return out;
}

// ── Casts ─────────────────────────────────────────────────────────────────────────────────────────────────────

/** For the VN validation (ValidateContext.cast): character → cast persona ids of one season (or of every season). */
export function castIndex(cast: ToonCast | null | undefined, season?: number): Record<string, string[]> {
	return Object.fromEntries(castEntries(cast, season).map((e) => [e.character, e.personas]));
}

/** The season numbers of a cast, ascending. */
export function castSeasons(cast: ToonCast | null | undefined): number[] {
	return Object.keys(cast?.seasons ?? {})
		.map(Number)
		.filter((n) => Number.isInteger(n) && n > 0)
		.sort((a, b) => a - b);
}

/** The entries of one season, or of every season (merged: first appearance order, personas united). */
export function castEntries(cast: ToonCast | null | undefined, season?: number): CastEntry[] {
	const seasons = season === undefined ? castSeasons(cast) : [season];
	const out: CastEntry[] = [];
	for (const s of seasons) {
		for (const entry of cast?.seasons?.[String(s)] ?? []) {
			if (!entry || typeof entry.character !== 'string') continue;
			const existing = out.find((e) => e.character === entry.character);
			const personas = Array.isArray(entry.personas) && entry.personas.length ? entry.personas : [REGULAR_PERSONA];
			if (existing) existing.personas = [...new Set([...existing.personas, ...personas])];
			else out.push({ ...entry, personas: [...personas] });
		}
	}
	return out;
}

/** Toons whose cast (any season) lists the character – "appears in". */
export function castsOf(id: string, casts: Record<string, ToonCast | null | undefined>): string[] {
	return Object.entries(casts)
		.filter(([, cast]) => castEntries(cast).some((e) => e.character === id))
		.map(([toon]) => toon)
		.sort();
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
	/** only this season's cast (default: every season) */
	season?: number;
}

function legacyLook(look: CharacterLook | undefined, toon: string): Record<string, CharacterImage> {
	const images: Record<string, CharacterImage> = {};
	for (const mood of Object.keys(look?.images ?? {})) {
		const resolved: CharacterImage = {};
		for (const view of CHARACTER_IMAGE_VARIANTS) {
			const path = lookImage(look, mood, view);
			if (path) resolved[view] = toonRelativePath(path, toon);
		}
		if (Object.keys(resolved).length) images[mood] = resolved;
	}
	return images;
}

/**
 * The legacy `characters.json` of a toon in one locale (`Character[]`, cast order): `name` = registry id, `label` =
 * name, `collection` = group, `images` = the regular persona's default look (what older readers show), `personas` =
 * every persona with its label ("Name · Label"), looks, default look and transitions; image paths relative to the toon
 * folder (see `toonRelativePath`). Cast entries that are not in the registry are skipped.
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
	for (const entry of castEntries(cast, options.season)) {
		const c = index.get(entry.character);
		if (!c) continue;
		const regular = findPersona(c, REGULAR_PERSONA);
		const description = pickText(c.description, locale, fallbacks);
		const personas: Record<string, ResolvedPersona> = {};
		for (const id of personaIds(c)) {
			const p = findPersona(c, id)!;
			personas[id] = {
				label: characterName(c, locale, id, fallbacks),
				defaultLook: defaultLookId(p),
				looks: Object.fromEntries((p.looks ?? []).filter((l) => l?.id).map((l) => [l.id, legacyLook(l, toon)])),
				...(p.transitions?.length ? { transitions: p.transitions.map((t) => ({ from: t.from, to: t.to, image: toonRelativePath(t.image, toon) })) } : {})
			};
		}
		out.push({
			name: c.id,
			label: characterName(c, locale, REGULAR_PERSONA, fallbacks),
			image: toonRelativePath(c.image ?? '', toon),
			color: c.color ?? '',
			images: legacyLook(findLook(regular, defaultLookId(regular)), toon),
			...(description ? { description } : {}),
			...(c.group ? { collection: c.group } : {}),
			personas
		});
	}
	return out;
}

// ── Editing helpers ───────────────────────────────────────────────────────────────────────────────────────────

/** Sets (or clears, with '') one cell of a look; empty moods are removed except `neutral`. */
export function setLookImage(look: CharacterLook, mood: string, view: string, path: string) {
	look.images ??= {};
	const slots = (look.images[mood] ??= {}) as Record<string, string>;
	if (path.trim()) slots[view] = path.trim();
	else delete slots[view];
	if (!Object.keys(slots).length && mood !== DEFAULT_MOOD) delete look.images[mood];
}

/** An avatar body for the reader: a character's A-pose look with an image. */
export interface AvatarBody {
	character: string;
	/** A-pose look id (`swim`, `naked`, …) */
	look: string;
	/** "Cat Pope (swim pants)" in the requested locale */
	label: string;
	path: string;
	anchors: Partial<Record<AvatarSlot, AposeAnchor>>;
	/** the character's default A-pose look */
	default: boolean;
}

/** Every A-pose look with an image (registry order) – what the reader's avatar picker offers. */
export function avatarBodies(registry: CharacterRegistry | null | undefined, locale = 'en'): AvatarBody[] {
	const out: AvatarBody[] = [];
	for (const c of registry?.characters ?? []) {
		if (!c?.id) continue;
		const looks = c.apose?.looks ?? [];
		const defaultId = looks.find((l) => l?.default)?.id ?? looks[0]?.id;
		for (const look of looks) {
			if (!look?.id || !look.path?.trim()) continue;
			const lookName = pickText(look.label, locale, ['en', 'de']) || look.id;
			out.push({
				character: c.id,
				look: look.id,
				label: `${characterName(c, locale)} (${lookName})`,
				path: look.path.trim(),
				anchors: look.anchors ?? {},
				default: look.id === defaultId
			});
		}
	}
	return out;
}

/** Every character (and persona) with an affiliation to a group, registry order. */
export function groupMembers(groupId: string, registry: CharacterRegistry | null | undefined) {
	const out: { character: RegistryCharacter; persona: string; affiliation: Affiliation }[] = [];
	for (const c of registry?.characters ?? []) {
		if (!c?.id) continue;
		for (const persona of personaIds(c)) {
			const affiliation = personaAffiliations(c, persona).find((a) => a?.group === groupId);
			// the regular persona stands for the character; other personas only when they have their own affiliations
			if (affiliation && (persona === REGULAR_PERSONA || findPersona(c, persona)?.affiliations)) out.push({ character: c, persona, affiliation });
		}
	}
	return out;
}

/** Characters matching a query (id, names in any language, persona labels, tags), best first. */
export function searchCharacters(characters: RegistryCharacter[], query: string, limit = 50): RegistryCharacter[] {
	const q = query.trim().toLowerCase();
	const all = characters.filter((c) => !!c?.id);
	if (!q) return all.slice(0, limit);
	const score = (c: RegistryCharacter) => {
		const names = [c.id, ...Object.values(c.name ?? {}), ...(c.personas ?? []).flatMap((p) => [p?.id, ...Object.values(p?.label ?? {})]), ...(c.tags ?? [])]
			.filter((n): n is string => typeof n === 'string')
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

function checkAffiliations(add: Add, value: unknown, path: Path, who: string, languages: string[], ctx: CharactersContext) {
	if (value === undefined) return;
	if (!Array.isArray(value)) return add('error', path, `${who}: affiliations must be a list`);
	const groups = new Set<string>();
	value.forEach((a: any, j: number) => {
		const ap: Path = [...path, j];
		if (!isObj(a) || !isStr(a.group)) return add('error', ap, `${who}: affiliation ${j + 1} must be { group, role?, since? }`);
		if (ctx.groups && !ctx.groups.includes(a.group)) add('error', [...ap, 'group'], `${who}: unknown group "${a.group}"`);
		if (groups.has(a.group)) add('warning', [...ap, 'group'], `${who}: in group "${a.group}" twice`);
		groups.add(a.group);
		checkText(add, a.role, [...ap, 'role'], `${who}: ${a.group} role`, languages);
		if (a.since !== undefined) checkStoryPoint(add, a.since, [...ap, 'since'], `${who}: ${a.group} since`, ctx.toons);
	});
}

export interface CharactersContext {
	/** the mood vocabulary (unknown moods are info; default: the seed vocabulary) */
	moods?: MoodVocabulary | null;
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
	registry.characters.forEach((c: any, i: number) => {
		const path: Path = ['characters', i];
		if (!isObj(c)) return add('error', path, `character ${i + 1} must be an object`);
		const who = isStr(c.id) && c.id ? c.id : `character ${i + 1}`;
		if (!isStr(c.id) || !CHARACTER_ID.test(c.id)) add('error', [...path, 'id'], `${who}: id must match ${CHARACTER_ID.source}`);
		else if (seen.has(c.id)) add('error', [...path, 'id'], `${who}: id is used twice`);
		else seen.add(c.id);
		const languages = typeof ctx.languages === 'function' ? ctx.languages(c as RegistryCharacter) : ctx.languages;

		if (!isObj(c.name) || !Object.values(c.name).some((t) => isStr(t) && t.trim())) add('error', [...path, 'name'], `${who}: name is empty`);
		else checkText(add, c.name, [...path, 'name'], `${who}: name`, languages);
		checkText(add, c.description, [...path, 'description'], `${who}: description`, languages);
		if (!isStr(c.color) || !c.color.trim()) add('warning', [...path, 'color'], `${who}: no color`);
		if (!isStr(c.group) || !c.group.trim()) add('warning', [...path, 'group'], `${who}: no cast group`);
		if (c.own_toon !== undefined && (!isStr(c.own_toon) || (ctx.toons && !ctx.toons.includes(c.own_toon)))) add('warning', [...path, 'own_toon'], `${who}: unknown own toon "${c.own_toon}"`);
		if (c.voiced_by !== undefined && (!isStr(c.voiced_by) || (ctx.people && !ctx.people.includes(c.voiced_by)))) add('error', [...path, 'voiced_by'], `${who}: voiced by an unknown person "${c.voiced_by}"`);
		if (c.mannequin !== undefined && !MANNEQUINS.some((b) => b.id === c.mannequin)) add('warning', [...path, 'mannequin'], `${who}: unknown mannequin "${c.mannequin}"`);
		if (c.tags !== undefined && !(Array.isArray(c.tags) && c.tags.every(isStr))) add('error', [...path, 'tags'], `${who}: tags must be a list of texts`);
		if (c.image !== undefined && c.image !== '' && (!isStr(c.image) || !IMAGE_PATH.test(c.image))) add('warning', [...path, 'image'], `${who}: image must be a CDN image path`);
		checkAffiliations(add, c.affiliations, [...path, 'affiliations'], who, languages, ctx);
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

		// the A-pose (avatar body): its own looks, one default, image + anchors
		if (c.apose !== undefined) {
			const ap: Path = [...path, 'apose'];
			if (!isObj(c.apose) || !Array.isArray(c.apose.looks)) add('error', ap, `${who}: apose must be { looks: [...] }`);
			else {
				const ids = new Set<string>();
				c.apose.looks.forEach((l: any, j: number) => {
					const lp: Path = [...ap, 'looks', j];
					if (!isObj(l) || !isStr(l.id) || !CHARACTER_ID.test(l.id)) return add('error', lp, `${who}: A-pose look ${j + 1} needs an id matching ${CHARACTER_ID.source}`);
					if (ids.has(l.id)) add('error', [...lp, 'id'], `${who}: A-pose look "${l.id}" is used twice`);
					ids.add(l.id);
					checkText(add, l.label, [...lp, 'label'], `${who}: A-pose ${l.id} label`, languages);
					if (l.path === undefined || l.path === '') add('warning', [...lp, 'path'], `${who}: A-pose "${l.id}" has no image yet`);
					else if (!isStr(l.path) || !IMAGE_PATH.test(l.path)) add('error', [...lp, 'path'], `${who}: A-pose "${l.id}" needs a CDN image path`);
					if (l.anchors !== undefined) {
						if (!isObj(l.anchors)) add('error', [...lp, 'anchors'], `${who}: A-pose anchors must be { <slot>: { x, y, size } }`);
						else
							for (const [slot, anchor] of Object.entries(l.anchors)) {
								if (!(AVATAR_SLOTS as string[]).includes(slot)) add('warning', [...lp, 'anchors', slot], `${who}: unknown avatar slot "${slot}" (${AVATAR_SLOTS.join(', ')})`);
								const a = anchor as any;
								if (!isObj(a) || ![a.x, a.y, a.size].every((n) => typeof n === 'number' && Number.isFinite(n) && n >= 0)) add('error', [...lp, 'anchors', slot], `${who}: anchor ${slot} must be { x, y, size } in px`);
							}
					}
				});
				const defaults = c.apose.looks.filter((l: any) => isObj(l) && l.default === true).length;
				if (c.apose.looks.length && defaults !== 1) add('error', [...ap, 'looks'], `${who}: exactly one A-pose look must be the default (${defaults} are)`);
			}
		}

		// personas → looks → moods × views
		if (!Array.isArray(c.personas)) {
			add('error', [...path, 'personas'], `${who}: personas must be a list (with "${REGULAR_PERSONA}")`);
			return;
		}
		if (!c.personas.some((p: any) => p?.id === REGULAR_PERSONA)) add('error', [...path, 'personas'], `${who}: the persona "${REGULAR_PERSONA}" is missing`);
		const personaSeen = new Set<string>();
		c.personas.forEach((p: any, j: number) => {
			const pp: Path = [...path, 'personas', j];
			if (!isObj(p) || !isStr(p.id) || !CHARACTER_ID.test(p.id)) return add('error', pp, `${who}: persona ${j + 1} needs an id matching ${CHARACTER_ID.source}`);
			const pwho = p.id === REGULAR_PERSONA ? who : `${who} · ${p.id}`;
			if (personaSeen.has(p.id)) add('error', [...pp, 'id'], `${who}: persona "${p.id}" is used twice`);
			personaSeen.add(p.id);
			if (p.id !== REGULAR_PERSONA && (!isObj(p.label) || !Object.values(p.label).some((t) => isStr(t) && t.trim()))) add('warning', [...pp, 'label'], `${pwho}: persona label is empty (e.g. "Biker")`);
			else checkText(add, p.label, [...pp, 'label'], `${pwho}: label`, languages);
			checkText(add, p.description, [...pp, 'description'], `${pwho}: description`, languages);
			if (p.introduced !== undefined) checkStoryPoint(add, p.introduced, [...pp, 'introduced'], `${pwho}: introduced`, ctx.toons);
			checkAffiliations(add, p.affiliations, [...pp, 'affiliations'], pwho, languages, ctx);

			if (!Array.isArray(p.looks) || !p.looks.length) {
				add('error', [...pp, 'looks'], `${pwho}: needs at least one look`);
				return;
			}
			const lookIds: string[] = [];
			p.looks.forEach((l: any, k: number) => {
				const lp: Path = [...pp, 'looks', k];
				if (!isObj(l) || !isStr(l.id) || !CHARACTER_ID.test(l.id)) return add('error', lp, `${pwho}: look ${k + 1} needs an id matching ${CHARACTER_ID.source}`);
				if (lookIds.includes(l.id)) add('error', [...lp, 'id'], `${pwho}: look "${l.id}" is used twice`);
				lookIds.push(l.id);
				const lwho = p.looks.length > 1 ? `${pwho} (${l.id})` : pwho;
				if (p.looks.length > 1 && (!isObj(l.label) || !Object.values(l.label).some((t) => isStr(t) && t.trim()))) add('warning', [...lp, 'label'], `${lwho}: look has no label`);
				else checkText(add, l.label, [...lp, 'label'], `${lwho}: look label`, languages);
				if (!isObj(l.images)) return add('error', [...lp, 'images'], `${lwho}: images must be { <mood>: { avatar?, portrait?, full? } }`);
				for (const [mood, slots] of Object.entries(l.images)) {
					const mp: Path = [...lp, 'images', mood];
					if (!MOOD_ID.test(mood)) add('error', mp, `${lwho}: mood "${mood}" must match ${MOOD_ID.source}`);
					else if (!isKnownMood(mood, ctx.moods ?? null)) add('warning', mp, `${lwho}: mood "${mood}" is not in the mood vocabulary – add it on the Moods tab`);
					if (!isObj(slots)) {
						add('error', mp, `${lwho}: ${mood} must be { avatar?, portrait?, full? }`);
						continue;
					}
					for (const [view, value] of Object.entries(slots)) {
						if (!(CHARACTER_IMAGE_VARIANTS as readonly string[]).includes(view)) add('warning', [...mp, view], `${lwho}: ${mood}: unknown view "${view}"`);
						else if (!isStr(value) || (value && !IMAGE_PATH.test(value))) add('warning', [...mp, view], `${lwho}: ${mood} ${view} must be a CDN image path`);
					}
				}
				// only `neutral` is required (the fallback anchor); portraits only (avatar is derived, full unused)
				if (!lookHasImages(l as CharacterLook)) add('warning', [...lp, 'images'], `${lwho}: no images yet (the reader falls back – neutral portrait needed)`);
				else if (!lookImage(l as CharacterLook, DEFAULT_MOOD, AUTHORED_VIEW)) add('error', [...lp, 'images', DEFAULT_MOOD, AUTHORED_VIEW], `${lwho}: no neutral portrait (the fallback anchor)`);
				else {
					const standard = sortedMoods(ctx.moods ?? defaultMoodVocabulary()).filter((m) => m.standard).map((m) => m.id);
					const missing = standard.filter((m) => !lookImage(l as CharacterLook, m, AUTHORED_VIEW));
					if (missing.length) add('info', [...lp, 'images'], `${lwho}: ${standard.length - missing.length}/${standard.length} of the standard set (missing ${missing.join(', ')})`);
				}
			});
			const defaults = p.looks.filter((l: any) => isObj(l) && l.default === true).length;
			if (defaults !== 1) add('error', [...pp, 'looks'], `${pwho}: exactly one look must be the default (${defaults} are)`);
			if (p.transitions !== undefined) {
				if (!Array.isArray(p.transitions)) add('error', [...pp, 'transitions'], `${pwho}: transitions must be a list`);
				else
					p.transitions.forEach((t: any, k: number) => {
						const tp: Path = [...pp, 'transitions', k];
						if (!isObj(t)) return add('error', tp, `${pwho}: transition ${k + 1} must be { from, to, image, label? }`);
						for (const key of ['from', 'to'] as const) if (!isStr(t[key]) || !lookIds.includes(t[key])) add('error', [...tp, key], `${pwho}: transition ${key} "${t[key]}" is not a look of this persona`);
						if (t.from === t.to) add('warning', tp, `${pwho}: transition from and to the same look`);
						if (!isStr(t.image) || !IMAGE_PATH.test(t.image)) add('warning', [...tp, 'image'], `${pwho}: transition ${t.from} → ${t.to} needs a CDN image path`);
						checkText(add, t.label, [...tp, 'label'], `${pwho}: transition label`, languages);
					});
			}
		});
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

/** scutoons/_/moods.json */
export function validateMoods(vocabulary: unknown, languages: string[] = [], file = MOODS_FILE): Problem[] {
	const problems: Problem[] = [];
	const add: Add = (level, path, message) => problems.push({ level, file, locale: null, path, message });
	if (!isObj(vocabulary) || !Array.isArray(vocabulary.moods)) {
		add('error', [], 'moods file must be { moods: [...] }');
		return problems;
	}
	const seen = new Set<string>();
	vocabulary.moods.forEach((m: any, i: number) => {
		const path: Path = ['moods', i];
		if (!isObj(m) || !isStr(m.id) || !MOOD_ID.test(m.id)) return add('error', path, `mood ${i + 1} needs an id matching ${MOOD_ID.source}`);
		if (seen.has(m.id)) add('error', [...path, 'id'], `mood "${m.id}" is used twice`);
		seen.add(m.id);
		if (!isObj(m.label) || !Object.values(m.label).some((t) => isStr(t) && t.trim())) add('warning', [...path, 'label'], `mood "${m.id}" has no label`);
		else checkText(add, m.label, [...path, 'label'], `mood "${m.id}" label`, languages);
		if (m.order !== undefined && typeof m.order !== 'number') add('error', [...path, 'order'], `mood "${m.id}": order must be a number`);
	});
	if (!seen.has(DEFAULT_MOOD)) add('error', ['moods'], `the vocabulary needs "${DEFAULT_MOOD}" (the fallback anchor)`);
	return problems;
}

/** scutoons/_/asset-requests.json: open requests are info (never errors). */
export function validateAssetRequests(requests: unknown, registry: CharacterRegistry | null | undefined, file = ASSET_REQUESTS_FILE): Problem[] {
	const problems: Problem[] = [];
	const add: Add = (level, path, message) => problems.push({ level, file, locale: null, path, message });
	if (!isObj(requests) || !Array.isArray(requests.requests)) {
		add('error', [], 'asset requests file must be { requests: [...] }');
		return problems;
	}
	const index = characterIndex(registry);
	const ids = new Set<string>();
	requests.requests.forEach((r: any, i: number) => {
		const path: Path = ['requests', i];
		if (!isObj(r) || !isStr(r.id) || !isStr(r.character)) return add('error', path, `request ${i + 1} must be { id, character, persona, look, mood, view, path, status }`);
		if (ids.has(r.id)) add('error', [...path, 'id'], `request id "${r.id}" is used twice`);
		ids.add(r.id);
		if (r.status !== 'open' && r.status !== 'done') add('error', [...path, 'status'], `request ${r.id}: status must be open or done`);
		const c = index.get(r.character);
		if (registry && !c) add('warning', [...path, 'character'], `request ${r.id}: unknown character "${r.character}"`);
		else if (c && !findPersona(c, r.persona)) add('warning', [...path, 'persona'], `request ${r.id}: ${r.character} has no persona "${r.persona}"`);
		if (r.status === 'open') add('info', path, `wanted: ${r.character} · ${r.persona} · ${r.look} · ${r.mood} (${r.view})${r.note ? ` – ${r.note}` : ''}`);
	});
	return problems;
}

export interface CastContext {
	registry?: CharacterRegistry | null;
	languages: string[];
	/** season numbers the toon has (unknown seasons are warnings) */
	seasons?: number[];
	file?: string;
}

/** <toon>/_/cast.json against the registry */
export function validateCast(cast: unknown, ctx: CastContext): Problem[] {
	const file = ctx.file ?? CAST_FILE;
	const problems: Problem[] = [];
	const add: Add = (level, path, message) => problems.push({ level, file, locale: null, path, message });
	if (!isObj(cast) || !isObj(cast.seasons)) {
		add('error', [], 'cast file must be { seasons: { "<n>": [{ character, personas }] } }');
		return problems;
	}
	const index = ctx.registry ? characterIndex(ctx.registry) : null;
	for (const [season, entries] of Object.entries(cast.seasons)) {
		const sp: Path = ['seasons', season];
		if (!/^[1-9]\d*$/.test(season)) add('error', sp, `season "${season}" must be a number`);
		else if (ctx.seasons && !ctx.seasons.includes(Number(season))) add('warning', sp, `the toon has no season ${season}`);
		if (!Array.isArray(entries)) {
			add('error', sp, `season ${season} must be a list`);
			continue;
		}
		const seen = new Set<string>();
		entries.forEach((e: any, i: number) => {
			const path: Path = [...sp, i];
			if (!isObj(e) || !isStr(e.character) || !e.character) return add('error', path, `season ${season}: entry ${i + 1} must be { character, personas }`);
			const who = `season ${season}: ${e.character}`;
			const c = index?.get(e.character);
			if (index && !c) add('error', [...path, 'character'], `${who} is not in the character registry`);
			if (seen.has(e.character)) add('warning', [...path, 'character'], `${who} is in the cast twice`);
			seen.add(e.character);
			if (!Array.isArray(e.personas) || !e.personas.length) add('warning', [...path, 'personas'], `${who}: no persona (the regular one is used)`);
			else
				e.personas.forEach((p: any, k: number) => {
					if (c && !findPersona(c, p)) add('error', [...path, 'personas', k], `${who}: unknown persona "${p}"`);
				});
			checkText(add, e.role, [...path, 'role'], `${who}: role`, ctx.languages);
		});
	}
	return problems;
}
