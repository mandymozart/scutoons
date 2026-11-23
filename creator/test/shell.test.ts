// The shell's pure parts (lib/shell): addresses ↔ routes, the tabs, this machine's settings.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { AREAS, addressOf, hrefOf, iconOf, isGone, pickedInArea, resolveTyped, startsAtTop, scopeOf, workspaceOf } from '../src/lib/shell/address.ts';
import { EMPTY, MAX_TABS, close, closeAll, closeOthers, keepIn, pageLabel, readTabs, rename, retarget, stepTab, visit } from '../src/lib/shell/tabs.ts';
import { NO_USE, ROW, entered, fits, readUse, startKinds, workspaceClick, workspaceFor, workspaceRow } from '../src/lib/shell/workspaces.ts';
import { DEFAULTS, clampScale, readSettings, readViews } from '../src/lib/shell/settings.ts';
import { PRESETS, availableTabs, panelSpecs, presetStart, tabIdOf } from '../src/lib/shell/presets.ts';
import { addSpore, hooksOf, removeSpore } from '../src/lib/editor/spores.ts';
import { clampSplit, isCanvas, freeRoom } from '../src/lib/shell/stage.ts';
import { ago, firstRing, greetName, isWelcome, rowStep, stepRing, workspaceCards } from '../src/lib/shell/dashboard.ts';
import { NO_CRAFTS, asksCrafts, readCrafts, showsTextLanguage } from '../src/lib/shell/crafts.ts';
import { NO_LAYOUTS, cleanName, fromAnswer, ownId, readChosen, readKept, shownWorkspace, startKind, startOf, toKept, withOwn, withSaved, without } from '../src/lib/shell/layouts.ts';
import { MAX_PICKS, isSearchKey, isSwitcherKey, rank, readPicks, recentFirst, remember, score, wordsIn, type LaunchEntry } from '../src/lib/shell/launcher.ts';

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
	assert.equal(addressOf('/characters', '?tab=wanted'), AREAS.characters, 'Wanted pictures is a Library filter now: the old address passes like the list');
	assert.equal(addressOf('/generate'), 'generator:new');
	assert.equal(addressOf('/map', '?location=crypt'), AREAS.map);
	assert.equal(addressOf('/scenes'), AREAS.scenes);
	assert.equal(addressOf('/scenes/duet'), 'scene:duet');
	assert.equal(addressOf('/experiences'), AREAS.experiences);
	assert.equal(addressOf('/experiences/poster-catpope'), 'experience:poster-catpope');
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
		['/generate', ''],
		['/map', ''],
		['/scenes/duet', ''],
		['/experiences', ''],
		['/experiences/poster-catpope', ''],
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
	assert.equal(workspaceOf('experience:poster-catpope'), 'experience');
	assert.equal(workspaceOf(AREAS.experiences), 'experience');
	assert.equal(iconOf('character:catpope'), 'smile');
	assert.equal(iconOf('experience:poster-catpope'), 'aperture');
	assert.equal(resolveTyped('experience:poster-catpope'), '/experiences/poster-catpope');
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

test('another item brought to the front starts at the top of the stage; Back and the first load keep the scroll', () => {
	const u = (p: string) => new URL(p, 'http://x');
	assert.equal(startsAtTop(u('/scenes/a'), u('/catalog/items?id=dago_hoodie'), 'goto'), true);
	assert.equal(startsAtTop(u('/catalog/items?id=camera'), u('/catalog/items?id=dago_hoodie'), 'link'), true);
	assert.equal(startsAtTop(u('/catalog/items?id=camera'), u('/catalog/items?id=camera&tab=x'), 'goto'), false);
	assert.equal(startsAtTop(u('/scenes/a'), u('/catalog/items?id=camera'), 'popstate'), false);
	assert.equal(startsAtTop(null, u('/scenes/a'), 'enter'), false);
});

test('an area\'s list tab that opens one of its items becomes that item, named by the page', () => {
	assert.equal(pickedInArea(AREAS.scenes, 'scene:avatar-home'), true);
	assert.equal(pickedInArea(AREAS.experiences, 'experience:poster'), true);
	assert.equal(pickedInArea(AREAS.scenes, 'experience:poster'), false);
	assert.equal(pickedInArea('scene:a', 'scene:b'), false);
	assert.equal(pickedInArea(AREAS.scenes, AREAS.scenes), false);
	assert.equal(pickedInArea(null, 'scene:a'), false);
	let s = visit(EMPTY, AREAS.scenes, '/scenes');
	s = retarget(s, 'scene:avatar-home', '/scenes/avatar-home');
	assert.deepEqual(s.tabs.map((t) => t.address), ['scene:avatar-home']);
	const crumbs = [{ href: '/scenes', label: 'Scenes' }, { href: '/scenes/avatar-home', label: 'Avatar home' }];
	assert.equal(pageLabel(crumbs, hrefOf('scene:avatar-home')), 'Avatar home');
	assert.equal(pageLabel(crumbs, '/scenes/other'), '');
	assert.equal(pageLabel(crumbs, null), '');
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
	assert.equal(DEFAULTS.gizmoAlways, false);
	assert.equal(readSettings({ gizmoAlways: true }).gizmoAlways, true);
	assert.equal(clampScale(1.04), 1);
	assert.equal(clampScale(0.1), 0.8);
	assert.equal(clampScale(NaN), 1);
});

const TABS = ['toon', 'tree', 'inspector', 'kanon', 'moods', 'flow', 'preview', 'log', 'timeline', 'library', 'settings'].map((id) => ({ id, name: id, icon: 'x', about: '' }));

test('a workspace is a preset: its panels hold only the tabs with content, Library, Settings and the Log everywhere', () => {
	const specs = panelSpecs('chapter', availableTabs(['tree', 'flow'], 'catpope'), TABS);
	const ids = specs.map((p) => p.id);
	assert.deepEqual(ids, ['left', 'right', 'library', 'settings']);
	assert.deepEqual(specs[0].tabs, ['toon', 'tree']);
	assert.deepEqual(specs[1].tabs, ['log', 'flow', 'weave'], 'the inspector without content is not offered; the Log and the Spores tab are the shell’s own');
	assert.ok(!ids.includes('preview'), 'a panel without a tab with content goes');
	assert.equal(specs.find((p) => p.id === 'settings')?.place.closed, true);
	assert.deepEqual(panelSpecs('experience', availableTabs(['tree', 'inspector', 'experience', 'scenes'], null), TABS).find((p) => p.id === 'right')?.tabs, ['inspector', 'experience', 'weave'], 'the XR workspace offers its weave (#763)');
});

test('a tab with content that no preset panel holds gets a closed panel of its own', () => {
	const specs = panelSpecs('map', availableTabs(['tree', 'flow'], null), TABS);
	const flow = specs.find((p) => p.id === 'flow');
	assert.deepEqual(flow?.tabs, ['flow']);
	assert.equal(flow?.place.closed, true);
	assert.ok(!specs.some((p) => p.tabs?.includes('toon')), 'the Toon only with a toon');
});

test('the Log is in every workspace; its old id Problems still names it', () => {
	for (const kind of Object.keys(PRESETS)) {
		const specs = panelSpecs(kind as keyof typeof PRESETS, availableTabs([], null), TABS);
		assert.ok(specs.some((p) => p.tabs?.includes('log')), kind);
	}
	assert.equal(panelSpecs('scene', availableTabs([], null), TABS).find((p) => p.id === 'log')?.place.closed, true, 'a workspace without it in its preset: a closed panel');
	assert.equal(tabIdOf('problems'), 'log');
});

test('an address as the Log’s scope: a toon’s items under the toon, a chapter under its season', () => {
	const season = (toon: string, chapter: number) => (toon === 'catpope' && chapter === 3 ? 1 : null);
	assert.equal(scopeOf('toon:catpope'), 'toon/catpope');
	assert.equal(scopeOf('season:catpope:2'), 'toon/catpope/season/2');
	assert.equal(scopeOf('chapter:catpope:3', season), 'toon/catpope/season/1/chapter/3');
	assert.equal(scopeOf('chapter:catpope:4', season), 'toon/catpope/chapter/4', 'the season not known: none');
	assert.equal(scopeOf('storyboard:catpope:3', season), 'toon/catpope/season/1/chapter/3/storyboard');
	assert.equal(scopeOf('character:catpope'), 'character/catpope');
	assert.equal(scopeOf(null), null);
});

test('every workspace kind has a preset; canvas views fill their cell', () => {
	for (const kind of ['home', 'toon', 'chapter', 'storyboard', 'character', 'catalog', 'map', 'scene', 'flow', 'sound', 'text', 'generate', 'help']) assert.ok(PRESETS[kind as keyof typeof PRESETS], kind);
	assert.ok(isCanvas('flow') && isCanvas('preview') && isCanvas('sheet') && isCanvas('3d') && isCanvas('canvas') && !isCanvas('log') && !isCanvas('expressions'));
	assert.equal(clampSplit(0.9), 0.75);
	assert.equal(clampSplit(Number.NaN), 0.5);
});

test('the free room: between the left and the right column of panels', () => {
	assert.deepEqual(freeRoom([], 1000), { left: 0, right: 1000 });
	assert.deepEqual(freeRoom([{ x: 0, w: 200 }, { x: 700, w: 300 }, { x: 650, w: 350 }], 1000), { left: 200, right: 650 });
	assert.deepEqual(freeRoom([{ x: 0, w: 600 }, { x: 550, w: 450 }], 1000), { left: 600, right: 600 }, 'never less than nothing');
});

test('one workspace model: a main view on the stage + panels; a main view always sits in a panel', () => {
	for (const [kind, preset] of Object.entries(PRESETS)) {
		if (!preset.main) {
			assert.equal(presetStart(kind as keyof typeof PRESETS), null, `${kind}: its page under the stage`);
			continue;
		}
		assert.ok(preset.panels.some((p) => p.tabs.includes(preset.main!)), `${kind}: its main view has a panel to go back to`);
		assert.deepEqual(presetStart(kind as keyof typeof PRESETS), { stage: { layout: 'one', slots: [preset.main] } });
	}
	assert.equal(PRESETS.character.main, 'sheet');
	assert.equal(PRESETS.generate.main, 'canvas');
});

test('the character workspace: views as tabs (Sheet on the stage, Expressions, 3D), no Tree; the old Moods tabs open Expressions', () => {
	assert.deepEqual(PRESETS.character.panels[0].tabs, ['kanon', 'expressions', 'rig', 'voice', 'cast', 'generations']);
	assert.ok(PRESETS.character.panels.some((p) => p.tabs.includes('sheet') && p.tabs.includes('3d')));
	assert.ok(!PRESETS.character.panels.some((p) => p.tabs.includes('tree')), 'the Library opens characters');
	assert.equal(PRESETS.character.clear, undefined, 'its views keep clear of the bar like every page');
	const available = availableTabs(['sheet', '3d', 'kanon', 'expressions', 'rig'], null);
	const specs = ['sheet', '3d', 'kanon', 'expressions', 'rig', 'library', 'log', 'settings', 'history'].map((id) => ({ id, name: id, icon: 'x', about: '' }));
	const ids = panelSpecs('character', available, specs).flatMap((p) => p.tabs);
	assert.ok(['sheet', '3d', 'expressions', 'rig'].every((t) => ids.includes(t)), 'every view has a panel');
	assert.equal(tabIdOf('moods'), 'expressions');
	assert.equal(tabIdOf('vocabulary'), 'expressions');
});

test('view settings: per workspace, every field checked', () => {
	assert.deepEqual(readViews({ chapter: { stage: ['flow', 'preview'], split: 0.9, clear: false }, map: { stage: 3, clear: 'no' }, bad: 1 }), { chapter: { split: 0.75, clear: false }, map: {} }, 'the stage lives with the panels now');
	assert.deepEqual(readViews([]), {});
	assert.deepEqual(readSettings({ views: { home: { clear: true } } }).views, { home: { clear: true } });
});

test('Ctrl Tab / Ctrl Shift Tab go along the strip in its order, round at the ends', () => {
	let s = visit(EMPTY, 'toon:catpope', '/catpope');
	s = visit(s, 'chapter:catpope:3', '/catpope/chapter/3');
	s = visit(s, 'map:world', '/map');
	assert.deepEqual(s.tabs.map((t) => t.address), ['toon:catpope', 'chapter:catpope:3', 'map:world']);
	assert.equal(stepTab(s, 1)?.address, 'toon:catpope', 'past the last: the first');
	assert.equal(stepTab(s, -1)?.address, 'chapter:catpope:3');
	const front = visit(s, 'chapter:catpope:3', '/catpope/chapter/3');
	assert.deepEqual(front.tabs.map((t) => t.address), s.tabs.map((t) => t.address), 'bringing a tab to the front never moves it');
	assert.equal(stepTab(visit(s, null, '/'), 1)?.address, 'toon:catpope', 'from Home: the first');
	assert.equal(stepTab(EMPTY, 1), null);
});

test('the workspace row: most used first, never grouped; each goes to its last item, an open one of it, or its start', () => {
	let use = entered(NO_USE, null, 'chapter:catpope:3', '/catpope/chapter/3');
	use = entered(use, 'chapter', 'map:world', '/map');
	use = entered(use, 'map', 'chapter:catpope:2', '/catpope/chapter/2');
	use = entered(use, 'chapter', 'chapter:catpope:2', '/catpope/chapter/2/script');
	assert.equal(use.uses.chapter, 2, 'moving inside the workspace counts no new use');
	assert.deepEqual(use.last.chapter, { address: 'chapter:catpope:2', href: '/catpope/chapter/2/script' });
	const tabs = [{ address: 'toon:catpope', href: '/catpope/season/1', label: 'Catpope' }];
	const row = workspaceRow(use, tabs);
	assert.deepEqual(row.slice(0, 3).map((r) => r.kind), ['chapter', 'map', 'toon']);
	assert.equal(row.find((r) => r.kind === 'toon')?.href, '/catpope/season/1', 'an open item of it');
	assert.equal(row.find((r) => r.kind === 'scene')?.href, '/scenes', 'its start');
	assert.ok(!row.some((r) => r.kind === 'storyboard' || r.kind === 'text' || r.kind === 'character'), 'a workspace with nowhere to go is not offered');
	assert.ok(fits('chapter:catpope:3', 'chapter'));
	assert.ok(!fits('chapter:catpope:3', 'character'), 'a chapter does not fit the character workspace: dimmed there');
	assert.ok(!fits(null, 'chapter'));
	assert.deepEqual(readUse(JSON.parse(JSON.stringify(use))), use);
	assert.deepEqual(readUse({ uses: { chapter: -1, nope: 3 }, last: { map: { address: 'chapter:x:1', href: '/x' } } }), NO_USE);
});

test("item ↔ workspace (point 48): an item comes back in its workspace; a new one in the front one if it fits, else its kind's", () => {
	const own = [{ id: 'own-abc123', kind: 'character' as const }];
	assert.equal(workspaceFor('character:catpope', 'own-abc123', null, {}, own), 'own-abc123', 'its tab brings it back in the workspace it remembers');
	assert.equal(workspaceFor('character:catpope', 'own-gone', null, {}, own), 'character', "a remembered workspace that is gone: its kind's");
	assert.equal(workspaceFor('character:catpope', 'chapter', null, {}, own), 'character', "a remembered workspace that does not fit: its kind's");
	assert.equal(workspaceFor('character:feilchen', undefined, { kind: 'character', id: 'own-abc123' }, {}, own), 'own-abc123', 'new: the front workspace, it fits');
	assert.equal(workspaceFor('character:feilchen', undefined, { kind: 'chapter', id: 'chapter' }, { character: 'own-abc123' }, own), 'own-abc123', 'new, the front one does not fit: the one its kind shows');
	assert.equal(workspaceFor('character:feilchen', undefined, null, {}, own), 'character');
	assert.equal(workspaceFor('characters:wanted', undefined, null, {}, own), null, 'a gone area has no workspace');
	assert.equal(workspaceFor('nothing', undefined, null, {}, own), null);
	assert.deepEqual(workspaceClick('character:catpope', '/characters?id=catpope', { id: 'own-abc123', kind: 'character' }, '/characters?id=feilchen'), { href: '/characters?id=catpope', keeps: true }, 'a workspace the item fits keeps it');
	assert.deepEqual(workspaceClick('character:catpope', '/characters?id=catpope', { id: 'chapter', kind: 'chapter' }, '/catpope/chapter/3'), { href: '/catpope/chapter/3', keeps: false }, 'one it does not fit (dimmed) opens its own last item');
	assert.deepEqual(workspaceClick(null, null, { id: 'scene', kind: 'scene' }, '/scenes'), { href: '/scenes', keeps: false });
	let s = visit(EMPTY, 'character:catpope', '/characters?id=catpope');
	s = keepIn(s, 'character:catpope', 'own-abc123');
	assert.equal(s.tabs[0].workspace, 'own-abc123');
	assert.equal(keepIn(s, 'character:catpope', 'own-abc123'), s, 'unchanged: the same state');
	assert.equal(visit(s, 'character:catpope', '/characters?id=catpope&persona=pope').tabs[0].workspace, 'own-abc123', 'moving inside the item keeps it');
	assert.equal(readTabs(JSON.parse(JSON.stringify(s))).tabs[0].workspace, 'own-abc123', 'kept across a reload');
});

test('a removed workspace leaves stored state quietly: tabs, uses, picks naming Wanted pictures go; Generate joins the row', () => {
	assert.ok(isGone('characters:wanted'));
	const kept = { tabs: [{ address: 'characters:wanted', href: '/characters?tab=wanted', label: 'Wanted' }, { address: 'map:world', href: '/map', label: 'Map' }], front: 'characters:wanted' };
	assert.deepEqual(readTabs(kept, isGone), { tabs: [{ address: 'map:world', href: '/map', label: 'Map' }], front: null });
	assert.deepEqual(readUse({ uses: { character: 2 }, last: { character: { address: 'characters:wanted', href: '/characters?tab=wanted' } } }), { uses: { character: 2 }, last: {} });
	assert.deepEqual(readPicks([{ id: 'item:characters:wanted', kind: 'item', label: 'Wanted', icon: 'users', address: 'characters:wanted', href: '/characters?tab=wanted' }]), []);
	assert.ok(!fits('characters:wanted', 'character'));
	assert.ok(ROW.includes('generate'));
	assert.equal(workspaceRow(NO_USE, []).find((r) => r.kind === 'generate')?.href, '/generate', 'Generate starts on an empty canvas');
	assert.equal(workspaceOf('generator:new'), 'generate');
	assert.equal(workspaceRow(NO_USE, []).find((r) => r.kind === 'experience')?.href, '/experiences', 'XR is in the row and starts on its list');
	assert.ok(fits('experience:poster', 'experience'));
});

test('the spore canvas stub: spores woven in with unique ids, gen-AI ones counted as hooks', () => {
	let s = addSpore([], 'library');
	s = addSpore(s, 'image');
	s = addSpore(s, 'image');
	assert.deepEqual(s.map((x) => x.id), ['library-1', 'image-1', 'image-2']);
	assert.equal(hooksOf(s), 2);
	s = removeSpore(s, 'image-1');
	assert.deepEqual(addSpore(s, 'image').map((x) => x.id), ['library-1', 'image-2', 'image-3'], 'never a taken id');
	assert.deepEqual(addSpore(s, 'nonsense'), s);
});

test('the dashboard: the front workspace first, then most used; each card shows what is open in it', () => {
	const tabs = [
		{ address: 'chapter:catpope:3', href: '/catpope/chapter/3', label: 'Chapter 3' },
		{ address: 'scene:duet', href: '/scenes/duet', label: 'Duet' },
		{ address: 'scene:crypt', href: '/scenes/crypt', label: 'Crypt' }
	];
	const use = { uses: { scene: 5, chapter: 2, map: 1 }, last: { scene: { address: 'scene:crypt', href: '/scenes/crypt' } } };
	const cards = workspaceCards(use, tabs, 'chapter:catpope:3');
	assert.deepEqual(cards.map((c) => c.kind).slice(0, 3), ['chapter', 'scene', 'map']);
	assert.equal(cards[0].front, true);
	assert.equal(cards[1].shown?.address, 'scene:crypt', 'a workspace shows its last item while it is open');
	assert.equal(cards[1].open.length, 2);
	assert.equal(cards[2].shown, null);
	assert.equal(cards[2].href, '/map', 'nothing open: its start');
	assert.equal(firstRing(cards), 1, 'the ring starts on the card after the front one');
	assert.equal(firstRing(workspaceCards(use, tabs, null)), 0);
});

test('the welcome: nothing used yet – every workspace of the row, empty', () => {
	assert.equal(isWelcome(NO_USE, [], 0), true);
	assert.equal(isWelcome(NO_USE, [], 1), false);
	const cards = workspaceCards(NO_USE, [], null, true);
	assert.ok(cards.length >= 8);
	assert.ok(cards.every((c) => !c.shown && c.href === null && !c.front));
	assert.equal(greetName('mira.k@example.org'), 'Mira');
	assert.equal(greetName(''), '');
});

test('crafts: the welcome and the row show the crafts\' workspaces, flat; used or open ones stay', () => {
	assert.deepEqual(startKinds(['character', 'storyboard', 'nope', 'character', 'text']), ['character', 'storyboard', 'text'], 'known ones, each once, in order');
	assert.deepEqual(startKinds(null), []);
	const starts = startKinds(['scene', 'map', 'catalog']);
	assert.deepEqual(workspaceCards(NO_USE, [], null, true, starts).map((c) => c.kind), ['scene', 'map', 'catalog'], 'the welcome: the crafts\' workspaces in their order');
	assert.deepEqual(workspaceRow(NO_USE, [], starts).map((r) => r.kind), ['scene', 'map', 'catalog'], 'only theirs, the crafts\' order before use');
	const tabs = [{ address: 'chapter:catpope:3', href: '/catpope/chapter/3', label: 'Chapter 3' }];
	const use = { uses: { catalog: 4, flow: 1 }, last: { flow: { address: 'flow', href: '/flow' } } };
	assert.deepEqual(workspaceRow(use, tabs, starts).map((r) => r.kind), ['catalog', 'flow', 'scene', 'map', 'chapter'], 'most used first; a used or open workspace outside the crafts stays');
	assert.ok(workspaceRow(NO_USE, [], []).length >= 4, 'no crafts: every workspace with somewhere to go');
});

test('crafts before the migration: a missing or failing API means no crafts – the usual workspaces, no question, nothing throws', async () => {
	const fail = (status: number) => () => Promise.reject(Object.assign(new Error(`HTTP ${status}`), { status }));
	const okList = () => Promise.resolve({ crafts: [{ id: 'writer', name: { en: 'Writer' }, icon: 'edit-3', workspaces: ['text'] }] });
	const okMine = () => Promise.resolve({ crafts: ['writer'], invited: [], workspaces: ['text'], ask: true, can_edit: true });
	for (const [list, mine] of [[fail(404), okMine], [okList, fail(404)], [fail(500), fail(500)], [okList, () => Promise.resolve({ status: 'error' })]] as const) {
		const state = await readCrafts(list, mine);
		assert.deepEqual(state, NO_CRAFTS);
		assert.equal(asksCrafts(state), false, 'no first-login question');
		assert.deepEqual(startKinds(state.mine?.workspaces), []);
		assert.deepEqual(workspaceRow(NO_USE, [], startKinds(state.mine?.workspaces)).map((r) => r.kind), workspaceRow(NO_USE, []).map((r) => r.kind), 'the bar falls back to every workspace');
		assert.ok(workspaceCards(NO_USE, [], null, true, startKinds(state.mine?.workspaces)).length >= 8, 'the welcome shows every workspace');
	}
	const ok = await readCrafts(okList, okMine);
	assert.equal(asksCrafts(ok), true);
	assert.equal(asksCrafts({ list: [], mine: ok.mine }), false, 'nothing to pick: no question');
});

test("the translator's bar carries the text language; other crafts and no crafts do not", () => {
	const mine = (crafts: string[]) => ({ crafts, invited: [], workspaces: [], ask: false, can_edit: false });
	assert.equal(showsTextLanguage(mine(['writer', 'translator'])), true);
	assert.equal(showsTextLanguage(mine(['writer'])), false);
	assert.equal(showsTextLanguage(null), false);
});

test('the focus ring goes round the ends and by rows; the dashboard says how long ago', () => {
	assert.equal(stepRing(0, -1, 5), 4);
	assert.equal(stepRing(4, 1, 5), 0);
	assert.equal(stepRing(0, 1, 0), 0);
	assert.equal(rowStep(1, 1, 7, 4), 5);
	assert.equal(rowStep(5, 1, 7, 4), 5, 'no card below: it stays');
	assert.equal(rowStep(5, -1, 7, 4), 1);
	const now = 10 * 86400000;
	assert.deepEqual(ago(now - 20000, now), { unit: 'now', n: 0 });
	assert.deepEqual(ago(now - 5 * 60000, now), { unit: 'minute', n: 5 });
	assert.deepEqual(ago(now - 3 * 3600000, now), { unit: 'hour', n: 3 });
	assert.deepEqual(ago(now - 2 * 86400000, now), { unit: 'day', n: 2 });
	assert.equal(ago(undefined, now), null);
});

const L = (id: string, kind: LaunchEntry['kind'], label: string, extra: Partial<LaunchEntry> = {}): LaunchEntry => ({ id, kind, label, icon: 'file', ...extra });

test('launcher: every word must appear; the address typed exactly wins, then the name, its start, a word, inside', () => {
	assert.ok(wordsIn('Cat Pope · Chapter 3', 'pope chap'));
	assert.ok(!wordsIn('Cat Pope', 'pope dog'));
	assert.ok(wordsIn('anything', '  '));
	const chapter = L('item:chapter:catpope:3', 'item', 'Chapter 3', { address: 'chapter:catpope:3', hint: 'Cat Pope' });
	assert.equal(score(chapter, 'chapter:catpope:3'), 1000);
	assert.equal(score(chapter, 'chapter 3'), 800);
	assert.equal(score(chapter, 'chapter:catpope'), 700);
	assert.equal(score(chapter, 'chap'), 600);
	assert.equal(score(L('a', 'item', 'The Big Map'), 'big'), 400);
	assert.equal(score(L('a', 'item', 'Moodboard'), 'board'), 300);
	assert.equal(score(chapter, 'pope'), 100);
	assert.equal(score(chapter, 'dog'), 0);
});

test('launcher: one ranked list across kinds, each id once; ties go to the recent pick, then the kind order', () => {
	const entries = [
		L('asset:map.png', 'asset', 'Map', { hint: 'media' }),
		L('command:theme', 'command', 'Switch to night', { keywords: 'theme day night' }),
		L('workspace:map', 'workspace', 'Map'),
		L('item:map:world', 'item', 'Map', { address: 'map:world' }),
		L('item:map:world', 'item', 'A duplicate'),
		L('view:library', 'view', 'Library')
	];
	assert.deepEqual(rank(entries, 'map').map((e) => e.id), ['item:map:world', 'workspace:map', 'asset:map.png']);
	assert.deepEqual(rank(entries, 'map', ['asset:map.png']).map((e) => e.id)[0], 'asset:map.png');
	assert.deepEqual(rank(entries, 'night').map((e) => e.id), ['command:theme']);
	assert.deepEqual(rank(entries, 'map:world').map((e) => e.id), ['item:map:world']);
	assert.equal(rank(entries, 'a', [], 2).length, 2);
});

test('launcher: nothing typed lists the recent picks first, then fills up; a pick goes first, once', () => {
	const now = [L('workspace:scene', 'workspace', 'Scenes', { href: '/scenes' }), L('command:settings', 'command', 'Settings')];
	const picks = remember(remember([], L('item:toon:catpope', 'item', 'Cat Pope', { href: '/catpope', address: 'toon:catpope' })), L('command:settings', 'command', 'Settings (old name)'));
	assert.deepEqual(picks.map((p) => p.id), ['command:settings', 'item:toon:catpope']);
	// a command as the lists know it now; an item not loaded yet as it was picked (it can open by its href)
	const shown = recentFirst(now, picks, now);
	assert.deepEqual(shown.map((e) => e.label), ['Settings', 'Cat Pope', 'Scenes']);
	// a recent command the lists do not offer now (no href) is left out
	assert.deepEqual(recentFirst([], [{ id: 'command:publish', kind: 'command', label: 'Publish', icon: 'x' }]).length, 0);
	let many = picks;
	for (let i = 0; i < 20; i++) many = remember(many, L(`item:x${i}`, 'item', `x${i}`));
	assert.equal(many.length, MAX_PICKS);
	assert.equal(remember(many, L('item:x3', 'item', 'x3')).filter((p) => p.id === 'item:x3').length, 1);
});

test('launcher: what the browser kept is checked', () => {
	assert.deepEqual(readPicks(null), []);
	assert.deepEqual(readPicks([{ id: 'a', kind: 'item', label: 'A', icon: 'file', href: '/a' }, { id: 'b', kind: 'nope', label: 'B', icon: 'f' }, { id: 'c', kind: 'item', label: 'C', icon: 'f', href: 'https://evil' }, 7]), [{ id: 'a', kind: 'item', label: 'A', icon: 'file', href: '/a' }]);
});

test('Search keys: Alt Space, Cmd Space, Ctrl or Cmd O, / outside a field – never Ctrl Space; the switcher: Ctrl or Cmd K, Alt Tab', () => {
	const k = (key: string, code: string, mods: Partial<{ altKey: boolean; ctrlKey: boolean; metaKey: boolean; shiftKey: boolean }> = {}) => ({ key, code, altKey: false, ctrlKey: false, metaKey: false, shiftKey: false, ...mods });
	assert.ok(isSearchKey(k(' ', 'Space', { altKey: true }), false));
	assert.ok(isSearchKey(k(' ', 'Space', { altKey: true }), true), 'macOS Option Space types a no-break space');
	assert.ok(isSearchKey(k(' ', 'Space', { metaKey: true }), true));
	assert.ok(!isSearchKey(k(' ', 'Space', { ctrlKey: true }), false), 'Ctrl Space is full screen');
	assert.ok(!isSearchKey(k(' ', 'Space'), false));
	assert.ok(!isSearchKey(k('k', 'KeyK', { ctrlKey: true }), true), 'Ctrl K is the switcher');
	assert.ok(isSwitcherKey(k('k', 'KeyK', { ctrlKey: true })));
	assert.ok(isSwitcherKey(k('K', 'KeyK', { metaKey: true })));
	assert.ok(isSwitcherKey(k('Tab', 'Tab', { altKey: true })), 'Alt Tab where the system leaves it to the page');
	assert.ok(!isSwitcherKey(k('Tab', 'Tab')));
	assert.ok(!isSwitcherKey(k('k', 'KeyK', { ctrlKey: true, shiftKey: true })));
	assert.ok(isSearchKey(k('o', 'KeyO', { ctrlKey: true }), false));
	assert.ok(!isSearchKey(k('k', 'KeyK', { ctrlKey: true, shiftKey: true }), false));
	assert.ok(isSearchKey(k('/', 'Slash'), false));
	assert.ok(!isSearchKey(k('/', 'Slash'), true), 'typing a slash in a field');
});

test('saved layouts: the server answer checked; the start is the last save, else the craft default, else the preset', () => {
	assert.equal(fromAnswer(null), null);
	assert.equal(fromAnswer({ available: false, layouts: [], defaults: [] }), null, 'before the migration: none (the browser copy stays)');
	const state = fromAnswer({
		available: true,
		layouts: [
			{ workspace: 'character', kind: null, name: null, layout: { stage: { layout: 'one', slots: ['kanon'] } }, saved_at: null },
			{ workspace: 'own-abc123', kind: 'character', name: 'Looks for 3D', layout: { places: {} }, saved_at: null },
			{ workspace: 'own-bad', kind: 'nowhere', name: 'Broken', layout: {}, saved_at: null },
			{ workspace: 'text', kind: null, name: null, layout: 'not an object', saved_at: null }
		],
		defaults: [
			{ workspace: 'character', craft: 'illustrator', layout: { places: { right: {} } } },
			{ workspace: 'storyboard', craft: 'illustrator', layout: { places: {} } }
		]
	});
	assert.ok(state);
	assert.deepEqual(Object.keys(state.saved), ['character', 'own-abc123'], 'broken rows are left out');
	assert.deepEqual(state.own, [{ id: 'own-abc123', kind: 'character', name: 'Looks for 3D' }]);
	assert.deepEqual(startOf(state, 'character'), { stage: { layout: 'one', slots: ['kanon'] } }, 'the last save');
	assert.deepEqual(startKind(state, 'character'), { kind: 'saved' });
	assert.deepEqual(startOf(state, 'storyboard'), { places: {} }, 'never saved: the craft default');
	assert.deepEqual(startKind(state, 'storyboard'), { kind: 'craft', craft: 'illustrator' });
	assert.equal(startOf(state, 'chapter'), null, 'neither: the code preset');
	assert.deepEqual(startKind(state, 'chapter'), { kind: 'preset' });
	assert.deepEqual(startOf(state, 'own-abc123'), { places: {} });
	assert.equal(startOf(NO_LAYOUTS, 'character'), null);
});

test('saved layouts: a save, an own workspace beside the one it is made from, a save taken back, the browser copy', () => {
	let state = withSaved(NO_LAYOUTS, 'chapter', { places: {} });
	assert.deepEqual(startOf(state, 'chapter'), { places: {} });
	state = withSaved(state, 'own-x1', { stage: {} }, { name: '  My   chapter ', kind: 'chapter' });
	assert.deepEqual(state.own, [{ id: 'own-x1', kind: 'chapter', name: 'My chapter' }]);
	state = withSaved(state, 'own-x1', { stage: {} }, { name: 'Writing', kind: 'chapter' });
	assert.deepEqual(state.own.map((o) => o.name), ['Writing'], 'renamed, not doubled');
	assert.equal(shownWorkspace({ chapter: 'own-x1' }, state.own, 'chapter'), 'own-x1');
	assert.equal(shownWorkspace({ chapter: 'own-gone' }, state.own, 'chapter'), 'chapter', 'a chosen one that is gone: the creator workspace');
	assert.equal(shownWorkspace({ chapter: 'own-x1' }, state.own, 'scene'), 'scene');
	const row = withOwn([{ kind: 'toon' as const, href: '/t' }, { kind: 'chapter' as const, href: '/c' }], state.own);
	assert.deepEqual(row.map((e) => [e.id, e.name ?? null, e.href]), [['toon', null, '/t'], ['chapter', null, '/c'], ['own-x1', 'Writing', '/c']]);
	const tabs = [{ address: 'chapter:catpope:3', href: '/catpope/chapter/3', label: 'Chapter 3' }];
	const cards = workspaceCards({ uses: { chapter: 1 }, last: {} }, tabs, 'chapter:catpope:3', false, [], state.own, { chapter: 'own-x1' });
	assert.deepEqual(cards.slice(0, 2).map((c) => [c.id, c.front]), [['own-x1', true], ['chapter', false]], 'the own workspace the kind shows is the front card');
	const kept = readKept(JSON.parse(JSON.stringify(toKept(state))));
	assert.deepEqual([kept.own, kept.saved], [state.own, state.saved]);
	assert.deepEqual(readKept({ saved: { a: 1 }, own: [{ id: 'own-y', kind: 'chapter', name: 'No layout' }] }), NO_LAYOUTS, 'an own workspace without its layout is left out');
	state = without(state, 'own-x1');
	assert.deepEqual([state.own, Object.keys(state.saved)], [[], ['chapter']]);
	assert.deepEqual(without(state, 'chapter').saved, {});
	assert.match(ownId([]), /^own-[a-z0-9]{6}$/);
	let n = 0;
	assert.equal(ownId(['own-000000'], () => (n++ ? 1 / 36 ** 6 : 0)), 'own-000001', 'a taken id is not used again');
	assert.equal(cleanName('  a \n b '), 'a b');
	assert.deepEqual(readChosen({ chapter: 'own-x1', nowhere: 'own-y', toon: 3 }), { chapter: 'own-x1' });
});

test('saved layouts: a missing or failing API means no layouts and nothing throws', async () => {
	const { readLayouts } = await import('../src/lib/shell/layouts.ts');
	assert.equal(await readLayouts(() => Promise.reject(new Error('404'))), null);
	assert.equal(await readLayouts(async () => ({ status: 'error' })), null);
	assert.equal(await readLayouts(async () => ({ available: true, layouts: 'x' })), null);
	assert.deepEqual(await readLayouts(async () => ({ available: true, layouts: [], defaults: [] })), NO_LAYOUTS);
});
