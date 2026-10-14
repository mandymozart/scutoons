// Characters: every character of the SCUTOONS universe in one global registry, and a cast per toon and season.
// Language-neutral store files with translated texts inside:
//
//   scutoons/_/characters.json      CharacterRegistry – every character, global (a character may appear in several
//                                                       toons; `own_toon` is optional – many characters have none)
//   scutoons/_/moods.json           MoodVocabulary    – every mood id with its label per locale (open-ended; the
//                                                       `standard` ones are a checklist, not a limit)
//   scutoons/_/asset-requests.json  AssetRequests     – wanted images (a note for the image supplier), open → done
//   <toon>/_/cast.json              ToonCast          – per season: which characters, with which personas
//
// Hierarchy (owner, 2026-09-30): Character → Persona → Look → Mood × View.
//   Character – the being. Identity-level: name, description, color, voiced_by, tags, the A-pose.
//   Persona   – a different identity or role of the same being (Cat Pope's `human`, `cat`, `biker`). Nested in the
//               character; every character has the persona `regular`. Changes who they are in the story.
//   Look      – the appearance right now: outfit, gear, accessories; can change within a scene (helmet on/off), one
//               default look per persona. Rule of thumb: if it could come off in the next panel, it's a look.
//               `transitions` (per persona) = an image shown while a look changes (helmet → default).
//   Mood      – the expression. Open vocabulary (moods.json); only `neutral` is required (the fallback anchor).
//   View      – the framing: avatar / portrait / full. The data field is called `variant` (backwards compatibility);
//               every UI label says "View" / "Ansicht". Only `portrait` is authored now: `avatar` is a derived 192px
//               downscale of the portrait, `full` is unused. Completeness counts portraits only.
//   A-pose    – `look.apose` (owner 2026-10-12: "a poses come per look"): the reader's avatar body in that look, one
//               image + its wear slots (slot instances; a duo/twins body has a slot type more than once). Never a
//               fallback; worn items are not drawn on it – they sit in the avatar page's slot tiles.
//
// Image fallback: `resolveExpression` (one implementation for the viewer VN and the creator). `resolveCast()` turns the
// registry + a cast into the legacy `characters.json` shape of one locale (`Character[]`, visual-novel.ts: `images` =
// regular persona, default look – what older readers show – plus `personas` with every persona's looks), so the viewer
// VN renderer keeps working; `<toon>/<locale>/characters.json` stays the static fallback. Old scene refs to former
// characters (plf, cat, mountain-tiger) resolve through `LEGACY_CHARACTER_IDS`.
import type { LocaleText } from './credits.ts';
import { pickText } from './credits.ts';
import type { Character, CharacterImage, ResolvedPersona } from './visual-novel.ts';
import type { Path } from './paths.ts';
import type { Level, Problem } from './validate.ts';
import { AVATAR_SLOTS, type AvatarSlot } from './items.ts';
import { mediaLocaleMessage, mediaPathsWithLocale } from './media.ts';
import type { AcceptedWarnings } from './accepted.ts';

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

/** Label of each standard mood per locale (the seed of moods.json). */
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
export interface MoodVocabulary extends AcceptedWarnings {
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
/** The A-pose view (asset requests of a look's `apose`): never a fallback. */
export const APOSE_VIEW = 'apose';
/** The view of an asset request for a look transition (`persona.transitions`, path = the transition file). */
export const TRANSITION_VIEW = 'transition';
/**
 * The view of an asset request for toon media that belongs to no character (a cover, a key visual, a thumbnail …;
 * owner 2026-10-01, "Language-independent media"): `toon` names the toon, `path` is from the CDN root, `character`,
 * `persona`, `look`, `mood` stay empty. Listed by `mediaRequests`, shown in the creator's "Wanted images" and on the page
 * that uses the file (e.g. the toon settings' cover).
 */
export const MEDIA_VIEW = 'media';

/**
 * Missing-image substitutes (owner, 2026-09-30): one on-brand "?" silhouette per base view, the very last fallback of
 * `resolveExpression` when nothing is set or the resolved file is known to be missing.
 * Files: `characters/_missing/missing_<view>.png` on the CDN
 * (avatar 192², portrait 600², full 600×1200, apose 768×1152). The creator still flags the real file as missing.
 */
export const MISSING_IMAGE_VIEWS = ['avatar', 'portrait', 'full', 'apose'] as const;
export const MISSING_IMAGE_ROOT = 'characters/_missing';
/** The substitute of a view (from the CDN root); unknown views get the portrait's. */
export function missingImagePath(view: string): string {
	const v = (MISSING_IMAGE_VIEWS as readonly string[]).includes(view) ? view : AUTHORED_VIEW;
	return `${MISSING_IMAGE_ROOT}/missing_${v}.png`;
}
/** true for a substitute path (with or without a leading `/`). */
export const isMissingImagePath = (path: string | null | undefined): boolean => !!path && path.replace(/^\/+/, '').startsWith(`${MISSING_IMAGE_ROOT}/`);

/** Options of `resolveExpression` / `characterImage`. */
export interface ResolveImageOptions {
	/** true for a path whose file is known to be missing (e.g. a failed CDN check) → the substitute */
	isMissing?: (path: string) => boolean;
	/** the substitute (default: `missingImagePath(view)`); '' = none (return '' when nothing is found) */
	substitute?: string;
}

/** Every character's base persona. */
export const REGULAR_PERSONA = 'regular';
/** The look of a persona without other looks (and the look whose convention path has no look segment). */
export const DEFAULT_LOOK = 'default';

/**
 * Former characters that are personas now (owner, 2026-09-30): old scene refs and old characters.json names resolve
 * through this map.
 */
export const LEGACY_CHARACTER_IDS: Record<string, { character: string; persona: string }> = {
	plf: { character: 'catpope', persona: 'human' },
	cat: { character: 'catpope', persona: 'cat' },
	'mountain-tiger': { character: 'catpope', persona: 'mountaintiger' }
};

/** A character id (maybe a legacy one) + persona → the current character + persona. */
export function resolveCharacterId(id: string, persona?: string): { character: string; persona: string } {
	const legacy = LEGACY_CHARACTER_IDS[id];
	if (legacy) return { character: legacy.character, persona: persona && persona !== REGULAR_PERSONA ? persona : legacy.persona };
	return { character: id, persona: persona || REGULAR_PERSONA };
}

// ── Files ─────────────────────────────────────────────────────────────────────────────────────────────────────

/** A point in a toon's story (a free reference for the editor and the story bible). */
export interface StoryPoint {
	toon: string;
	season?: number;
	chapter?: number;
}

/** mood → view → CDN path */
export type MoodImages = Partial<Record<string, CharacterImage>>;

/** An appearance of a persona (with / without helmet): its own mood set. */
export interface CharacterLook {
	/** `^[a-z0-9][a-z0-9-]*$`; `default` = no look segment in the convention path */
	id: string;
	label?: LocaleText;
	/** exactly one look of a persona is the default (VN refs without `look`) */
	default?: boolean;
	/** mood → view → CDN path (from the CDN root) */
	images: MoodImages;
	/** the look's A-pose – the reader's avatar body in this look; none = the look is no avatar body */
	apose?: LookApose;
	/** this look's own 3D model (a .glb, CDN path from the CDN root) – wins over the persona's (`personaModel`) */
	model?: string;
}

/** An image shown while a persona's look changes (the mouse taking her helmet off). */
export interface LookTransition {
	from: string;
	to: string;
	/** CDN path from the CDN root */
	image: string;
	label?: LocaleText;
}

/** A different identity or role of the same being. `regular` = the base persona every character has. */
export interface CharacterPersona {
	/** `regular` or `^[a-z0-9][a-z0-9-]*$` (e.g. `human`, `cat`, `biker`) */
	id: string;
	/** appended to the name: "Cat Pope · Biker" (the regular persona needs none) */
	label?: LocaleText;
	/** where the persona first appears (none = it has always been there) */
	introduced?: StoryPoint;
	description?: LocaleText;
	/** the persona's own color (VN avatar background); none = the character's */
	color?: string;
	/** the persona's own testo level 0–100 (none = the character's) – substances.ts */
	testo?: number;
	looks: CharacterLook[];
	transitions?: LookTransition[];
	/** the persona's 3D model (a .glb, CDN path from the CDN root); a look may have its own (`personaModel`) */
	model?: string;
	/** the body preset for the 3D fallback (which mannequin): `more-legs` (four or more), `wings`; none = two legs */
	body?: PersonaBody;
}

/**
 * One wear slot of an A-pose body (a slot instance): a tile on the reader's avatar page that holds an item of slot type
 * `slot`. A body may have a slot type more than once (twins, a duo: `torso-left`, `torso-right`); a single figure has
 * one instance per type with `id` = the type. The reader's avatar stores worn items by instance id (agents/ITEMS.md
 * "Slots per avatar body"). No image anchors since 2026-10-13: items are not drawn on the body.
 */
export interface AvatarSlotInstance {
	/** unique per body, `^[a-z0-9][a-z0-9-]*$`, at most 32 characters (`torso`, `torso-left`) */
	id: string;
	/** the slot type (AVATAR_SLOTS: head, face, neck, torso, hands, legs, feet, back, accessory) */
	slot: AvatarSlot;
	/** the tile label ("Kopf links" / "head left"); none = the slot type's label */
	label?: LocaleText;
}

/** The A-pose of a look (full body, neutral, 768 × 1152): the reader's avatar body in that look. */
export interface LookApose {
	/** CDN path from the CDN root (e.g. `characters/catpope/catpope_apose.webp`); none = still wanted */
	path?: string;
	/** the wear slots (slot instances); none = one instance per slot type */
	slots?: AvatarSlotInstance[];
}

/** Slot instance ids: unique per body, stored by the server (`avatar_slots.slot`, 32 characters). */
export const SLOT_INSTANCE_ID = /^[a-z0-9][a-z0-9-]{0,31}$/;

/** The wear slots of a body: its `slots`, or – when it has none – one unplaced instance per slot type (id = type). */
export function bodySlots(slots: AvatarSlotInstance[] | null | undefined): AvatarSlotInstance[] {
	if (Array.isArray(slots) && slots.length) return slots;
	return AVATAR_SLOTS.map((slot) => ({ id: slot, slot }));
}

/**
 * The worn items moved onto another body's slots (owner rule 2026-09-30, the server does the same in AvatarService):
 * an item stays on the instance with the same id when the target has it (and the type fits), otherwise it goes to the
 * first free instance of its slot type; without a free one it comes off. `typeOf` = the worn entry's item slot type
 * (undefined = unknown: kept only by id). One entry is worn at most once. Returns every target instance id (null = empty).
 */
export function remapAvatarSlots<E extends string | number>(
	worn: Record<string, E | null | undefined>,
	target: Pick<AvatarSlotInstance, 'id' | 'slot'>[],
	typeOf: (entry: E) => AvatarSlot | null | undefined
): Record<string, E | null> {
	const out: Record<string, E | null> = Object.fromEntries(target.map((t) => [t.id, null]));
	const placed = new Set<string>();
	const pending: E[] = [];
	for (const [id, entry] of Object.entries(worn)) {
		if (entry == null || placed.has(String(entry))) continue;
		const inst = target.find((t) => t.id === id);
		const type = typeOf(entry);
		if (inst && out[id] == null && (!type || type === inst.slot)) {
			out[id] = entry;
			placed.add(String(entry));
		} else pending.push(entry);
	}
	for (const entry of pending) {
		if (placed.has(String(entry))) continue;
		const type = typeOf(entry);
		const free = type ? target.find((t) => t.slot === type && out[t.id] == null) : undefined;
		if (!free) continue;
		out[free.id] = entry;
		placed.add(String(entry));
	}
	return out;
}

/** One character of the registry. Image values are CDN paths from the CDN root (`catpope/images/characters/…`). */
export interface RegistryCharacter {
	/** stable id, referenced by casts and by VN events – `^[a-z0-9][a-z0-9-]*$` */
	id: string;
	/** display name per locale (the VN speaker plate) */
	name: LocaleText;
	description?: LocaleText;
	/** CSS color (VN avatar background, speaker accents) */
	color: string;
	/**
	 * Testo level 0–100 (owner 2026-10-01: no male/female – the more testo, the more of its read characteristics; the
	 * visual mapping comes later with the art, `testoBand`). none = not set.
	 */
	testo?: number;
	/** story/workflow tendencies (`psychedelics` …), reference only, no runtime effect */
	tendencies?: string[];
	/** the toon this character is the protagonist of (optional – most characters have none) */
	own_toon?: string;
	/** person id of the voice (scutoons/_/people.json) */
	voiced_by?: string;
	/** search words */
	tags?: string[];
	/** the region the character comes from (a map region id – agents/MAP.md §8.10); fixed */
	origin?: string;
	/** the region the character is registered in by default (a `migrate` in the story moves it – the save game) */
	residence?: string;
	/**
	 * The avatar's home (research/CAVE.md D1): a map place id – none = where the character is first introduced in a
	 * story (the creator suggests it); a missing place is added to the map as a new one
	 */
	home?: string;
	/** what its home looks like: a catalog home scene id (`cave`, `tatami_room`; set per avatar – none = `cave`) */
	home_scene?: string;
	/** `regular` first */
	personas: CharacterPersona[];
}

/** `scutoons/_/characters.json` */
export interface CharacterRegistry extends AcceptedWarnings {
	characters: RegistryCharacter[];
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
export interface ToonCast extends AcceptedWarnings {
	seasons: Record<string, CastEntry[]>;
}

/** A wanted image: one cell of a look (portrait) or an A-pose look, with the convention path it should get. */
export interface AssetRequest {
	id: string;
	character: string;
	persona: string;
	look: string;
	mood: string;
	view: string;
	/** where the file should go (convention path, from the CDN root) */
	path: string;
	/** media requests (`view: 'media'`): the toon the file belongs to */
	toon?: string;
	/** media requests: the size wanted, e.g. "1080 × 1080" */
	size?: string;
	note?: string;
	/**
	 * a full brief for an image generator or a designer (the map places' requests, derived from the map – `mapMediaRequests`
	 * in map.ts; never stored)
	 */
	brief?: string;
	/** email / name of who asked */
	requested_by?: string | null;
	/** ISO date-time */
	requested_at: string;
	status: 'open' | 'done';
	done_at?: string;
}

/** `scutoons/_/asset-requests.json` */
export interface AssetRequests extends AcceptedWarnings {
	requests: AssetRequest[];
}

// ── Store paths ───────────────────────────────────────────────────────────────────────────────────────────────

/** Registry file in the pseudo-toon `scutoons` (CREDITS_SCOPE), locale `_`. */
export const CHARACTERS_FILE = 'characters.json';
/** The mood vocabulary (pseudo-toon `scutoons`, locale `_`). */
export const MOODS_FILE = 'moods.json';
/** Wanted images (pseudo-toon `scutoons`, locale `_`). */
export const ASSET_REQUESTS_FILE = 'asset-requests.json';
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
 * `characters/<character>[-<persona>]/<character>[-<persona>][_<look>]_<view>-<mood>.webp` – the regular persona and the
 * default look are left out. Stored paths are always explicit (existing files keep their paths). The supplier delivers
 * PNGs under the same name; the creator's asset import writes the WebP next to them; the derived 192 px avatar comes from the Luma generation workflows – agents/PERFORMANCE.md P3.
 */
export function conventionImagePath(character: string, view: string, mood: string, look: string = DEFAULT_LOOK, persona: string = REGULAR_PERSONA): string {
	const name = conventionName(character, persona);
	const lookPart = look && look !== DEFAULT_LOOK ? `_${look}` : '';
	return `${CHARACTER_IMAGE_ROOT}/${name}/${name}${lookPart}_${view}-${mood}.webp`;
}

/** Where a delivered file goes (`matchCharacterImageFile`). */
export type CharacterFileSlot =
	| { kind: 'expression'; persona: string; look: string; newLook: boolean; view: CharacterImageVariant; mood: string; path: string }
	| { kind: 'apose'; persona: string; look: string; newLook: boolean; path: string }
	| { kind: 'transition'; persona: string; from: string; to: string; path: string }
	| { kind: 'unknown'; reason: 'no-slot' | 'other-character' | 'unknown-persona' | 'unknown-look'; name?: string };

/**
 * The slot of a character a delivered file is for, by the convention name (owner 2026-10-03: "the import should work
 * for the entire character set – re-uploading replaces what is there"); case and extension don't matter:
 *   `boy_portrait-neutral.png`, `boy_helmet_full-happy.png`, `catpope-biker_avatar-sad.png` → an expression of that
 *     persona / look (no look part = the `default` look, else the persona's default look; a look the persona lacks is
 *     `newLook`; the name may be left out for the regular persona: `portrait-neutral.png`)
 *   `boy_apose.png`, `catpope-biker_helmet_apose.png` → the A-pose of that persona / look (look part as above)
 *   `boy_transition-helmet-to-default.png`, `catpope-biker_transition-…` → a transition (both looks must exist)
 * `path` is where the WebP goes (the convention path). Anything else: `unknown` with the reason.
 */
export function matchCharacterImageFile(file: string, character: Pick<RegistryCharacter, 'id' | 'personas'>): CharacterFileSlot {
	const stem = file.split(/[\/]/).pop()!.toLowerCase().replace(/\.[a-z0-9]+$/, '');
	const id = character.id.toLowerCase();
	const personas = [...(character.personas ?? [])].filter((p) => p?.id).sort((a, b) => conventionName(id, b.id).length - conventionName(id, a.id).length);
	/** the persona whose convention name starts the prefix, and what follows it */
	const personaOf = (prefix: string): { persona: CharacterPersona; rest: string } | CharacterFileSlot => {
		for (const p of personas) {
			const name = conventionName(id, p.id).toLowerCase();
			if (prefix === name) return { persona: p, rest: '' };
			if (prefix.startsWith(`${name}_`)) return { persona: p, rest: prefix.slice(name.length + 1) };
		}
		if (prefix.startsWith(`${id}-`)) return { kind: 'unknown', reason: 'unknown-persona', name: prefix.slice(id.length + 1).split('_')[0] };
		return { kind: 'unknown', reason: 'other-character', name: prefix.split(/[_-]/)[0] };
	};
	/** the persona + look of a name prefix (none = the regular persona; no look part = `default`, else its default look) */
	const lookOf = (prefix: string): { persona: CharacterPersona; look: string } | CharacterFileSlot => {
		const found = prefix ? personaOf(prefix) : personas.find((p) => p.id === REGULAR_PERSONA) ? { persona: findPersona(character, REGULAR_PERSONA)!, rest: '' } : null;
		if (!found) return { kind: 'unknown', reason: 'unknown-persona', name: REGULAR_PERSONA };
		if ('kind' in found) return found;
		return { persona: found.persona, look: found.rest || (findLook(found.persona, DEFAULT_LOOK) ? DEFAULT_LOOK : defaultLookId(found.persona)) };
	};
	const apose = stem.match(/^(?:(.*)_)?apose$/);
	if (apose) {
		const found = lookOf(apose[1] ?? '');
		if ('kind' in found) return found;
		const { persona, look } = found;
		return { kind: 'apose', persona: persona.id, look, newLook: !findLook(persona, look), path: aposeConventionPath(character.id, persona.id, look) };
	}
	const transition = stem.match(/^(.+)_transition-([a-z0-9][a-z0-9-]*?)-to-([a-z0-9][a-z0-9-]*)$/);
	if (transition) {
		const found = personaOf(transition[1]);
		if ('kind' in found) return found;
		if (found.rest) return { kind: 'unknown', reason: 'no-slot' };
		const [, , from, to] = transition;
		const missing = [from, to].find((l) => !findLook(found.persona, l));
		if (missing) return { kind: 'unknown', reason: 'unknown-look', name: missing };
		return { kind: 'transition', persona: found.persona.id, from, to, path: conventionTransitionPath(character.id, found.persona.id, from, to) };
	}
	const expression = stem.match(new RegExp(`^(?:(.*)_)?(${CHARACTER_IMAGE_VARIANTS.join('|')})-([a-z][a-z0-9-]*)$`));
	if (!expression) return { kind: 'unknown', reason: 'no-slot' };
	const [, prefix = '', view, mood] = expression;
	const found = lookOf(prefix);
	if ('kind' in found) return found;
	const { persona, look } = found;
	return {
		kind: 'expression',
		persona: persona.id,
		look,
		newLook: !findLook(persona, look),
		view: view as CharacterImageVariant,
		mood,
		path: conventionImagePath(character.id, view, mood, look, persona.id)
	};
}

/** `characters/<character>[-<persona>]/<character>[-<persona>]_transition-<from>-to-<to>.webp` (delivered as PNG, see above) */
export function conventionTransitionPath(character: string, persona: string, from: string, to: string): string {
	const name = conventionName(character, persona);
	return `${CHARACTER_IMAGE_ROOT}/${name}/${name}_transition-${from}-to-${to}.webp`;
}

/** `characters/<character>[-<persona>]/<character>[-<persona>][_<look>]_apose.webp` (a look's A-pose; .png works too) */
export function aposeConventionPath(character: string, persona: string, look: string, extension = 'webp'): string {
	const name = conventionName(character, persona);
	const lookPart = look && look !== DEFAULT_LOOK ? `_${look}` : '';
	return `${CHARACTER_IMAGE_ROOT}/${name}/${name}${lookPart}_${APOSE_VIEW}.${extension}`;
}

export const emptyCharacterRegistry = (): CharacterRegistry => ({ characters: [] });
export const emptyCast = (): ToonCast => ({ seasons: {} });
export const emptyAssetRequests = (): AssetRequests => ({ requests: [] });

export function newLook(id: string = DEFAULT_LOOK, label?: LocaleText, isDefault = true): CharacterLook {
	return { id, ...(label ? { label } : {}), ...(isDefault ? { default: true } : {}), images: {} };
}
export function newPersona(id: string, label?: LocaleText): CharacterPersona {
	return { id, ...(label ? { label } : {}), looks: [newLook()] };
}
export function newCharacter(id: string, name: LocaleText, extra: Partial<RegistryCharacter> = {}): RegistryCharacter {
	return { id, name, color: 'black', personas: [newPersona(REGULAR_PERSONA)], ...extra };
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

/**
 * A persona's body preset for its 3D fallback (owner 2026-10-14: "2 legs or more legs is enough as preset"; "winged
 * creatures … come with a different preset. a bird for now").
 */
export const PERSONA_BODIES = ['two-legs', 'more-legs', 'wings'] as const;
export type PersonaBody = (typeof PERSONA_BODIES)[number];

/**
 * The 3D fallback for every persona / look without its own model, per body preset (owner 2026-10-14: "a simple
 * mannequin … if there is no model there yet"; "some characters have 4 or even more legs"): two legs = three.js's
 * example mannequin (Xbot, Mixamo; Mixamo bone names), more legs = a four-legged one (the Khronos fox), wings = a bird (the storyboard's crow figure). One asset each on the CDN.
 */
export const MANNEQUINS: Record<PersonaBody, string> = {
	'two-legs': 'scutoons/3d/mannequin.glb',
	'more-legs': 'scutoons/3d/mannequin-more-legs.glb',
	wings: 'scutoons/3d/mannequin-wings.glb'
};

/**
 * The 3D model of a persona in a look (owner 2026-10-13: one per persona, looks may have their own): the look's
 * `model`, else the persona's – else the mannequin of the persona's body preset, marked `fallback` (what a preview /
 * the storyboard renderer shows; agents/research/CHARACTERS3D.md, FEATURES 10.95). `path` '' = no persona.
 */
export function personaModelSource(character: RegistryCharacter | null | undefined, persona: string = REGULAR_PERSONA, look?: string): { path: string; fallback: boolean } {
	const p = findPersona(character, persona);
	if (!p) return { path: '', fallback: false };
	const own = findLook(p, look || defaultLookId(p))?.model?.trim() || p.model?.trim();
	return own ? { path: own, fallback: false } : { path: MANNEQUINS[p.body ?? 'two-legs'] ?? MANNEQUINS['two-legs'], fallback: true };
}

/** The stored 3D model of a persona in a look (the look's, else the persona's), '' when none – no mannequin. */
export function personaModel(character: RegistryCharacter | null | undefined, persona: string = REGULAR_PERSONA, look?: string): string {
	const s = personaModelSource(character, persona, look);
	return s.fallback ? '' : s.path;
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

/** The explicit path of one mood/view of a look (no fallback), '' when not set. */
export function lookImage(look: Pick<CharacterLook, 'images'> | null | undefined, mood: string, view: string): string {
	const value = (look?.images?.[mood] as Record<string, unknown> | undefined)?.[view];
	return typeof value === 'string' ? value.trim() : '';
}

export function lookHasImages(look: Pick<CharacterLook, 'images'> | null | undefined): boolean {
	return Object.values(look?.images ?? {}).some((slots) => Object.values(slots ?? {}).some((p) => typeof p === 'string' && p.trim()));
}

/** Moods a look has an authored (portrait) image for. */
export function lookPortraitMoods(look: Pick<CharacterLook, 'images'> | null | undefined): string[] {
	return Object.keys(look?.images ?? {}).filter((m) => !!lookImage(look, m, AUTHORED_VIEW));
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

// ── The fallback rule ─────────────────────────────────────────────────────────────────────────────────────────

/**
 * THE fallback rule of character images (owner, 2026-09-29), regressive, one step at a time:
 *   1. persona + look + mood
 *   2. persona + look + neutral                (a missing mood never changes the look)
 *   3. persona + default look + mood
 *   4. persona + default look + neutral
 *   5. regular persona + default look + mood
 *   6. regular persona + default look + neutral
 * Within every step the requested view comes first, then the same mood's portrait (only `portrait` is authored: an
 * avatar or full view without its own file shows the portrait before any mood/look/persona step).
 * `lookSet(persona, look)` returns a look's mood → view → path map; `defaultLook(persona)` its default look id.
 *   7. the missing-image substitute of the view (`missingImagePath`) – also when the resolved file is known to be
 *      missing (`options.isMissing`)
 * The A-pose view never falls back to expressions (see `characterImage`).
 */
export function resolveExpression(
	lookSet: (persona: string, look: string) => Record<string, CharacterImage | undefined> | undefined,
	defaultLook: (persona: string) => string,
	ref: { persona?: string; look?: string; mood?: string },
	view: string,
	options: ResolveImageOptions = {}
): string {
	const substitute = options.substitute ?? missingImagePath(view);
	const found = resolveExpressionPath(lookSet, defaultLook, ref, view);
	return !found || options.isMissing?.(found) ? substitute : found;
}

function resolveExpressionPath(
	lookSet: (persona: string, look: string) => Record<string, CharacterImage | undefined> | undefined,
	defaultLook: (persona: string) => string,
	ref: { persona?: string; look?: string; mood?: string },
	view: string
): string {
	if (view === APOSE_VIEW) return '';
	const persona = ref.persona || REGULAR_PERSONA;
	const mood = ref.mood || DEFAULT_MOOD;
	const personaDefault = defaultLook(persona);
	const look = ref.look || personaDefault;
	const regularDefault = defaultLook(REGULAR_PERSONA);
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
	return '';
}

export interface ExpressionRef {
	persona?: string;
	look?: string;
	mood?: string;
}

/**
 * The image of a registry character (`resolveExpression`; the A-pose view: the A-pose of `ref.persona` / `ref.look`, no
 * fallback to other looks – only to the A-pose substitute). No character → ''.
 */
export function characterImage(character: RegistryCharacter | null | undefined, ref: ExpressionRef, view: string, options: ResolveImageOptions = {}): string {
	if (!character) return '';
	if (view === APOSE_VIEW) {
		const persona = findPersona(character, ref.persona);
		const path = findLook(persona, ref.look ?? defaultLookId(persona))?.apose?.path?.trim() ?? '';
		return !path || options.isMissing?.(path) ? (options.substitute ?? missingImagePath(APOSE_VIEW)) : path;
	}
	return resolveExpression(
		(p, l) => findLook(findPersona(character, p), l)?.images,
		(p) => defaultLookId(findPersona(character, p)),
		ref,
		view,
		options
	);
}

/**
 * The image a VN renderer shows for a resolved `Character` (characters.json shape) – same rule (`resolveExpression`);
 * the substitute comes from the CDN root (`/characters/_missing/…`, the legacy "from the CDN root" form).
 */
export function legacyCharacterImage(character: Character | null | undefined, ref: ExpressionRef, view: string, options: ResolveImageOptions = {}): string {
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
		{ substitute: `/${missingImagePath(view)}`, ...options }
	);
}

/** The shown name of a resolved `Character` for a persona ("Cat Pope · Biker"). */
export function legacyCharacterLabel(character: Character | null | undefined, persona?: string): string {
	if (!character) return '';
	return (persona && character.personas?.[persona]?.label) || character.label;
}

/** The color of a resolved `Character` for a persona (its own, else the character's). */
export function legacyCharacterColor(character: Character | null | undefined, persona?: string): string {
	if (!character) return '';
	return (persona && character.personas?.[persona]?.color) || character.color;
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

// ── Scene refs ────────────────────────────────────────────────────────────────────────────────────────────────

/** A VN character reference as authored: the current form `{ character, persona, look?, mood?, variant? }` or the old one `{ name, position: { variant }, mood? }`. */
export interface SceneRefInput {
	character?: string;
	name?: string;
	persona?: string;
	look?: string;
	mood?: string;
	variant?: string;
	position?: { variant?: string };
}

/**
 * The current form of a VN character ref: `character` (legacy ids mapped to their persona), `persona`, `look?`, `mood?`,
 * `variant?` (the view). `null` when it names no character.
 */
export function normalizeSceneRef(ref: SceneRefInput | null | undefined): { character: string; persona: string; look?: string; mood?: string; variant?: string } | null {
	const id = ref?.character ?? ref?.name;
	if (!id) return null;
	const { character, persona } = resolveCharacterId(id, ref?.persona);
	const variant = ref?.variant ?? ref?.position?.variant;
	return { character, persona, ...(ref?.look ? { look: ref.look } : {}), ...(ref?.mood ? { mood: ref.mood } : {}), ...(variant ? { variant } : {}) };
}

// ── Casts ─────────────────────────────────────────────────────────────────────────────────────────────────────

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

/** For the VN validation (ValidateContext.cast): character → cast persona ids of one season (or of every season). */
export function castIndex(cast: ToonCast | null | undefined, season?: number): Record<string, string[]> {
	return Object.fromEntries(castEntries(cast, season).map((e) => [e.character, e.personas]));
}

/** The toon's star: the registry character whose `own_toon` is this toon (e.g. catpope → Cat Pope), or null. */
export function toonStar(registry: CharacterRegistry | null | undefined, toon: string): string | null {
	return registry?.characters?.find((c) => c?.own_toon === toon)?.id ?? null;
}

/**
 * The colour of a toon's landing tile: the toon settings' `tile.color`, else the colour of the toon's star (the registry
 * character whose `own_toon` is the toon: catpope blue, yellowmouse yellow, toptoy black), else null.
 */
export function toonTileColor(registry: CharacterRegistry | null | undefined, toon: string, tile?: { color?: string } | null): string | null {
	if (tile?.color) return tile.color;
	return registry?.characters?.find((c) => c?.own_toon === toon)?.color ?? null;
}

/**
 * The star is in every season's cast (owner rule, agents/ITEMS.md "Cast & featuring"): adds `star` at the front of each
 * season in `seasons` (and of every season the cast already has) where it is missing, with its regular persona.
 * Mutates `cast`; returns true when it changed something. Casts are saved with it (creator) and served with it (library).
 */
export function ensureStar(cast: ToonCast, star: string | null | undefined, seasons: number[] = []): boolean {
	if (!star) return false;
	cast.seasons ??= {};
	let changed = false;
	for (const season of new Set([...Object.keys(cast.seasons), ...seasons.map(String)])) {
		if (!/^[1-9]\d*$/.test(season)) continue;
		const list = (cast.seasons[season] ??= []);
		if (list.some((e) => e?.character === star)) continue;
		list.unshift({ character: star, personas: [REGULAR_PERSONA] });
		changed = true;
	}
	return changed;
}

/** Every character ref of a VN event list (joinScene / leaveScene refs, showMessage speakers), normalized. */
function eventCharacters(events: unknown, out: string[]) {
	if (!Array.isArray(events)) return;
	for (const e of events as any[]) {
		const refs: SceneRefInput[] =
			e?.type === 'joinScene' || e?.type === 'leaveScene'
				? (e.params?.characters ?? [])
				: e?.type === 'showMessage' && e.params?.character
					? [{ character: e.params.character, persona: e.params.persona }]
					: [];
		for (const ref of refs) {
			const id = normalizeSceneRef(ref)?.character;
			if (id && !out.includes(id)) out.push(id);
		}
	}
}

/**
 * The characters a chapter's VN scenes use, in order of first appearance: the events of its groups (inline groups and
 * the scene documents, passed in as groups) and of their slices (slice triggers). The suggestion for `Chapter.featuring`.
 */
export function chapterSceneCharacters(groups: unknown[]): string[] {
	const out: string[] = [];
	for (const g of groups as any[]) {
		eventCharacters(g?.events, out);
		for (const slice of g?.slices ?? []) eventCharacters(slice?.events, out);
	}
	return out;
}

/**
 * The suggested `featuring` of a chapter: the toon's star first, then everyone its VN scenes use (first appearance).
 * Non-VN chapters get the star only; the author edits the list by hand.
 */
export function suggestFeaturing(groups: unknown[], star: string | null | undefined): string[] {
	return [...new Set([...(star ? [star] : []), ...chapterSceneCharacters(groups)])];
}

/**
 * The characters shown as "featuring" (reader): `featuring` in order, each resolved from the loaded cast; ids without a
 * character are skipped.
 */
export function featuredCharacters<T extends { name: string }>(featuring: string[] | null | undefined, characters: T[] | null | undefined): T[] {
	if (!Array.isArray(featuring) || !characters?.length) return [];
	return featuring.map((id) => characters.find((c) => c.name === id)).filter((c): c is T => !!c);
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
 * name, `images` = the regular persona's default look (what older readers show), `personas` =
 * every persona with its label ("Name · Label"), color, looks, default look and transitions; image paths relative to
 * the toon folder (see `toonRelativePath`). Cast entries that are not in the registry are skipped.
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
				...(p.color ? { color: p.color } : {}),
				defaultLook: defaultLookId(p),
				looks: Object.fromEntries((p.looks ?? []).filter((l) => l?.id).map((l) => [l.id, legacyLook(l, toon)])),
				...(p.transitions?.length ? { transitions: p.transitions.map((t) => ({ from: t.from, to: t.to, image: toonRelativePath(t.image, toon) })) } : {})
			};
		}
		out.push({
			name: c.id,
			label: characterName(c, locale, REGULAR_PERSONA, fallbacks),
			color: c.color ?? '',
			images: legacyLook(findLook(regular, defaultLookId(regular)), toon),
			...(description ? { description } : {}),
			personas
		});
	}
	return out;
}

// ── Editing / listing helpers ─────────────────────────────────────────────────────────────────────────────────

/** Sets (or clears, with '') one cell of a look; empty moods are removed except `neutral`. */
export function setLookImage(look: CharacterLook, mood: string, view: string, path: string) {
	look.images ??= {};
	const slots = (look.images[mood] ??= {}) as Record<string, string>;
	if (path.trim()) slots[view] = path.trim();
	else delete slots[view];
	if (!Object.keys(slots).length && mood !== DEFAULT_MOOD) delete look.images[mood];
}

/** Characters matching a query (id, names in any language, persona ids + labels, tags), best first. */
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

/** A look as an avatar body, in unlock lists (`/me/ avatar.unlocked[].looks`): `<persona>/<look>` (`regular/default`). */
export const bodyLookKey = (persona: string, look: string) => `${persona}/${look}`;

/** An avatar body for the reader: a look of a character with an A-pose image. */
export interface AvatarBody {
	character: string;
	persona: string;
	look: string;
	/** "Cat Pope · Biker (with helmet)" in the requested locale (the look only when the persona has several) */
	label: string;
	/** the character's name ("Cat Pope") */
	name: string;
	/** the look without the name: "papal", "Biker (with helmet)" – the reader's look select */
	lookLabel: string;
	path: string;
	/** the body's wear slots (`bodySlots`: the A-pose's slot instances, or one instance per slot type) */
	slots: AvatarSlotInstance[];
	/** the character's default body: the regular persona's default look (else the character's first body) */
	default: boolean;
}

/**
 * Every look with an A-pose image (registry order: character → persona → look) – what the reader's avatar picker
 * offers. The character's default body is the regular persona's default look when it has one, else its first body.
 */
export function avatarBodies(registry: CharacterRegistry | null | undefined, locale = 'en'): AvatarBody[] {
	const out: AvatarBody[] = [];
	for (const c of registry?.characters ?? []) {
		if (!c?.id) continue;
		const name = characterName(c, locale);
		const own: AvatarBody[] = [];
		for (const persona of c.personas ?? []) {
			for (const look of persona?.looks ?? []) {
				const path = look?.apose?.path?.trim();
				if (!look?.id || !path) continue;
				const personaName = characterName(c, locale, persona.id);
				const lookName = lookLabel(persona, look.id, locale);
				const label = persona.looks.length > 1 ? `${personaName} (${lookName})` : personaName;
				const lookText = persona.id === REGULAR_PERSONA ? lookName : persona.looks.length > 1 ? `${personaLabel(c, persona.id, locale)} (${lookName})` : personaLabel(c, persona.id, locale);
				own.push({ character: c.id, persona: persona.id, look: look.id, label, name, lookLabel: lookText, path, slots: bodySlots(look.apose?.slots), default: false });
			}
		}
		const regular = findPersona(c, REGULAR_PERSONA);
		const preferred = own.find((b) => b.persona === REGULAR_PERSONA && b.look === defaultLookId(regular)) ?? own[0];
		if (preferred) preferred.default = true;
		out.push(...own);
	}
	return out;
}

/**
 * The avatar bodies a reader may pick: `avatarBodies(registry)` filtered to the unlocked characters / looks (unlock
 * order, then the registry's order). A character unlock includes its default body; `looks` adds further ones
 * (`bodyLookKey`). Locked characters are left out.
 */
export function unlockedBodies(bodies: AvatarBody[], unlocked: { character: string; looks: string[] }[]): AvatarBody[] {
	const out: AvatarBody[] = [];
	for (const u of unlocked) {
		for (const b of bodies) if (b.character === u.character && (b.default || u.looks.includes(bodyLookKey(b.persona, b.look)))) out.push(b);
	}
	return out;
}

// ── Wanted images (asset requests) ────────────────────────────────────────────────────────────────────────────

/** The cell an asset request is about: `character/persona/look/mood/view`. */
export const cellKey = (r: Pick<AssetRequest, 'character' | 'persona' | 'look' | 'mood' | 'view'>) => `${r.character}/${r.persona}/${r.look}/${r.mood}/${r.view}`;

/** A missing image: a standard mood without a portrait, a look's A-pose without an image, or an open request. */
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
 * Every wanted image of the registry (or of one character): per persona and look an A-pose without an image (a look
 * with `apose` but no path), then the standard moods of the vocabulary (`standard: true`) plus every mood with an open request that have no portrait;
 * ordered character → persona → look → vocabulary order. `exists(path)` (optional) drops cells whose file is already at
 * the convention path.
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
		for (const personaId of personaIds(c)) {
			const persona = findPersona(c, personaId)!;
			for (const look of persona.looks ?? []) {
				if (!look?.id) continue;
				if (look.apose && !look.apose.path?.trim()) {
					const request = open.find((r) => r.character === c.id && r.persona === personaId && r.look === look.id && r.view === APOSE_VIEW);
					const path = request?.path ?? aposeConventionPath(c.id, personaId, look.id);
					if (!options.exists?.(path)) out.push({ character: c.id, persona: personaId, look: look.id, mood: DEFAULT_MOOD, view: APOSE_VIEW, path, ...(request ? { request } : {}) });
				}
				const requested = open.filter((r) => r.character === c.id && r.persona === personaId && r.look === look.id && r.view === AUTHORED_VIEW);
				const moods = [...new Set([...standard, ...requested.map((r) => r.mood)])].sort((a, b) => {
					const ia = order.indexOf(a);
					const ib = order.indexOf(b);
					return (ia < 0 ? Infinity : ia) - (ib < 0 ? Infinity : ib);
				});
				for (const mood of moods) {
					if (lookImage(look, mood, AUTHORED_VIEW)) continue;
					const path = conventionImagePath(c.id, AUTHORED_VIEW, mood, look.id, personaId);
					if (options.exists?.(path)) continue;
					const request = requested.find((r) => r.mood === mood);
					out.push({ character: c.id, persona: personaId, look: look.id, mood, view: AUTHORED_VIEW, path, ...(request ? { request } : {}) });
				}
			}
		}
	}
	return out;
}

/** Open media requests (`view: 'media'`: covers, key visuals … – no character), optionally of one toon or one path prefix */
export function mediaRequests(requests: AssetRequests | null | undefined, options: { toon?: string; status?: 'open' | 'done' | 'all' } = {}): AssetRequest[] {
	const status = options.status ?? 'open';
	return (requests?.requests ?? []).filter(
		(r) => r?.view === MEDIA_VIEW && (status === 'all' || r.status === status) && (!options.toon || r.toon === options.toon)
	);
}

/** Media requests as text lines for the image supplier (grouped by toon) */
export function mediaRequestsText(requests: AssetRequest[]): string {
	const lines: string[] = [];
	let group = '';
	for (const r of requests) {
		const head = `${r.toon ?? '–'} · media`;
		if (head !== group) {
			if (lines.length) lines.push('');
			lines.push(head);
			group = head;
		}
		lines.push(`  ${r.path}${r.size ? `  ${r.size}` : ''}${r.note ? `  – ${r.note}` : ''}`);
		if (r.brief) for (const line of r.brief.split(String.fromCharCode(10))) lines.push(`    ${line}`);
	}
	return lines.join('\n');
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
		lines.push(`  ${w.path.split('/').pop()}  (${w.view === APOSE_VIEW ? 'A-pose' : w.mood}${w.request?.note ? ` – ${w.request.note}` : ''})`);
	}
	return lines.join('\n');
}

// ── Prompts (briefs for an image generator or an artist) ─────────────────────────────────────────────────────

const pngName = (path: string) => (path.split('/').pop() ?? path).replace(/\.webp$/, '.png');

/**
 * The base-views brief of one look (workflows/characters/base-views.md "Brief"), filled in: attach any one reference
 * image of the character; the result is avatar, portrait, full and the A-pose, named by convention (PNG).
 */
export function characterBaseViewsBrief(character: RegistryCharacter, persona: string, look: string, locale: string): string {
	const name = characterName(character, locale, persona);
	const lookName = lookLabel(findPersona(character, persona), look, locale);
	const file = (view: string) => pngName(conventionImagePath(character.id, view, DEFAULT_MOOD, look, persona));
	return [
		`Attached is a reference image of the character ${name} (look: ${lookName}).`,
		'It is the starting point. Create this character\'s complete set of base views as SEPARATE images, recreating the character EXACTLY: same character, same art style and rendering, same colours, lighting, proportions, face, markings and clothing of this look. Do not redesign, restyle, add props or change anything else.',
		'',
		'First decide what the attached image is:',
		'- If it already IS one of the views below (same framing and pose), keep it as that view and create ONLY the views that are missing.',
		'- If it is NOT one of the views (a snapshot, an action pose, a sketch, a scene), use it ONLY as the identity reference and create ALL views below.',
		'',
		'The views (all with a neutral expression):',
		'1. AVATAR – head with neck, facing the viewer, eyes on a horizontal line, fully inside the frame, 600 × 600 px, transparent background.',
		'2. PORTRAIT – head down to above the waist with arms, facing the viewer, eyes on a horizontal line at 40 % of the image height, head centred, 600 × 600 px, transparent background.',
		'3. FULL – the whole figure standing in its natural, characteristic pose, feet fully visible, small even margin above the head and below the feet, 600 × 1200 px, transparent background.',
		'4. A-POSE – strict front-view A-pose, symmetrical, arms 30–40° away from the body with palms facing the thighs, legs slightly apart, feet flat and fully visible, figure centred, head top at ~5 % and soles at ~92 % of the height, 768 × 1152 px, transparent background with a soft shadow under the feet, wearing the clothing of this look.',
		'',
		'Deliver each view as its own image, named:',
		`  ${file('avatar')}`,
		`  ${file(AUTHORED_VIEW)}`,
		`  ${file('full')}`,
		`  ${pngName(aposeConventionPath(character.id, persona, look))}`,
		'Consistency check before delivering: side by side with the reference, the character must be recognisably identical in every view; only framing and pose may differ.'
	].join('\n');
}

/**
 * The moods brief of one look: attach its neutral portrait; one portrait per listed mood, only the expression changes
 * (moods are variations of the portrait view only – base-views.md). `moods`: id + label, in the order wanted.
 */
export function characterMoodsBrief(character: RegistryCharacter, persona: string, look: string, moods: { id: string; label: string }[], locale: string): string {
	const name = characterName(character, locale, persona);
	const lookName = lookLabel(findPersona(character, persona), look, locale);
	return [
		`Attached is the approved NEUTRAL PORTRAIT of the character ${name} (look: ${lookName}) – the only reference.`,
		'Create one new portrait per mood below, as SEPARATE images. Each is the attached portrait with ONLY the facial expression changed (and, where the mood needs it, the hands and shoulders): same character, art style, rendering, colours, lighting, proportions, clothing, framing and scale – eyes on the same horizontal line at 40 % of the image height, head centred, 600 × 600 px, transparent background. No text, no props, nothing added.',
		'',
		'Moods (mood – file name):',
		...moods.map((x) => `  ${x.label}${x.label !== x.id ? ` (${x.id})` : ''} – ${pngName(conventionImagePath(character.id, AUTHORED_VIEW, x.id, look, persona))}`),
		'',
		'Consistency check before delivering: laid over the neutral portrait, only the expression (and hands) may differ; the mood must read clearly at 192 px.'
	].join('\n');
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
		if (r.view === MEDIA_VIEW) {
			// toon media: done once the file exists (the page that uses it switches its path itself)
			if (exists?.(r.path)) {
				r.status = 'done';
				r.done_at = now;
				closed++;
			}
			continue;
		}
		const c = index.get(r.character);
		const done =
			r.view === APOSE_VIEW
				? !!findLook(findPersona(c, r.persona), r.look)?.apose?.path?.trim()
				: r.view === TRANSITION_VIEW
					? !!findPersona(c, r.persona)?.transitions?.some((t) => t?.image === r.path)
					: !!lookImage(findLook(findPersona(c, r.persona), r.look), r.mood, r.view);
		if (done || exists?.(r.path)) {
			r.status = 'done';
			r.done_at = now;
			closed++;
		}
	}
	return closed;
}

// ── Validation ────────────────────────────────────────────────────────────────────────────────────────────────

const isObj = (v: unknown): v is Record<string, any> => !!v && typeof v === 'object' && !Array.isArray(v);
const isStr = (v: unknown): v is string => typeof v === 'string';
const IMAGE_PATH = /\.(jpe?g|png|webp|gif|svg)$/i;
const MODEL_PATH = /^[^/\s]\S*\.glb$/i;

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
	/** the mood vocabulary (unknown moods are warnings; default: the seed vocabulary) */
	moods?: MoodVocabulary | null;
	/** languages the texts must exist in: per character (its own toon / the toons whose cast lists it) or for all */
	languages: string[] | ((character: RegistryCharacter) => string[]);
	/** known person ids (voiced_by) */
	people?: string[];
	/** known toon ids (own_toon, introduced) */
	toons?: string[];
	/** known map region ids (origin, residence – agents/MAP.md §8.10) */
	regions?: string[];
	/** known map place ids (home) */
	places?: string[];
	/** known home scene ids (home_scene – the catalog) */
	homeScenes?: string[];
	file?: string;
}

/** A look's A-pose: { path?, slots?: [{ id, slot, label? }] } (the avatar body in that look) */
function checkApose(add: Add, apose: any, ap: Path, who: string, languages: string[]) {
	if (!isObj(apose)) return add('error', ap, `${who}: apose must be { path, slots? }`);
	if (apose.path === undefined || apose.path === '') add('warning', [...ap, 'path'], `${who}: the A-pose has no image yet`);
	else if (!isStr(apose.path) || !IMAGE_PATH.test(apose.path)) add('error', [...ap, 'path'], `${who}: the A-pose needs a CDN image path`);
	if (apose.slots === undefined) return;
	if (!Array.isArray(apose.slots)) return add('error', [...ap, 'slots'], `${who}: A-pose slots must be a list of { id, slot, label? }`);
	const slotIds = new Set<string>();
	apose.slots.forEach((s: any, k: number) => {
		const sp: Path = [...ap, 'slots', k];
		if (!isObj(s) || !isStr(s.id) || !SLOT_INSTANCE_ID.test(s.id)) return add('error', sp, `${who}: A-pose slot ${k + 1} needs an id matching ${SLOT_INSTANCE_ID.source}`);
		if (slotIds.has(s.id)) add('error', [...sp, 'id'], `${who}: the A-pose has the slot id "${s.id}" twice`);
		slotIds.add(s.id);
		if (!(AVATAR_SLOTS as string[]).includes(s.slot)) add('error', [...sp, 'slot'], `${who}: slot "${s.id}" has an unknown slot type "${s.slot}" (${AVATAR_SLOTS.join(', ')})`);
		if (s.label !== undefined) checkText(add, s.label, [...sp, 'label'], `${who}: slot "${s.id}" label`, languages);
	});
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
	// media are language-independent (owner rule 2026-10-01): a locale segment in an image path is a smell
	for (const [path, value, locale] of mediaPathsWithLocale(registry)) add('warning', path, mediaLocaleMessage(value, locale));
	const seen = new Set<string>();
	registry.characters.forEach((c: any, i: number) => {
		const path: Path = ['characters', i];
		if (!isObj(c)) return add('error', path, `character ${i + 1} must be an object`);
		const who = isStr(c.id) && c.id ? c.id : `character ${i + 1}`;
		if (!isStr(c.id) || !CHARACTER_ID.test(c.id)) add('error', [...path, 'id'], `${who}: id must match ${CHARACTER_ID.source}`);
		else if (seen.has(c.id)) add('error', [...path, 'id'], `${who}: id is used twice`);
		else seen.add(c.id);
		if (isStr(c.id) && LEGACY_CHARACTER_IDS[c.id]) add('warning', [...path, 'id'], `${who}: this id is a legacy alias of ${LEGACY_CHARACTER_IDS[c.id].character} · ${LEGACY_CHARACTER_IDS[c.id].persona} – old scene refs would resolve to this character`);
		const languages = typeof ctx.languages === 'function' ? ctx.languages(c as RegistryCharacter) : ctx.languages;

		if (!isObj(c.name) || !Object.values(c.name).some((t) => isStr(t) && t.trim())) add('error', [...path, 'name'], `${who}: name is empty`);
		else checkText(add, c.name, [...path, 'name'], `${who}: name`, languages);
		checkText(add, c.description, [...path, 'description'], `${who}: description`, languages);
		if (!isStr(c.color) || !c.color.trim()) add('warning', [...path, 'color'], `${who}: no color`);
		if (c.testo !== undefined && !(typeof c.testo === 'number' && c.testo >= 0 && c.testo <= 100)) add('error', [...path, 'testo'], `${who}: testo must be a number 0–100`);
		if (c.tendencies !== undefined && !(Array.isArray(c.tendencies) && c.tendencies.every((t: unknown) => isStr(t) && CHARACTER_ID.test(t)))) add('error', [...path, 'tendencies'], `${who}: tendencies must be a list of ids`);
		if (c.own_toon !== undefined && (!isStr(c.own_toon) || (ctx.toons && !ctx.toons.includes(c.own_toon)))) add('warning', [...path, 'own_toon'], `${who}: unknown own toon "${c.own_toon}"`);
		if (c.voiced_by !== undefined && (!isStr(c.voiced_by) || (ctx.people && !ctx.people.includes(c.voiced_by)))) add('error', [...path, 'voiced_by'], `${who}: voiced by an unknown person "${c.voiced_by}"`);
		if (c.tags !== undefined && !(Array.isArray(c.tags) && c.tags.every(isStr))) add('error', [...path, 'tags'], `${who}: tags must be a list of texts`);
		for (const field of ['origin', 'residence'] as const)
			if (c[field] !== undefined && (!isStr(c[field]) || !c[field] || (ctx.regions && !ctx.regions.includes(c[field])))) add('warning', [...path, field], `${who}: unknown region "${c[field]}" (${field})`);
		if (c.home !== undefined && (!isStr(c.home) || (ctx.places && !ctx.places.includes(c.home)))) add('warning', [...path, 'home'], `${who}: the home "${c.home}" is not a place on the map – add it in the map builder`);
		if (c.home_scene !== undefined && (!isStr(c.home_scene) || (ctx.homeScenes && !ctx.homeScenes.includes(c.home_scene)))) add('warning', [...path, 'home_scene'], `${who}: unknown home scene "${c.home_scene}"`);

		if (c.apose !== undefined) add('error', [...path, 'apose'], `${who}: the A-pose belongs to a look now (look.apose) – the character-level "apose" is gone`);

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
			if (p.color !== undefined && !isStr(p.color)) add('error', [...pp, 'color'], `${pwho}: color must be a text`);
			if (p.testo !== undefined && !(typeof p.testo === 'number' && p.testo >= 0 && p.testo <= 100)) add('error', [...pp, 'testo'], `${pwho}: testo must be a number 0–100`);

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
				if (l.apose !== undefined) checkApose(add, l.apose, [...lp, 'apose'], lwho, languages);
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
			// 3D models: binary glTF only (agents/ASSETS.md)
			if (p.model !== undefined && (!isStr(p.model) || !MODEL_PATH.test(p.model))) add('warning', [...pp, 'model'], `${pwho}: the 3D model must be a CDN path to a .glb`);
			if (p.body !== undefined && !(PERSONA_BODIES as readonly unknown[]).includes(p.body)) add('warning', [...pp, 'body'], `${pwho}: body must be one of ${PERSONA_BODIES.join(', ')}`);
			p.looks.forEach((l: any, k: number) => {
				if (isObj(l) && l.model !== undefined && (!isStr(l.model) || !MODEL_PATH.test(l.model))) add('warning', [...pp, 'looks', k, 'model'], `${pwho}: look ${l.id}: the 3D model must be a CDN path to a .glb`);
			});
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
		if (r.view === MEDIA_VIEW) {
			if (!isStr(r.toon) || !r.toon) add('warning', [...path, 'toon'], `request ${r.id}: a media request names its toon`);
			if (!isStr(r.path) || !r.path) add('error', [...path, 'path'], `request ${r.id}: path is missing`);
			if (r.status === 'open') add('info', path, `wanted: ${r.toon ?? '?'} · ${r.path}${r.size ? ` (${r.size})` : ''}${r.note ? ` – ${r.note}` : ''}`);
			return;
		}
		const c = index.get(r.character);
		if (registry && !c) add('warning', [...path, 'character'], `request ${r.id}: unknown character "${r.character}"`);
		else if (c && !findPersona(c, r.persona)) add('warning', [...path, 'persona'], `request ${r.id}: ${r.character} has no persona "${r.persona}"`);
		const what = r.view === APOSE_VIEW ? `${r.persona} · ${r.look} · A-pose` : r.view === TRANSITION_VIEW ? `${r.persona} · transition from ${r.look}` : `${r.persona} · ${r.look} · ${r.mood} (${r.view})`;
		if (r.status === 'open') add('info', path, `wanted: ${r.character} · ${what}${r.note ? ` – ${r.note}` : ''}`);
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
