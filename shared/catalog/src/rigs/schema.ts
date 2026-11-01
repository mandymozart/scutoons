// The structural half of the rig validator (#269): a document checked against schema.json, the JSON schema schemars
// writes from deck-space's serde forms (`UPDATE_FIXTURES=1 cargo test -p deck-space rig_schema`). Only the subset
// schemars emits for those types is read here: object / array / string / number / integer / boolean / null types,
// properties + required + additionalProperties (deny_unknown_fields), items with min / max (fixed arrays), enum,
// anyOf (Option<T>), $ref into $defs, `format: float` (serde's f32). Rust stays the source of truth
// (agents/research/INSTRUMENTS.md §7 "TS types"); the semantic rules (graph, lineage, tracks) are validate.ts.
import schemaJson from './schema.json' with { type: 'json' };

export type Schema = boolean | { [key: string]: any };

export interface RigSchema {
	instrument: Schema;
	preset: Schema;
	rig: Schema;
	/** graph.rs's node kinds: kind → its domain and its ports with their types, in graph.rs's order */
	nodes: Record<string, NodeKind>;
	/** kinds a scene owns once (instrument.rs SCENE_ONLY): never inside an instrument */
	scene_only: string[];
	/** deck-events patch.rs KNOBS: an events node's knobs a, b, c, d */
	event_knobs: number;
}

export type PortType = 'audio' | 'control' | 'video';
export interface NodeKind {
	domain: PortType;
	inputs: Record<string, PortType>;
	outputs: Record<string, PortType>;
}

/** the file's ports are [name, type] pairs (so graph.rs's order survives); read into records, which keep it */
const ports = (pairs: [string, PortType][]): Record<string, PortType> => Object.fromEntries(pairs);
export const RIG_SCHEMA: RigSchema = {
	...(schemaJson as unknown as RigSchema),
	nodes: Object.fromEntries(
		Object.entries(schemaJson.nodes as unknown as Record<string, { domain: PortType; inputs: [string, PortType][]; outputs: [string, PortType][] }>).map(([kind, k]) => [kind, { domain: k.domain, inputs: ports(k.inputs), outputs: ports(k.outputs) }])
	)
};

/** serde reads JSON numbers into f32 – past its range they would turn infinite */
const F32_MAX = 3.4028234663852886e38;
export const isNumber = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v) && Math.abs(v) <= F32_MAX;

const isObject = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v);

const typeOf = (v: unknown): string => (v === null ? 'null' : Array.isArray(v) ? 'array' : typeof v === 'number' ? (Number.isInteger(v) ? 'integer' : 'number') : typeof v);

const matchesType = (type: string, v: unknown): boolean => {
	const t = typeOf(v);
	return type === t || (type === 'number' && t === 'integer');
};

/** the field order of an object's properties, for the "unknown field" message (as RigValidator::fields words it) */
const known = (s: Record<string, any>) => Object.keys(s.properties ?? {}).join(', ');

/**
 * `value` against `schema` (a node of `root`), the problems worded like the server's (`<who>: <field> is required`,
 * `<who>: unknown field "x" (…)`, `<who>: <field> must be …`); [] = it fits. `who` names the document ("instrument x").
 */
export function checkSchema(root: Schema, schema: Schema, value: unknown, who: string): string[] {
	if (schema === true) return [];
	if (schema === false) return [`${who} is not allowed here`];
	if (typeof schema.$ref === 'string') {
		const target = resolve(root, schema.$ref);
		return target === undefined ? [`${who}: the schema has no ${schema.$ref}`] : checkSchema(root, target, value, who);
	}
	if (Array.isArray(schema.anyOf) || Array.isArray(schema.oneOf)) {
		const branches: Schema[] = schema.anyOf ?? schema.oneOf;
		const words = literals(branches);
		if (words) return words.includes(value) ? [] : [`${who} must be one of ${words.join(', ')}`];
		let best: string[] | null = null;
		for (const b of branches) {
			const p = checkSchema(root, b, value, who);
			if (!p.length) return [];
			if (!best || p.length < best.length) best = p;
		}
		return best ?? [];
	}
	const types: string[] = Array.isArray(schema.type) ? schema.type : schema.type ? [schema.type] : [];
	if (types.length && !types.some((t) => matchesType(t, value))) return [`${who} must be ${describe(root, schema)}`];
	if (Array.isArray(schema.enum) && !schema.enum.includes(value)) return [`${who} must be one of ${schema.enum.join(', ')}`];
	if ('const' in schema && schema.const !== value) return [`${who} must be ${JSON.stringify(schema.const)}`];
	const p: string[] = [];
	if (typeof value === 'number' && (schema.format === 'float' || schema.format === 'double') && !isNumber(value)) p.push(`${who} must be a number`);
	if (isObject(value)) {
		const props: Record<string, Schema> = schema.properties ?? {};
		for (const k of schema.required ?? []) if (value[k] === undefined || value[k] === null) p.push(`${who}: ${k} is required`);
		for (const [k, v] of Object.entries(value)) {
			if (k in props) {
				// a left-out optional field reads as its default; `null` is not a value serde knows for it
				if (v === null && !(schema.required ?? []).includes(k)) {
					if (!allowsNull(root, props[k])) p.push(`${who}: ${k} must be ${describe(root, props[k])} – leave it out instead of null`);
					continue;
				}
				p.push(...checkSchema(root, props[k], v, `${who}: ${k}`));
			} else if (schema.additionalProperties === false) p.push(`${who}: unknown field "${k}" (${known(schema)})`);
			else if (schema.additionalProperties !== undefined && schema.additionalProperties !== true) p.push(...checkSchema(root, schema.additionalProperties, v, `${who}: ${k}`));
		}
	}
	if (Array.isArray(value)) {
		if (typeof schema.minItems === 'number' && value.length < schema.minItems) p.push(`${who} must have ${schema.minItems} entries`);
		if (typeof schema.maxItems === 'number' && value.length > schema.maxItems) p.push(`${who} must have ${schema.maxItems} entries`);
		if (schema.items !== undefined) value.forEach((v, i) => p.push(...checkSchema(root, schema.items, v, `${who}[${i}]`)));
	}
	return p;
}

/** a serde enum: schemars writes the undocumented unit variants as one `enum`, the documented ones as `const`s */
function literals(branches: Schema[]): unknown[] | null {
	const out: unknown[] = [];
	for (const b of branches) {
		if (typeof b !== 'object') return null;
		if ('const' in b) out.push(b.const);
		else if (Array.isArray(b.enum)) out.push(...b.enum);
		else return null;
	}
	return out.length ? out : null;
}

function allowsNull(root: Schema, s: Schema): boolean {
	if (s === true) return true;
	if (s === false) return false;
	if (typeof s.$ref === 'string') return allowsNull(root, resolve(root, s.$ref) ?? false);
	const types: string[] = Array.isArray(s.type) ? s.type : s.type ? [s.type] : [];
	return types.includes('null') || (s.anyOf ?? s.oneOf ?? []).some((b: Schema) => allowsNull(root, b));
}

/** `#/$defs/Track` → that schema */
function resolve(root: Schema, ref: string): Schema | undefined {
	if (root === true || root === false || !ref.startsWith('#/')) return undefined;
	let at: any = root;
	for (const part of ref.slice(2).split('/')) {
		at = at?.[part.replace(/~1/g, '/').replace(/~0/g, '~')];
		if (at === undefined) return undefined;
	}
	return at;
}

/** a type's name in a message: "a list of node.port", "{ from, to, scale?, offset? }", "a number" … */
export function describe(root: Schema, s: Schema): string {
	if (s === true) return 'anything';
	if (s === false) return 'nothing';
	if (typeof s.$ref === 'string') return describe(root, resolve(root, s.$ref) ?? true);
	if (Array.isArray(s.anyOf ?? s.oneOf)) {
		const branches: Schema[] = (s.anyOf ?? s.oneOf).filter((b: Schema) => !(typeof b === 'object' && b.type === 'null'));
		const words = literals(branches);
		if (words) return `one of ${words.join(', ')}`;
		return branches.map((b) => describe(root, b)).join(' or ') || 'nothing';
	}
	if (Array.isArray(s.enum)) return `one of ${s.enum.join(', ')}`;
	const types: string[] = Array.isArray(s.type) ? s.type.filter((t: string) => t !== 'null') : s.type ? [s.type] : [];
	const type = types[0];
	if (type === 'object') {
		if (s.properties) {
			const req: string[] = s.required ?? [];
			const optional = Object.keys(s.properties).filter((k) => !req.includes(k));
			return `{ ${[...req, ...optional.map((k) => `${k}?`)].join(', ')} }`;
		}
		return s.additionalProperties && typeof s.additionalProperties === 'object' ? `{ <key>: ${describe(root, s.additionalProperties)} }` : 'an object';
	}
	if (type === 'array') {
		const n = s.minItems !== undefined && s.minItems === s.maxItems ? `${s.minItems} ` : '';
		return s.items !== undefined ? `a list of ${n}${describe(root, s.items)}` : 'a list';
	}
	if (type === 'number' || type === 'integer') return 'a number';
	if (type === 'boolean') return 'true or false';
	if (type === 'string') return 'a string';
	return type ?? 'a value';
}
