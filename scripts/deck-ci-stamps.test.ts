// the key and skip logic of deck:ci's green stamps (#524): node --experimental-transform-types --test scripts/deck-ci-stamps.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, utimesSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { WATCH, Stamps, contentKey, decide, parseLsTree, parseStatus, watched, type Stamp } from './deck-ci-stamps.ts';

const ls = (files: Record<string, string>) =>
	Object.entries(files)
		.map(([path, sha]) => `100644 blob ${sha}\t${path}`)
		.join('\0') + '\0';

const HEAD = {
	'deck/core/src/lib.rs': 'aaa1',
	'deck/Cargo.lock': 'aaa2',
	'shared/live-deck/src/client.ts': 'aaa3',
	'scripts/deck-ci.mjs': 'aaa4',
	'viewer/src/app.ts': 'aaa5',
	'tools/clock-measure/src/main.ts': 'aaa6',
	'package-lock.json': 'aaa7'
};
const key = (files: Record<string, string>, over: Partial<Parameters<typeof contentKey>[0]> = {}) =>
	contentKey({ step: '🦀 tests', tier: 'default', toolchain: 'rustc 1.99.1', tree: parseLsTree(ls(files)), paths: WATCH, ...over });
const stamp: Stamp = { step: '🦀 tests', tier: 'default', date: '2026-11-13T10:00:00.000Z', seconds: 1958, head: 'abc', toolchain: 'rustc 1.99.1' };

test('watched: folders, files and prefix globs; no neighbour folders', () => {
	assert.ok(watched('deck/core/src/lib.rs', WATCH));
	assert.ok(watched('shared/live/src/protocol.ts', WATCH));
	assert.ok(watched('scripts/check-deck-deps.test.mjs', WATCH));
	assert.ok(watched('scripts/deck-ci-stamps.test.ts', WATCH));
	assert.ok(!watched('shared/liveness/x.ts', WATCH));
	assert.ok(!watched('viewer/src/app.ts', WATCH));
	assert.ok(!watched('deckhand.txt', WATCH));
});

test('parseLsTree and parseStatus read git -z output, renames with both sides', () => {
	const tree = parseLsTree(ls({ 'deck/a b.rs': 'f00' }));
	assert.equal(tree.get('deck/a b.rs'), '100644 f00');
	assert.deepEqual(parseStatus(' M deck/core/src/lib.rs\0?? deck/new.rs\0R  deck/to.rs\0deck/from.rs\0'), [
		'deck/core/src/lib.rs',
		'deck/new.rs',
		'deck/to.rs',
		'deck/from.rs'
	]);
	assert.deepEqual(parseStatus(''), []);
});

test('the key: the same tree, step, tier and toolchain give the same key', () => {
	assert.equal(key(HEAD), key({ ...HEAD }));
	assert.equal(key(HEAD).length, 64);
});

test('the key: a committed change under deck/ (or the script) changes it, a change outside the watched paths does not', () => {
	assert.notEqual(key(HEAD), key({ ...HEAD, 'deck/core/src/lib.rs': 'bbb1' }));
	assert.notEqual(key(HEAD), key({ ...HEAD, 'deck/Cargo.lock': 'bbb2' }));
	assert.notEqual(key(HEAD), key({ ...HEAD, 'scripts/deck-ci.mjs': 'bbb4' }));
	assert.notEqual(key(HEAD), key({ ...HEAD, 'deck/new.rs': 'bbb8' }), 'a new file');
	const { ['shared/live-deck/src/client.ts']: _, ...without } = HEAD;
	assert.notEqual(key(HEAD), key(without), 'a deleted file');
	assert.equal(key(HEAD), key({ ...HEAD, 'viewer/src/app.ts': 'bbb5' }), 'the viewer is not watched');
	assert.equal(key(HEAD), key({ ...HEAD, 'package-lock.json': 'bbb7' }), 'not a cargo step’s input');
});

test('the key: a step’s own paths count for that step only', () => {
	const paths = [...WATCH, 'tools/clock-measure', 'package-lock.json'];
	const changed = { ...HEAD, 'tools/clock-measure/src/main.ts': 'bbb6' };
	assert.notEqual(key(HEAD, { paths }), key(changed, { paths }));
	assert.equal(key(HEAD), key(changed));
});

test('the key: step, tier and toolchain are part of it', () => {
	assert.notEqual(key(HEAD), key(HEAD, { step: '🦀 clippy' }));
	assert.notEqual(key(HEAD), key(HEAD, { tier: 'full' }));
	assert.notEqual(key(HEAD), key(HEAD, { toolchain: 'rustc 1.100.0' }));
});

test('decide: a stamp on a clean tree skips; none runs and stamps', () => {
	assert.deepEqual(decide({ dirty: [], paths: WATCH, stampable: true, fresh: false, found: stamp }), { action: 'skip', stamp });
	assert.deepEqual(decide({ dirty: [], paths: WATCH, stampable: true, fresh: false }), { action: 'run', stamp: true, why: 'no stamp for this tree' });
});

test('decide: an uncommitted or untracked change under a watched path is never skipped and never stamped', () => {
	for (const dirty of [['deck/core/src/lib.rs'], ['deck/untracked.rs'], ['scripts/deck-ci.mjs']]) {
		const p = decide({ dirty, paths: WATCH, stampable: true, fresh: false, found: stamp });
		assert.equal(p.action, 'run');
		assert.equal(p.action === 'run' && p.stamp, false);
	}
});

test('decide: a dirty WATCH path stops every step, a dirty own path only its step, a dirty elsewhere none', () => {
	const own = [...WATCH, 'tools/clock-measure'];
	assert.equal(decide({ dirty: ['deck/x.rs'], paths: own, stampable: true, fresh: false, found: stamp }).action, 'run');
	assert.equal(decide({ dirty: ['tools/clock-measure/src/main.ts'], paths: own, stampable: true, fresh: false, found: stamp }).action, 'run');
	assert.equal(decide({ dirty: ['tools/clock-measure/src/main.ts'], paths: WATCH, stampable: true, fresh: false, found: stamp }).action, 'skip');
	assert.equal(decide({ dirty: ['viewer/src/app.ts'], paths: WATCH, stampable: true, fresh: false, found: stamp }).action, 'skip');
});

test('decide: --fresh runs and stamps; a never-stamped step always runs', () => {
	assert.deepEqual(decide({ dirty: [], paths: WATCH, stampable: true, fresh: true, found: stamp }), { action: 'run', stamp: true, why: 'fresh' });
	assert.deepEqual(decide({ dirty: [], paths: WATCH, stampable: false, fresh: false, found: stamp }), { action: 'run', stamp: false, why: 'never stamped' });
});

test('Stamps: write, find, prune', () => {
	const dir = mkdtempSync(join(tmpdir(), 'deck-ci-stamps-'));
	try {
		const s = new Stamps(join(dir, 'deck-ci-stamps'));
		assert.equal(s.find('k1'), undefined);
		assert.equal(s.prune(), 0, 'no folder yet');
		s.write('k1', stamp);
		s.write('k2', stamp);
		assert.deepEqual(s.find('k1'), stamp);
		const old = (Date.now() - 40 * 864e5) / 1000;
		utimesSync(join(dir, 'deck-ci-stamps', 'k2.json'), old, old);
		assert.equal(s.prune(30), 1);
		assert.deepEqual(s.find('k1'), stamp);
		assert.equal(s.find('k2'), undefined);
	} finally {
		rmSync(dir, { recursive: true, force: true });
	}
});
