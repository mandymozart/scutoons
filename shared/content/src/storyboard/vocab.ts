// The storyboard's vocabulary and the rhythm model (from scripts/storyboard/lib.mjs, #119): the shot fields' allowed
// words, the light words, the reading-time model and the safe-format envelope. Rules: workflows/scenes/storyboard.md;
// concept: agents/STORYBOARD.md §1, §6. Pure – no Node, no Svelte.

/** shot.format: a stop (one screen), a move (the scroll is the camera), a panorama the reader drags, a sideways pass */
export const FORMATS = ['viewport', 'vertical', 'loop360', 'strip'] as const;
export const PROJECTIONS = ['perspective', 'top-down', 'isometric', 'polar'] as const;
export const MOVES = ['hold', 'tilt-down', 'tilt-up', 'fall', 'climb', 'travel', 'descend', 'reveal', 'look-around', 'pass'] as const;
/** shot.shot.size and figures[].framing */
export const SIZES = ['ecu', 'cu', 'mcu', 'medium', 'full', 'wide', 'ews'] as const;
export const ANGLES = ['eye', 'high', 'low', 'bird', 'worm', 'dutch'] as const;
/** light (workflows/scenes/storyboard.md "light"): extremely hard, one key, never diffuse (owner 2026-10-09) */
export const LIGHT = {
	key: ['left', 'right', 'top', 'bottom', 'front', 'back', 'top-left', 'top-right', 'bottom-left', 'bottom-right'],
	height: ['below', 'low', 'eye', 'high', 'overhead'],
	source: ['sun', 'moon', 'snow', 'window', 'door', 'candle', 'screen', 'neon', 'fire', 'lightning', 'flash', 'energy', 'none'],
	shadow: ['long', 'short', 'none']
} as const;
/** the poses the 3D mannequin knows */
export const POSES = ['stand', 'walk', 'run', 'jump', 'fall', 'slip', 'crouch', 'kneel', 'crawl', 'sit', 'lie', 'reach', 'point', 'throw', 'cast', 'arms-up', 'hug-self', 'stamp', 'push', 'look-back', 'pray'] as const;
export const FACING = ['front', 'away', 'left', 'right', 'three-quarter-left', 'three-quarter-right'] as const;
/** whose perspective a shot is (workflow "Personas, looks and perspective") */
export const VIEWS = ['real', 'belief', 'narration', 'memory'] as const;
/** source.type (where the image comes from now) and source.want (what it should come from) */
export const SOURCES = ['schematic', 'photo', 'unreal', 'direct', 'generated', 'art'] as const;
/** a shot's production status (STORYBOARD §7 "Parallel work") */
export const STATUS = ['draft', 'layout-ok', 'refs-locked', 'generating', 'still-ok', 'in-chapter'] as const;
/** review.status: the owner's verdict */
export const REVIEWS = ['ok', 'change'] as const;
/** the storyboard's dialog variants, fonts and fixed tails (a subset of content.ts Dialog's) */
export const DIALOG_VARIANTS_SB = ['narration', 'bubble', 'caption', 'lyrics'] as const;
export const DIALOG_FONTS_SB = ['mono', 'lettering'] as const;

export type ShotFormat = (typeof FORMATS)[number];
export type Projection = (typeof PROJECTIONS)[number];
export type Move = (typeof MOVES)[number];
export type ShotSize = (typeof SIZES)[number];
export type ShotAngle = (typeof ANGLES)[number];
export type LightKey = (typeof LIGHT.key)[number];
export type LightHeight = (typeof LIGHT.height)[number];
export type LightSource = (typeof LIGHT.source)[number];
export type LightShadow = (typeof LIGHT.shadow)[number];
export type Pose = (typeof POSES)[number];
export type Facing = (typeof FACING)[number];
export type View = (typeof VIEWS)[number];
export type SourceKind = (typeof SOURCES)[number];
export type ShotStatus = (typeof STATUS)[number];
export type ReviewStatus = (typeof REVIEWS)[number];

/** true when v is one of the list's words (a type guard for the literal unions) */
export function isIn<T extends string>(list: readonly T[], v: unknown): v is T {
	return (list as readonly unknown[]).includes(v);
}

/**
 * The rhythm model (STORYBOARD.md §6), calibrated on chapter 1 (owner 2026-10-09: careful reading 15–20 min; chapters
 * 1 and 2 about the same length, ch2 with more frames and less text). Seconds.
 */
export const RHYTHM = {
	scrollPerScreen: 1.2, // empty scrolling
	lookPerScreen: { careful: 10, skim: 3 }, // looking at the image; careful fitted to ch1 (rhythm-report --fit)
	perBox: 0.8, // finding a text box / message
	wordsPerSecond: 4, // ~240 wpm
	perClick: 1, // a VN message waiting for a tap
	wall: { words: 35, boxes: 3 }, // per screen
	choicelessMessages: 6, // VN "talking heads"
	airBeat: 0.25, // × W between two beats
	airScene: [0.75, 1.25] as [number, number] // × W at a scene change
};

/** Length target for a chapter (owner 2026-10-09): about chapters 1 and 2 – ~70 screens, 15–20 min careful */
export const TARGET = { screens: [60, 80] as [number, number], minutes: [15, 20] as [number, number] };

/**
 * Safe formats (owner 2026-10-09: "a restriction for what we prompt and suggest"): what the generators are proven to
 * deliver today. Suggest inside, plan below the maximum; beyond = a warning, never an error.
 */
export const SAFE = {
	viewport: { aspect: '9:16', note: 'one screen' },
	vertical: { maxTall: 7, plan: 4, proved: 'generators (1:4 rare, 1:7 the limit)', date: '2026-10-09' }, // h/w
	loop360: { maxScreens: 13, plan: 8, proved: 'Luma Labs (widest without massive problems; usually less)', date: '2026-10-09' } // width in screens of 390
};

/** a storyboard text: a locale map ({ de: '…' }) or a plain string */
export type SbText = string | Record<string, string>;

/** the words of a text (German first, else the first locale): tokens with a letter or a digit */
export function words(text: SbText | null | undefined): number {
	const t = typeof text === 'string' ? text : (text?.de ?? Object.values(text ?? {})[0] ?? '');
	return String(t).split(/\s+/).filter((w) => /[\p{L}\p{N}]/u.test(w)).length;
}

/** a text in one locale (else the first one) */
export function textOf(text: SbText | null | undefined, locale = 'de'): string {
	if (typeof text === 'string') return text;
	return text?.[locale] ?? Object.values(text ?? {})[0] ?? '';
}

/** reading time of one text cluster (a box or a message) in seconds */
export function readSeconds(n: number): number {
	return RHYTHM.perBox + n / RHYTHM.wordsPerSecond;
}

/** HTML / SVG escape */
export function esc(s: unknown): string {
	const map: Record<string, string> = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' };
	return String(s ?? '').replace(/[&<>"']/g, (c) => map[c]!);
}

/** seconds → "12.3 min" */
export const fmtMin = (sec: number): string => `${(sec / 60).toFixed(1)} min`;
