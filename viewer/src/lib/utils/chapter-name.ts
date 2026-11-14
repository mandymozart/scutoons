import { chapterLabel, type LoadedSequenceItem, type Season, type ToonBundle } from '@scutoons/content';
import { page } from '$app/state';
import { m } from '$lib/paraglide/messages.js';
import { isListed } from '$lib/draft-preview.svelte.ts';

/** A season of the toon as the toon layout loads it (BACKLOG 10.41) */
export interface ToonSeason {
	number: number;
	season: Season;
	/** the stored sequence – every entry, drafts too (the positions of /season/S/chapter/K) */
	sequence: Season['sequence'];
	/** the listed chapters (the toon page's list) */
	chapters: LoadedSequenceItem[];
}

/**
 * every season of a toon's library bundle (BACKLOG 10.41), by number: its stored sequence (positions: /season/S/chapter/K,
 * the labels "Staffel 2 · Kapitel 1") and its listed chapters (the toon page's list) – the toon layout's, the conductor's
 * linked-reader picker (#529)
 */
export function toonSeasons(bundle: Pick<ToonBundle, 'seasons'>): ToonSeason[] {
	return bundle.seasons
		.map((o) => ({ number: o.number, season: o.season, sequence: o.season?.sequence ?? [], chapters: o.chapters.filter((item) => isListed(item)) }))
		.sort((a, b) => a.number - b.number);
}

/** the seasons a reader sees (any listed chapter) – two or more: the labels name the season */
export function shownSeasons(seasons: readonly ToonSeason[] | undefined): ToonSeason[] {
	return (seasons ?? []).filter((s) => s.chapters.length > 0);
}

/**
 * "Kapitel 2" / "Chapter 2", "Staffel 2 · Kapitel 1" / "Season 2 · Chapter 1" with two or more seasons (the position in
 * its season, never the absolute number – @scutoons/content chapterLabel), and for chapter 0 (the prologue) "Prolog" /
 * "Prologue". `seasons`: the toon's (default: the toon layout's, `page.data.seasons`); [] = the plain number (another toon).
 */
export function chapterName(number: number | string, seasons: readonly ToonSeason[] | undefined = page.data.seasons as ToonSeason[] | undefined): string {
	if (String(number).trim() === '' || !Number.isFinite(Number(number))) return m['common.chapter']({ number });
	const label = chapterLabel(shownSeasons(seasons), Number(number));
	if (label.prologue) return m['chapter.prologue']();
	return label.season === null ? m['common.chapter']({ number: label.chapter }) : m['common.season_chapter']({ season: label.season, number: label.chapter });
}

/** The position shown for a chapter in its season's list ("#1"), or null (the prologue) */
export function chapterShownNumber(number: number, seasons: readonly ToonSeason[] | undefined = page.data.seasons as ToonSeason[] | undefined): number | null {
	const label = chapterLabel(shownSeasons(seasons), number);
	return label.prologue ? null : label.chapter;
}
