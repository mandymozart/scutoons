// Defaults for new content, following the existing catpope configs and agents/DESIGN.md (SCUTOONS style).
import type { Chapter, Dialog, Group, Slice } from '@scutoons/content';
import { Renderer, Status } from '@scutoons/content';

/** SCUTOONS slice format (DESIGN.md, PLAN.md decisions): 390×625 at 1x, 1170×1875 at 3x. */
export const SLICE_FORMAT = { width1x: 390, height1x: 625, width3x: 1170, height3x: 1875 };
export const SLICE_RATIO = SLICE_FORMAT.height1x / SLICE_FORMAT.width1x;

/** `s1c2` for season 1 chapter 2 – prefix of image names and dialog ids */
export const chapterSlug = (chapter: number, season = 1) => `s${season}c${chapter}`;

export function isoDate(date = new Date()) {
	return date.toISOString().slice(0, 10);
}

export function newChapter(number: number, locale: string, author = 'Mandy Mozart', season = 1): Chapter {
	const slug = chapterSlug(number, season);
	const release = new Date();
	release.setDate(release.getDate() + 30);
	return {
		title: locale === 'de' ? `Kapitel ${number}` : `Chapter ${number}`,
		author,
		number,
		language: locale as Chapter['language'],
		published: false,
		release: isoDate(release),
		status: Status.DRAFT,
		description: '',
		thumbnail: `images/${slug}-thumbnail.jpg`,
		keyvisual: { image: `images/1x/${slug}-key-visual.jpg` },
		groups: [newGroup('scene-1')]
	};
}

export function newGroup(label: string): Group {
	return { label, renderer: { type: Renderer.VERTICAL }, slices: [] };
}

export function newSlice(chapter: number, index: number, season = 1): Slice {
	// language-neutral path: text is live, the art is text-free (DESIGN.md Part 2)
	return { image: `images/1x/${chapterSlug(chapter, season)}-slice-${index + 1}.jpg` };
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
