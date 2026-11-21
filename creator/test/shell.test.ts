// The shell's pure parts (lib/shell): addresses ↔ routes, the tabs, this machine's settings.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { AREAS, addressOf, hrefOf, iconOf, resolveTyped, workspaceOf } from '../src/lib/shell/address.ts';
import { EMPTY, MAX_TABS, close, closeAll, closeOthers, readTabs, rename, retarget, visit } from '../src/lib/shell/tabs.ts';
import { DEFAULTS, clampScale, readSettings, readViews } from '../src/lib/shell/settings.ts';
import { PRESETS, availableTabs, panelSpecs, stageOf } from '../src/lib/shell/presets.ts';
import { back, clampSplit, dropPlan, isCanvas, leaving, arriving, readStage, send, splitIn } from '../src/lib/shell/stage.ts';

test('a route reads as the address of the item it edits; Home is none', () => {
	assert.equal(addressOf('/'), null);
	assert.equal(addressOf('/toons'), null);
	assert.equal(addressOf('/catpope'), 'toon:catpope');
	for (const sub of ['credits', 'script', 'toon']) assert.equal(addressOf(`/catpope/${sub}`), 'toon:catpope', sub);
	assert.equal(addressOf('/catpope/season/1'), 'season:catpope:1');
	assert.equal(addressOf('/catpope/season/1/cover'), 'season:catpope:1');
	assert.equal(addressOf('/catpope/chapter/3'), 'chapter:catpope:3');
	assert.equal(addressOf('/catpope/chapter/3/script'), 'chapter:catpope:3');
	assert.equal(addressOf('/catpope/chapter/3/storyboard'), 'storyboard:catpope:3');
	assert.equal(addressOf('/characters'), AREAS.characters);
	assert.equal(addressOf('/characters', '?id=catpope'), 'character:catpope');
	assert.equal(addressOf('/characters', '?tab=moods'), AREAS.characters, 'the mood words are a panel tab, not a page');
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
	s = visit(s, 'toon:catpope', '/catpope/credits');
	s = visit(s, 'character:catpope', '/characters?id=catpope');
	assert.deepEqual(s.tabs.map((t) => t.address), ['toon:catpope', 'character:catpope', 'chapter:catpope:3']);
	assert.equal(s.front, 'character:catpope');
	assert.equal(s.tabs[0].href, '/catpope/credits', 'the tab keeps the page it was last on');
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

test('machine settings: day by default (owner 2026-11-21), every field checked', () => {
	assert.deepEqual(readSettings(undefined), DEFAULTS);
	assert.equal(DEFAULTS.theme, 'day');
	assert.deepEqual(readSettings({ theme: 'night', scale: 9, reduceMotion: 'yes', reopenTabs: false }), { ...DEFAULTS, theme: 'night', scale: 1.5, reopenTabs: false });
	assert.equal(readSettings({ theme: 'dusk' }).theme, 'day');
	assert.equal(clampScale(1.04), 1);
	assert.equal(clampScale(0.1), 0.8);
	assert.equal(clampScale(NaN), 1);
});

const TABS = ['toon', 'tree', 'inspector', 'kanon', 'moods', 'flow', 'preview', 'problems', 'timeline', 'library', 'settings'].map((id) => ({ id, name: id, icon: 'x', about: '' }));

test('a workspace is a preset: its panels hold only the tabs with content, Library and Settings everywhere', () => {
	const specs = panelSpecs('chapter', availableTabs(['tree', 'flow', 'problems'], 'catpope'), TABS);
	const ids = specs.map((p) => p.id);
	assert.deepEqual(ids, ['left', 'right', 'library', 'settings']);
	assert.deepEqual(specs[0].tabs, ['toon', 'tree']);
	assert.deepEqual(specs[1].tabs, ['problems', 'flow'], 'the inspector without content is not offered');
	assert.ok(!ids.includes('preview'), 'a panel without a tab with content goes');
	assert.equal(specs.find((p) => p.id === 'settings')?.place.closed, true);
});

test('a tab with content that no preset panel holds gets a closed panel of its own', () => {
	const specs = panelSpecs('map', availableTabs(['tree', 'flow'], null), TABS);
	const flow = specs.find((p) => p.id === 'flow');
	assert.deepEqual(flow?.tabs, ['flow']);
	assert.equal(flow?.place.closed, true);
	assert.ok(!specs.some((p) => p.tabs?.includes('toon')), 'the Toon only with a toon');
});

test('every workspace kind has a preset; the stage holds the person’s tab, else the preset’s', () => {
	for (const kind of ['home', 'toon', 'chapter', 'storyboard', 'character', 'catalog', 'map', 'scene', 'flow', 'sound', 'text', 'help']) assert.ok(PRESETS[kind as keyof typeof PRESETS], kind);
	assert.deepEqual(stageOf('chapter', {}), []);
	assert.deepEqual(stageOf('chapter', { stage: ['flow'] }), ['flow']);
	assert.deepEqual(stageOf('chapter', { stage: [] }), []);
	// a preset without `main`: its own view is the stage, full-bleed under the bar
	for (const [kind, preset] of Object.entries(PRESETS)) if (!preset.main) assert.deepEqual(stageOf(kind, {}), [], kind);
});

test('taking the stage: ◱ alone, a drop splits, never more than two', () => {
	assert.deepEqual(send([], 'flow'), ['flow']);
	assert.deepEqual(send(['flow'], 'flow'), [], 'pressed again: back');
	assert.deepEqual(send(['flow', 'preview'], 'problems'), ['problems'], 'this one alone; the others fly back');
	assert.deepEqual(splitIn(['flow'], 'preview', 'right'), ['flow', 'preview']);
	assert.deepEqual(splitIn(['flow'], 'preview', 'left'), ['preview', 'flow']);
	assert.deepEqual(splitIn(['flow', 'preview'], 'problems', 'left'), ['problems', 'preview'], 'replaces that half');
	assert.deepEqual(splitIn([], 'flow', 'left'), ['flow']);
	assert.deepEqual(back(['flow', 'preview']), ['flow'], 'Esc: the last one');
	assert.deepEqual(back(['flow', 'preview'], 'flow'), ['preview'], 'the other keeps the whole stage');
	assert.deepEqual(leaving(['flow', 'preview'], ['problems']), ['flow', 'preview']);
	assert.deepEqual(arriving(['flow'], ['flow', 'preview']), ['preview']);
	assert.ok(isCanvas('flow') && isCanvas('preview') && !isCanvas('problems'));
});

test('a tab dragged over the stage: takes it, splits a half, replaces a half', () => {
	assert.deepEqual(dropPlan([], 'flow', 0.2), { kind: 'take' });
	assert.deepEqual(dropPlan(['flow'], 'preview', 0.7), { kind: 'split', side: 'right' });
	assert.deepEqual(dropPlan(['flow', 'preview'], 'problems', 0.1), { kind: 'replace', side: 'left', out: 'flow' });
	assert.equal(dropPlan(['flow'], 'flow', 0.5), null);
	assert.equal(clampSplit(0.9), 0.75);
	assert.equal(clampSplit(Number.NaN), 0.5);
	assert.deepEqual(readStage(['a', 'a', 3, 'b', 'c']), ['a', 'b']);
	assert.equal(readStage('flow'), undefined);
});

test('view settings: per workspace, every field checked', () => {
	assert.deepEqual(readViews({ chapter: { stage: ['flow', 'preview'], split: 0.9, clear: false }, map: { stage: 3, clear: 'no' }, bad: 1 }), { chapter: { stage: ['flow', 'preview'], split: 0.75, clear: false }, map: {} });
	assert.deepEqual(readViews([]), {});
	assert.deepEqual(readSettings({ views: { home: { clear: true } } }).views, { home: { clear: true } });
});
