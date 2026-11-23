// The shell's pure parts (lib/shell): addresses ↔ routes, the tabs, this machine's settings.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { AREAS, addressOf, hrefOf, iconOf, resolveTyped, scopeOf, workspaceOf } from '../src/lib/shell/address.ts';
import { EMPTY, MAX_TABS, close, closeAll, closeOthers, readTabs, rename, retarget, stepTab, visit } from '../src/lib/shell/tabs.ts';
import { NO_USE, entered, fits, readUse, startKinds, workspaceRow } from '../src/lib/shell/workspaces.ts';
import { DEFAULTS, clampScale, readSettings, readViews } from '../src/lib/shell/settings.ts';
import { PRESETS, availableTabs, panelSpecs, tabIdOf } from '../src/lib/shell/presets.ts';
import { clampSplit, isCanvas, freeRoom } from '../src/lib/shell/stage.ts';
import { ago, firstRing, greetName, isWelcome, rowStep, stepRing, workspaceCards } from '../src/lib/shell/dashboard.ts';
import { MAX_PICKS, isLaunchKey, rank, readPicks, recentFirst, remember, score, wordsIn, type LaunchEntry } from '../src/lib/shell/launcher.ts';

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

const TABS = ['toon', 'tree', 'inspector', 'kanon', 'moods', 'flow', 'preview', 'log', 'timeline', 'library', 'settings'].map((id) => ({ id, name: id, icon: 'x', about: '' }));

test('a workspace is a preset: its panels hold only the tabs with content, Library, Settings and the Log everywhere', () => {
	const specs = panelSpecs('chapter', availableTabs(['tree', 'flow'], 'catpope'), TABS);
	const ids = specs.map((p) => p.id);
	assert.deepEqual(ids, ['left', 'right', 'library', 'settings']);
	assert.deepEqual(specs[0].tabs, ['toon', 'tree']);
	assert.deepEqual(specs[1].tabs, ['log', 'flow'], 'the inspector without content is not offered; the Log is the shell’s own');
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
	for (const kind of ['home', 'toon', 'chapter', 'storyboard', 'character', 'catalog', 'map', 'scene', 'flow', 'sound', 'text', 'help']) assert.ok(PRESETS[kind as keyof typeof PRESETS], kind);
	assert.ok(isCanvas('flow') && isCanvas('preview') && !isCanvas('log'));
	assert.equal(clampSplit(0.9), 0.75);
	assert.equal(clampSplit(Number.NaN), 0.5);
});

test('the free room: between the left and the right column of panels', () => {
	assert.deepEqual(freeRoom([], 1000), { left: 0, right: 1000 });
	assert.deepEqual(freeRoom([{ x: 0, w: 200 }, { x: 700, w: 300 }, { x: 650, w: 350 }], 1000), { left: 200, right: 650 });
	assert.deepEqual(freeRoom([{ x: 0, w: 600 }, { x: 550, w: 450 }], 1000), { left: 600, right: 600 }, 'never less than nothing');
});

test('the character workspace: its sections in one panel, the old Moods tabs open Expressions', () => {
	assert.deepEqual(PRESETS.character.panels[0].tabs, ['kanon', 'expressions', 'rig', 'voice', 'cast', 'generations']);
	assert.equal(PRESETS.character.clear, false);
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

test('launcher keys: Alt Space, Cmd Space, Ctrl or Cmd K and O, / outside a field – never Ctrl Space', () => {
	const k = (key: string, code: string, mods: Partial<{ altKey: boolean; ctrlKey: boolean; metaKey: boolean; shiftKey: boolean }> = {}) => ({ key, code, altKey: false, ctrlKey: false, metaKey: false, shiftKey: false, ...mods });
	assert.ok(isLaunchKey(k(' ', 'Space', { altKey: true }), false));
	assert.ok(isLaunchKey(k(' ', 'Space', { altKey: true }), true), 'macOS Option Space types a no-break space');
	assert.ok(isLaunchKey(k(' ', 'Space', { metaKey: true }), true));
	assert.ok(!isLaunchKey(k(' ', 'Space', { ctrlKey: true }), false), 'Ctrl Space is full screen');
	assert.ok(!isLaunchKey(k(' ', 'Space'), false));
	assert.ok(isLaunchKey(k('k', 'KeyK', { ctrlKey: true }), true));
	assert.ok(isLaunchKey(k('K', 'KeyK', { metaKey: true }), true));
	assert.ok(isLaunchKey(k('o', 'KeyO', { ctrlKey: true }), false));
	assert.ok(!isLaunchKey(k('k', 'KeyK', { ctrlKey: true, shiftKey: true }), false));
	assert.ok(isLaunchKey(k('/', 'Slash'), false));
	assert.ok(!isLaunchKey(k('/', 'Slash'), true), 'typing a slash in a field');
});
