// A document made of many documents (DRAFT, agents/MAP-DOCUMENTS.md – owner 2026-10-12: "alle einen eigenen Doc, den man
// versionieren kann; dann ist das gleichmäßig refactored"): a COLLECTION is one anchor document (its own fields – the
// "settings") plus one document per entry of its lists, each versioned on its own (drafts, publish, history, restore –
// entity_versions). Everything that reads it still works on ONE composed document in the old shape: composed here (TS)
// and in `Domain\Content\CollectionDocs` (PHP – keep both in step), split again for saving. Pure: no Svelte, no DOM.
//
//   the world map        map         + place:<id> (→ locations) + route:<id> (→ routes)        map-docs.ts
//   the characters       characters  + character:<id> (→ characters)                            character-docs.ts
//
// An entry's document is the entry without its `id` (the key holds it). Removing an entry: a never-published one is
// deleted (owner D3); a published one gets a last version with `removed: true` (a tombstone: the id stays taken – events,
// casts and readers' save games reference it; publishing it takes the entry off readers'; restoring brings it back).
// Order in a list = creation order of the documents (the first version's id): adding an entry touches no other document.
import { acceptedWarningsOf, ACCEPTED_WARNINGS_FIELD } from './accepted.ts';
import { parseEntityKey, type EntityKind } from './entities.ts';

/** Which documents a collection is made of */
export interface CollectionSpec {
	/** the anchor document's key (and kind): its fields are the collection's own settings */
	key: EntityKind;
	/** each part kind → the list of the composed document its entries form */
	parts: readonly { kind: EntityKind; field: string }[];
}

/** One entry's document (`<kind>:<id>`), in creation order */
export interface CollectionPart<D = Record<string, any>> {
	id: string;
	doc: D;
}

/** A collection's documents, as stored (tombstones included) */
export interface CollectionDocs {
	settings: Record<string, any> | null;
	/** part kind → its entries' documents */
	parts: Record<string, CollectionPart[]>;
}

export type CollectionChangeKind = 'added' | 'changed' | 'removed' | 'restored' | 'settings';

export interface CollectionChange {
	key: string;
	kind: CollectionChangeKind;
	/** what to store */
	doc: Record<string, any>;
}

/** A problem of the composed document, addressed to the stored document it is in */
export interface CollectionDocProblem {
	level: 'error' | 'warning' | 'info';
	path: (string | number)[];
	message: string;
	/** the stored key the problem is in */
	key: string;
	/** the path inside that document */
	docPath: (string | number)[];
}

const isObj = (v: unknown): v is Record<string, any> => !!v && typeof v === 'object' && !Array.isArray(v);
const partKey = (kind: string, id: string) => `${kind}:${id}`;

/** every kind a collection is stored as (the anchor first) – GET /entities?kinds= */
export const collectionKinds = (spec: CollectionSpec): string[] => [spec.key, ...spec.parts.map((p) => p.kind)];

/** is a stored key one of the collection's documents? */
export function isCollectionKey(spec: CollectionSpec, key: string): boolean {
	const kind = parseEntityKey(key)?.kind;
	return !!kind && collectionKinds(spec).includes(kind);
}

export const emptyCollectionDocs = (spec: CollectionSpec): CollectionDocs => ({ settings: null, parts: Object.fromEntries(spec.parts.map((p) => [p.kind, []])) });

// ── accepted warnings: an entry's own keys are relative to it; the composed document's are `<field>.@<id>…` ─────────────

/** `en:locations.@cathedral.hidden#x1` → scope `en:`, rest */
function splitScope(key: string): [string, string] {
	const m = /^([a-z]{2}:)(.*)$/.exec(key);
	return m ? [m[1], m[2]] : ['', key];
}

/** an entry's accepted key → the composed document's (`hidden#x1` → `locations.@cathedral.hidden#x1`; `#x1` → `locations.@cathedral#x1`) */
function absoluteAccepted(field: string, id: string, key: string): string {
	const [scope, rest] = splitScope(key);
	return `${scope}${field}.@${id}${rest.startsWith('#') ? '' : '.'}${rest}`;
}

/** the composed document's accepted key → the entry it belongs to and its own key, null for a key of the settings */
function relativeAccepted(spec: CollectionSpec, key: string): { field: string; id: string; key: string } | null {
	const [scope, rest] = splitScope(key);
	const m = /^([a-z_]+)\.@([a-z0-9][a-z0-9_-]{0,63})(?:\.(.*)|(#.*))$/.exec(rest);
	if (!m || !spec.parts.some((p) => p.field === m[1])) return null;
	return { field: m[1], id: m[2], key: `${scope}${m[3] ?? m[4]}` };
}

// ── compose / split ─────────────────────────────────────────────────────────────────────────────────────────────

const withoutMeta = (doc: Record<string, any>) => {
	const { removed: _removed, [ACCEPTED_WARNINGS_FIELD]: _accepted, id: _id, ...rest } = doc;
	return rest;
};

/**
 * The one composed document: the settings' fields, then per part kind its list of entries that are not removed (in the
 * given order – creation order). The entries' accepted warnings move into the document's list with their place in it.
 */
export function composeCollection(spec: CollectionSpec, docs: CollectionDocs): Record<string, any> {
	const s = isObj(docs.settings) ? docs.settings : {};
	const accepted = acceptedWarningsOf(s);
	const out: Record<string, any> = withoutMeta(s);
	for (const { kind, field } of spec.parts) {
		out[field] = (docs.parts[kind] ?? [])
			.filter((p) => isObj(p.doc) && !p.doc.removed)
			.map((p) => {
				for (const k of acceptedWarningsOf(p.doc)) accepted.push(absoluteAccepted(field, p.id, k));
				return { id: p.id, ...withoutMeta(p.doc) };
			});
	}
	if (accepted.length) out[ACCEPTED_WARNINGS_FIELD] = accepted;
	return out;
}

/** The documents of one composed document (no tombstones: an entry that is not in it is simply not there) */
export function splitCollection(spec: CollectionSpec, composed: Record<string, any>): { settings: Record<string, any>; parts: Record<string, CollectionPart[]> } {
	const entryAccepted = new Map<string, string[]>();
	const settingsAccepted: string[] = [];
	for (const key of acceptedWarningsOf(composed)) {
		const rel = relativeAccepted(spec, key);
		if (!rel) settingsAccepted.push(key);
		else entryAccepted.set(`${rel.field}:${rel.id}`, [...(entryAccepted.get(`${rel.field}:${rel.id}`) ?? []), rel.key]);
	}
	const fields = new Set(spec.parts.map((p) => p.field));
	const settings: Record<string, any> = Object.fromEntries(Object.entries(composed).filter(([k]) => !fields.has(k) && k !== ACCEPTED_WARNINGS_FIELD));
	if (settingsAccepted.length) settings[ACCEPTED_WARNINGS_FIELD] = settingsAccepted;
	const parts: Record<string, CollectionPart[]> = {};
	for (const { kind, field } of spec.parts) {
		parts[kind] = (Array.isArray(composed[field]) ? composed[field] : [])
			.filter((e: unknown) => isObj(e) && typeof e.id === 'string')
			.map((e: Record<string, any>) => {
				const { id, ...rest } = e;
				const accepted = entryAccepted.get(`${field}:${id}`);
				return { id, doc: accepted?.length ? { ...rest, [ACCEPTED_WARNINGS_FIELD]: accepted } : rest };
			});
	}
	return { settings, parts };
}

/** A key → document record of a composed document (the documents to store) */
export function collectionDocsOf(spec: CollectionSpec, composed: Record<string, any>): Record<string, Record<string, any>> {
	const { settings, parts } = splitCollection(spec, composed);
	return { [spec.key]: settings, ...Object.fromEntries(spec.parts.flatMap(({ kind }) => (parts[kind] ?? []).map((p) => [partKey(kind, p.id), p.doc]))) };
}

/**
 * The documents of a list of stored entities in creation order (GET /entities?kinds=… – the creator, the MCP server);
 * other keys and entities without a document are left out. `replace` swaps one key's document (a draft to check before
 * it is saved; null takes it out).
 */
export function collectionDocsFromEntities(spec: CollectionSpec, list: readonly { key: string; content: unknown }[], replace?: { key: string; doc: unknown }): CollectionDocs {
	const docs = emptyCollectionDocs(spec);
	const entries = list.map((e) => ({ key: e.key, content: replace && e.key === replace.key ? replace.doc : e.content }));
	if (replace && !list.some((e) => e.key === replace.key)) entries.push({ key: replace.key, content: replace.doc });
	for (const e of entries) {
		if (!isObj(e.content)) continue;
		const ref = parseEntityKey(e.key);
		if (!ref) continue;
		if (ref.kind === spec.key && !ref.id) docs.settings = e.content;
		else if (ref.id && docs.parts[ref.kind]) docs.parts[ref.kind].push({ id: ref.id, doc: e.content });
	}
	return docs;
}

/** JSON with sorted object keys: two documents are the same when this is (the editors don't keep key order) */
export function stableJson(value: unknown): string {
	return JSON.stringify(value, (_k, v) => (isObj(v) ? Object.fromEntries(Object.keys(v).sort().map((k) => [k, v[k]])) : v));
}

/**
 * What to store so that the documents become `next`: the settings when they changed, every added / changed entry, a
 * tombstone for every one that left (its last document + `removed: true` – the caller deletes it instead when it was
 * never published), a removed one that is back. `stored` = the documents as they are now (tombstones included).
 */
export function collectionChanges(spec: CollectionSpec, stored: CollectionDocs, next: Record<string, any>): CollectionChange[] {
	const out: CollectionChange[] = [];
	const want = splitCollection(spec, next);
	if (!stored.settings || stableJson(stored.settings) !== stableJson(want.settings)) out.push({ key: spec.key, kind: 'settings', doc: want.settings });
	for (const { kind } of spec.parts) {
		const before = stored.parts[kind] ?? [];
		const after = want.parts[kind] ?? [];
		const was = new Map(before.map((p) => [p.id, p.doc]));
		const now = new Set(after.map((p) => p.id));
		for (const p of after) {
			const old = was.get(p.id);
			if (!old) out.push({ key: partKey(kind, p.id), kind: 'added', doc: p.doc });
			else if (old.removed) out.push({ key: partKey(kind, p.id), kind: 'restored', doc: p.doc });
			else if (stableJson(old) !== stableJson(p.doc)) out.push({ key: partKey(kind, p.id), kind: 'changed', doc: p.doc });
		}
		for (const p of before) if (!now.has(p.id) && !p.doc.removed) out.push({ key: partKey(kind, p.id), kind: 'removed', doc: { ...p.doc, removed: true } });
	}
	return out;
}

/** The documents after storing `changes` and deleting `deleted` keys (the saved state of an editor) */
export function applyCollectionChanges(spec: CollectionSpec, stored: CollectionDocs, changes: readonly CollectionChange[], deleted: readonly string[] = []): CollectionDocs {
	const out: CollectionDocs = { settings: stored.settings, parts: Object.fromEntries(Object.entries(stored.parts).map(([k, list]) => [k, [...list]])) };
	for (const c of changes) {
		const ref = parseEntityKey(c.key);
		if (!ref) continue;
		if (ref.kind === spec.key && !ref.id) out.settings = c.doc;
		else if (ref.id && out.parts[ref.kind]) {
			const list = out.parts[ref.kind];
			const i = list.findIndex((p) => p.id === ref.id);
			if (i < 0) list.push({ id: ref.id, doc: c.doc });
			else list[i] = { id: ref.id, doc: c.doc };
		}
	}
	for (const key of deleted) {
		const ref = parseEntityKey(key);
		if (ref?.id && out.parts[ref.kind]) out.parts[ref.kind] = out.parts[ref.kind].filter((p) => p.id !== ref.id);
	}
	return out;
}

/** Every id stored for a part kind (tombstones included): a new entry must not take one of them */
export const takenCollectionIds = (docs: CollectionDocs, kind: string): string[] => (docs.parts[kind] ?? []).map((p) => p.id);

/** a composed document's path → the stored document and the path inside it (`['locations', 3, 'x']` → `place:<id>`, `['x']`) */
export function collectionDocOfPath(spec: CollectionSpec, composed: Record<string, any>, path: readonly (string | number)[]): { key: string; path: (string | number)[] } {
	const [field, index, ...rest] = path;
	const part = spec.parts.find((p) => p.field === field);
	if (part && typeof index === 'number') {
		const id = composed[part.field]?.[index]?.id;
		if (typeof id === 'string') return { key: partKey(part.kind, id), path: rest };
	}
	return { key: spec.key, path: [...path] };
}

/** problems of the composed document (the collection's own check) addressed to their stored documents */
export function addressProblems<P extends { level: 'error' | 'warning' | 'info'; path: (string | number)[]; message: string }>(
	spec: CollectionSpec,
	composed: Record<string, any>,
	problems: readonly P[]
): (P & CollectionDocProblem)[] {
	return problems.map((p) => {
		const at = collectionDocOfPath(spec, composed, p.path);
		return { ...p, key: at.key, docPath: at.path };
	});
}
