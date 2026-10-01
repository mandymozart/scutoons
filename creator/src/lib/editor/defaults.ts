// Defaults for new content, following the existing catpope configs and agents/DESIGN.md (SCUTOONS style).
import type { Chapter, Dialog, Group, Season, Slice, ToonIndex, ToonSettings } from '@scutoons/content';
import { DEFAULT_SLICE, Renderer, Status } from '@scutoons/content';

/** The standard slice (PLAN.md rule, owner 2026-09-30): 390 × 625 px, delivered at 1x as WebP; other heights are fine. */
export const SLICE_RATIO = DEFAULT_SLICE.height / DEFAULT_SLICE.width;

/** `s1c2` for season 1 chapter 2 – prefix of image names and dialog ids */
export const chapterSlug = (chapter: number, season = 1) => `s${season}c${chapter}`;

export function isoDate(date = new Date()) {
	return date.toISOString().slice(0, 10);
}

/** The title of a new chapter per language (any locale of the toon; unknown ones get the English title). */
const CHAPTER_TITLES: Record<string, (n: number) => string> = {
	de: (n) => `Kapitel ${n}`,
	en: (n) => `Chapter ${n}`,
	es: (n) => `Capítulo ${n}`,
	jp: (n) => `第${n}話`,
	kr: (n) => `제${n}화`
};
export const chapterTitle = (number: number, locale: string) => (CHAPTER_TITLES[locale] ?? CHAPTER_TITLES.en)(number);

export function newChapter(number: number, locale: string, author = 'Mandy Mozart', season = 1): Chapter {
	const slug = chapterSlug(number, season);
	const release = new Date();
	release.setDate(release.getDate() + 30);
	return {
		title: chapterTitle(number, locale),
		author,
		number,
		language: locale as Chapter['language'],
		release: isoDate(release),
		status: Status.DRAFT,
		description: '',
		thumbnail: `images/${slug}-thumbnail.jpg`,
		keyvisual: { image: `images/${slug}-key-visual.jpg` },
		groups: [newGroup('scene-1')]
	};
}

/** The toon's name in a locale from its index entry (i18n name / title), else the id. */
export function toonName(entry: ToonIndex | null | undefined, locale: string, fallback: string) {
	const i18n = entry?.i18n ?? {};
	const texts = i18n[locale] ?? Object.values(i18n)[0] ?? {};
	return texts.name || texts.title || fallback;
}

const SEASON_TITLES: Record<string, (n: number) => string> = {
	de: (n) => `Staffel ${n}`,
	en: (n) => `Season ${n}`,
	es: (n) => `Temporada ${n}`,
	jp: (n) => `シーズン${n}`,
	kr: (n) => `시즌 ${n}`
};

/** A new season file (empty sequence; chapters are added with "New chapter" / "Add to season"). */
export function newSeason(locale: string, authors: string[] = [], number = 1): Season {
	return {
		title: (SEASON_TITLES[locale] ?? SEASON_TITLES.en)(number),
		authors,
		language: locale,
		number,
		release: isoDate(),
		description: '',
		cover: { square: `images/s${number}-cover-square.webp`, portrait: `images/s${number}-cover-portrait.webp` },
		sequence: []
	};
}

/**
 * New per-language toon settings (title / description from the index entry; downloads are per season). The
 * language-neutral settings (authors, map, tile …) start empty in the index entry and are edited on the toon settings page.
 */
export function newToon(id: string, locale: string, entry: ToonIndex | null | undefined): ToonSettings {
	return {
		id,
		title: toonName(entry, locale, id),
		language: locale,
		release: isoDate(),
		status: Status.DRAFT,
		description: entry?.i18n?.[locale]?.description ?? '',
		cover: { square: `images/s1-cover-square.webp`, portrait: `images/s1-cover-portrait.webp` }
	};
}

export function newGroup(label: string): Group {
	return { label, renderer: { type: Renderer.VERTICAL }, slices: [] };
}

/** Where the slices of a chapter live: `images/webtoon/<sXcY>/<sXcY>-slice-N.webp` (1x, 390 px wide). */
export const sliceImagePath = (chapter: number, index: number, season = 1) =>
	`images/webtoon/${chapterSlug(chapter, season)}/${chapterSlug(chapter, season)}-slice-${index + 1}.webp`;

/** A new slice: the standard size until the real image size is read from the CDN (`readImageSize`). */
export function newSlice(chapter: number, index: number, season = 1): Slice {
	// language-neutral path: text is live, the art is text-free (DESIGN.md Part 2)
	return { image: sliceImagePath(chapter, index, season), width: DEFAULT_SLICE.width, height: DEFAULT_SLICE.height };
}

export function newDialog(id: string, rect: Dialog['rect'], variant: Dialog['variant'] = 'bubble'): Dialog {
	return { id, rect, text: '', variant, size: 4 };
}

/** A free dialog id `s1c2-<slice>-<n>` (slice number is 1-based, like chapter 1) */
export function nextDialogId(chapter: number, sliceIndex: number, existing: (Dialog | undefined)[], season = 1) {
	const prefix = `${chapterSlug(chapter, season)}-${sliceIndex + 1}-`;
	const used = new Set(existing.map((d) => d?.id));
	let n = existing.length + 1;
	while (used.has(prefix + n)) n++;
	return prefix + n;
}
