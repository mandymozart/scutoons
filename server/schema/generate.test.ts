// server/schema/generate.ts (#477): one source, the same shape in Rust, TS and PHP; tables refuse what they cannot mean
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { addressBookPhp, addressBookRs, addressBookTs, paths, scenePhp, sceneTs, tablesPhp, tablesRs, tablesTs, type ContractRoot, type Draft07 } from './generate.ts';

const root = JSON.parse(readFileSync(paths.schema, 'utf8')) as ContractRoot;
const legacy = JSON.parse(readFileSync(paths.legacyIds, 'utf8'));
const order = JSON.parse(readFileSync(paths.localeOrder, 'utf8'));

test('every language gets every definition, under this file\'s header and without the protocol\'s version', () => {
	const ts = addressBookTs(root);
	const rs = addressBookRs(root);
	const php = addressBookPhp(root);
	for (const out of [ts, rs, php]) {
		assert.match(out, /GENERATED from server\/schema\/api\/address-book\.schema\.json by server\/schema\/generate\.ts/);
		assert.doesNotMatch(out, /PROTOCOL_VERSION|deck\/protocol\/schema\.json/);
	}
	for (const name of Object.keys(root.definitions)) {
		assert.match(ts, new RegExp(`export (type|interface) ${name}\\b`), `TS ${name}`);
		assert.match(rs, new RegExp(`pub (struct|enum|type) ${name}\\b`), `Rust ${name}`);
		assert.match(php, new RegExp(`@phpstan-type ${name} `), `PHP ${name}`);
	}
	// every entry has its reference; an address only where a receiver exists (decisions/deck.md [address])
	assert.match(php, /@phpstan-type AddressEntry array\{ref: string, address\?: string, kind: AddressKind, name: string, parent\?: string, labels: LocaleText, origin: Origin, moods\?: list<string>, set\?: string, spot\?: string, version\?: int\}/);
	assert.match(php, /public const ADDRESS_KIND = \['character', 'persona', 'look', 'location', 'item', 'chapter', 'group', 'variable', 'season', 'toon', 'product', 'region', 'scene'\];/);
	// #487: a scene's version is a number in every language
	assert.match(ts, /version\?: number;/);
	assert.match(rs, /pub version: Option<i64>,/);
	assert.match(php, /public const FORMAT = 2;/);
});

test('the tables: the same data in each language', () => {
	assert.match(tablesTs(legacy, order), /plf: \{ character: 'catpope', persona: 'human' \}/);
	assert.match(tablesPhp(legacy, order), /'plf' => \['character' => 'catpope', 'persona' => 'human'\]/);
	assert.match(tablesRs(order), /LOCALE_ORDER: \[Step; 4\] = \[Step::Asked, Step::Source, Step::Locale\("en"\), Step::Any\]/);
	assert.match(tablesTs(legacy, order), /export const LOCALE_ORDER = \['asked', 'source', 'en', 'any'\] as const;/);
});

test('a table that cannot mean anything is refused', () => {
	assert.throws(() => tablesTs(legacy, { ...order, order: ['asked', 'en'] }), /`any` last/);
	assert.throws(() => tablesTs(legacy, { ...order, order: ['asked', 'English', 'any'] }), /no step and no locale/);
	assert.throws(() => tablesPhp({ ...legacy, ids: { plf: { character: 'catpope', persona: '' } } }, order), /character and persona/);
});

const scene = JSON.parse(readFileSync(paths.sceneSchema, 'utf8')) as Draft07;

test('the scene document (#243, format 2 #489): every definition in TS and PHP, the format, holders and the filter', () => {
	const ts = sceneTs(scene);
	const php = scenePhp(scene);
	for (const name of [...Object.keys(scene.definitions ?? {}), 'SceneDocument']) {
		assert.match(ts, new RegExp(`export type ${name} = `), `TS ${name}`);
		assert.match(php, new RegExp(`@phpstan-type ${name} `), `PHP ${name}`);
	}
	assert.match(ts, /export const SCENE_FORMAT = 2 as const;/);
	assert.match(php, /public const FORMAT = 2;/);
	assert.match(ts, /\tformat: 2;\n/);
	assert.match(ts, /kind: \('character' \| 'persona' \| 'look' \| 'item' \| 'location'\)\[\];/, "a filter's kinds are the address book's");
	assert.match(ts, /export type LocationSlot = string \| LocationPlaceholder;/);
	assert.match(php, /@phpstan-type Take 'one' \| 'many'/);
	assert.doesNotMatch(ts + php, /\bbeing\??: /, 'the being left the document');
	assert.match(ts, /map\?: \[number, number\]\[\];/, 'a channel pair is a tuple');
	assert.doesNotMatch(ts + php, /\bset\??: /, 'no field named set beside the space node param');
});

test('a scene schema outside the subset is refused, not guessed', () => {
	assert.throws(() => sceneTs({ ...scene, title: 'Scene' }), /root is SceneDocument/);
	const odd: Draft07 = { ...scene, properties: { ...scene.properties, toons: { type: 'array', items: { type: 'string' }, uniqueItems: true } } };
	assert.throws(() => sceneTs(odd), /unsupported schema key uniqueItems/);
});
