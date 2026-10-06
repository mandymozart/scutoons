// Accepted warnings (owner 2026-10-04): a warning the creator checked and found fine ("Looks fine") is listed in the
// file it is about, in that locale's document (a language-neutral document holds every locale's, a per-locale warning
// with its locale in the key), so the decision is versioned with the content (saved, published, undone like any edit).
// An entry names the place (array items by their id where they have one, so inserting a slice does not move it) and a short hash of the message and the value there: when the value changes, the warning is back.
import { getAt, type Path } from './paths.ts';

export const ACCEPTED_WARNINGS_FIELD = 'accepted_warnings';

/** The optional field on every content document the creator validates. Readers ignore it. */
export interface AcceptedWarnings {
	/** warnings accepted in the creator: `<place>#<hash>` (acceptedWarningKey) */
	accepted_warnings?: string[];
}

/** the place of a path: an array item with a string `id` is `@<id>`, other items their index */
function stablePlace(data: unknown, path: Path): string {
	const out: string[] = [];
	let node: any = data;
	for (const segment of path) {
		const item = node !== null && typeof node === 'object' ? node[segment] : undefined;
		out.push(typeof segment === 'number' && item && typeof item === 'object' && typeof item.id === 'string' && item.id ? `@${item.id}` : String(segment));
		node = item;
	}
	return out.join('.');
}

/** FNV-1a, 32 bit, base 36 – a fingerprint, not a secret */
function fingerprint(text: string): string {
	let h = 0x811c9dc5;
	for (let i = 0; i < text.length; i++) {
		h ^= text.charCodeAt(i);
		h = Math.imul(h, 0x01000193);
	}
	return (h >>> 0).toString(36);
}

/** The warning an entry is about: its place and message (+ the locale of a language-neutral document's per-locale warning) */
export interface AcceptableWarning {
	path: Path;
	message: string;
	/**
	 * set only for a language-neutral document's warning about one locale ("missing en text"): kept in the key
	 * (`en:<place>#<hash>`). Per-locale documents leave it out (their keys stay as they were).
	 */
	scope?: string | null;
}

/** The entry for a warning about `data` (the document of the warning's locale, or the language-neutral document). A
 *  warning about the whole document (empty path) is keyed by its message only. */
export function acceptedWarningKey(warning: AcceptableWarning, data: unknown): string {
	const value = warning.path.length ? JSON.stringify(getAt(data, warning.path) ?? null) : '';
	const key = `${stablePlace(data, warning.path)}#${fingerprint(`${warning.message}\u0000${value}`)}`;
	return warning.scope ? `${warning.scope}:${key}` : key;
}

export function acceptedWarningsOf(data: unknown): string[] {
	const list = data !== null && typeof data === 'object' ? (data as AcceptedWarnings).accepted_warnings : undefined;
	return Array.isArray(list) ? list.filter((k) => typeof k === 'string') : [];
}

export function isAcceptedWarning(warning: AcceptableWarning, data: unknown): boolean {
	return acceptedWarningsOf(data).includes(acceptedWarningKey(warning, data));
}

/** The list with the warning accepted (`on`) or not; undefined when it ends up empty (the field is left out). */
export function withAcceptedWarning(warning: AcceptableWarning, data: unknown, on: boolean): string[] | undefined {
	const key = acceptedWarningKey(warning, data);
	const list = acceptedWarningsOf(data).filter((k) => k !== key);
	if (on) list.push(key);
	return list.length ? list : undefined;
}

/** The catalog kinds a catalog warning (validateCatalog) is stored in (admin → Catalog, agents/CATALOG.md) */
export type CatalogWarningKind = 'items' | 'recipes' | 'albums' | 'meta';

/** Where a catalog warning is accepted: the catalog entry it is about and the path inside that entry */
export interface CatalogWarningPlace {
	kind: CatalogWarningKind;
	id: string;
	path: Path;
}

/** the meta entry that holds what is accepted about a whole list (the file `items` / `albums` came from) */
export const CATALOG_LIST_META = 'items';

/**
 * Where a warning of `validateCatalog` (a path into `{ items, recipes, albums }`) is accepted (admin, owner 2026-10-04):
 * one about an entry (`items.3.category`) in that entry – kind + id, the rest of the path – so it is versioned with
 * the entry (history, undo); one about a whole list (`items`: "no player plays …") in the catalog's `meta` entry
 * `items`, keyed by its message. null = not about a stored entry (stats are code, an entry without an id).
 */
export function catalogWarningPlace(problem: { path: Path }, catalog: unknown): CatalogWarningPlace | null {
	const [list, index, ...rest] = problem.path;
	if (list !== 'items' && list !== 'recipes' && list !== 'albums') return null;
	if (index === undefined) return { kind: 'meta', id: CATALOG_LIST_META, path: [] };
	const entries = catalog !== null && typeof catalog === 'object' ? (catalog as Record<string, unknown>)[list] : undefined;
	const entry = typeof index === 'number' && Array.isArray(entries) ? entries[index] : undefined;
	const id = entry !== null && typeof entry === 'object' ? (entry as { id?: unknown }).id : undefined;
	return typeof id === 'string' && id ? { kind: list, id, path: rest } : null;
}
