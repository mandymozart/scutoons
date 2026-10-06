// Module-level state once per page (src/lib/utils/shared.ts): 2026-10-06 the dev server served `expr-context.svelte.ts`
// and `flow-pictures.svelte.ts` under two `?t=` timestamps – two module copies, two ExprContexts (the builder offered no
// story variables), two `Symbol('flow-pictures')` context keys (every Flow picture an empty slot). A query makes Node
// load a second copy of a module, like the dev server did.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import * as one from '../src/lib/utils/shared.ts';

const copy = async (): Promise<typeof one> => import(`../src/lib/utils/shared.ts?copy=${Math.random()}`);

test('shared: two copies of the module hand out the same instance', async () => {
	const other = await copy();
	assert.notEqual(other, one, 'the query loads a second copy');
	const a = one.shared('test-shared', () => ({ n: 1 }));
	const b = other.shared('test-shared', () => ({ n: 2 }));
	assert.equal(a, b);
	assert.equal(b.n, 1);
	assert.notEqual(one.shared('test-other', () => ({})), a);
});

test('contextKey: the same key in every copy, a different one per name', async () => {
	const other = await copy();
	assert.equal(one.contextKey('flow-pictures'), other.contextKey('flow-pictures'));
	assert.notEqual(one.contextKey('flow-pictures'), one.contextKey('flow-lazy'));
});

test('creator sources: context keys and module-level singletons go through utils/shared.ts', () => {
	const src = fileURLToPath(new URL('../src', import.meta.url));
	const files: string[] = [];
	const walk = (dir: string) => {
		for (const e of readdirSync(dir, { withFileTypes: true })) {
			const p = join(dir, e.name);
			if (e.isDirectory()) {
				if (e.name !== 'paraglide') walk(p);
			} else if (/\.(ts|svelte)$/.test(e.name)) files.push(p);
		}
	};
	walk(src);
	const bad: string[] = [];
	for (const file of files) {
		const lines = readFileSync(file, 'utf8').split('\n');
		lines.forEach((line, i) => {
			if (/^\s*(\/\/|\*|\/\*)/.test(line)) return; // comments
			const at = `${relative(src, file).replace(/\\/g, '/')}:${i + 1}`;
			// a module-local symbol is a different key in a second copy of the module (use contextKey)
			if (/\bSymbol\(/.test(line)) bad.push(`${at} Symbol(…) – use contextKey()`);
			// a module-level instance of a class is a second, empty one in a second copy (use shared())
			if (/^export const \w+(\s*:\s*[\w<>]+)?\s*=\s*new \w+\(/.test(line) && /\.svelte\.ts$/.test(file)) bad.push(`${at} module-level new – use shared()`);
		});
	}
	assert.deepEqual(bad, []);
});
