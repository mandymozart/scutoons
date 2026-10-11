/**
 * Publishing part of a document (owner 2026-10-11/12: "die Orte sind Unterebenen der Karte" – the same for the
 * characters): a document that holds lists of entries with an `id` (the map: places, routes, layers; the characters
 * file: characters) is compared with its published version entry by entry; each change is a line of the publish
 * checklist, and the ticked ones are put on top of the published version – the rest stays a draft.
 * One document stays one document (the map is not split into files): the creator saves "published + the ticked changes"
 * as a version, publishes it, and saves the whole draft again on top (creator/src/lib/editor/publish-tree.ts).
 */

/** which lists of a document are parts (by stored key) */
export const DOCUMENT_PARTS: Record<string, readonly string[]> = {
	map: ['locations', 'routes', 'layers'],
	characters: ['characters']
};

export type PartChangeKind = 'added' | 'changed' | 'removed' | 'settings';

export interface PartChange {
	/** a stable line id: `<field>:<id>`, or `settings` for everything outside the lists */
	line: string;
	field: string | null;
	id: string | null;
	kind: PartChangeKind;
	/** the entry's name (a locale map or a string – the caller picks the language) */
	name: unknown;
}

type Doc = Record<string, unknown>;
type Entry = { id?: unknown; name?: unknown } & Record<string, unknown>;

const obj = (v: unknown): Doc => (v && typeof v === 'object' && !Array.isArray(v) ? (v as Doc) : {});
const list = (v: unknown): Entry[] => (Array.isArray(v) ? (v as Entry[]).filter((e) => e && typeof e === 'object' && typeof e.id === 'string') : []);
const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);
const outside = (doc: Doc, fields: readonly string[]) => Object.fromEntries(Object.entries(doc).filter(([k]) => !fields.includes(k)));

/** the changes from `published` to `draft` (empty = no difference); `fields` = the document's part lists */
export function diffParts(published: unknown, draft: unknown, fields: readonly string[]): PartChange[] {
	const pub = obj(published);
	const dra = obj(draft);
	const out: PartChange[] = [];
	for (const field of fields) {
		const before = new Map(list(pub[field]).map((e) => [e.id as string, e]));
		const after = new Map(list(dra[field]).map((e) => [e.id as string, e]));
		for (const [id, e] of after) {
			const old = before.get(id);
			if (!old) out.push({ line: `${field}:${id}`, field, id, kind: 'added', name: e.name });
			else if (!same(old, e)) out.push({ line: `${field}:${id}`, field, id, kind: 'changed', name: e.name });
		}
		for (const [id, e] of before) if (!after.has(id)) out.push({ line: `${field}:${id}`, field, id, kind: 'removed', name: e.name });
	}
	if (!same(outside(pub, fields), outside(dra, fields))) out.push({ line: 'settings', field: null, id: null, kind: 'settings', name: null });
	return out;
}

/**
 * The published version with only the ticked changes of the draft (`lines` = PartChange.line). The lists keep the
 * draft's order; an entry the draft removed but the reader keeps stays where it was.
 */
export function applyParts(published: unknown, draft: unknown, fields: readonly string[], lines: ReadonlySet<string>): Doc {
	const pub = obj(published);
	const dra = obj(draft);
	const out: Doc = { ...(lines.has('settings') ? outside(dra, fields) : outside(pub, fields)) };
	for (const field of fields) {
		if (!(field in pub) && !(field in dra)) continue;
		const before = list(pub[field]);
		const beforeById = new Map(before.map((e) => [e.id as string, e]));
		const after = list(dra[field]);
		const afterIds = new Set(after.map((e) => e.id as string));
		const take = (id: string) => lines.has(`${field}:${id}`);
		// the draft's order: its entries – changed / added ones only when ticked
		const result: Entry[] = [];
		for (const e of after) {
			const id = e.id as string;
			const old = beforeById.get(id);
			if (old) result.push(take(id) ? e : old);
			else if (take(id)) result.push(e);
		}
		// removed in the draft but not ticked: they stay, near where they were
		before.forEach((e, i) => {
			const id = e.id as string;
			if (afterIds.has(id) || take(id)) return;
			const prev = before.slice(0, i).reverse().find((p) => result.some((r) => r.id === p.id));
			const at = prev ? result.findIndex((r) => r.id === prev.id) + 1 : 0;
			result.splice(at, 0, e);
		});
		out[field] = result;
	}
	return out;
}
