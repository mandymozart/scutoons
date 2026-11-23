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

export type FieldKind = 'number' | 'text' | 'bool' | 'color' | 'readonly';

/** one row of the Inspector: `at` is the value's path inside the node (or the target / entry when read only) */
export interface Field {
	label: string;
	kind: FieldKind;
	value: string | number | boolean;
	at: (string | number)[];
}
export interface Group {
	label: string;
	fields: Field[];
}

const AXES = ['x', 'y', 'z', 'w'];
const COLOR = /^#[0-9a-f]{6}$/i;
const round = (v: number) => Math.round(v * 10000) / 10000;
const shown = (v: unknown): string => (v === null || v === undefined ? '' : typeof v === 'object' ? JSON.stringify(v) : String(v));

/** a value's rows: numbers, words, switches and colours can be edited; a list of numbers is one row per axis; anything else is shown */
function fieldsOf(o: Obj | null, base: (string | number)[], skip: readonly string[] = [], editable = true): Field[] {
	return Object.entries(o ?? {})
		.filter(([k]) => !skip.includes(k))
		.flatMap(([label, value]): Field[] => {
			const at = [...base, label];
			if (!editable) return [{ label, kind: 'readonly', value: shown(value), at }];
			if (typeof value === 'number') return [{ label, kind: 'number', value: round(value), at }];
			if (typeof value === 'boolean') return [{ label, kind: 'bool', value, at }];
			if (typeof value === 'string') return [{ label, kind: COLOR.test(value) ? 'color' : 'text', value, at }];
			if (Array.isArray(value) && value.length && value.length <= 4 && value.every((x) => typeof x === 'number'))
				return value.map((x: number, i) => ({ label: `${label} ${AXES[i]}`, kind: 'number' as const, value: round(x), at: [...at, i] }));
			return [{ label, kind: 'readonly', value: shown(value), at }];
		});
}

/**
 * What the Inspector shows of a selection, as it is stored: a node's name, placement, files, a light's values, each
 * component with its parameters (editable); a target's place on its surface (editable, mm), its picture and its entry
 * (read only).
 */
export function inspection(configuration: unknown, ref: NodeRef | null): { title: string; kind: string; groups: Group[] } | null {
	if (!ref) return null;
	const c = obj(configuration) ?? {};
	const entry = list(c.entries).map(obj).find((e) => str(obj(e?.target)?.id) === ref.target);
	const target = obj(entry?.target);
	if (!entry || !target) return null;
	if (ref.path === null) {
		const shared = str(obj(target.entity)?.ref);
		const groups: Group[] = [
			{
				label: 'target',
				fields: [
					{ label: 'id', kind: 'readonly', value: ref.target, at: ['id'] },
					{ label: 'imageSrc', kind: 'readonly', value: str(target.imageSrc), at: ['imageSrc'] },
					...fieldsOf(obj(target.place), ['place'], ['surface']),
					...(str(obj(target.place)?.surface) ? [{ label: 'surface', kind: 'readonly' as const, value: str(obj(target.place)?.surface), at: ['place', 'surface'] }] : []),
					...(shared ? [{ label: 'entity', kind: 'readonly' as const, value: shared, at: ['entity'] }] : [])
				]
			},
			{ label: 'entry', fields: fieldsOf(entry, [], ['target'], false) }
		];
		return { title: ref.target, kind: 'target', groups };
	}
	const node = nodeAtPath(entityOf(c, target), ref.path);
	if (!node) return null;
	const groups: Group[] = [
		{
			label: 'node',
			fields: [
				{ label: 'type', kind: 'readonly', value: str(node.type), at: ['type'] },
				{ label: 'name', kind: 'text', value: str(node.name), at: ['name'] }
			]
		}
	];
	if (obj(node.params)) groups.push({ label: 'placement', fields: fieldsOf(obj(node.params), ['params']) });
	const files = list(node.assets).map(obj).filter((a): a is Obj => !!a);
	if (files.length) groups.push({ label: 'files', fields: files.map((a, i) => ({ label: str(a.assetType) || str(a.id), kind: 'readonly' as const, value: str(a.src), at: ['assets', i, 'src'] })) });
	if (obj(node.light)) groups.push({ label: 'light', fields: fieldsOf(obj(node.light), ['light']) });
	list(node.components).forEach((component, i) => {
		const o = obj(component);
		if (o) groups.push({ label: str(o.type) || 'component', fields: fieldsOf(o, ['components', i], ['type']) });
	});
	const other = fieldsOf(node, [], ['type', 'name', 'params', 'assets', 'light', 'components', 'children'], false);
	if (other.length) groups.push({ label: 'more', fields: other });
	return { title: nodeName(node), kind: str(node.type), groups };
}

/**
 * Where a node lives in the experience's document: in the shared entity the target refers to (every target that shows
 * it changes with it) or in the target's own entity. Null: no such node.
 */
export function nodeDocPath(configuration: unknown, ref: NodeRef): (string | number)[] | null {
	if (ref.path === null) return null;
	const c = obj(configuration) ?? {};
	const entries = list(c.entries).map(obj);
	const index = entries.findIndex((e) => str(obj(e?.target)?.id) === ref.target);
	const target = obj(entries[index]?.target);
	if (index < 0 || !target || !nodeAtPath(entityOf(c, target), ref.path)) return null;
	const shared = str(obj(target.entity)?.ref);
	const root: (string | number)[] = shared ? ['configuration', 'entities', shared] : ['configuration', 'entries', index, 'target', 'entity'];
	return [...root, ...ref.path.flatMap((i) => ['children', i])];
}

/** where a target lives in the experience's document (its entry's `target`); null: no such target */
export function targetDocPath(configuration: unknown, target: string): (string | number)[] | null {
	const index = list(obj(configuration)?.entries).map(obj).findIndex((e) => str(obj(e?.target)?.id) === target);
	return index < 0 ? null : ['configuration', 'entries', index, 'target'];
}

/** a target's place as stored (null: it has none) */
export function targetPlace(configuration: unknown, target: string): { surface?: string; x: number; y: number; width: number; height?: number } | null {
	const t = obj(list(obj(configuration)?.entries).map(obj).find((e) => str(obj(e?.target)?.id) === target)?.target);
	const place = obj(t?.place);
	if (!place || typeof place.x !== 'number' || typeof place.y !== 'number' || typeof place.width !== 'number') return null;
	return { ...(str(place.surface) ? { surface: str(place.surface) } : {}), x: place.x, y: place.y, width: place.width, ...(typeof place.height === 'number' ? { height: place.height } : {}) };
}

/** the targets whose entity is the same tree as this one's (a shared entity: all that refer to it) */
export function targetsSharing(configuration: unknown, target: string): string[] {
	const c = obj(configuration) ?? {};
	const entries = list(c.entries).map(obj);
	const own = obj(entries.find((e) => str(obj(e?.target)?.id) === target)?.target);
	const shared = str(obj(own?.entity)?.ref);
	if (!shared) return own ? [target] : [];
	return entries.flatMap((e) => {
		const t = obj(e?.target);
		return t && str(obj(t.entity)?.ref) === shared ? [str(t.id)] : [];
	});
}

/**
 * Where a target's whole entity lives in the document and what it is now: the shared entity it refers to (`shared`: its
 * name; every target that shows it changes with it) or its own (`at` also when it has none yet). Null: no such target.
 * `own` is the target's own `entity` field – removing the content clears it (a shared entity stays for the others).
 */
export function entitySlot(configuration: unknown, target: string): { at: (string | number)[]; own: (string | number)[]; tree: Obj | null; shared: string | null } | null {
	const c = obj(configuration) ?? {};
	const entries = list(c.entries).map(obj);
	const index = entries.findIndex((e) => str(obj(e?.target)?.id) === target);
	const t = obj(entries[index]?.target);
	if (index < 0 || !t) return null;
	const own: (string | number)[] = ['configuration', 'entries', index, 'target', 'entity'];
	const shared = str(obj(t.entity)?.ref) || null;
	return { at: shared ? ['configuration', 'entities', shared] : own, own, tree: entityOf(c, t), shared };
}

export interface XrFile {
	id: string;
	assetType: 'glb' | 'gltf' | 'video' | 'image' | 'audio';
	src: string;
}
const FILE_TYPES: readonly string[] = ['glb', 'gltf', 'video', 'image', 'audio'];

/**
 * The files the experience's entities use, then its targets' pictures, once each by `src` (in the order met): what a new
 * part can take until files can be added to the experience.
 */
export function experienceFiles(configuration: unknown): XrFile[] {
	const c = obj(configuration) ?? {};
	const seen = new Map<string, XrFile>();
	const walk = (node: Obj | null) => {
		if (!node) return;
		for (const a of list(node.assets).map(obj)) {
			const src = str(a?.src);
			const assetType = str(a?.assetType);
			if (src && FILE_TYPES.includes(assetType) && !seen.has(src)) seen.set(src, { id: str(a?.id) || src, assetType: assetType as XrFile['assetType'], src });
		}
		list(node.children).forEach((child) => walk(obj(child)));
	};
	for (const e of list(c.entries).map(obj)) {
		const entity = obj(obj(e?.target)?.entity);
		if (entity && !str(entity.ref)) walk(entity);
	}
	Object.values(obj(c.entities) ?? {}).forEach((entity) => walk(obj(entity)));
	// the targets' printed pictures are images of the experience too (an image part can show one)
	for (const e of list(c.entries).map(obj)) {
		const t = obj(e?.target);
		const src = str(t?.imageSrc);
		if (src && !seen.has(src)) seen.set(src, { id: str(t?.id) || src, assetType: 'image', src });
	}
	return [...seen.values()];
}

/** a file's name for a list: the last part of its path */
export const fileName = (src: string): string => src.split(/[?#]/)[0].split('/').pop() || src;

/** what the Tree's row menu and keys do to a part */
export type PartAction = 'up' | 'down' | 'indent' | 'outdent' | 'duplicate' | 'group' | 'remove';
