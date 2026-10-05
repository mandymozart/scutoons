// The catalog's problems in the admin (owner 2026-10-04): validateCatalog over the stored entries (items, recipes,
// albums; the wear overlays against the published character registry – what readers render with), each mapped to the
// entry it is about. A warning can be accepted ("Looks fine"): `accepted_warnings` in that entry's data (a list-wide
// one in the meta entry `items`), saved like any change, so the catalog history keeps it and undo takes it back.
import { catalogWarningPlace, isAcceptedWarning, validateCatalog, withAcceptedWarning, type CatalogWarningPlace, type CharacterRegistry, type Problem } from '@scutoons/content';
import type { CatalogEntry } from '@scutoons/api';
import type { CatalogLists, Data } from './kinds.ts';

export interface CatalogProblem extends Problem {
	/** the list it is shown on (items / albums …) */
	list: string;
	/** where it is stored when accepted; null = not about a stored entry */
	place: CatalogWarningPlace | null;
}

const dataOf = (entries: CatalogEntry[] | undefined) => (entries ?? []).map((e) => e.data);

/** every problem of the catalog as the lists stand (the server checks errors on save; this adds the warnings) */
export function catalogProblems(lists: CatalogLists, registry: CharacterRegistry | null = null): CatalogProblem[] {
	if (!lists.items) return [];
	const catalog = { items: dataOf(lists.items), recipes: dataOf(lists.recipes), albums: dataOf(lists.albums) };
	return validateCatalog(catalog, registry, 'catalog').map((p) => ({ ...p, list: String(p.path[0] ?? 'items'), place: catalogWarningPlace(p, catalog) }));
}

/** the stored entry a problem is about */
export function entryOf(lists: CatalogLists, place: CatalogWarningPlace | null): CatalogEntry | null {
	return place ? ((lists[place.kind] ?? []).find((e) => e.id === place.id) ?? null) : null;
}

/** the warning as it is keyed in its entry */
const local = (p: CatalogProblem) => ({ path: p.place?.path ?? [], message: p.message });

export function canAccept(lists: CatalogLists, p: CatalogProblem): boolean {
	const entry = entryOf(lists, p.place);
	return p.level === 'warning' && !!entry && entry.data !== null && typeof entry.data === 'object';
}

/** accepted in `data` (the entry's stored data, or the open form's draft of it) */
export function isAcceptedIn(data: unknown, p: CatalogProblem): boolean {
	return p.level === 'warning' && isAcceptedWarning(local(p), data);
}

export function isAccepted(lists: CatalogLists, p: CatalogProblem): boolean {
	const entry = entryOf(lists, p.place);
	return !!entry && isAcceptedIn(entry.data, p);
}

/** `data` with the warning accepted (`on`) or not – the field is left out when nothing is accepted */
export function withAccepted(data: Data, p: CatalogProblem, on: boolean): Data {
	const { accepted_warnings: _old, ...rest } = data;
	const list = withAcceptedWarning(local(p), data, on);
	return list ? { ...rest, accepted_warnings: list } : rest;
}
