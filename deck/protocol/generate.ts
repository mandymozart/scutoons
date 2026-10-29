// deck/protocol/generate.ts – one schema, two outputs (CYBERDECKS.md §4, LIVE.md B.6).
// Reads deck/protocol/schema.json and writes
//   shared/live/src/protocol.ts   (TypeScript types, `@scutoons/live`)
//   deck/protocol/src/generated.rs (serde types, the `protocol` crate)
// Run: `npm run generate -w shared/live` (`--check` only compares, exit 1 when stale – the CI step).
// Why a script and not typify / json-schema-to-typescript: deck/README.md "Protocol".
// The schema subset it understands is listed in the schema's description; anything else throws here, so a new
// construct is a change to this file first.

import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

type Schema = {
	description?: string;
	type?: string;
	enum?: string[];
	const?: string | number;
	properties?: Record<string, Schema>;
	required?: string[];
	additionalProperties?: boolean | Schema;
	items?: Schema;
	$ref?: string;
	oneOf?: Schema[];
	allOf?: Schema[];
	minimum?: number;
	maximum?: number;
};

export type Root = { definitions: Record<string, Schema>; description?: string };

const HEADER = 'GENERATED from deck/protocol/schema.json by deck/protocol/generate.ts – edit the schema, run `npm run generate -w shared/live`.';

const refName = (ref: string): string => {
	const m = /^#\/definitions\/([A-Za-z0-9_]+)$/.exec(ref);
	if (!m) throw new Error(`unsupported $ref ${ref}`);
	return m[1];
};

const isMap = (s: Schema): s is Schema & { additionalProperties: Schema } =>
	s.type === 'object' && !s.properties && typeof s.additionalProperties === 'object';
const isLoose = (s: Schema): boolean => s.type === 'object' && !s.properties && s.additionalProperties === true;
const isEmpty = (s: Schema): boolean => s.type === 'object' && !s.properties && s.additionalProperties === false;
/** A `oneOf` of `{ kind: const, p: $ref }` objects – the discriminated union of B.6. */
const isTagged = (s: Schema): boolean =>
	Array.isArray(s.oneOf) &&
	s.oneOf.every((v) => v.type === 'object' && v.properties?.kind?.const !== undefined && v.properties?.p);

// ---------------------------------------------------------------- TypeScript

const tsType = (s: Schema): string => {
	if (s.$ref) return refName(s.$ref);
	if (s.enum) return s.enum.map((v) => `'${v}'`).join(' | ');
	if (s.const !== undefined) return typeof s.const === 'string' ? `'${s.const}'` : String(s.const);
	switch (s.type) {
		case 'string':
			return 'string';
		case 'integer':
		case 'number':
			return 'number';
		case 'boolean':
			return 'boolean';
		case 'array':
			if (!s.items) throw new Error('array without items');
			return `${tsType(s.items)}[]`;
		case 'object':
			if (isMap(s)) return `Record<string, ${tsType(s.additionalProperties)}>`;
			if (isLoose(s)) return 'Record<string, unknown>';
			if (isEmpty(s)) return 'Record<string, never>';
			throw new Error('inline object with properties – make it a definition');
	}
	throw new Error(`unsupported schema ${JSON.stringify(s)}`);
};

const tsDoc = (text: string | undefined, indent = ''): string =>
	text ? `${indent}/** ${text.replace(/\*\//g, '* /')} */\n` : '';

const tsFields = (s: Schema): string =>
	Object.entries(s.properties ?? {})
		.map(([name, prop]) => {
			const opt = s.required?.includes(name) ? '' : '?';
			return `${tsDoc(prop.description, '\t')}\t${name}${opt}: ${tsType(prop)};\n`;
		})
		.join('');

export function emitTypeScript(root: Root): string {
	const out: string[] = [`// ${HEADER}\n// ${root.description ?? ''}\n`];
	for (const [name, def] of Object.entries(root.definitions)) {
		out.push(tsDoc(def.description));
		if (def.enum) {
			out.push(`export type ${name} = ${tsType(def)};\n`);
		} else if (isTagged(def)) {
			const variants = def.oneOf!.map((v) => `\t| { kind: '${v.properties!.kind.const}'; p: ${tsType(v.properties!.p)} }`);
			out.push(`export type ${name} =\n${variants.join('\n')};\n`);
		} else if (def.allOf) {
			const [base, ...rest] = def.allOf;
			if (!base.properties || rest.some((r) => !r.$ref)) throw new Error(`allOf ${name}: object first, then $refs`);
			out.push(`export interface ${name}Base {\n${tsFields(base)}}\n`);
			out.push(`export type ${name} = ${name}Base & ${rest.map((r) => refName(r.$ref!)).join(' & ')};\n`);
		} else if (def.type === 'object' && def.properties) {
			out.push(`export interface ${name} {\n${tsFields(def)}}\n`);
		} else {
			out.push(`export type ${name} = ${tsType(def)};\n`);
		}
		out.push('\n');
	}
	out.push(`export const PROTOCOL_VERSION = 1 as const;\n`);
	return out.join('');
}

// ---------------------------------------------------------------------- Rust

const RUST_KEYWORDS = new Set(['type', 'match', 'ref', 'self', 'use', 'mod', 'fn', 'in', 'as', 'loop', 'move', 'box']);
const snake = (s: string): string => s.replace(/([a-z0-9])([A-Z])/g, '$1_$2').toLowerCase();
const pascal = (s: string): string =>
	s
		.split(/[^A-Za-z0-9]+/)
		.filter(Boolean)
		.map((w) => w[0].toUpperCase() + w.slice(1))
		.join('');

const rustType = (s: Schema): string => {
	if (s.$ref) return refName(s.$ref);
	if (s.const !== undefined && typeof s.const === 'number') return 'i64';
	switch (s.type) {
		case 'string':
			return 'String';
		case 'integer':
			if ((s.minimum ?? -1) >= 0 && (s.maximum ?? 256) <= 255) return 'u8';
			if ((s.minimum ?? -1) >= 0) return 'u64';
			return 'i64';
		case 'number':
			return 'f64';
		case 'boolean':
			return 'bool';
		case 'array':
			if (!s.items) throw new Error('array without items');
			return `Vec<${rustType(s.items)}>`;
		case 'object':
			if (isMap(s)) return `std::collections::HashMap<String, ${rustType(s.additionalProperties)}>`;
			if (isLoose(s)) return 'Loose';
			if (isEmpty(s)) return 'Empty';
			throw new Error('inline object with properties – make it a definition');
	}
	throw new Error(`unsupported schema ${JSON.stringify(s)}`);
};

const rustDoc = (text: string | undefined, indent = ''): string =>
	text ? text.split('\n').map((l) => `${indent}/// ${l}\n`).join('') : '';

const rustFields = (s: Schema): string =>
	Object.entries(s.properties ?? {})
		.map(([name, prop]) => {
			let field = snake(name);
			const attrs: string[] = [];
			if (RUST_KEYWORDS.has(field)) field = `r#${field}`;
			if (field !== name) attrs.push(`rename = "${name}"`);
			const required = s.required?.includes(name) ?? false;
			let ty = rustType(prop);
			if (!required) {
				ty = `Option<${ty}>`;
				attrs.push('default', 'skip_serializing_if = "Option::is_none"');
			}
			const attr = attrs.length ? `\t#[serde(${attrs.join(', ')})]\n` : '';
			return `${rustDoc(prop.description, '\t')}${attr}\tpub ${field}: ${ty},\n`;
		})
		.join('');

const DERIVE = '#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]';

export function emitRust(root: Root): string {
	const out: string[] = [
		`// ${HEADER}\n// ${root.description ?? ''}\n#![allow(clippy::all)]\nuse serde::{Deserialize, Serialize};\n\n`,
		'/// A payload whose shape is not fixed yet: any JSON object.\npub type Loose = serde_json::Map<String, serde_json::Value>;\n\n'
	];
	for (const [name, def] of Object.entries(root.definitions)) {
		if (name === 'Loose') continue;
		out.push(rustDoc(def.description));
		if (def.enum) {
			const variants = def.enum.map((v) => `\t#[serde(rename = "${v}")]\n\t${pascal(v)},\n`).join('');
			out.push(`#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]\npub enum ${name} {\n${variants}}\n`);
			out.push(`\nimpl ${name} {\n\tpub fn as_str(self) -> &'static str {\n\t\tmatch self {\n`);
			out.push(def.enum.map((v) => `\t\t\t${name}::${pascal(v)} => "${v}",\n`).join(''));
			out.push(`\t\t}\n\t}\n}\n`);
		} else if (isTagged(def)) {
			const variants = def
				.oneOf!.map((v) => {
					const tag = String(v.properties!.kind.const);
					return `\t#[serde(rename = "${tag}")]\n\t${pascal(tag)}(${rustType(v.properties!.p)}),\n`;
				})
				.join('');
			out.push(`${DERIVE}\n#[serde(tag = "kind", content = "p")]\npub enum ${name} {\n${variants}}\n`);
			out.push(`\nimpl ${name} {\n\t/// The \`kind\` string on the wire.\n\tpub fn kind(&self) -> &'static str {\n\t\tmatch self {\n`);
			out.push(def.oneOf!.map((v) => `\t\t\t${name}::${pascal(String(v.properties!.kind.const))}(_) => "${v.properties!.kind.const}",\n`).join(''));
			out.push(`\t\t}\n\t}\n}\n`);
		} else if (def.allOf) {
			const [base, ...rest] = def.allOf;
			if (!base.properties || rest.some((r) => !r.$ref)) throw new Error(`allOf ${name}: object first, then $refs`);
			const flat = rest.map((r) => `\t#[serde(flatten)]\n\tpub ${snake(refName(r.$ref!))}: ${refName(r.$ref!)},\n`).join('');
			out.push(`${DERIVE}\npub struct ${name} {\n${rustFields(base)}${flat}}\n`);
		} else if (def.type === 'object' && def.properties) {
			out.push(`${DERIVE}\npub struct ${name} {\n${rustFields(def)}}\n`);
		} else if (isEmpty(def)) {
			out.push(`#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]\npub struct ${name} {}\n`);
		} else {
			out.push(`pub type ${name} = ${rustType(def)};\n`);
		}
		out.push('\n');
	}
	out.push('pub const PROTOCOL_VERSION: i64 = 1;\n');
	return out.join('');
}

// ----------------------------------------------------------------------- CLI

export const paths = (() => {
	const here = dirname(fileURLToPath(import.meta.url));
	const repo = join(here, '..', '..');
	return {
		schema: join(here, 'schema.json'),
		ts: join(repo, 'shared', 'live', 'src', 'protocol.ts'),
		rs: join(here, 'src', 'generated.rs')
	};
})();

export function generate(root: Root): { ts: string; rs: string } {
	return { ts: emitTypeScript(root), rs: emitRust(root) };
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
	const check = process.argv.includes('--check');
	const root = JSON.parse(readFileSync(paths.schema, 'utf8')) as Root;
	const { ts, rs } = generate(root);
	let stale = 0;
	for (const [file, content] of [
		[paths.ts, ts],
		[paths.rs, rs]
	] as const) {
		const current = existsSync(file) ? readFileSync(file, 'utf8').replace(/\r\n/g, '\n') : '';
		if (current === content) continue;
		if (check) {
			console.error(`stale: ${file}`);
			stale++;
		} else {
			writeFileSync(file, content, 'utf8');
			console.log(`wrote ${file}`);
		}
	}
	if (stale) process.exit(1);
}
