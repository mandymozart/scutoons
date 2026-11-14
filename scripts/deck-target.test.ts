// scripts/deck-target.ts (#526): the worktree id, the written cargo config, what a folder in the root is, what a clear deletes.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { configToml, findSccache, kindOf, sizeOf, toClear, worktreeId, type Entry } from './deck-target.ts';

test('a linked worktree is named by its git dir, the main checkout has no id', () => {
	assert.equal(worktreeId('C:/repo/.git/worktrees/agent-a1', 'C:/repo/.git'), 'agent-a1');
	assert.equal(worktreeId('C:\\repo\\.git', 'c:/repo/.git'), null);
	assert.equal(worktreeId('/home/x/repo/.git', '/home/x/repo/.git'), null);
});

test('the config sets the target dir; sccache and its env only when installed', () => {
	const withIt = configToml({ target: 'C:\\repo\\.git\\deck-target\\agent-a1', wrapper: 'C:\\u\\.cargo\\bin\\sccache.exe', sccacheDir: 'C:\\repo\\.git\\deck-target\\sccache' });
	assert.match(withIt, /^# Written by scripts\/deck-target\.ts \(#526\)/);
	assert.match(withIt, /\[build\]\ntarget-dir = "C:\/repo\/\.git\/deck-target\/agent-a1"\nrustc-wrapper = "C:\/u\/\.cargo\/bin\/sccache\.exe"\n/);
	assert.match(withIt, /\[env\]\nSCCACHE_DIR = "C:\/repo\/\.git\/deck-target\/sccache"\nSCCACHE_CACHE_SIZE = "\d+G"\nSCCACHE_IDLE_TIMEOUT = "\d+"\n$/);
	const without = configToml({ target: '/r/.git/deck-target/w', wrapper: null, sccacheDir: '/r/.git/deck-target/sccache' });
	assert.doesNotMatch(without, /rustc-wrapper|\[env\]|SCCACHE/);
	assert.match(without, /target-dir = "\/r\/\.git\/deck-target\/w"\n$/);
});

test('sccache is found on PATH or in cargo home, else null', () => {
	const dir = mkdtempSync(join(tmpdir(), 'deck-target-'));
	const exe = process.platform === 'win32' ? 'sccache.exe' : 'sccache';
	assert.equal(findSccache({ PATH: dir, CARGO_HOME: join(dir, 'none') }), null);
	mkdirSync(join(dir, 'bin'));
	writeFileSync(join(dir, 'bin', exe), '');
	assert.equal(findSccache({ PATH: '', CARGO_HOME: dir }), join(dir, 'bin', exe));
	assert.equal(findSccache({ PATH: join(dir, 'bin'), CARGO_HOME: join(dir, 'none') }), join(dir, 'bin', exe));
});

test('a folder in the root is this worktree, an alive one, a stale one or sccache', () => {
	const alive = new Set(['agent-a1', 'agent-b2']);
	assert.equal(kindOf('sccache', 'agent-a1', alive), 'sccache');
	assert.equal(kindOf('agent-a1', 'agent-a1', alive), 'this');
	assert.equal(kindOf('agent-b2', 'agent-a1', alive), 'alive');
	assert.equal(kindOf('agent-gone', 'agent-a1', alive), 'stale');
	assert.equal(kindOf('agent-b2', null, alive), 'alive');
});

test('a clear deletes the stale dirs; --all the root; --legacy and --main only by their flag', () => {
	const e = (name: string, kind: Entry['kind']): Entry => ({ name, path: `/r/${name}`, kind, bytes: 1 });
	const entries = [e('agent-a1', 'this'), e('agent-b2', 'alive'), e('agent-gone', 'stale'), e('sccache', 'sccache')];
	const legacy = ['/w/agent-b2/deck/target'];
	const none = { all: false, legacy: false, main: false };
	assert.deepEqual(toClear(entries, none, legacy, '/main/deck/target'), ['/r/agent-gone']);
	assert.deepEqual(toClear(entries, { ...none, all: true }, legacy, '/main/deck/target'), ['/r/agent-a1', '/r/agent-b2', '/r/agent-gone', '/r/sccache']);
	assert.deepEqual(toClear(entries, { ...none, legacy: true }, legacy, '/main/deck/target'), ['/r/agent-gone', ...legacy]);
	assert.deepEqual(toClear(entries, { ...none, main: true }, legacy, '/main/deck/target'), ['/r/agent-gone', '/main/deck/target']);
	assert.deepEqual(toClear(entries, { ...none, main: true }, legacy, null), ['/r/agent-gone']);
});

test('a folder size counts every file once', async () => {
	const dir = mkdtempSync(join(tmpdir(), 'deck-target-size-'));
	mkdirSync(join(dir, 'a', 'b'), { recursive: true });
	writeFileSync(join(dir, 'x'), Buffer.alloc(100));
	writeFileSync(join(dir, 'a', 'b', 'y'), Buffer.alloc(23));
	assert.equal(await sizeOf(dir), 123);
	assert.equal(await sizeOf(join(dir, 'missing')), 0);
});
