// The XR workspace's selection (#482): what the Tree lists and the Inspector shows of an experience's configuration – the
// scene's surfaces, its entries, each entry's target and the target's entity as its node tree (OSCT's editor:
// editor/src/lib/tree-rows.ts, selection.ts). Read from the configuration as stored; pure.

type Obj = Record<string, unknown>;
const obj = (v: unknown): Obj | null => (v && typeof v === 'object' && !Array.isArray(v) ? (v as Obj) : null);
const list = (v: unknown): unknown[] => (Array.isArray(v) ? v : []);
const str = (v: unknown): string => (typeof v === 'string' ? v : '');

/** a node of a target's entity (`path` from the root, `[]` = the root) or the target's printed image (`path` null) */
export interface NodeRef {
	target: string;
	path: number[] | null;
}

export const sameRef = (a: NodeRef | null, b: NodeRef | null): boolean =>
	!!a && !!b && a.target === b.target && (a.path === null ? b.path === null : b.path !== null && a.path.length === b.path.length && a.path.every((v, i) => v === b.path![i]));

/** a target's entity: its own, or the shared entity it refers to (`{ ref }`) */
export function entityOf(configuration: unknown, target: unknown): Obj | null {
	const entity = obj(obj(target)?.entity);
	if (!entity) return null;
	const ref = str(entity.ref);
	if (!ref) return entity;
	const shared = obj(obj(configuration)?.entities);
	return obj(shared?.[ref]);
}

/** the node at `path` in an entity's tree (null: no such node) */
export function nodeAtPath(entity: Obj | null, path: readonly number[]): Obj | null {
	let at: Obj | null = entity;
	for (const i of path) at = obj(list(at?.children)[i]);
	return at;
}

/** a node's name, else its type */
export const nodeName = (node: Obj): string => str(node.name) || str(node.type) || '?';

export type TreeRow =
	| { kind: 'surface'; id: string; label: string; depth: 0 }
	| { kind: 'entry'; id: string; label: string; depth: number }
	| { kind: 'target'; ref: NodeRef; label: string; depth: number; shared: string | null }
	| { kind: 'node'; ref: NodeRef; label: string; type: string; depth: number; children: number };

/**
 * The open scene as rows: its surfaces, each with its entries (an entry without a surface sits on the scene's only one,
 * else after them), each entry's target, the target's entity as its node tree – folded nodes (by `targetId:path`)
 * hide their children.
 */
export function sceneRows(configuration: unknown, scene: string, folded: ReadonlySet<string> = new Set()): TreeRow[] {
	const c = obj(configuration) ?? {};
	const s = list(c.scenes).map(obj).find((x) => str(x?.id) === scene);
	if (!s) return [];
	const surfaces = list(s.surfaces).map(obj).filter((x): x is Obj => !!x);
	const only = surfaces.length === 1 ? str(surfaces[0].id) : '';
	const entries = list(c.entries).map(obj).filter((e): e is Obj => !!e && str(e.scene) === scene);
	const rows: TreeRow[] = [];
	const entryRows = (entry: Obj, depth: number) => {
		rows.push({ kind: 'entry', id: str(entry.id), label: str(entry.title) || str(entry.id), depth });
		const target = obj(entry.target);
		const id = str(target?.id);
		if (!target || !id) return;
		const ref = str(obj(target.entity)?.ref) || null;
		rows.push({ kind: 'target', ref: { target: id, path: null }, label: id, depth: depth + 1, shared: ref });
		const walk = (node: Obj, path: number[], d: number) => {
			const children = list(node.children).map(obj).filter((x): x is Obj => !!x);
			rows.push({ kind: 'node', ref: { target: id, path }, label: nodeName(node), type: str(node.type), depth: d, children: children.length });
			if (folded.has(`${id}:${path.join('.')}`)) return;
			children.forEach((child, i) => walk(child, [...path, i], d + 1));
		};
		const entity = entityOf(c, target);
		if (entity) walk(entity, [], depth + 2);
	};
	const placed = new Set<Obj>();
	for (const x of surfaces) {
		rows.push({ kind: 'surface', id: str(x.id), label: str(x.label) || str(x.id), depth: 0 });
		for (const e of entries.filter((e) => (str(e.surface) || only) === str(x.id))) {
			placed.add(e);
			entryRows(e, 1);
		}
	}
	for (const e of entries.filter((e) => !placed.has(e))) entryRows(e, 0);
	return rows;
}

export interface Field {
	label: string;
	value: string;
}
export interface Group {
	label: string;
	fields: Field[];
}

const show = (v: unknown): string => {
	if (v === null || v === undefined) return '';
	if (typeof v === 'number') return String(Math.round(v * 10000) / 10000);
	if (Array.isArray(v) && v.every((x) => typeof x === 'number')) return v.map(show).join('  ');
	if (typeof v === 'object') return JSON.stringify(v);
	return String(v);
};
const fieldsOf = (o: Obj | null, skip: readonly string[] = []): Field[] =>
	Object.entries(o ?? {})
		.filter(([k]) => !skip.includes(k))
		.map(([label, value]) => ({ label, value: show(value) }));

/**
 * What the Inspector shows of a selection, as it is stored: a node's placement, its files, a light's values, each
 * component with its parameters; a target's place and picture with its entry's facts. Empty: nothing found.
 */
export function inspection(configuration: unknown, ref: NodeRef | null): { title: string; kind: string; groups: Group[] } | null {
	if (!ref) return null;
	const c = obj(configuration) ?? {};
	const entry = list(c.entries).map(obj).find((e) => str(obj(e?.target)?.id) === ref.target);
	const target = obj(entry?.target);
	if (!entry || !target) return null;
	if (ref.path === null) {
		const groups: Group[] = [
			{ label: 'target', fields: [{ label: 'id', value: ref.target }, { label: 'imageSrc', value: str(target.imageSrc) }, ...fieldsOf(obj(target.place)), ...(str(obj(target.entity)?.ref) ? [{ label: 'entity', value: str(obj(target.entity)?.ref) }] : [])] },
			{ label: 'entry', fields: fieldsOf(entry, ['target']) }
		];
		return { title: ref.target, kind: 'target', groups };
	}
	const node = nodeAtPath(entityOf(c, target), ref.path);
	if (!node) return null;
	const groups: Group[] = [{ label: 'node', fields: [{ label: 'type', value: str(node.type) }, { label: 'name', value: str(node.name) }] }];
	const params = obj(node.params);
	if (params) groups.push({ label: 'placement', fields: fieldsOf(params) });
	const files = list(node.assets).map(obj).filter((a): a is Obj => !!a);
	if (files.length) groups.push({ label: 'files', fields: files.map((a) => ({ label: str(a.assetType) || str(a.id), value: str(a.src) })) });
	if (obj(node.light)) groups.push({ label: 'light', fields: fieldsOf(obj(node.light)) });
	for (const component of list(node.components).map(obj).filter((x): x is Obj => !!x)) groups.push({ label: str(component.type) || 'component', fields: fieldsOf(component, ['type']) });
	const other = fieldsOf(node, ['type', 'name', 'params', 'assets', 'light', 'components', 'children']);
	if (other.length) groups.push({ label: 'more', fields: other });
	return { title: nodeName(node), kind: str(node.type), groups };
}
