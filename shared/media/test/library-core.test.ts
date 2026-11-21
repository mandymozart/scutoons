// The Library's core through the wasm the page loads (space-web's `Library`, deck-catalog `library::shelf` – its rules
// are tested in Rust: `cargo test -p deck-catalog`): the typed calls carry the JSON both ways. Skips when space-web is
// not built (npm run build:wasm -w shared/live-deck).
//   npm test -w @scutoons/media
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { libraryCore, libraryOpener, type LibraryModule } from '../src/library/core.ts';
import type { LibraryQuery } from '../src/library/types.ts';

const dir = new URL('../../live-deck/wasm/', import.meta.url);
const NO_CORE = existsSync(new URL('space_web_bg.wasm', dir)) && existsSync(new URL('space_web.js', dir)) ? '' : 'space-web is not built (npm run build:wasm -w shared/live-deck)';

async function core(): Promise<LibraryModule> {
	const mod = await import(new URL('space_web.js', dir).href);
	await mod.default({ module_or_path: readFileSync(new URL('space_web_bg.wasm', dir)) });
	return mod as LibraryModule;
}

const file = (path: string, extra: Record<string, unknown> = {}) => ({ path, size: 100, origin: 'imported', width: null, height: null, duration: null, ...extra });
const answer = {
	items: [
		{ id: 'a1', slug: 's1c1-slice-2', kind: 'image', title: '', tags: [], version: 1, usages: 2, updated_at: null, renditions: [file('catpope/s1/c1/s1c1-slice-2.webp', { width: 390, height: 624 }), file('catpope/s1/c1/s1c1-slice-2.2x.webp')] },
		{ id: 'a2', slug: 'bells', kind: 'audio', title: 'Bells', tags: [], version: 1, usages: 0, updated_at: null, renditions: [file('catpope/s1/c1/bells.128.mp3')] },
		{ id: 'a3', slug: 'catpope_avatar-angry', kind: 'image', title: '', tags: [], version: 2, usages: 0, updated_at: null, renditions: [file('characters/catpope/catpope_avatar-angry.webp', { width: 512, height: 512 })] }
	]
};
const query = (patch: Partial<LibraryQuery> = {}): LibraryQuery => ({ scope: 'all', grouping: 'use', sort: 'name', query: '', kind: null, special: null, origin: null, fit: null, single: false, now: 0, ...patch });

test('the core: families, the open item pinned, fits, selection, upload path', { skip: NO_CORE || false }, async () => {
	const lib = libraryCore(await core(), answer);
	const families = lib.families(false);
	assert.deepEqual(families.map((f) => [f.key, f.kind, f.versions.length]), [
		['catpope/s1/c1/bells', 'audio', 1],
		['catpope/s1/c1/s1c1-slice-2', 'image', 2],
		['characters/catpope/catpope_avatar-angry', 'image', 1]
	]);
	assert.equal(lib.families(true).length, 4);
	lib.setContext({ open: 'chapter:catpope:1', toon: 'catpope', uses: [{ path: 'characters/catpope/catpope_avatar-angry.webp', item: 'chapter:catpope:1', part: 'slice 1 · speaker', at: 's1' }] });
	const all = lib.view(query());
	assert.deepEqual(all.groups.map((g) => [g.what, g.pinned, g.folded, g.count]), [
		['open', true, false, 1],
		['unused', false, true, 1],
		['toon', false, true, 1]
	]);
	assert.deepEqual(lib.view(query({ scope: 'context' })).order, ['characters/catpope/catpope_avatar-angry']);
	assert.deepEqual(lib.uses('characters/catpope/catpope_avatar-angry')[0].part, 'slice 1 · speaker');
	assert.deepEqual(lib.takes('catpope/s1/c1/bells.128.mp3', { kinds: ['image'] }), { ok: false, wants: ['image'], got: 'audio', square: false });
	assert.equal(lib.startKind({ kinds: ['audio'] }), 'audio');
	assert.deepEqual(lib.select(['a'], 'a', 'c', ['a', 'b', 'c'], { shift: true }), { chosen: ['a', 'b', 'c'], anchor: 'a' });
	assert.equal(lib.uploadPath('New Bell.MP3'), 'catpope/s1/c1/new-bell.mp3');
	lib.free();
});

test('the open rule: where an address opens in the creator, or why not', { skip: NO_CORE || false }, async () => {
	const opener = libraryOpener(await core(), 'creator');
	assert.deepEqual(opener.open('place:harbour'), { ok: true, address: 'place:harbour', workspace: 'map' });
	assert.deepEqual(opener.open('instrument:organ'), { ok: false, why: 'instruments have no workspace yet' });
	const kinds = opener.kinds();
	assert.deepEqual(kinds.find((k) => k.kind === 'character'), { kind: 'character', words: 'characters', workspace: 'character', opens: 'the character workspace' });
	assert.ok(kinds.some((k) => k.kind === 'show' && k.why));
});
