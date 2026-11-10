// server/schema/generate.ts (#477): one source, the same shape in Rust, TS and PHP; tables refuse what they cannot mean
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { addressBookPhp, addressBookRs, addressBookTs, paths, tablesPhp, tablesRs, tablesTs, type ContractRoot } from './generate.ts';

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
	assert.match(php, /@phpstan-type AddressEntry array\{address: string, kind: AddressKind, name: string, parent\?: string, labels: LocaleText, origin: Origin, moods\?: list<string>\}/);
	assert.match(php, /public const ADDRESS_KIND = \['character', 'persona', 'look', 'location', 'item', 'chapter', 'group', 'variable'\];/);
	assert.match(php, /public const FORMAT = 1;/);
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
