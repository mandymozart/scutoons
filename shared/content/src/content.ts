// TODO: merge content and visual novel since both basically became one.
import { Renderer, Language, Status, type EventCategory, OnomatopoeiaVariant, OnomatopoeiaEffect, JumpToTargetType } from './enums.ts';
import type { AudioEvents, AudioConfiguration } from './audio.ts';
import type { VNEvent, VNSliceEvent } from './visual-novel.ts';

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

export interface MapLocation {
	id: string;
	label: string;
	toonId: string;
	chapterNumber?: string;
	type: string;
	image: string;
	coordinates: MapCoordinates;
}

export interface MapRoute {
	id: string;
	title: string;
	locations: string[];
}

export interface Slice {
	image: string;
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

export interface Checkpoint {
	uri: string; // Store checkpoints as stateful uris
	isComplete: boolean;
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

export interface Download {
	complete: string | null;
	music: string;
}

export interface Toon {
	id: string;
	title: string;
	authors: string[];
	chapters: number;
	language: string;
	release: string;
	status: Status;
	description: string;
	thumbnail: string;
	cover: Cover;
	map: Map;
	download: Download;
}

/**
 * One toon's entry in `toons.json` (language-neutral). The creator keeps the newest version of it in the content store
 * as `<toon>/_/index.json` (see `TOON_INDEX_FILE`); read the languages with `toonLanguages()`.
 */
export interface ToonIndex {
	id: string;
	uri: string;
	status: Status;
	avatar: string;
	i18n: Record<string, Record<string, string>>;
	/** The language the toon is written in; every other language is a translation of it. Default: `languages[0]`, else the first `i18n` key, else `de`. */
	sourceLanguage?: Language;
	/** Every language of the toon, the source first. Default: the `i18n` keys (source first). */
	languages?: Language[];
}

/** A season sequence item with its chapter file loaded; `status` is the season's (never the chapter file's). */
export type LoadedSequenceItem = Omit<Chapter, 'status'> & {
	type: 'chapter';
	number: number;
	status: Status;
};
