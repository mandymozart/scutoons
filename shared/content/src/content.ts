// TODO: merge content and visual novel since both basically became one.
import { Renderer, Language, Status, type EventCategory, JumpToTargetType } from './enums.ts';
import type { AudioEvents, AudioConfiguration } from './audio.ts';
import type { VNEvent, VNSliceEvent } from './visual-novel.ts';
import type { QualityVariants } from './media-quality.ts';
import type { ToonMapView } from './map.ts';
import type { SliceItemRespawn } from './respawn.ts';
import type { FxSoundChoice, Highlighter } from './fx.ts';
import type { Repeat } from './recurrence.ts';
import type { CoverLettering } from './cover.ts';
import type { AcceptedWarnings } from './accepted.ts';
import type { SensitiveFlag } from './sensitive.ts';
import type { LocaleText } from './credits.ts';

export interface KeyVisual {
	image: string;
	video?: string;
}

/**
 * An onomatopoeia is a text box too (the "onomatopoeia" text preset: white letters with an ink outline); a `showTextBox`
 * event reveals any text box with an effect (`VNShowTextBoxEvent` `params.dialog`).
 */
export type DialogVariant = 'bubble' | 'caption' | 'narration';

/** Where a drawn speech bubble's tail points (`Dialog.tail`). */
export const DIALOG_TAILS = ['bottom-left', 'bottom', 'bottom-right', 'top-left', 'top', 'top-right', 'none'] as const;
/**
 * A drawn bubble's outline (`Dialog.shape`): `box` = speech (a rectangle with a tail – owner 2026-10-10: "alle sprechblasen
 * in rechteckige mit einem pfeil … ich brauche keine runden", "oval is redundant": an oval cut its text), `wobbly` =
 * shouting / singing (a trembling, hand-cut edge). `oval` is still read (older data) and drawn as a box.
 */
export const DIALOG_SHAPES = ['box', 'wobbly', 'oval'] as const;
export type DialogShape = (typeof DIALOG_SHAPES)[number];
export type DialogTail = (typeof DIALOG_TAILS)[number];

export type DialogFont = 'lettering' | 'mono';

/** A live, translatable text box on top of a slice image (see agents/DESIGN.md → Part 2). */
export interface Dialog {
	/** Stable key (Figma node id) → i18n file. */
	id?: string;
	/** % of the slice box, top-left origin. `height` optional (needed for `valign` and auto-fit). */
	rect: {
		x: number;
		y: number;
		width: number;
		height?: number;
	};
	/** Array entries are rendered as separate lines / paragraphs; `\n` inside an entry is a line break. */
	text: string | string[];
	/** Default 'bubble'. */
	variant?: DialogVariant;
	/**
	 * caption / narration: draw the paper box + ink border, default true (false = bare text, use `mask`).
	 * bubble: draw a speech bubble (paper, ink outline, a tail), default false – the bubble shape of chapter 1/2 art is in
	 * the image; the creator sets `true` for new bubbles on text-free art.
	 */
	frame?: boolean;
	/** a drawn bubble's outline (bubble with `frame: true`), default 'box' ('oval' = older data, drawn as a box) */
	shape?: DialogShape;
	/** a drawn bubble's tail (bubble with `frame: true`), default 'bottom-left' for a box, 'none' for a wobbly one */
	tail?: DialogTail;
	/** Font size in % of the slice width (rendered as cqw). Default 4. */
	size?: number;
	/** Default 'center' for bubble, 'left' for caption/narration. */
	align?: 'left' | 'center' | 'right';
	/** Default 'middle' (only has an effect with `rect.height`). */
	valign?: 'top' | 'middle' | 'bottom';
	/** Default 'lettering' for bubble, 'mono' for caption/narration (matches chapter 1). */
	font?: DialogFont;
	weight?: 400 | 700;
	uppercase?: boolean;
	/** Text color, default ink (#000). */
	color?: string;
	/**
	 * Outline of the letters (a CSS colour), e.g. white letters with an ink outline for an onomatopoeia; none = no outline.
	 */
	stroke?: string;
	/** outline width in em of the font size, default 0.14 */
	strokeWidth?: number;
	/** letter spacing in em of the font size (owner 2026-10-03), e.g. 0.05; default the font's own (0) */
	letterSpacing?: number;
	/** Degrees, around the box's centre. */
	rotation?: number;
	/**
	 * Uniform scale of the whole box (frame, text, mask) around its centre, after `rect` and `size` are laid out.
	 * Default 1. Lets an onomatopoeia grow or shrink without re-wrapping its text.
	 */
	scale?: number;
	/**
	 * Fill rect (+ pad) before drawing the text, to cover text baked into legacy art. true = '#ffffff'.
	 * @deprecated owner 2026-10-03: the chapter's art is re-imported text-free; the dialog data stays. Still drawn for old data.
	 */
	mask?: string | true;
	/** Default 'rect'. @deprecated with `mask` */
	maskShape?: 'rect' | 'ellipse';
	/** Extra mask padding in % of slice width. Default 0.5. @deprecated with `mask` */
	pad?: number;
	/**
	 * Place in the slice's ONE layer stack of text boxes, placed items and click / hover areas (`SliceItem.layer`,
	 * layers.ts): higher = drawn on top. None = the old order (sample areas, items, text boxes, event areas) – see
	 * `sliceLayers`.
	 */
	layer?: number;
}

export interface JumpToTarget {
	type: JumpToTargetType;
	value: string;
	/**
	 * `type: 'chapter'` only: the chapter (and `label` / `anyOf`) of ANOTHER toon (owner 2026-10-06, "at label");
	 * absent = this toon. `type: 'toon'` stays the toon's page. Variables travel along (one save game); their names
	 * have a domain (variables.ts: `x` is the toon's own, `global.x` every toon's, `toptoy.x` toptoy's).
	 */
	toon?: string;
	label?: string;
	/**
	 * a random group of this chapter instead of `label` (owner 2026-10-03, the chapter 2 stroll): one of these labels, never
	 * the group the reader is on (`pickJumpLabel`)
	 */
	anyOf?: string[];
}

/**
 * The OLD per-toon map settings (`toon.map` in the reader's bundle): since the world map (entity `map`, agents/MAP.md) the
 * library computes it from the world + the toon's start view, for older viewer builds only (one release, then dropped).
 */
export interface Map {
	layers: MapLayers,
	width: number,
	height: number, 
	center: MapCoordinates
}

export interface MapCoordinates {
	x: number;
	y: number;
}

export interface MapLayers {
	terrain: string;
}

/** @deprecated the old `locations:<toon>:<l>` shape – computed from the world map for older viewer builds (`legacyToonMap`) */
export interface MapLocation {
	id: string;
	label: string;
	toonId: string;
	chapterNumber?: string;
	type: string;
	image: string;
	coordinates: MapCoordinates;
}

/** @deprecated the old `routes:<toon>:<l>` shape – computed from the world map for older viewer builds (`legacyToonMap`) */
export interface MapRoute {
	id: string;
	title: string;
	locations: string[];
}

/**
 * The standard slice (owner 2026-09-30): 390 × 625 px, the Naver webtoon size. Slices are delivered and served at 1x as
 * one WebP (`s<n>/c<n>/<sXcY>-slice-N.webp`, agents/ASSETS.md "Folder convention"); other heights are fine, the width stays 390.
 */
export const DEFAULT_SLICE = { width: 390, height: 625 } as const;

/** One encoding of the season trailer's loop: a CDN path + an optional media query (`<source media>`) */
export interface SliceVideoSource {
	src: string;
	media?: string;
}

/**
 * Where a full-viewport slice video sits (`SliceVideo.viewport`, owner 2026-10-01). The slice then fills the whole
 * viewport (its whole width, the screen's height) instead of the reading column, and the video – and its poster – is
 * placed in that box (owner 2026-10-01, four fits):
 * - `contain` "Ganz sichtbar" / "Show all" (the default): the whole video, as big as the box allows, ink bars where needed;
 * - `width` "An Breite einpassen" / "Fit width": the video is as wide as the viewport; a taller video is cropped at the
 *   top/bottom, a flatter one gets bars above/below;
 * - `height` "An Höhe einpassen" / "Fit height": the video is as high as the viewport; a wider video is cropped at the
 *   sides, a narrower one gets bars left/right;
 * - `cover` "Füllen (beschnitten)" / "Fill (cropped)": always fills the viewport, the overflow is cropped.
 * `position` = which part stays when the video is cropped vertically, or where it sits when there are bars above/below.
 * Cropping at the sides always keeps the centre. `viewportObjectFit` turns width/height into contain or cover for a box.
 */
export interface SliceViewport {
	/** default 'center' */
	position?: 'top' | 'center' | 'bottom';
	/** default 'contain' */
	fit?: ViewportFit;
}

export const VIEWPORT_FITS = ['contain', 'width', 'height', 'cover'] as const;
export type ViewportFit = (typeof VIEWPORT_FITS)[number];
export const VIEWPORT_POSITIONS = ['top', 'center', 'bottom'] as const;

/**
 * How a full-viewport video is drawn in a box: `width` / `height` are `contain` or `cover` depending on the two ratios
 * (fit width = contain when the video is flatter than the box, else cover; fit height the other way round).
 */
export function viewportObjectFit(fit: ViewportFit | undefined, videoRatio: number, boxRatio: number): 'contain' | 'cover' {
	if (fit === 'cover') return 'cover';
	if (fit === 'width') return videoRatio >= boxRatio ? 'contain' : 'cover';
	if (fit === 'height') return videoRatio <= boxRatio ? 'contain' : 'cover';
	return 'contain';
}

/** CSS object-position for a placement (horizontal cropping keeps the centre) */
export function viewportObjectPosition(position: SliceViewport['position']): string {
	return position === 'top' ? '50% 0%' : position === 'bottom' ? '50% 100%' : '50% 50%';
}

/**
 * The box of a video slice in the reading column (`SliceVideo.frame`, owner 2026-10-01): a ratio instead of the
 * media's own ("Original" = no `frame`). The slice is 390 px wide and `width` / `height` follow the ratio (390 × 693 for
 * 9:16), so the reader reserves the space before anything loads; the media is placed in the box with `fit` / `position`
 * like a full-viewport video (`SliceViewport`). Doesn't apply in full viewport (the box is the viewport there).
 */
export interface SliceFrame {
	/** "w:h" (a preset like "9:16", or any) or [w, h]; between 4:1 and 1:4 */
	ratio: string | [number, number];
	/** default 'contain' */
	fit?: ViewportFit;
	/** default 'center' */
	position?: 'top' | 'center' | 'bottom';
}

/** Frame presets of the creator (the key visual = the chapter key visuals, 390 × 845 = 6:13). */
export const FRAME_PRESETS = [
	{ id: 'slice', ratio: '5:8' },
	{ id: 'keyvisual', ratio: '6:13' },
	{ id: 'hd-portrait', ratio: '9:16' },
	{ id: 'square', ratio: '1:1' }
] as const;
/** width / height limits of a frame: 4:1 (widest) … 1:4 (tallest) */
export const FRAME_RATIO_MIN = 1 / 4;
export const FRAME_RATIO_MAX = 4;

/** [w, h] of a frame ratio ("9:16" or [9, 16]), null when it isn't one */
export function parseFrameRatio(ratio: unknown): [number, number] | null {
	let pair: unknown[] | null = null;
	if (typeof ratio === 'string') pair = ratio.split(':').map((part) => Number(part.trim().replace(',', '.')));
	else if (Array.isArray(ratio)) pair = ratio;
	if (!pair || pair.length !== 2) return null;
	const [w, h] = pair as number[];
	return typeof w === 'number' && typeof h === 'number' && w > 0 && h > 0 && Number.isFinite(w) && Number.isFinite(h) ? [w, h] : null;
}

/** within 4:1 … 1:4 */
export function isFrameRatioAllowed(ratio: unknown): boolean {
	const pair = parseFrameRatio(ratio);
	if (!pair) return false;
	const r = pair[0] / pair[1];
	return r >= FRAME_RATIO_MIN - 1e-9 && r <= FRAME_RATIO_MAX + 1e-9;
}

/** the slice size for a frame at the slice width (390 × round(390 · h / w)) */
export function frameSliceSize(ratio: unknown, width: number = DEFAULT_SLICE.width): { width: number; height: number } | null {
	const pair = parseFrameRatio(ratio);
	return pair ? { width, height: Math.round((width * pair[1]) / pair[0]) } : null;
}

/** "16:9" for common ratios (±1 %), else "1,89:1" / "1.89:1" (decimal comma in German) */
export function aspectRatioLabel(width: number, height: number, locale = 'en'): string {
	if (!(width > 0) || !(height > 0)) return '–';
	const r = width / height;
	const common: [number, number][] = [[1, 1], [4, 3], [3, 2], [16, 10], [16, 9], [2, 1], [21, 9], [9, 16], [3, 4], [2, 3], [4, 5]];
	const hit = common.find(([a, b]) => Math.abs(a / b - r) / r <= 0.01);
	if (hit) return `${hit[0]}:${hit[1]}`;
	const n = r >= 1 ? r : 1 / r;
	const text = n.toFixed(2).replace(/\.?0+$/, '');
	const local = locale.startsWith('de') ? text.replace('.', ',') : text;
	return r >= 1 ? `${local}:1` : `1:${local}`;
}

/**
 * A video instead of the slice image. The slice's `image` is its poster. By default a muted, inline loop that plays while
 * in view; with `player` a video to watch (a music video, a trailer).
 */
export interface SliceVideo {
	/**
	 * CDN path of an H.264 MP4, relative to the toon's folder like `image`: the default file – it plays when no variant
	 * fits the reader's quality (`pickQualityVariant`) and in readers that don't know `variants`.
	 */
	src: string;
	/**
	 * One file per quality level (media-quality.ts): low = 480p, medium = 720p, high = 1080p. The reader takes its quality
	 * setting's level, else the nearest lower one, else `src`. No adaptive streaming (HLS) – one file per load.
	 */
	variants?: QualityVariants;
	/** default true (ignored with `player`) */
	loop?: boolean;
	/** default true: plays while the slice is in view (paused outside, poster only with reduced motion; ignored with `player`) */
	autoplay?: boolean;
	/**
	 * A video player instead of the loop: sound, native controls (fullscreen through them), started by the reader, no
	 * loop. The slice's `width` / `height` are the video's aspect ratio at the slice width (no 390 × 625 crop). While it
	 * plays the chapter's music is paused; watching it to its end (≥ 95 %) is the slice's reading progress.
	 */
	player?: boolean;
	/** the browser's basic controls (play/pause, time, volume, fullscreen); default: on for a player, off for a loop */
	controls?: boolean;
	/** fills the viewport instead of the reading column, placed like this (`SliceViewport`); omitted = in the column */
	viewport?: SliceViewport;
	/** in the column: the box's ratio + how the video sits in it (`SliceFrame`); omitted = "Original" (the media's ratio) */
	frame?: SliceFrame;
}

/** What tapping a placed item does (`SliceItem.action`); the creator chooses it per placed item. */
export const SLICE_ITEM_ACTIONS = ['none', 'pickup', 'events'] as const;
export type SliceItemAction = (typeof SLICE_ITEM_ACTIONS)[number];


/**
 * A catalog item (its cut-out icon) placed on a slice, like a text box: position, size, rotation. Language-neutral
 * geometry (the same in every locale file, like the text boxes' rects).
 *
 * What a tap does is the creator's choice (`action`, default `none`):
 * - `none`: just an image on the slice;
 * - `pickup`: the reader takes it – like a choice's `give <item> <qty>` (BACKLOG 10.38: the inventory only, no story variable; a story grant with a ref per
 *   place, so the catalog's rules apply: grantable, max count, one-time refs); a taken item is gone from the slice
 *   for that reader (game state, synced with the account's progress);
 * - `events`: runs `events` – the same VN events as a slice's / group's script (showMessage, addChoice → give an item …),
 *   like a click trigger whose area is the item.
 */
export interface SliceItem {
	/** stable key of this placement (pickups are remembered by it); `<item>-<n>` */
	id: string;
	/** catalog item id (`ItemDef.id`) */
	item: string;
	/** % of the slice box, top-left origin; `height` omitted = the icon's own ratio */
	rect: {
		x: number;
		y: number;
		width: number;
		height?: number;
	};
	/** degrees, around the centre */
	rotation?: number;
	/** default 'none' */
	action?: SliceItemAction;
	/** pickup: pieces, default 1 */
	qty?: number;
	/** events: run on tap (no trigger – the tap is the trigger) */
	events?: VNEvent[];
	/** events: how often a tap runs them (recurrence.ts, like a slice trigger's `repeat`); none = every tap */
	repeat?: Repeat;
	/**
	 * pickup: when it is back after being taken (respawn.ts). None = the item's default (catalog `respawn`; an item without
	 * one never reappears); `never` = once here even if the item could reappear; an interval = an override within the
	 * item's range (every visit, every n minutes / hours / days). Once per period; the catalog's limits still apply.
	 */
	respawn?: SliceItemRespawn;
	/**
	 * pickup: the sound when it lands in the bag (agents/FX.md) – none = the item's (catalog `sound`), else "pling";
	 * `none` = silent
	 */
	sound?: FxSoundChoice;
	/** pickup: one of the item's variants (its icon here, the variant goes into the inventory); none = the item itself */
	variant?: string;
	/** place in the slice's one layer stack of text boxes, placed items and areas (`Dialog.layer`, layers.ts) */
	layer?: number;
}

/**
 * One slice. `sensitive: true` (SensitiveFlag, the same in every language) = while the reader's "Hide sensitive content"
 * is on, the image / video is blurred, the slice's sounds stay silent and a small note sits on it (agents/ITEMS.md
 * "Sensitive slices").
 */
export interface Slice extends SensitiveFlag {
	/** CDN path of the slice image (1x WebP, 390 px wide) relative to the toon's folder; the poster of a video slice */
	image: string;
	/** Size of the image in px (1x): the reader reserves the space before the lazy image loads. */
	width: number;
	height: number;
	/**
	 * The image per quality level (owner 2026-10-03, PLAN.md "Slices"): low = 1x (= `image`), medium = 2x, high = 3x
	 * (390 / 780 / 1170 px wide WebP, `<name>.2x.webp`). The reader picks by its media quality (`pickQualityVariant`);
	 * none = just `image`.
	 */
	imageVariants?: QualityVariants;
	video?: SliceVideo;
	// TODO: refactor at some point to use a unified entity system
	// trigger: any[]; // placeholder for combined event system audio and vn
	// entities: Entity[]; // even more general but requires massive refactor
	audio?: AudioEvents;
	dialog?: Dialog[];
	/**
	 * catalog items placed on the slice, drawn above the image; with the text boxes ONE layer stack (`layer` on each
	 * entry, `sliceLayers`) – without any `layer` below the text boxes
	 */
	items?: SliceItem[];
	events?: Array<VNSliceEvent>;
	_type?: string;
	_label?: string;
}

export interface RendererParams {
	autoPlay?: {
		enabled?: boolean;
		delay?: number;
	};
	loop?: boolean;
	flip?: boolean;
	keyboard?: boolean;
	cube?: boolean;
	freeMode?: boolean;
	/**
	 * Full width (owner 2026-10-06): instead of the swiper, the group leaves the reading column and becomes a strip of
	 * its slices side by side, each at the screen's full height. Scrolling down pins the strip and moves it sideways
	 * (1:1, no snapping), then reading goes on downwards; on a desktop the strip can go full screen (← →). The swiper
	 * params above don't apply to it.
	 */
	fullWidth?: boolean;
}

/**
 * One entry of the reader's progress (the viewer's game state per toon: localStorage `scutoons_game_<toon>`, and for a
 * signed-in member `/api/progress/<toon>`). Chapters use `uri` = `<toon>/chapter/<n>` (`chapterCheckpointUri`, progress.ts).
 */
export interface Checkpoint {
	uri: string; // Store checkpoints as stateful uris
	/** the chapter's end was reached at least once */
	isComplete: boolean;
	/** furthest point of the current read-through, 0..1 (slices seen / the chapter's slices; 1 = the end); a finished chapter opened again starts a new read-through */
	position?: number;
	/**
	 * the furthest spot of the current read-through (BACKLOG 10.69): the group (index in the chapter's groups) and the
	 * slice (index in that group) read last; "continue" opens the chapter there. Missing in checkpoints from before.
	 */
	group?: number;
	slice?: number;
	created_at: string;
	updated_at: string;
}

export interface Group extends AcceptedWarnings {
	label?: string;
	renderer?: {
		type: Renderer;
		params?: RendererParams;
	};
	slices?: Slice[];
	events?: VNEvent[];
	ref?: RefLink;
	/**
	 * Linked groups (owner 2026-10-06): this group flows straight into the next group on the same page, no follow-up bar
	 * between. A chain = consecutive groups where every one but the last continues (`chainOf` in chains.ts). Any group may
	 * have a VN script or slice events (their scripts run one after another on the page); after a group whose script can
	 * jump away, the next group needs a label.
	 */
	continues?: boolean;
	/**
	 * Where the reader goes when this group – the chain's last – is done, instead of the next group / chapter. Ignored on a
	 * group that `continues`.
	 */
	end?: GroupEnd;
}

/** A group's authored end (`Group.end`): the destination; `auto` = go there at once, else the follow-up bar offers it */
export interface GroupEnd {
	target: JumpToTarget;
	auto?: boolean;
}

/** A group stored on its own (a scene): `src` = `chapter<N>/<name>.json` (EntityKey::sceneOfRef); id / type are unused leftovers */
export interface RefLink {
	src: string;
	id?: string;
	type?: 'chapter' | 'group';
}

export interface Chapter extends AcceptedWarnings {
	title: string;
	author: string;
	number: number;
	language: Language;
	release: string;
	/**
	 * @deprecated The season is the source of a chapter's visibility: `Season.sequence[].status`, per locale. The creator
	 * mirrors the season status here on save, only for older viewer builds; nothing reads it any more.
	 */
	status?: Status;
	description: string;
	thumbnail: string;
	/** a plain string = the image path (older chapters, e.g. catpope 3–5; the viewer reads both) */
	keyvisual?: KeyVisual | string;
	audio?: AudioConfiguration;
	groups: Group[];
	isPreview?: boolean;
	/**
	 * Who appears in this chapter: registry character ids, a subset of the season's cast (`cast:<toon>`), in display
	 * order. Optional and editorial – suggested from the chapter's VN scenes (`chapterSceneCharacters`), set by hand for
	 * plain scroll chapters. The reader shows it on the chapter list item and at the chapter start.
	 */
	featuring?: string[];
	/**
	 * Items that go with this chapter (catalog item ids, in order; language-neutral – the creator writes the same list to
	 * every locale), agents/ITEMS.md "Players, albums, related items". Stored + edited; the reader does not show it yet.
	 */
	related_items?: string[];
	/** how "Show hints" highlights the chapter's tap / hover / sound areas (fx.ts `HIGHLIGHTERS`); none = fireflies */
	highlight?: Highlighter;
}

export interface ChapterWithId extends Chapter {
	id: number;
}

/**
 * A toon's / season's cover (language-neutral, owner 2026-10-01: media are shared by all languages). The artwork carries
 * no text: with `lettering` the UI sets the title lines over it in the reader's language (`CoverArt` in
 * `@scutoons/reader`, agents/DATA.md "Language-independent media"). Without it the image is shown as it is – for artwork
 * that still has its title baked in (catpope until the lettering-free cover is delivered).
 */
export interface Cover {
	square?: string;
	portrait?: string;
	/**
	 * the image is lettering-free: the UI renders the title over it (default false = shown as it is). `true` = the default
	 * layout (the toon's name small, the season's title big in quotes); an object = a layout per format from the creator's
	 * cover editor (`CoverLettering`, cover.ts) – texts from the text fields, never baked
	 */
	lettering?: boolean | CoverLettering;
}

/** @deprecated moved to `<toon>/_/credits.json` (`ToonCredits`, credits.ts); the creator shows it read-only */
export interface Credits {
	artist?: string[];
	writer?: string;
	music?: string[];
	sound?: string[];
}

/** @deprecated the old per-locale `<toon>/<locale>/credits.json` (fallback of the viewer's credits page); new: `CreditPerson` + `ToonCredits` */
export interface Person {
	name: string;
	roles: string[];
}

/* @deprecated */
export interface PerformanceCast {
	name: string;
	venue: string;
	date: string;
	conductor?: string;
	members: Person[];
}

/* @deprecated */
export interface SongCredits {
	title: string;
	credits: Person[];
}

/* @deprecated */
export interface FullCredits {
	general: Person[];
	songs: SongCredits[];
	performances: PerformanceCast[];
	specialThanks: string[];
}

/** A season's `sequence` entry: always a chapter (owner 2026-10-07, BACKLOG 10.42 – no `type`). */
export interface SequenceItem {
	number: number;
	/**
	 * Visibility of the chapter in this locale – the single source of truth (a toon may release de before en):
	 * `published` = listed and readable, `draft` = hidden (shown in dev only), `coming-soon` = listed, not readable.
	 */
	status: Status;
}

export interface Season extends AcceptedWarnings {
	title: string;
	authors?: string[];
	language?: string;
	number?: number;
	release?: string;
	description?: string;
	/**
	 * The season's cover (owner 2026-10-02: covers are per season; a toon is represented by its avatar): text-free art +
	 * the lettering layout, language-neutral like `download` / `trailer` (the creator's cover editor writes it to every
	 * locale). The toon page shows the cover of the season it presents.
	 */
	cover?: Cover;
	/** the season's downloads (the music album), shown below its chapter list */
	download?: Download;
	/** who supported the season, in order: "Supported by" at the end of the reader's credits fine print */
	supporters?: Supporter[];
	/** the toon page's desktop hero: a muted background loop + the button to the trailer chapter (language-neutral) */
	trailer?: SeasonTrailer;
	/**
	 * Items that go with this season (catalog item ids, in order; language-neutral like `download` – the creator writes
	 * the same list to every locale): the toon page's "related" block (icon, name, and for physical items with a product
	 * the link to buy it), agents/ITEMS.md "Players, albums, related items".
	 */
	related_items?: string[];
	/** @deprecated moved to `<toon>/_/credits.json` (the creator shows it read-only); the viewer never rendered it */
	credits?: Credits;
	sequence: SequenceItem[];
}

/** One entry of the platform-wide `events` document (viewer /events, edited in the admin's /calendar) */
export interface Event {
	title: string;
	/** local time `YYYY-MM-DDTHH:MM:SS` */
	date: string;
	location: string;
	city: string;
	category: EventCategory;
	link?: string;
	/** the toons the event belongs to (not shown yet) */
	toon?: string[];
}

/** The `events` document: every event, in any order (the viewer sorts) */
export type EventList = Event[];

/** A supporter of a season (a logo row at the end of the credits fine print; without a logo its name as text) */
export interface Supporter {
	name: string;
	/** logo image path in the toon's CDN folder (like the season's other images) */
	logo?: string;
	/** link of the logo / name */
	url?: string;
}

/**
 * The season's trailer on the toon page (owner 2026-10-01, desktop hero): a short muted montage looping behind the title,
 * and "Trailer ansehen" / "Ton an" open the trailer chapter (the prologue) with sound. Language-neutral like `download`.
 */
export interface SeasonTrailer {
	/** CDN path of the muted background loop (H.264 MP4, no audio track needed); the default file */
	loop: string;
	/**
	 * Other encodings of the loop per media quality, like slice videos (owner 2026-10-02: no media queries to type) –
	 * the reader's quality setting picks one (`pickQualityVariant`), `loop` when none fits.
	 */
	variants?: QualityVariants;
	/** @deprecated older data: encodings with a media query – still played when there are no `variants` */
	sources?: SliceVideoSource[];
	/** CDN path of the poster image (shown before the loop plays and with reduced motion) */
	poster: string;
	/** the chapter the trailer buttons open, default 0 (the prologue) */
	chapter?: number;
}

/** A season's downloads (the reader's `/<toon>/download` page, linked below the season's chapter list) */
export interface Download {
	/** the music album (a zip), path in the toon's CDN folder; no path = no download */
	music?: string;
	/** everything of the season in one file, if there is one */
	complete?: string | null;
	/** decorative image beside the reader's music button (e.g. the record's cut-out), path in the toon's CDN folder */
	image?: string;
}

/**
 * A toon's settings as the reader gets them (`/library/toons/<toon>` → `toon`): the toon entry `toon:<toon>` resolved
 * for the served locale – its locale maps (`TOON_TEXT_FIELDS`: title, description, release, the language's status) and
 * its language-neutral settings (`ToonNeutralSettings`). Before the entry has texts (production before its conversion,
 * agents/NEUTRAL.md step 4) the per-language document `toon-settings:<toon>:<l>` (`ToonSettings`) with the entry's
 * neutral settings merged in.
 */
export interface Toon extends AcceptedWarnings {
	id: string;
	title: string;
	authors: string[];
	chapters: number;
	language: string;
	/** per language (a translation can be released later) */
	release: string;
	/** per language (a translation can stay a draft) */
	status: Status;
	description: string;
	thumbnail: string;
	/**
	 * @deprecated covers belong to the season (`Season.cover`, owner 2026-10-02); older per-language toon settings may still
	 * carry one – the reader falls back to it when the season has none
	 */
	cover?: Cover;
	/** the old per-toon map (older viewer builds), filled by the library from the world map + the toon's start view */
	map?: Map;
	/**
	 * Show the toon publicly before it is published (owner 2026-10-01): the landing page's tile and the main menu list it
	 * as "bald" / "soon" (not clickable, no dates, no chapters). Language-neutral.
	 */
	announce?: boolean;
	/** The landing page's tile (language-neutral); without it the tile uses the toon's avatar */
	tile?: ToonTile;
}

/**
 * The toon settings that are the same in every language (owner 2026-10-01): stored once in the toon's index entry
 * `toon:<toon>` (`ToonIndex`), never in the per-language `toon-settings:<toon>:<l>` (the validation warns there).
 * Text stays per language; `release` and `status` too (see `Toon`). `cover` was here 2026-10-01 – 2026-10-02; covers
 * belong to the season now (`Season.cover`).
 */
export const TOON_NEUTRAL_FIELDS = ['authors', 'chapters', 'thumbnail', 'map', 'announce', 'tile'] as const;
export type ToonNeutralField = (typeof TOON_NEUTRAL_FIELDS)[number];

/**
 * The language-neutral toon settings, stored in the index entry `toon:<toon>` (all optional there). `map` is the toon's
 * start view on the world map (`{ center?, zoom? }`, agents/MAP.md §2.2); width, height and layers belong to the world.
 */
export type ToonNeutralSettings = Partial<Pick<Toon, Exclude<ToonNeutralField, 'map'>>> & { map?: ToonMapView };

/**
 * The per-language document `toon-settings:<toon>:<l>` (file `<toon>/<l>/toon.json`): the `Toon` without the neutral
 * settings.
 * @deprecated legacy since 2026-10-06 (agents/NEUTRAL.md step 4): the texts, release and status per language live in the
 * toon entry (`ToonIndex.title` …, `TOON_TEXT_FIELDS`); the library reads these documents only while the entry has none
 */
// AcceptedWarnings added back explicitly: through `Omit` the schema generator made the inherited optional field required
export type ToonSettings = Omit<Toon, ToonNeutralField | 'cover' | 'accepted_warnings'> & AcceptedWarnings;

/** A toon's tile on the landing page: a square image on a colour */
export interface ToonTile {
	/** image path in the toon's CDN folder (`../characters/<id>/…` for a registry portrait), e.g. the star's neutral portrait */
	image: string;
	/** CSS colour behind the image; default: the colour of the toon's star in the character registry (`own_toon`) */
	color?: string;
}

/**
 * One toon's entry in `toons.json` (language-neutral). The creator keeps the newest version of it in the content store
 * as `<toon>/_/index.json` (see `TOON_INDEX_FILE`); read the languages with `toonLanguages()`.
 */
export interface ToonIndex extends ToonNeutralSettings, AcceptedWarnings {
	id: string;
	uri: string;
	/** the whole toon's visibility (the admin catalog / creator); each language has its own on top (`languageStatus`) */
	status: Status;
	/**
	 * The toon's title per language – THE toon's name (the reader's `Toon.title`, the lists' / menu's / creator's name;
	 * read it with `toonName(entry, locale)`; agents/NEUTRAL.md step 4 + decision 4). With it the entry holds the toon's
	 * settings in every language – the per-language `toon-settings:<toon>:<l>` documents are legacy then.
	 */
	title?: LocaleText;
	/** the toon's description per language (`Toon.description`) */
	description?: LocaleText;
	/** the release date per language, YYYY-MM-DD (a translation can be released later) */
	release?: LocaleText;
	/**
	 * The visibility per language (`Toon.status`; a translation can stay a draft): the library gates the toon page of a
	 * language on it and the toons list of a locale. A language without one has the source's; none at all = published.
	 */
	languageStatus?: Record<string, Status>;
	avatar: string;
	/**
	 * @deprecated the old per-language name (`i18n.<l>.name`) – the name is `title` now (owner decision 4, 2026-10-06);
	 * never stored any more, only read from older entries (`toonName`). The library still answers it, computed from
	 * `title`, for older viewer builds (legacy, drop later).
	 */
	i18n?: Record<string, Record<string, string>>;
	/** The language the toon is written in; every other language is a translation of it. Default: `languages[0]`, else the first `title` key, else `de`. */
	sourceLanguage?: Language;
	/** Every language of the toon, the source first. Default: the `title` keys, else the old `i18n` keys (source first). */
	languages?: Language[];
	/**
	 * Library answer only (`/library/toons`): the toon is not published but announced (`announce`) – list it as "soon",
	 * never link it (its pages stay 404 publicly)
	 */
	announced?: boolean;
}

/**
 * The toon entry's per-language fields (locale maps, agents/NEUTRAL.md step 4) – resolved into the reader's `Toon`
 * (`title`, `description`, `release`, `status`), never part of the `entry` the library answers with
 */
export const TOON_TEXT_FIELDS = ['title', 'description', 'release', 'languageStatus'] as const;

/** Whether a toon entry holds the toon's texts (converted from the per-language toon settings, or made so) */
export function hasToonTexts(entry: unknown): boolean {
	const title = (entry as { title?: unknown } | null | undefined)?.title;
	return title !== null && typeof title === 'object' && !Array.isArray(title);
}

/** A toon the reader lists (landing page, main menu): published, or announced before its release (`announced`) */
export function isToonListed(toon: Pick<ToonIndex, 'status' | 'announced'>): boolean {
	return toon.status === Status.PUBLISHED || toon.announced === true;
}

/**
 * A chapter in a season's overview (the library's `ChapterSummary`): the chapter without its content (`groups`,
 * `audio`) and its deprecated own `status`, plus the season entry's `number`, `status` (never the chapter file's).
 */
export type LoadedSequenceItem = Omit<Chapter, 'status' | 'groups' | 'audio'> & {
	number: number;
	status: Status;
};

/** A season with its chapter list (library `/library/toons/<toon>/seasons/<n>`) */
export interface SeasonOverview {
	number: number;
	season: Season;
	chapters: LoadedSequenceItem[];
}

/** Everything the toon pages need in one answer (library `/library/toons/<toon>?locale=`) */
export interface ToonBundle {
	/** the toon's index entry (null only for creators' draft toons without one) */
	entry: ToonIndex | null;
	toon: Toon;
	seasons: SeasonOverview[];
	locations: MapLocation[];
	routes: MapRoute[];
}
