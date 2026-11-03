// check-core-only's self-tests (#357): a violating file fails, a clean one passes, the baseline only shrinks.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { compare, ratchet, scan, shrunk } from './check-core-only.mjs';

const fixture = (n) => readFileSync(new URL(`./fixtures/core-only/${n}`, import.meta.url), 'utf8');

test('a violating file: every shape of deck logic is a hit, an escape without a ticket is refused', () => {
	const s = scan(fixture('violating.ts'));
	assert.deepEqual(s.counts, { 'scene-literal': 1, 'tempo-math': 2, 'midi-bytes': 2, 'core-text': 1, clamp: 1, random: 1, 'kind-branch': 1 });
	assert.equal(s.bad.length, 1, 'core-only-ok without #N');
	const errors = compare({ 'x.ts': s.counts }, {});
	assert.equal(errors.length, 7);
	assert.match(errors.join('\n'), /x\.ts: 1 × scene-literal – new/);
});

test('a clean file: comments, frame rates, masks, the core\'s line and a ticketed escape pass', () => {
	const s = scan(fixture('clean.ts'));
	assert.deepEqual(s.counts, {});
	assert.deepEqual(s.bad, []);
	assert.deepEqual(s.excused.map((e) => [e.line, e.pattern]), [[8, 'midi-bytes']]);
});

test('the baseline: more hits than it allows fail; it only shrinks against main, unless the owner approved', () => {
	const baseline = { 'a.ts': { clamp: { count: 2, ticket: '#357' } } };
	assert.deepEqual(compare({ 'a.ts': { clamp: 2 } }, baseline), []);
	assert.match(compare({ 'a.ts': { clamp: 3 } }, baseline)[0], /3 × clamp, the baseline allows 2 \(#357\)/);
	// the file shrank: the baseline can follow (never up)
	assert.deepEqual(shrunk({ 'a.ts': { clamp: 1 } }, baseline), { 'a.ts': { clamp: { count: 1, ticket: '#357' } } });
	assert.deepEqual(shrunk({}, baseline), {});
	// an agent grows the baseline to get past: refused, unless a commit carries the owner's approval
	const grown = { 'a.ts': { clamp: { count: 3, ticket: '#357' } }, 'b.ts': { random: { count: 1, ticket: '#999' } } };
	const errors = ratchet(baseline, grown, false);
	assert.equal(errors.length, 2);
	assert.match(errors.join('\n'), /a\.ts clamp grew 2 → 3/);
	assert.match(errors.join('\n'), /b\.ts random is new/);
	assert.deepEqual(ratchet(baseline, grown, true), []);
	assert.deepEqual(ratchet(baseline, { 'a.ts': { clamp: { count: 1, ticket: '#357' } } }, false), []);
});
