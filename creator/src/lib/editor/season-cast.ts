// A season's cast as edited in the toon's season (lap2 C1, LIBRARY.md "an assignment is edited in the thing that holds
// it"): the edits of one season's list in `cast:<toon>` – order, take out, personas, the role per language. Each mutates
// the cast file it is handed (inside one `doc.edit`, one undo step). The toon's star stays (ensureStar on save).
import { REGULAR_PERSONA, castSeasons, type CastEntry, type ToonCast } from '@scutoons/content';
import { moveAt } from './move.ts';

const listOf = (cast: ToonCast, season: number): CastEntry[] => cast.seasons?.[String(season)] ?? [];
const entryOf = (cast: ToonCast, season: number, character: string) => listOf(cast, season).find((e) => e.character === character);

/** the season's place `from` goes to `to` */
export const moveInCast = (cast: ToonCast, season: number, from: number, to: number): boolean => moveAt(listOf(cast, season), from, to);

/** takes a character out of the season (the star is never taken out) */
export function takeOutOfCast(cast: ToonCast, season: number, character: string, star: string | null): void {
	if (character === star || !cast.seasons?.[String(season)]) return;
	cast.seasons[String(season)] = listOf(cast, season).filter((e) => e.character !== character);
}

/** a persona on or off for the season; the last one stays (a cast entry always appears as someone) */
export function setCastPersona(cast: ToonCast, season: number, character: string, persona: string, on: boolean): void {
	const entry = entryOf(cast, season, character);
	if (!entry) return;
	const next = on ? [...new Set([...entry.personas, persona])] : entry.personas.filter((p) => p !== persona);
	entry.personas = next.length ? next : [REGULAR_PERSONA];
}

/** the role in one language; a role without any text goes */
export function setCastRole(cast: ToonCast, season: number, character: string, locale: string, text: string): void {
	const entry = entryOf(cast, season, character);
	if (!entry) return;
	const role = { ...(entry.role ?? {}), [locale]: text };
	if (Object.values(role).some((t) => t?.trim())) entry.role = role;
	else delete entry.role;
}

/** one season this character is cast in: where, at which place, as whom */
export interface CastIn {
	toon: string;
	season: number;
	personas: string[];
}

/** every season of every toon whose cast lists `character`, toon by toon, seasons ascending */
export function castIn(character: string, casts: Record<string, ToonCast | null | undefined>): CastIn[] {
	return Object.keys(casts)
		.sort()
		.flatMap((toon) =>
			castSeasons(casts[toon]).flatMap((season) => {
				const entry = listOf(casts[toon]!, season).find((e) => e?.character === character);
				return entry ? [{ toon, season, personas: entry.personas?.length ? entry.personas : [REGULAR_PERSONA] }] : [];
			})
		);
}
