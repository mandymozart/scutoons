// The "Continue reading" shortcut's target (lib/continue-reading.ts, agents/research/PWA.md §1 – manifest shortcuts):
//   npm test -w viewer
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { continuePath } from '../src/lib/continue-reading.ts';

const cp = (uri: string, updated: string, extra: Record<string, unknown> = {}) => ({
	uri,
	isComplete: false,
	created_at: updated,
	updated_at: updated,
	...extra
});

test('nothing read: stay on the start page', () => {
	assert.equal(continuePath([]), null);
	assert.equal(continuePath(null), null);
	assert.equal(continuePath([cp('catpope/slice/3', '2026-10-01T00:00:00Z')]), null, 'not a chapter checkpoint');
});

test('the chapter opened last, of any toon, where the reader stopped', () => {
	const progress = [
		cp('catpope/chapter/1', '2026-10-01T10:00:00Z', { position: 0.4, group: 0, slice: 3 }),
		cp('other/chapter/2', '2026-10-02T10:00:00Z', { position: 0.5, group: 1, slice: 0 }),
		cp('catpope/chapter/2', '2026-09-30T10:00:00Z', { position: 0.1 })
	];
	assert.equal(continuePath(progress), '/other/chapter/2?resume');
});

test('without a stored spot: the chapter from its start', () => {
	assert.equal(continuePath([cp('catpope/chapter/3', '2026-10-01T10:00:00Z', { position: 0.2 })]), '/catpope/chapter/3');
});

test('a finished chapter: its toon page (the start button offers the next one)', () => {
	assert.equal(continuePath([cp('catpope/chapter/3', '2026-10-01T10:00:00Z', { position: 1, isComplete: true })]), '/catpope');
	assert.equal(continuePath([cp('catpope/chapter/0', '2026-10-01T10:00:00Z', { isComplete: true })]), '/catpope', 'an old checkpoint without position');
});

test('odd uris are ignored, never turned into a path', () => {
	assert.equal(continuePath([cp('../evil/chapter/1', '2026-10-01T10:00:00Z', { position: 0.2 })]), null);
	assert.equal(continuePath([cp('a b/chapter/1', '2026-10-01T10:00:00Z')]), null);
});
