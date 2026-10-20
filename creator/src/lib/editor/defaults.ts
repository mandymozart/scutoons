// Defaults for new content, following the existing catpope configs and agents/DESIGN.md (SCUTOONS style).
import type { Chapter, ChapterDoc, Dialog, DialogDoc, Group, Season, SeasonDoc, SequenceItemDoc, Slice } from '@scutoons/content';
import { DEFAULT_SLICE, Renderer, Status } from '@scutoons/content';

/** The standard slice (owner 2026-10-20, #78): 390 × 624 px (3x = WEBTOON's 800 × 1280), 1x WebP + `.2x` / `.3x`; other heights are fine. */
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
		// the chapter's folder (agents/ASSETS.md "Folder convention"): <toon>/s<season>/c<chapter>/
		thumbnail: `${chapterFolder(number, season)}/${slug}-thumbnail.webp`,
		keyvisual: { image: `${chapterFolder(number, season)}/${slug}-key-visual.webp` },
		groups: [newGroup('scene-1')]
	};
}

/** A new language-neutral chapter (agents/NEUTRAL.md): its title per language, the same release in every language */
export function newChapterDoc(number: number, languages: readonly string[], author = 'Mandy Mozart', season = 1): ChapterDoc {
	const { title, release, description, language: _language, status: _status, ...rest } = newChapter(number, languages[0] ?? 'en', author, season);
	return {
		...rest,
		title: Object.fromEntries(languages.map((l) => [l, chapterTitle(number, l)])),
		description: {},
		release: Object.fromEntries(languages.map((l) => [l, release])),
		groups: rest.groups as ChapterDoc['groups']
	};
}

/** A season entry's status in every language (a new chapter starts as a draft everywhere) */
export const sequenceStatus = (languages: readonly string[], status: Status = Status.DRAFT): SequenceItemDoc['status'] =>
	Object.fromEntries(languages.map((l) => [l, status]));

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
		// the season's folder (agents/ASSETS.md "Folder convention"): <toon>/s<season>/
		cover: { square: `s${number}/s${number}-cover-square.webp`, portrait: `s${number}/s${number}-cover-portrait.webp` },
		sequence: []
	};
}

/** A new language-neutral season (empty sequence): its title per language */
export function newSeasonDoc(languages: readonly string[], authors: string[] = [], number = 1): SeasonDoc {
	const { title, release, description, language: _language, sequence: _sequence, ...rest } = newSeason(languages[0] ?? 'en', authors, number);
	return {
		...rest,
		title: Object.fromEntries(languages.map((l) => [l, (SEASON_TITLES[l] ?? SEASON_TITLES.en)(number)])),
		description: {},
		release: Object.fromEntries(languages.map((l) => [l, release ?? isoDate()])),
		sequence: []
	};
}

export function newGroup(label: string): Group {
	return { label, renderer: { type: Renderer.VERTICAL }, slices: [] };
}

/** A chapter's folder in the toon's CDN folder: `s<season>/c<chapter>` (agents/ASSETS.md "Folder convention") */
export const chapterFolder = (chapter: number, season = 1) => `s${season}/c${chapter}`;

/** Where the slices of a chapter live: `s<n>/c<n>/<sXcY>-slice-N.webp` (1x, 390 px wide; density variants `.2x` / `.3x`). */
export const sliceImagePath = (chapter: number, index: number, season = 1) =>
	`${chapterFolder(chapter, season)}/${chapterSlug(chapter, season)}-slice-${index + 1}.webp`;

/** A new slice: the standard size until the real image size is read from the CDN (`readImageSize`). */
export function newSlice(chapter: number, index: number, season = 1): Slice {
	// language-neutral path: text is live, the art is text-free (DESIGN.md Part 2)
	return { image: sliceImagePath(chapter, index, season), width: DEFAULT_SLICE.width, height: DEFAULT_SLICE.height };
}

/** A new text box: a caption by default (owner 2026-10-03); its text per language starts empty */
export function newDialog(id: string, rect: Dialog['rect'], variant: Dialog['variant'] = 'caption'): DialogDoc {
	return { id, rect, text: {}, variant, size: 4 };
}

/**
 * A free text box id `s1c2-t<n>`: a chapter counter (agents/research/TEXT-ACROSS-SLICES.md §5 step 2 – a box may move to
 * another slice, so its id no longer names one; older ids `s1c2-<slice>-<n>` stay as they are). `taken` = the chapter's
 * ids (unique in the chapter – linked groups share a page; the author never types them): the next after the highest
 * counter there.
 */
export function nextDialogId(chapter: number, taken: Iterable<string> = [], season = 1) {
	const prefix = `${chapterSlug(chapter, season)}-t`;
	const used = new Set<string>(taken);
	let n = 0;
	for (const id of used) {
		const m = id.startsWith(prefix) ? /^\d+$/.exec(id.slice(prefix.length)) : null;
		if (m) n = Math.max(n, Number(m[0]));
	}
	n++;
	while (used.has(prefix + n)) n++;
	return prefix + n;
}
