// TODO: merge content and visual novel since both basically became one.
import { Renderer, Language, Status, type EventCategory, OnomatopoeiaVariant, OnomatopoeiaEffect, JumpToTargetType } from './enums.ts';
import type { AudioEvents, AudioConfiguration } from './audio.ts';
import type { VNEvent, VNSliceEvent } from './visual-novel.ts';
import type { ToonMapView } from './map.ts';

export interface KeyVisual {
	image: string;
	video?: string;
}

export type DialogVariant = 'bubble' | 'caption' | 'narration' | 'sfx';

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
	/** caption / narration only: draw the paper box + ink border. Default true; false = bare text (use `mask`). */
	frame?: boolean;
	/** Font size in % of the slice width (rendered as cqw). Default 4. */
	size?: number;
	/** Default 'center' for bubble/sfx, 'left' for caption/narration. */
	align?: 'left' | 'center' | 'right';
	/** Default 'middle' (only has an effect with `rect.height`). */
	valign?: 'top' | 'middle' | 'bottom';
	/** Default 'lettering' for bubble/sfx, 'mono' for caption/narration (matches chapter 1). */
	font?: DialogFont;
	weight?: 400 | 700;
	uppercase?: boolean;
	/** Text color, default ink (#000). */
	color?: string;
	/** Degrees. */
	rotation?: number;
	/** Fill rect (+ pad) before drawing the text, to cover text baked into legacy art. true = '#ffffff'. */
	mask?: string | true;
	/** Default 'rect'. */
	maskShape?: 'rect' | 'ellipse';
	/** Extra mask padding in % of slice width. Default 0.5. */
	pad?: number;
}

export interface Onomatopoeia {
	position?: {
		x?: number;
		y?: number;
	};
	size?: number;
	align?: 'left' | 'right' | 'center';
	rotation?: number;
	variant?: OnomatopoeiaVariant;
	effect?: OnomatopoeiaEffect;
	duration?: number,
	delay?: number,
	text: string | string[];
}

export interface JumpToTarget {
	type: JumpToTargetType;
	value: string;
	label?: string;
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
 * one WebP (`images/webtoon/<sXcY>/<sXcY>-slice-N.webp`); other heights are fine, the width stays 390.
 */
export const DEFAULT_SLICE = { width: 390, height: 625 } as const;

/** One encoding of a slice video: a CDN path like `SliceVideo.src` + an optional media query (`<source media>`) */
export interface SliceVideoSource {
	src: string;
	media?: string;
}

/**
 * A video instead of the slice image. The slice's `image` is its poster. By default a muted, inline loop that plays while
 * in view; with `player` a video to watch (a music video, a trailer).
 */
export interface SliceVideo {
	/** CDN path of an H.264 MP4, relative to the toon's folder like `image`; with `sources` the default (the last source) */
	src: string;
	/**
	 * Other encodings, tried before `src` in this order: the browser takes the first whose `media` query matches when the
	 * video loads (e.g. 480p for `(max-width: 600px)`). No adaptive streaming (HLS) – one file per load.
	 */
	sources?: SliceVideoSource[];
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
}

export interface Slice {
	/** CDN path of the slice image (1x WebP, 390 px wide) relative to the toon's folder; the poster of a video slice */
	image: string;
	/** Size of the image in px (1x): the reader reserves the space before the lazy image loads. */
	width: number;
	height: number;
	video?: SliceVideo;
	// TODO: refactor at some point to use a unified entity system
	// trigger: any[]; // placeholder for combined event system audio and vn
	// layers: any[]; // placeholder for combined dialog, onomatopoeia and image layers
	// entities: Entity[]; // even more general but requires massive refactor
	audio?: AudioEvents;
	dialog?: Dialog[];
	onomatopoeia?: Onomatopoeia[];
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
	created_at: string;
	updated_at: string;
}

export interface Group {
	label?: string;
	renderer?: {
		type: Renderer;
		params?: RendererParams;
	};
	slices?: Slice[];
	events?: VNEvent[];
	ref?: RefLink;
}

export interface RefLink {
	id: string;
	src: string;
	type: 'chapter' | 'group'; // db type
}

export interface Chapter {
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
	keyvisual?: KeyVisual;
	audio?: AudioConfiguration;
	groups: Group[];
	isPreview?: boolean;
	/**
	 * Who appears in this chapter: registry character ids, a subset of the season's cast (`cast:<toon>`), in display
	 * order. Optional and editorial – suggested from the chapter's VN scenes (`chapterSceneCharacters`), set by hand for
	 * plain scroll chapters. The reader shows it on the chapter list item and at the chapter start.
	 */
	featuring?: string[];
}

export interface ChapterWithId extends Chapter {
	id: number;
}

export interface Cover {
	square?: string;
	portrait?: string;
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

export interface SequenceItem {
	type: 'chapter';
	number: number;
	/**
	 * Visibility of the chapter in this locale – the single source of truth (a toon may release de before en):
	 * `published` = listed and readable, `draft` = hidden (shown in dev only), `coming-soon` = listed, not readable.
	 */
	status: Status;
}

export interface Season {
	title: string;
	authors?: string[];
	language?: string;
	number?: number;
	release?: string;
	description?: string;
	cover?: Cover;
	/** the season's downloads (the music album), shown below its chapter list */
	download?: Download;
	/** who supported the season, in order: "Supported by" at the end of the reader's credits fine print */
	supporters?: Supporter[];
	/** the toon page's desktop hero: a muted background loop + the button to the trailer chapter (language-neutral) */
	trailer?: SeasonTrailer;
	/** @deprecated moved to `<toon>/_/credits.json` (the creator shows it read-only); the viewer never rendered it */
	credits?: Credits;
	sequence: SequenceItem[];
}

export interface Event {
	title: string;
	date: string;
	location: string;
	city: string;
	category: EventCategory;
	link?: string;
}

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
	/** CDN path of the muted background loop (H.264 MP4, no audio track needed); the default after `sources` */
	loop: string;
	/** other encodings of the loop, tried first in this order (e.g. 1920 px for `(min-width: 1600px)`) */
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
 * A toon's settings as the reader gets them (`/library/toons/<toon>` → `toon`): the per-language document
 * `toon-settings:<toon>:<l>` (`ToonSettings`: title, description, language, release, status, cover) with the
 * language-neutral settings of the index entry `toon:<toon>` (`ToonNeutralSettings`) merged in by the library.
 */
export interface Toon {
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
	/** per language (covers can carry lettering) */
	cover: Cover;
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
 * Text stays per language; `release`, `status` and `cover` too (see `Toon`).
 */
export const TOON_NEUTRAL_FIELDS = ['authors', 'chapters', 'thumbnail', 'map', 'announce', 'tile'] as const;
export type ToonNeutralField = (typeof TOON_NEUTRAL_FIELDS)[number];

/**
 * The language-neutral toon settings, stored in the index entry `toon:<toon>` (all optional there). `map` is the toon's
 * start view on the world map (`{ center?, zoom? }`, agents/MAP.md §2.2); width, height and layers belong to the world.
 */
export type ToonNeutralSettings = Partial<Pick<Toon, Exclude<ToonNeutralField, 'map'>>> & { map?: ToonMapView };

/** The per-language document `toon-settings:<toon>:<l>` (file `<toon>/<l>/toon.json`): the `Toon` without the neutral settings */
export type ToonSettings = Omit<Toon, ToonNeutralField>;

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
export interface ToonIndex extends ToonNeutralSettings {
	id: string;
	uri: string;
	status: Status;
	avatar: string;
	i18n: Record<string, Record<string, string>>;
	/** The language the toon is written in; every other language is a translation of it. Default: `languages[0]`, else the first `i18n` key, else `de`. */
	sourceLanguage?: Language;
	/** Every language of the toon, the source first. Default: the `i18n` keys (source first). */
	languages?: Language[];
	/**
	 * Library answer only (`/library/toons`): the toon is not published but announced (`announce`) – list it as "soon",
	 * never link it (its pages stay 404 publicly)
	 */
	announced?: boolean;
}

/** A toon the reader lists (landing page, main menu): published, or announced before its release (`announced`) */
export function isToonListed(toon: Pick<ToonIndex, 'status' | 'announced'>): boolean {
	return toon.status === Status.PUBLISHED || toon.announced === true;
}

/** A season sequence item with its chapter file loaded; `status` is the season's (never the chapter file's). */
/**
 * A chapter in a season's overview (the library's `ChapterSummary`): the chapter without its content (`groups`,
 * `audio`) and its deprecated own `status`, plus the season entry's `type`, `number`, `status`.
 */
export type LoadedSequenceItem = Omit<Chapter, 'status' | 'groups' | 'audio'> & {
	type: 'chapter';
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
