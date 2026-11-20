// The Library's scope, filters and order (one panel, pre-filtered by the workspace, widened to the toon, all or the
// unused): which families a scope holds, the counts on its pills and kind chips, the
// search, the sort, the browser's places (to do, the folder tree, where from) and what a picker's slot takes.
// Pure: test/library-filter.test.ts.
import { LIBRARY_KINDS, type AssetFamily, type LibraryKind } from './families.ts';

export type LibraryScope = 'context' | 'toon' | 'all' | 'unused';

/** what the front workspace narrows the Library to */
export interface LibraryContext {
	/** what the pill says: chapter, character, items … */
	kind: 'toon' | 'season' | 'chapter' | 'character' | 'characters' | 'items' | 'map';
	/** the item's own name for the pill's title (`catpope:3`) */
	id: string;
	/** a family is in the context when its key starts like this (a regex source, tested on the key) */
	pattern: string;
	/** where an upload goes without a slot of its own (CDN-root folder, ends with `/`) – null: the person picks a folder */
	folder: string | null;
}

const esc = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

/**
 * The context of the open item (its `kind:id` address, the shell's): a chapter's or a storyboard's files are in
 * `<toon>/s<n>/c<number>/`, a season's in `<toon>/s<n>/`, a toon's in `<toon>/`, a character's in `characters/<id>` and its
 * persona folders `characters/<id>-<persona>`, catalog items in `<…>/items/`, the map in `<…>/map/`. Home and the rest: none.
 */
export function contextOf(address: string | null | undefined): LibraryContext | null {
	if (!address) return null;
	const [kind, ...rest] = address.split(':');
	const id = rest.join(':');
	switch (kind) {
		case 'toon':
			return id ? { kind: 'toon', id, pattern: `^${esc(id)}/`, folder: `${id}/` } : null;
		case 'season': {
			const [toon, n] = id.split(':');
			return toon && /^\d+$/.test(n ?? '') ? { kind: 'season', id, pattern: `^${esc(toon)}/s${n}/`, folder: `${toon}/s${n}/` } : null;
		}
		case 'chapter':
		case 'storyboard': {
			const [toon, n] = id.split(':');
			return toon && /^\d+$/.test(n ?? '') ? { kind: 'chapter', id, pattern: `^${esc(toon)}/s\\d+/c${n}/`, folder: null } : null;
		}
		case 'character':
			return id ? { kind: 'character', id, pattern: `^characters/${esc(id)}[/-]`, folder: `characters/${id}/` } : null;
		case 'characters':
			return { kind: 'characters', id, pattern: '^characters/', folder: null };
		case 'catalog':
		case 'item':
			return { kind: 'items', id, pattern: '^[^/]+/items/', folder: null };
		case 'map':
		case 'place':
		case 'region':
		case 'route':
			return { kind: 'map', id, pattern: '^[^/]+/map/', folder: null };
	}
	return null;
}

/** the toon an address belongs to (`chapter:catpope:3` → catpope), null for the universe's areas */
export function toonOfAddress(address: string | null | undefined): string | null {
	if (!address) return null;
	const [kind, id = ''] = address.split(':');
	return ['toon', 'season', 'chapter', 'storyboard'].includes(kind) && id ? id.split(':')[0] || null : null;
}

export function inContext(family: Pick<AssetFamily, 'key'>, context: LibraryContext | null): boolean {
	return !context || new RegExp(context.pattern).test(family.key);
}

export function inScope(family: AssetFamily, scope: LibraryScope, context: LibraryContext | null, toon: string | null): boolean {
	switch (scope) {
		case 'context':
			return inContext(family, context);
		case 'toon':
			return !toon || family.key.startsWith(`${toon}/`);
		case 'unused':
			return family.usages === 0;
		default:
			return true;
	}
}

export function scopeCounts(families: readonly AssetFamily[], context: LibraryContext | null, toon: string | null): Record<LibraryScope, number> {
	const counts: Record<LibraryScope, number> = { context: 0, toon: 0, all: families.length, unused: 0 };
	const pattern = context ? new RegExp(context.pattern) : null;
	for (const f of families) {
		if (!pattern || pattern.test(f.key)) counts.context++;
		if (!toon || f.key.startsWith(`${toon}/`)) counts.toon++;
		if (f.usages === 0) counts.unused++;
	}
	return counts;
}

export function kindCounts(families: readonly AssetFamily[]): Record<LibraryKind, number> {
	const counts = Object.fromEntries(LIBRARY_KINDS.map((k) => [k, 0])) as Record<LibraryKind, number>;
	for (const f of families) counts[f.kind]++;
	return counts;
}

/** every word in the key, the title, a tag or a version's path */
export function matchesQuery(family: AssetFamily, query: string): boolean {
	const words = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
	if (!words.length) return true;
	const hay = `${family.key} ${family.title} ${family.tags.join(' ')} ${family.versions.map((v) => v.label).join(' ')}`.toLowerCase();
	return words.every((w) => hay.includes(w));
}

/** the browser's to-do places: versions a family should have and lacks; nothing uses it */
export type LibraryTodo = 'missing' | 'unused';
/** where the source came from */
export type LibraryOrigin = AssetFamily['origin'];

export interface LibraryFilter {
	scope: LibraryScope;
	context: LibraryContext | null;
	toon: string | null;
	kind: LibraryKind | 'all';
	query?: string;
	todo?: LibraryTodo | null;
	/** a folder of the tree (a CDN-root prefix without the trailing `/`) */
	folder?: string | null;
	origin?: LibraryOrigin | null;
}

/** the families before the kind chips: scope, search, the browser's places – the chips count these */
export function beforeKinds(families: readonly AssetFamily[], f: Omit<LibraryFilter, 'kind'>): AssetFamily[] {
	return families.filter(
		(x) =>
			inScope(x, f.scope, f.context, f.toon) &&
			(!f.query || matchesQuery(x, f.query)) &&
			(!f.todo || (f.todo === 'missing' ? x.missing.length > 0 : x.usages === 0)) &&
			(!f.folder || x.key.startsWith(`${f.folder}/`)) &&
			(!f.origin || x.origin === f.origin)
	);
}

export function filterFamilies(families: readonly AssetFamily[], f: LibraryFilter): AssetFamily[] {
	const scoped = beforeKinds(families, f);
	return f.kind === 'all' ? scoped : scoped.filter((x) => x.kind === f.kind);
}

export type LibrarySort = 'name' | 'updated' | 'uses' | 'size';
export const LIBRARY_SORTS: readonly LibrarySort[] = ['name', 'updated', 'uses', 'size'];

/** natural order by name (slice 2 before slice 10); the newest, the most used and the biggest first */
export function sortFamilies(families: readonly AssetFamily[], by: LibrarySort): AssetFamily[] {
	const name = (a: AssetFamily, b: AssetFamily) => a.key.localeCompare(b.key, undefined, { numeric: true });
	const list = [...families];
	switch (by) {
		case 'updated':
			return list.sort((a, b) => (b.updated ?? '').localeCompare(a.updated ?? '') || name(a, b));
		case 'uses':
			return list.sort((a, b) => b.usages - a.usages || name(a, b));
		case 'size':
			return list.sort((a, b) => b.size - a.size || name(a, b));
		default:
			return list.sort(name);
	}
}

/** the browser's grid: one group per kind, in the kinds' order, empty ones left out */
export function groupByKind(families: readonly AssetFamily[]): { kind: LibraryKind; items: AssetFamily[] }[] {
	return LIBRARY_KINDS.map((kind) => ({ kind, items: families.filter((f) => f.kind === kind) })).filter((g) => g.items.length > 0);
}

export interface FolderNode {
	/** CDN-root prefix without the trailing `/` */
	path: string;
	name: string;
	count: number;
	children: FolderNode[];
}

/** the folders of the families as a tree (counts include the subfolders), `root` = only below it */
export function folderTree(families: readonly AssetFamily[], root = ''): FolderNode[] {
	const top: FolderNode = { path: root, name: root, count: 0, children: [] };
	const prefix = root ? `${root}/` : '';
	for (const f of families) {
		if (prefix && !f.folder.startsWith(prefix) && f.folder !== root) continue;
		const rest = f.folder === root ? '' : f.folder.slice(prefix.length);
		top.count++;
		let node = top;
		for (const part of rest ? rest.split('/') : []) {
			const path = node.path ? `${node.path}/${part}` : part;
			let child = node.children.find((c) => c.name === part);
			if (!child) node.children.push((child = { path, name: part, count: 0, children: [] }));
			child.count++;
			node = child;
		}
	}
	const sort = (nodes: FolderNode[]) => {
		nodes.sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true }));
		for (const n of nodes) sort(n.children);
	};
	sort(top.children);
	return top.children;
}

export function todoCounts(families: readonly AssetFamily[]): Record<LibraryTodo, number> {
	return { missing: families.filter((f) => f.missing.length > 0).length, unused: families.filter((f) => f.usages === 0).length };
}

export function originCounts(families: readonly AssetFamily[]): Record<LibraryOrigin, number> {
	const counts: Record<LibraryOrigin, number> = { uploaded: 0, imported: 0, generated: 0 };
	for (const f of families) counts[f.origin]++;
	return counts;
}

/** what a blank slot takes (a picker): its kinds, a square image (a mood portrait) */
export interface SlotFit {
	kinds: readonly LibraryKind[];
	square?: boolean;
}

/** does the family fit the slot? A square slot takes a picture whose sides differ by at most 2 % (unknown sizes fit) */
export function fits(family: AssetFamily, slot: SlotFit | null): boolean {
	if (!slot) return true;
	if (!slot.kinds.includes(family.kind)) return false;
	if (slot.square && family.width && family.height) return Math.abs(family.width - family.height) <= Math.max(family.width, family.height) * 0.02;
	return true;
}
