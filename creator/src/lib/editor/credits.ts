// Credits editor: three language-neutral entities edited on one page – the toon's `credits:<toon>` and the global
// `people` + `roles` (see @scutoons/content credits.ts). All three are ContentDocs with the single locale `_`, saved /
// published together through the page's ActionBar.
import {
	entities,
	NEUTRAL_LOCALE,
	slugId,
	uniqueId,
	type CreditEntry,
	type CreditPerson,
	type CreditRole,
	type CreditSection,
	type CreditSectionKind,
	type LocaleText,
	type PeopleRegistry,
	type RoleVocabulary,
	type ToonCredits
} from '@scutoons/content';
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
import { neutralDoc } from '$lib/editor/languages.ts';

const N = NEUTRAL_LOCALE;

export function creditsDocs(toon: string) {
	return {
		credits: neutralDoc(entities.credits(toon)),
		people: neutralDoc(entities.people),
		roles: neutralDoc(entities.roles)
	};
}

export const fileOf = <T>(doc: ContentDoc): T | null => (doc.data[N] ?? null) as T | null;
export const creditsOf = (doc: ContentDoc) => fileOf<ToonCredits>(doc);
export const peopleOf = (doc: ContentDoc) => fileOf<PeopleRegistry>(doc);
export const rolesOf = (doc: ContentDoc) => fileOf<RoleVocabulary>(doc);

/** Edits the neutral file of a doc (one undo step; same `key` within ~1 s = one step). */
export function edit<T>(doc: ContentDoc, change: (file: T) => void, key?: string) {
	doc.structural((file) => change(file as T), key);
}

export function newSection(kind: CreditSectionKind, credits: ToonCredits, languages: string[]): CreditSection {
	const ids = credits.sections.map((s) => s.id);
	const text = (): LocaleText => Object.fromEntries(languages.map((l) => [l, '']));
	const numbers = (k: string) => credits.sections.filter((s) => s.kind === k).map((s) => (s as any).number as number);
	switch (kind) {
		case 'toon':
			return { id: uniqueId('toon', ids), kind, lead: [], entries: [] };
		case 'season': {
			const number = Math.max(0, ...numbers('season')) + 1;
			return { id: uniqueId(`season-${number}`, ids), kind, number, entries: [] };
		}
		case 'chapter': {
			const number = Math.max(0, ...numbers('chapter')) + 1;
			return { id: uniqueId(`chapter-${number}`, ids), kind, number, entries: [] };
		}
		case 'song':
			return { id: uniqueId('song', ids), kind, title: text(), entries: [] };
		case 'performance':
			return { id: uniqueId('performance', ids), kind, title: text(), venue: text(), date: '', entries: [] };
		case 'thanks':
			return { id: uniqueId('thanks', ids), kind, entries: [] };
	}
}

export function newPerson(name: string, registry: PeopleRegistry): CreditPerson {
	return { id: uniqueId(slugId(name), registry.people.map((p) => p.id)), name: name.trim() };
}

export function newRole(label: LocaleText, category: CreditRole['category'], taken: string[]): CreditRole {
	const base = slugId(Object.values(label).find((t) => t.trim()) ?? 'role');
	return { id: uniqueId(base, taken), category, label };
}

/** Moves an item of a list (drag and drop, keyboard): `moveAt` (lib/editor/move.ts). */
export { moveAt as move } from './move.ts';

/** People matching a query (name, aliases, id), best first; merged people are left out. */
export function searchPeople(people: CreditPerson[], query: string, limit = 12): CreditPerson[] {
	const q = query.trim().toLowerCase();
	const active = people.filter((p) => !p.merged_into);
	if (!q) return active.slice(0, limit);
	const score = (p: CreditPerson) => {
		const names = [p.name, ...(p.aliases ?? []), p.id].map((n) => n.toLowerCase());
		if (names.some((n) => n === q)) return 0;
		if (names.some((n) => n.startsWith(q))) return 1;
		if (names.some((n) => n.split(/[\s-]+/).some((w) => w.startsWith(q)))) return 2;
		if (names.some((n) => n.includes(q))) return 3;
		return 9;
	};
	return active
		.map((p) => [p, score(p)] as const)
		.filter(([, s]) => s < 9)
		.sort((a, b) => a[1] - b[1] || a[0].name.localeCompare(b[0].name))
		.slice(0, limit)
		.map(([p]) => p);
}

export type EntryList = CreditEntry[];
