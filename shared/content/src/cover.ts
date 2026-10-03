// Cover lettering layouts (owner 2026-10-02, "a season cover image editor view for multi-language support"; agents/DATA.md
// "Language-independent media" → "Cover lettering layout"). The cover art is text-free and shared by every language; the
// title lines are set over it by the UI in the reader's language (`CoverArt` in `@scutoons/reader`). A layout only says
// WHERE and HOW a line sits – its text always comes from a text field: the toon's title, the season's title (per
// language) or the line's own `custom` text per language. Nothing is baked.
//
// `Cover.lettering`:
//   - absent / false → no lettering, the image as it is (artwork whose title is still baked in, catpope today);
//   - true           → the default layout (the toon's name small, the season's title big in quotes, tilted) – as before;
//   - { square?, portrait? } → one layout per format; a format without its own lines falls back to the default layout,
//     an empty list = no lettering on that format.
import type { Localized } from './items.ts';

export const COVER_FORMATS = ['square', 'portrait'] as const;
export type CoverFormat = (typeof COVER_FORMATS)[number];

/** width ÷ height of a format (square 1080², portrait 1080 × 1920) */
export const COVER_RATIOS: Record<CoverFormat, number> = { square: 1, portrait: 9 / 16 };

/** where a line's text comes from: the toon's title, the season's title, or the line's own text per language */
export const COVER_TEXT_SOURCES = ['toon', 'season', 'custom'] as const;
export type CoverTextSource = (typeof COVER_TEXT_SOURCES)[number];

export const COVER_ALIGNS = ['left', 'center', 'right'] as const;
export type CoverAlign = (typeof COVER_ALIGNS)[number];

/** the lettering colours: the design system's palette only (DESIGN.md "Color"), no free colours */
export const COVER_COLORS = ['paper', 'ink', 'cyan', 'purple', 'green', 'red'] as const;
export type CoverColor = (typeof COVER_COLORS)[number];
export const COVER_COLOR_VALUES: Record<CoverColor, string> = {
	paper: '#ffffff',
	ink: '#000000',
	cyan: '#00d1ff',
	purple: '#b70aff',
	green: '#3fff0a',
	red: '#ff210a'
};

/** limits of the numbers (the creator's fields and the validation use them) */
export const COVER_LIMITS = {
	/** the line's centre, % of the cover's width (x) / height (y) */
	x: { min: 0, max: 100 },
	y: { min: 0, max: 100 },
	/** font size, % of the cover's width (cqw) */
	size: { min: 1, max: 40 },
	/** the line's box, % of the cover's width: longer texts shrink to fit it */
	maxWidth: { min: 5, max: 100 },
	rotate: { min: -180, max: 180 },
	skew: { min: -45, max: 45 }
} as const;

/** One line of lettering. Geometry is language-neutral; the text comes from a text field (never baked). */
export interface CoverLine {
	/** the text: the toon's title, the season's title, or `custom` */
	text: CoverTextSource;
	/** the line's own text per language (only with `text: 'custom'`), e.g. { de: "Staffel 1", en: "Season 1" } */
	custom?: Localized;
	/** set the text in quotes ("…") */
	quotes?: boolean;
	/** the centre of the line's box in % of the cover's width */
	x: number;
	/** the centre of the line's box in % of the cover's height */
	y: number;
	/** font size in % of the cover's width (the lettering scales with the art) */
	size: number;
	/** the line's box width in % of the cover's width: a longer text (a translation) shrinks to fit it */
	maxWidth: number;
	/** degrees, negative = tilted up to the right like the hand-lettered catpope cover */
	rotate?: number;
	/** horizontal skew in degrees */
	skew?: number;
	/** the text inside its box (default center) */
	align?: CoverAlign;
	/** fill colour (default paper) */
	color?: CoverColor;
	/** outline + hard shadow colour (default ink) */
	outline?: CoverColor;
}

/** a layout per format; a format without lines uses the default layout */
export type CoverLettering = Partial<Record<CoverFormat, CoverLine[]>>;

/**
 * The default layout as lines (the one `lettering: true` renders): the toon's name small on top, the season's title big
 * in quotes beneath, tilted −5° / skewed −4°, in the upper part of the art. The starting point of a custom layout.
 */
export function defaultCoverLines(format: CoverFormat): CoverLine[] {
	const tilt = { rotate: -5, skew: -4 };
	return format === 'portrait'
		? [
				{ text: 'toon', x: 50, y: 16.5, size: 6, maxWidth: 84, ...tilt },
				{ text: 'season', quotes: true, x: 50, y: 22, size: 12.5, maxWidth: 84, ...tilt }
			]
		: [
				// where the hand lettering sat on the catpope cover: in the sky above the halo
				{ text: 'toon', x: 50, y: 19, size: 6, maxWidth: 84, ...tilt },
				{ text: 'season', quotes: true, x: 50, y: 31, size: 12.5, maxWidth: 84, ...tilt }
			];
}

/** is any lettering on (true or a layout object)? */
export function coverLetteringOn(lettering: unknown): boolean {
	return lettering === true || (typeof lettering === 'object' && lettering !== null && !Array.isArray(lettering));
}

/**
 * The lines of a format: its own layout, or null = the default layout (`lettering: true`, or a layout object without
 * this format). Only meaningful when `coverLetteringOn`.
 */
export function coverLines(lettering: unknown, format: CoverFormat): CoverLine[] | null {
	if (!coverLetteringOn(lettering) || lettering === true) return null;
	const lines = (lettering as CoverLettering)[format];
	return Array.isArray(lines) ? lines : null;
}

/** The texts a line can show, in one language. */
export interface CoverTexts {
	/** the toon's title (toon settings `title`) */
	toon: string;
	/** the season's title (season `title`) */
	season: string;
}

/**
 * A line's text in `locale`: from the toon / season title, or its own `custom` text (missing translation → the first
 * language that has one, so a line never disappears silently; the validation warns). Quotes when `quotes`.
 */
export function coverLineText(line: CoverLine, texts: CoverTexts, locale: string | undefined): string {
	let text = '';
	if (line.text === 'toon') text = texts.toon;
	else if (line.text === 'season') text = texts.season;
	else if (line.custom) text = (locale && line.custom[locale]) || Object.values(line.custom).find((v) => typeof v === 'string' && v.trim()) || '';
	text = (text ?? '').trim();
	return text && line.quotes ? `"${text}"` : text;
}

/**
 * The size factor that makes `text` fit its box before the browser has measured it (server render, first paint):
 * Bangers is condensed, ≈ 0.5 em per character. The reader measures the real width afterwards.
 */
export function estimateCoverFit(text: string, size: number, maxWidth: number): number {
	const width = text.length * 0.5 * size;
	return width > maxWidth ? Math.max(0.35, maxWidth / width) : 1;
}
