// server/schema/generate.ts (#477) – the server's contracts for every language, beside their sources (as deck/protocol/generate.ts
// sits beside the relay protocol's schema). Run by `npm run generate` (scripts/generate.ts; `--check` only compares, exit 1 when stale).
//
//   server/schema/api/address-book.schema.json → deck/protocol/src/address_book.rs   (Rust serde types: protocol::address_book)
//                                                shared/api/src/address-book.ts       (TS types, @scutoons/api)
//                                                server/src/Generated/AddressBookTypes.php (PHPStan shapes + the enums' values)
//   server/schema/tables/*.json                → deck/protocol/src/content_text.rs    (the locale order + its pick: protocol::content_text)
//                                                shared/content/src/tables.ts         (legacy character ids, the locale order + its pick)
//                                                server/src/Generated/ContentTables.php (the same)
//   server/schema/content/scene.schema.json    → shared/api/src/scene.ts              (TS types of the stored scene document, #243)
//   (itself written from deck-space's Rust      server/src/Generated/SceneTypes.php  (PHPStan shapes + the format's number)
//    SceneDocument by deck-schema, deck:ci)
//
// The Rust and TS types come from the relay protocol's emitters (same schema subset, same output style); only their file header
// differs. The PHP shapes and the tables are written here. A construct outside the subset throws, so a new one is a change here first.
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { emitRust, emitTypeScript, type Root } from '../../deck/protocol/generate.ts';

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
};
export type ContractRoot = Root & { title: string };
export type LegacyIds = { description: string; ids: Record<string, { character: string; persona: string }> };
export type LocaleOrder = { description: string; order: string[] };

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, '..', '..');
const rel = (p: string) => relative(repo, p).replace(/\\/g, '/');

export const paths = {
	schema: join(here, 'api', 'address-book.schema.json'),
	legacyIds: join(here, 'tables', 'legacy-character-ids.json'),
	localeOrder: join(here, 'tables', 'locale-order.json'),
	rs: join(repo, 'deck', 'protocol', 'src', 'address_book.rs'),
	ts: join(repo, 'shared', 'api', 'src', 'address-book.ts'),
	php: join(repo, 'server', 'src', 'Generated', 'AddressBookTypes.php'),
	tablesRs: join(repo, 'deck', 'protocol', 'src', 'content_text.rs'),
	tablesTs: join(repo, 'shared', 'content', 'src', 'tables.ts'),
	tablesPhp: join(repo, 'server', 'src', 'Generated', 'ContentTables.php'),
	sceneSchema: join(here, 'content', 'scene.schema.json'),
	sceneTs: join(repo, 'shared', 'api', 'src', 'scene.ts'),
	scenePhp: join(repo, 'server', 'src', 'Generated', 'SceneTypes.php')
};

const header = (source: string) => `GENERATED from ${source} by server/schema/generate.ts – edit the source, run \`npm run generate\`.`;

/** The protocol emitter's output with this file's header instead of the protocol's, and without its version constant. */
function rehead(out: string, comment: string, trailer: string, source: string): string {
	const first = out.indexOf('\n');
	if (!out.startsWith(`${comment} GENERATED from deck/protocol/schema.json`) || !out.endsWith(trailer)) {
		throw new Error('the protocol emitter changed its header or trailer – adapt rehead()');
	}
	return `${comment} ${header(source)}${out.slice(first, out.length - trailer.length).trimEnd()}\n`;
}

// ---------------------------------------------------------------------------------------------------------- the schema

export function addressBookTs(root: ContractRoot): string {
	return rehead(emitTypeScript(root), '//', 'export const PROTOCOL_VERSION = 1 as const;\n', rel(paths.schema));
}

export function addressBookRs(root: ContractRoot): string {
	return rehead(emitRust(root), '//', 'pub const PROTOCOL_VERSION: i64 = 1;\n', rel(paths.schema));
}

const refName = (ref: string): string => {
	const m = /^#\/definitions\/([A-Za-z0-9_]+)$/.exec(ref);
	if (!m) throw new Error(`unsupported $ref ${ref}`);
	return m[1];
};

const phpType = (s: Schema): string => {
	if (s.$ref) return refName(s.$ref);
	if (s.enum) return s.enum.map((v) => `'${v}'`).join('|');
	if (s.const !== undefined) return typeof s.const === 'string' ? `'${s.const}'` : String(s.const);
	switch (s.type) {
		case 'string':
			return 'string';
		case 'integer':
			return 'int';
		case 'number':
			return 'float';
		case 'boolean':
			return 'bool';
		case 'array':
			if (!s.items) throw new Error('array without items');
			return `list<${phpType(s.items)}>`;
		case 'object':
			if (typeof s.additionalProperties === 'object' && !s.properties) return `array<string, ${phpType(s.additionalProperties)}>`;
			if (s.properties) {
				const fields = Object.entries(s.properties).map(([name, p]) => `${name}${s.required?.includes(name) ? '' : '?'}: ${phpType(p)}`);
				return `array{${fields.join(', ')}}`;
			}
	}
	throw new Error(`unsupported schema ${JSON.stringify(s)}`);
};

const screaming = (name: string): string => name.replace(/([a-z0-9])([A-Z])/g, '$1_$2').toUpperCase();
const phpString = (s: string): string => `'${s.replace(/\\/g, '\\\\').replace(/'/g, "\\'")}'`;
const phpList = (list: string[]): string => `[${list.map(phpString).join(', ')}]`;

/** PHPStan shapes (`@phpstan-type`, imported with `@phpstan-import-type … from AddressBookTypes`) and each enum's values. */
export function addressBookPhp(root: ContractRoot): string {
	const types = Object.entries(root.definitions).map(([name, def]) => ` * @phpstan-type ${name} ${phpType(def as Schema)}`);
	const enums = Object.entries(root.definitions)
		.filter(([, def]) => (def as Schema).enum)
		.map(([name, def]) => `    /** @var list<${name}> ${(def as Schema).description ?? ''} */\n    public const ${screaming(name)} = ${phpList((def as Schema).enum!)};\n`);
	const format = (root.definitions[root.title] as Schema).properties?.address_book?.const;
	if (typeof format !== 'number') throw new Error(`${root.title}.address_book must be a numeric const`);
	return [
		'<?php\n',
		`// ${header(rel(paths.schema))}\n`,
		'declare(strict_types=1);\n\nnamespace Scutoons\\Generated;\n\n',
		`/**\n * ${root.description}\n *\n${types.join('\n')}\n */\n`,
		'final class AddressBookTypes\n{\n',
		`    /** the format's version (\`address_book\`) */\n    public const FORMAT = ${format};\n`,
		...enums,
		'}\n'
	].join('');
}

// ---------------------------------------------------------------------------------------------------------- the tables

const STEPS = new Set(['asked', 'source', 'any']);

function checkTables(legacy: LegacyIds, order: LocaleOrder): void {
	if (!order.order.includes('asked') || order.order.at(-1) !== 'any') throw new Error('locale order: `asked` in it, `any` last');
	for (const step of order.order) if (!STEPS.has(step) && !/^[a-z]{2,3}$/.test(step)) throw new Error(`locale order: '${step}' is no step and no locale`);
	for (const [id, t] of Object.entries(legacy.ids)) {
		if (!/^[a-z0-9][a-z0-9-]*$/.test(id) || !t.character || !t.persona) throw new Error(`legacy character id '${id}': character and persona`);
	}
}

const tsDoc = (text: string) => `/** ${text.replace(/\*\//g, '* /')} */\n`;
const rsDoc = (text: string) => `/// ${text}\n`;

export function tablesTs(legacy: LegacyIds, order: LocaleOrder): string {
	checkTables(legacy, order);
	const ids = Object.entries(legacy.ids)
		.map(([id, t]) => `\t${/^[a-z0-9_]+$/.test(id) ? id : `'${id}'`}: { character: '${t.character}', persona: '${t.persona}' }`)
		.join(',\n');
	return [
		`// ${header('server/schema/tables/*.json')}\n\n`,
		tsDoc(legacy.description),
		`export const LEGACY_CHARACTER_IDS: Record<string, { character: string; persona: string }> = {\n${ids}\n};\n\n`,
		tsDoc(order.description),
		`export const LOCALE_ORDER = [${order.order.map((s) => `'${s}'`).join(', ')}] as const;\n\n`,
		tsDoc('Content text in the one locale order (LOCALE_ORDER): the first step with a text, undefined when none has one.'),
		'export function pickContentText(text: Readonly<Record<string, unknown>> | null | undefined, asked: string, source?: string | null): string | undefined {\n',
		'\tif (!text) return undefined;\n',
		"\tconst present = (value: unknown): value is string => typeof value === 'string' && value.trim() !== '';\n",
		'\tfor (const step of LOCALE_ORDER as readonly string[]) {\n',
		"\t\tif (step === 'any') {\n",
		'\t\t\tfor (const value of Object.values(text)) if (present(value)) return value;\n',
		'\t\t\tcontinue;\n',
		'\t\t}\n',
		"\t\tconst locale = step === 'asked' ? asked : step === 'source' ? source : step;\n",
		'\t\tconst value = locale ? (Object.hasOwn(text, locale) ? text[locale] : undefined) : undefined;\n',
		'\t\tif (present(value)) return value;\n',
		'\t}\n',
		'\treturn undefined;\n',
		'}\n'
	].join('');
}

export function tablesPhp(legacy: LegacyIds, order: LocaleOrder): string {
	checkTables(legacy, order);
	const ids = Object.entries(legacy.ids)
		.map(([id, t]) => `        ${phpString(id)} => ['character' => ${phpString(t.character)}, 'persona' => ${phpString(t.persona)}],\n`)
		.join('');
	return [
		'<?php\n',
		`// ${header('server/schema/tables/*.json')}\n`,
		'declare(strict_types=1);\n\nnamespace Scutoons\\Generated;\n\n',
		'/** Content tables, the same in Rust, TS and PHP (#477) */\n',
		'final class ContentTables\n{\n',
		`    /**\n     * ${legacy.description}\n     * @var array<string, array{character: string, persona: string}>\n     */\n`,
		`    public const LEGACY_CHARACTER_IDS = [\n${ids}    ];\n\n`,
		`    /**\n     * ${order.description}\n     * @var list<string>\n     */\n`,
		`    public const LOCALE_ORDER = ${phpList(order.order)};\n\n`,
		'    /** Content text in the one locale order (LOCALE_ORDER): the first step with a text, null when none has one */\n',
		'    public static function pickText(mixed $text, string $asked, ?string $source = null): ?string\n    {\n',
		'        if (is_object($text)) $text = get_object_vars($text);\n',
		'        if (!is_array($text)) return null;\n',
		'        foreach (self::LOCALE_ORDER as $step) {\n',
		"            if ($step === 'any') {\n",
		'                foreach ($text as $value) {\n',
		'                    if (self::present($value)) return $value;\n',
		'                }\n',
		'                continue;\n',
		'            }\n',
		"            $locale = match ($step) { 'asked' => $asked, 'source' => $source, default => $step };\n",
		"            $value = $locale !== null && $locale !== '' ? ($text[$locale] ?? null) : null;\n",
		'            if (self::present($value)) return $value;\n',
		'        }\n',
		'        return null;\n',
		'    }\n\n',
		'    /** @phpstan-assert-if-true string $value a string with more than white space */\n',
		'    private static function present(mixed $value): bool\n    {\n',
		"        return is_string($value) && preg_match('/^[\\s\\p{Z}\\x{FEFF}]*$/u', $value) !== 1;\n",
		'    }\n',
		'}\n'
	].join('');
}

export function tablesRs(order: LocaleOrder): string {
	checkTables({ description: '', ids: {} }, order);
	const step = (s: string) => (s === 'asked' ? 'Step::Asked' : s === 'source' ? 'Step::Source' : s === 'any' ? 'Step::Any' : `Step::Locale("${s}")`);
	return [
		`// ${header('server/schema/tables/locale-order.json')}\n`,
		`//! The one locale order for content text and its pick (#477).\n\n`,
		rsDoc('A step of the locale order'),
		'#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum Step {\n',
		'    /// the asked locale\n    Asked,\n    /// the document\'s source language\n    Source,\n    /// this locale\n    Locale(&\'static str),\n',
		'    /// the first text in the map\'s own order (a `BTreeMap`: sorted)\n    Any,\n}\n\n',
		rsDoc(order.description),
		`pub const LOCALE_ORDER: [Step; ${order.order.length}] = [${order.order.map(step).join(', ')}];\n\n`,
		rsDoc('Content text in the one locale order: the first step with a text (more than white space), none when none has one.'),
		"pub fn pick_text<'a, M>(text: M, asked: &str, source: Option<&str>) -> Option<&'a str>\nwhere\n    M: IntoIterator<Item = (&'a String, &'a String)> + Copy,\n{\n",
		'    let present = |v: &str| !v.trim().is_empty();\n',
		'    for step in LOCALE_ORDER {\n',
		'        let locale = match step {\n',
		'            Step::Asked => Some(asked),\n            Step::Source => source,\n            Step::Locale(l) => Some(l),\n',
		'            Step::Any => return text.into_iter().find(|(_, v)| present(v.as_str())).map(|(_, v)| v.as_str()),\n',
		'        };\n',
		'        if let Some(l) = locale {\n',
		'            if let Some((_, v)) = text.into_iter().find(|(k, v)| k.as_str() == l && present(v.as_str())) {\n',
		'                return Some(v.as_str());\n',
		'            }\n',
		'        }\n',
		'    }\n',
		'    None\n}\n\n',
		'#[cfg(test)]\nmod tests {\n',
		'    use std::collections::BTreeMap;\n\n',
		'    /// the cases every language runs (server/schema/tables/locale-order.vectors.json)\n',
		'    #[test]\n    fn the_shared_cases() {\n',
		'        let file: serde_json::Value = serde_json::from_str(include_str!("../../../server/schema/tables/locale-order.vectors.json")).unwrap();\n',
		'        for case in file["cases"].as_array().unwrap() {\n',
		'            let text: BTreeMap<String, String> = serde_json::from_value(case["text"].clone()).unwrap();\n',
		'            let picked = super::pick_text(&text, case["asked"].as_str().unwrap(), case["source"].as_str());\n',
		'            assert_eq!(picked, case["expect"].as_str(), "{case}");\n',
		'        }\n',
		'    }\n}\n'
	].join('');
}

// ---------------------------------------------------------------------------------------------------------- the scene document

/** The draft-07 subset schemars writes for deck-space's SceneDocument (#243); anything else throws, so a new construct is a change here first. */
export type Draft07 = {
	$ref?: string;
	type?: string | string[];
	const?: string | number | boolean;
	enum?: (string | number)[];
	oneOf?: Draft07[];
	anyOf?: Draft07[];
	allOf?: Draft07[];
	items?: Draft07;
	minItems?: number;
	maxItems?: number;
	properties?: Record<string, Draft07>;
	required?: string[];
	additionalProperties?: boolean | Draft07;
	definitions?: Record<string, Draft07>;
	title?: string;
	description?: string;
	[key: string]: unknown;
};

/** what only annotates a schema; every other key must be one the emitters read */
const ANNOTATIONS = new Set(['description', 'title', 'default', 'format', 'minimum', 'maximum', '$schema', '$id', '$comment', 'definitions']);
const SHAPES = new Set(['$ref', 'type', 'const', 'enum', 'oneOf', 'anyOf', 'allOf', 'items', 'minItems', 'maxItems', 'properties', 'required', 'additionalProperties']);

type Field = { name: string; type: string; required: boolean; doc?: string };
type Lang = {
	lit: (v: string | number | boolean) => string;
	scalar: Record<string, string>;
	list: (item: string, tuple: number | null) => string;
	object: (fields: Field[], open: boolean) => string;
	map: (value: string) => string;
	unknown: string;
};

function typeOf(s: Draft07 | boolean, lang: Lang, where: string): string {
	if (s === false) throw new Error(`${where}: schema false`);
	if (s === true) return lang.unknown;
	const keys = Object.keys(s).filter((k) => !ANNOTATIONS.has(k));
	for (const k of keys) if (!SHAPES.has(k)) throw new Error(`${where}: unsupported schema key ${k}`);
	if (!keys.length) return lang.unknown;
	if (s.$ref) return refName(s.$ref);
	if (s.const !== undefined) return lang.lit(s.const);
	if (s.enum) return s.enum.map(lang.lit).join(' | ');
	for (const list of [s.oneOf, s.anyOf]) if (list) return list.map((x, i) => typeOf(x, lang, `${where}[${i}]`)).join(' | ');
	if (s.allOf) {
		if (s.allOf.length !== 1) throw new Error(`${where}: allOf of ${s.allOf.length}`);
		return typeOf(s.allOf[0], lang, where);
	}
	const types = Array.isArray(s.type) ? s.type : s.type ? [s.type] : [];
	if (!types.length) throw new Error(`${where}: no type`);
	return types
		.map((t) => {
			if (lang.scalar[t]) return lang.scalar[t];
			if (t === 'array') {
				if (!s.items) throw new Error(`${where}: array without items`);
				const tuple = s.minItems !== undefined && s.minItems === s.maxItems ? s.minItems : null;
				return lang.list(typeOf(s.items, lang, `${where}[]`), tuple);
			}
			if (t === 'object') {
				const open = s.additionalProperties === undefined || s.additionalProperties === true;
				if (!s.properties) return lang.map(typeof s.additionalProperties === 'object' ? typeOf(s.additionalProperties, lang, `${where}{}`) : lang.unknown);
				const fields = Object.entries(s.properties).map(([name, p]) => ({ name, type: typeOf(p, lang, `${where}.${name}`), required: s.required?.includes(name) ?? false, doc: p.description }));
				return lang.object(fields, open);
			}
			throw new Error(`${where}: unsupported type ${t}`);
		})
		.join(' | ');
}

const oneLine = (text: string) => text.replace(/\*\//g, '* /').replace(/\s*\n\s*/g, ' ');
const tsComment = (text: string | undefined, indent = '') => (text ? `${indent}/** ${oneLine(text)} */\n` : '');

const TS: Lang = {
	lit: (v) => (typeof v === 'string' ? `'${v}'` : String(v)),
	scalar: { string: 'string', integer: 'number', number: 'number', boolean: 'boolean', null: 'null' },
	list: (item, tuple) => (tuple !== null ? `[${Array(tuple).fill(item).join(', ')}]` : item.includes(' | ') ? `(${item})[]` : `${item}[]`),
	object: (fields, open) =>
		`{\n${fields.map((f) => `${tsComment(f.doc, '\t')}\t${f.name}${f.required ? '' : '?'}: ${f.type};\n`).join('')}${open ? '\t[key: string]: unknown;\n' : ''}}`,
	map: (value) => `Record<string, ${value}>`,
	unknown: 'unknown'
};

const PHP: Lang = {
	lit: (v) => (typeof v === 'string' ? `'${v}'` : String(v)),
	scalar: { string: 'string', integer: 'int', number: 'float', boolean: 'bool', null: 'null' },
	list: (item, tuple) => (tuple !== null ? `array{${Array(tuple).fill(item).join(', ')}}` : `list<${item}>`),
	// a shape with more keys than it names (a node: id, kind + its params) is a map in PHPStan's words
	object: (fields, open) => (open ? 'array<string, mixed>' : `array{${fields.map((f) => `${f.name}${f.required ? '' : '?'}: ${f.type}`).join(', ')}}`),
	map: (value) => `array<string, ${value === 'unknown' ? 'mixed' : value}>`,
	unknown: 'mixed'
};

/** the definitions and the root (SceneDocument) in the schema's order, the format's number */
function sceneParts(schema: Draft07): { title: string; format: number; defs: [string, Draft07][] } {
	const format = schema.properties?.format?.const;
	if (schema.title !== 'SceneDocument' || typeof format !== 'number') throw new Error('scene.schema.json: the root is SceneDocument with a numeric `format` const');
	const { definitions = {}, ...root } = schema;
	return { title: schema.title, format, defs: [...Object.entries(definitions), [schema.title, root as Draft07]] };
}

export function sceneTs(schema: Draft07): string {
	const { format, defs } = sceneParts(schema);
	const types = defs.map(([name, def]) => `${tsComment(def.description)}export type ${name} = ${typeOf(def, TS, name)};\n`);
	return [
		`// ${header(rel(paths.sceneSchema))}\n`,
		"// The stored scene document `scene:<id>` (#243, research/SCENES.md §3): deck-space's SceneDocument (deck/space/src/scene_document.rs) →\n",
		"// its JSON Schema (deck-schema) → these types. Whether a scene plays is the core's (the build service's /validate at publish).\n\n",
		`/** the format the API stores and the decks read */\nexport const SCENE_FORMAT = ${format} as const;\n\n`,
		types.join('\n')
	].join('');
}

export function scenePhp(schema: Draft07): string {
	const { format, defs, title } = sceneParts(schema);
	const types = defs.map(([name, def]) => ` * @phpstan-type ${name} ${typeOf(def, PHP, name)}`);
	return [
		'<?php\n',
		`// ${header(rel(paths.sceneSchema))}\n`,
		'declare(strict_types=1);\n\nnamespace Scutoons\\Generated;\n\n',
		`/**\n * The stored scene document \`scene:<id>\` (#243; deck-space's ${title}, its JSON Schema written by deck-schema)\n *\n${types.join('\n')}\n */\n`,
		'final class SceneTypes\n{\n',
		`    /** the document's format (\`format\`) */\n    public const FORMAT = ${format};\n`,
		'}\n'
	].join('');
}

// ---------------------------------------------------------------------------------------------------------- CLI

const read = <T>(file: string): T => JSON.parse(readFileSync(file, 'utf8')) as T;

export function generate(): Record<string, string> {
	const root = read<ContractRoot>(paths.schema);
	const legacy = read<LegacyIds>(paths.legacyIds);
	const order = read<LocaleOrder>(paths.localeOrder);
	const scene = read<Draft07>(paths.sceneSchema);
	return {
		[paths.sceneTs]: sceneTs(scene),
		[paths.scenePhp]: scenePhp(scene),
		[paths.rs]: addressBookRs(root),
		[paths.ts]: addressBookTs(root),
		[paths.php]: addressBookPhp(root),
		[paths.tablesRs]: tablesRs(order),
		[paths.tablesTs]: tablesTs(legacy, order),
		[paths.tablesPhp]: tablesPhp(legacy, order)
	};
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
	const check = process.argv.includes('--check');
	let stale = 0;
	for (const [file, content] of Object.entries(generate())) {
		const current = existsSync(file) ? readFileSync(file, 'utf8').replace(/\r\n/g, '\n') : '';
		if (current === content) continue;
		if (check) {
			console.error(`stale: ${rel(file)}`);
			stale++;
		} else {
			writeFileSync(file, content, 'utf8');
			console.log(`wrote ${rel(file)}`);
		}
	}
	if (stale) process.exit(1);
}
