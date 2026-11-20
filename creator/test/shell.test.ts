// The shell's pure parts (lib/shell): addresses ↔ routes, the tabs, this machine's settings.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { AREAS, addressOf, hrefOf, iconOf, resolveTyped, workspaceOf } from '../src/lib/shell/address.ts';
import { EMPTY, MAX_TABS, close, closeAll, closeOthers, readTabs, rename, retarget, visit } from '../src/lib/shell/tabs.ts';
import { DEFAULTS, clampScale, readSettings } from '../src/lib/shell/settings.ts';

test('a route reads as the address of the item it edits; Home is none', () => {
	assert.equal(addressOf('/'), null);
	assert.equal(addressOf('/toons'), null);
	assert.equal(addressOf('/catpope'), 'toon:catpope');
	for (const sub of ['cast', 'credits', 'script', 'toon']) assert.equal(addressOf(`/catpope/${sub}`), 'toon:catpope', sub);
	assert.equal(addressOf('/catpope/season/1'), 'season:catpope:1');
	assert.equal(addressOf('/catpope/season/1/cover'), 'season:catpope:1');
	assert.equal(addressOf('/catpope/chapter/3'), 'chapter:catpope:3');
	assert.equal(addressOf('/catpope/chapter/3/script'), 'chapter:catpope:3');
	assert.equal(addressOf('/catpope/chapter/3/storyboard'), 'storyboard:catpope:3');
	assert.equal(addressOf('/characters'), AREAS.characters);
	assert.equal(addressOf('/characters', '?id=catpope'), 'character:catpope');
	assert.equal(addressOf('/characters', '?tab=moods'), AREAS.moods);
	assert.equal(addressOf('/map', '?location=crypt'), AREAS.map);
	assert.equal(addressOf('/scenes'), AREAS.scenes);
	assert.equal(addressOf('/scenes/duet'), 'scene:duet');
	assert.equal(addressOf('/flow'), AREAS.flow);
	assert.equal(addressOf('/catalog/items'), 'catalog:items');
	assert.equal(addressOf('/catalog/items', '?id=camera'), 'item:camera');
	assert.equal(addressOf('/catalog/recipes', '?id=repair'), 'recipe:repair');
	assert.equal(addressOf('/catalog/items/camera/collage'), 'item:camera');
	assert.equal(addressOf('/edit/rig:character.catpope'), 'rig:character.catpope');
	assert.equal(addressOf('/edit/nonsense'), null);
	assert.equal(addressOf('/kanon/character.catpope'), 'kanon:character.catpope');
});

test('an address leads back to its page, and every route round-trips', () => {
	for (const [path, search] of [
		['/catpope', ''],
		['/catpope/season/1', ''],
		['/catpope/chapter/3', ''],
		['/catpope/chapter/3/storyboard', ''],
		['/characters', '?id=catpope'],
		['/characters', '?tab=wanted'],
		['/map', ''],
		['/scenes/duet', ''],
		['/flow', ''],
		['/catalog/items', '?id=camera'],
		['/catalog/recipes', ''],
		['/kanon/character.catpope', '']
	]) {
		const address = addressOf(path, search)!;
		assert.equal(addressOf(...(hrefOf(address)!.split(/(?=\?)/) as [string, string?])), address, `${path}${search}`);
	}
	assert.equal(hrefOf('place:crypt'), '/map?location=crypt');
	assert.equal(hrefOf('bogus:thing'), null);
});

test('every address has a workspace and an icon; a typed address resolves', () => {
	assert.equal(workspaceOf('chapter:catpope:3'), 'chapter');
	assert.equal(workspaceOf('season:catpope:1'), 'toon');
	assert.equal(workspaceOf('item:camera'), 'catalog');
	assert.equal(workspaceOf('module:walk'), 'sound');
	assert.equal(workspaceOf('nothing'), null);
	assert.equal(iconOf('character:catpope'), 'smile');
	assert.equal(resolveTyped(' chapter:catpope:2 '), '/catpope/chapter/2');
	assert.equal(resolveTyped('catpope'), null);
});

test('visiting opens a tab right of the front one, or brings its tab to the front', () => {
	let s = visit(EMPTY, 'toon:catpope', '/catpope', 'CatPope');
	s = visit(s, 'chapter:catpope:3', '/catpope/chapter/3');
	s = visit(s, 'toon:catpope', '/catpope/cast');
	s = visit(s, 'character:catpope', '/characters?id=catpope');
	assert.deepEqual(s.tabs.map((t) => t.address), ['toon:catpope', 'character:catpope', 'chapter:catpope:3']);
	assert.equal(s.front, 'character:catpope');
	assert.equal(s.tabs[0].href, '/catpope/cast', 'the tab keeps the page it was last on');
	assert.equal(visit(s, null, '/').front, null, 'Home keeps the tabs');
	assert.equal(visit(s, null, '/').tabs.length, 3);
});

test('a pick inside the page retargets the front tab, unless the item has its own', () => {
	let s = visit(EMPTY, AREAS.characters, '/characters');
	s = retarget(s, 'character:catpope', '/characters?id=catpope');
	assert.deepEqual(s.tabs.map((t) => t.address), ['character:catpope']);
	s = visit(s, 'chapter:catpope:3', '/catpope/chapter/3');
	s = visit(s, 'character:yellowmouse', '/characters?id=yellowmouse');
	s = retarget(s, 'character:catpope', '/characters?id=catpope');
	assert.equal(s.front, 'character:catpope');
	assert.equal(s.tabs.length, 3);
	assert.equal(rename(s, 'character:catpope', 'Catpope').tabs[0].label, 'Catpope');
});

test('closing hands the front over to the right, then the left, then Home', () => {
	let s = EMPTY;
	for (const a of ['toon:a', 'toon:b', 'toon:c']) s = visit(s, a, `/${a.slice(5)}`);
	s = visit(s, 'toon:b', '/b');
	assert.equal(close(s, 'toon:b').front, 'toon:c');
	assert.equal(close(visit(s, 'toon:c', '/c'), 'toon:c').front, 'toon:b');
	assert.equal(close(s, 'toon:a').front, 'toon:b', 'a tab behind closes without moving the front');
	assert.deepEqual(closeOthers(s).tabs.map((t) => t.address), ['toon:b']);
	assert.deepEqual(closeAll(), EMPTY);
	assert.equal(close(close(close(s, 'toon:a'), 'toon:b'), 'toon:c').front, null);
});

test('the tabs stay below the limit and a broken store opens empty', () => {
	let s = EMPTY;
	for (let i = 0; i < MAX_TABS + 5; i++) s = visit(s, `toon:t${i}`, `/t${i}`);
	assert.equal(s.tabs.length, MAX_TABS);
	assert.equal(s.front, `toon:t${MAX_TABS + 4}`);
	assert.deepEqual(readTabs(null), EMPTY);
	assert.deepEqual(readTabs({ tabs: [{ address: 'toon:a', href: 'https://x' }, { address: 'toon:b', href: '/b' }, { address: 'toon:b', href: '/b2' }], front: 'toon:a' }), {
		tabs: [{ address: 'toon:b', href: '/b', label: 'toon:b' }],
		front: null
	});
});

test('machine settings: night by default, every field checked', () => {
	assert.deepEqual(readSettings(undefined), DEFAULTS);
	assert.equal(DEFAULTS.theme, 'night');
	assert.deepEqual(readSettings({ theme: 'day', scale: 9, reduceMotion: 'yes', reopenTabs: false }), { ...DEFAULTS, theme: 'day', scale: 1.5, reopenTabs: false });
	assert.equal(clampScale(1.04), 1);
	assert.equal(clampScale(0.1), 0.8);
	assert.equal(clampScale(NaN), 1);
});
