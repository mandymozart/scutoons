// deck:ci's changed tier (#604): node --experimental-transform-types --test scripts/deck-ci-scope.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { closure, owner, scope, type Crate } from './deck-ci-scope.ts';

const C: Crate[] = [
	{ name: 'deck-core', dir: 'deck/core', deps: [] },
	{ name: 'deck-kit', dir: 'deck/kit', deps: ['deck-core'] },
	{ name: 'deck-native', dir: 'deck/native', deps: ['deck-core'] },
	{ name: 'deck-editor', dir: 'deck/editor', deps: ['deck-kit', 'deck-native'] },
	{ name: 'deck-update', dir: 'deck/update', deps: [] },
	{ name: 'adapters-core', dir: 'deck/adapters/core', deps: [] },
	{ name: 'adapters-midi', dir: 'deck/adapters/midi', deps: ['adapters-core'] },
	{ name: 'deck-module', dir: 'deck/module', deps: [] },
	{ name: 'walk-module', dir: 'deck/modules/walk', deps: ['deck-module'] }
];

test('a file belongs to the deepest crate dir it lies under', () => {
	assert.equal(owner('deck/adapters/midi/src/lib.rs', C)?.name, 'adapters-midi');
	assert.equal(owner('deck/adapters/core/src/lib.rs', C)?.name, 'adapters-core');
	assert.equal(owner('deck/README.md', C), undefined);
});

test('the closure holds every crate that depends on a touched one', () => {
	assert.deepEqual(closure(['deck-native'], C), ['deck-editor', 'deck-native']);
	assert.deepEqual(closure(['adapters-core'], C), ['adapters-core', 'adapters-midi']);
});

test('a leaf change runs its crate and its users only', () => {
	assert.deepEqual(scope(['deck/editor/src/main.rs'], C), { kind: 'crates', touched: ['deck-editor'], closure: ['deck-editor'] });
	assert.deepEqual(scope(['deck/native/src/a.rs', 'deck/update/src/b.rs'], C), { kind: 'crates', touched: ['deck-native', 'deck-update'], closure: ['deck-editor', 'deck-native', 'deck-update'] });
});

test('a hub crate, the workspace files, a shared package or a stray file run the whole default tier', () => {
	assert.equal(scope(['deck/core/src/lib.rs'], C).kind, 'all');
	assert.equal(scope(['deck/kit/src/floats.rs'], C).kind, 'all');
	assert.equal(scope(['deck/Cargo.lock'], C).kind, 'all');
	assert.equal(scope(['shared/live-core/src/x.ts'], C).kind, 'all');
	assert.equal(scope(['scripts/deck-ci.mjs'], C).kind, 'all');
	assert.equal(scope(['deck/README.md'], C).kind, 'all');
});

test('a module crate on its own is a leaf; nothing the deck builds runs only the guards', () => {
	assert.equal(scope(['deck/modules/walk/src/lib.rs'], C).kind, 'crates');
	assert.deepEqual(scope(['viewer/src/app.ts', 'agents/research/X.md'], C), { kind: 'none' });
});
