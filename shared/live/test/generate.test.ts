// The generator (deck/protocol/generate.ts): the committed outputs match the schema, and the emitters handle the subset.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { generate, emitRust, emitTypeScript, paths, type Root } from '../../../deck/protocol/generate.ts';

const root = JSON.parse(readFileSync(paths.schema, 'utf8')) as Root;
const read = (p: string) => readFileSync(p, 'utf8').replace(/\r\n/g, '\n');

test('committed outputs are current (else: npm run generate -w shared/live)', () => {
	const { ts, rs } = generate(root);
	assert.equal(read(paths.ts), ts);
	assert.equal(read(paths.rs), rs);
});

test('a tagged union becomes a serde adjacently tagged enum and a TS discriminated union', () => {
	const mini: Root = {
		definitions: {
			A: { type: 'object', properties: { n: { type: 'integer', minimum: 0, maximum: 255 } }, required: ['n'] },
			M: { oneOf: [{ type: 'object', properties: { kind: { const: 'vote.open' }, p: { $ref: '#/definitions/A' } }, required: ['kind', 'p'] }] },
			E: { allOf: [{ type: 'object', properties: { seq: { type: 'integer' }, camelCase: { type: 'string' } }, required: ['seq'] }, { $ref: '#/definitions/M' }] }
		}
	};
	const rs = emitRust(mini);
	assert.match(rs, /#\[serde\(tag = "kind", content = "p"\)\]\npub enum M \{\n\t#\[serde\(rename = "vote.open"\)\]\n\tVoteOpen\(A\),/);
	assert.match(rs, /pub n: u8,/);
	assert.match(rs, /#\[serde\(rename = "camelCase", default, skip_serializing_if = "Option::is_none"\)\]\n\tpub camel_case: Option<String>,/);
	assert.match(rs, /#\[serde\(flatten\)\]\n\tpub m: M,/);
	const ts = emitTypeScript(mini);
	assert.match(ts, /export type M =\n\t\| \{ kind: 'vote.open'; p: A \};/);
	assert.match(ts, /export interface EBase \{\n\tseq: number;\n\tcamelCase\?: string;\n\}\nexport type E = EBase & M;/);
});

test('an unknown construct throws instead of emitting something wrong', () => {
	const bad: Root = { definitions: { X: { type: 'object', properties: { y: { type: 'object', properties: { z: { type: 'string' } } } } } } };
	assert.throws(() => emitRust(bad), /inline object/);
	assert.throws(() => emitTypeScript(bad), /inline object/);
});
