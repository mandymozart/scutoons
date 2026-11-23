// Resuming like a hibernated computer (lib/shell/resume.ts): the kept state read back, the start merged with the URL
// opened, members switching on one browser, items gone.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { EMPTY, type TabsState } from '../src/lib/shell/tabs.ts';
import { MAX_OTHERS, MAX_SCROLLS, NO_RESUME, NO_SESSION, panelKey, readResume, stageKey, startOf, stillThere, switchMember, withFull, withScroll, withoutGone, type ResumeStore } from '../src/lib/shell/resume.ts';

const three: TabsState = {
	tabs: [
		{ address: 'toon:catpope', href: '/catpope', label: 'Cat Pope' },
		{ address: 'character:priestess', href: '/characters?id=priestess', label: 'Priestess' },
		{ address: 'scene:rave', href: '/scenes/rave', label: 'Rave' }
	],
	front: 'character:priestess'
};

test('a broken, foreign or older store reads as nothing kept', () => {
	assert.deepEqual(readResume(null), NO_RESUME);
	assert.deepEqual(readResume('nonsense'), NO_RESUME);
	assert.deepEqual(readResume([1, 2]), NO_RESUME);
	assert.deepEqual(readResume({ v: 2, dashboard: 'cards' }), NO_RESUME);
});

test('a kept store reads back field by field; bad parts are left out', () => {
	const read = readResume({
		v: 1,
		member: 'm1',
		dashboard: 'cards',
		full: { 'creator-host-character': 'moods', bad: 3, empty: '' },
		scroll: { 'stage:toon:catpope': { top: 420.4 }, 'panel:w/p/t': { top: 30, path: [0, 2] }, 'stage:x': { top: -1 }, other: { top: 5 }, 'panel:w/q/t': { top: 9, path: [-1] } },
		others: { m2: { dashboard: 'cards', tabs: { tabs: [{ address: 'scene:a', href: '/scenes/a' }], front: 'scene:a' } }, '': {} }
	});
	assert.equal(read.member, 'm1');
	assert.equal(read.dashboard, 'cards');
	assert.deepEqual(read.full, { 'creator-host-character': 'moods' });
	assert.deepEqual(read.scroll, { 'stage:toon:catpope': { top: 420 }, 'panel:w/p/t': { top: 30, path: [0, 2] }, 'panel:w/q/t': { top: 9 } });
	assert.deepEqual(Object.keys(read.others), ['m2']);
	assert.equal(read.others.m2.dashboard, 'cards');
	assert.equal(read.others.m2.tabs.front, 'scene:a');
	assert.equal(readResume({ v: 1, dashboard: 'open' }).dashboard, false);
	assert.equal(readResume({ v: 1, dashboard: 'views' }).dashboard, false, 'Views live in the menu now');
});

test('opened at Home: the front tab comes back, the dashboard as it was', () => {
	const start = startOf(three, { ...NO_SESSION, dashboard: 'cards' }, true, { address: null, href: '/' });
	assert.equal(start.go, '/characters?id=priestess');
	assert.equal(start.dashboard, 'cards');
	assert.deepEqual(start.tabs, three);
	assert.equal(startOf(three, NO_SESSION, true, { address: null, href: '/' }).dashboard, false);
});

test('opened at Home with nothing in front, or nothing kept: the dashboard', () => {
	assert.deepEqual(startOf(EMPTY, NO_SESSION, true, { address: null, href: '/' }), { tabs: EMPTY, go: null, dashboard: 'cards' });
	const home = startOf({ ...three, front: null }, NO_SESSION, true, { address: null, href: '/' });
	assert.equal(home.go, null);
	assert.equal(home.dashboard, 'cards');
	assert.equal(home.tabs.tabs.length, 3);
});

test('reopening tabs off: an empty start', () => {
	assert.deepEqual(startOf(three, { ...NO_SESSION, dashboard: 'cards' }, false, { address: null, href: '/' }), { tabs: EMPTY, go: null, dashboard: 'cards' });
	const linked = startOf(three, NO_SESSION, false, { address: 'scene:rave', href: '/scenes/rave' });
	assert.deepEqual(linked.tabs.tabs.map((t) => t.address), ['scene:rave']);
});

test('a reload on the front tab keeps it and the dashboard as it was', () => {
	const start = startOf(three, { ...NO_SESSION, dashboard: 'cards' }, true, { address: 'character:priestess', href: '/characters?id=priestess' });
	assert.equal(start.go, null);
	assert.equal(start.dashboard, 'cards');
	assert.deepEqual(start.tabs, three);
});

test('a link opened wins the front but keeps the other tabs, no dashboard', () => {
	const known = startOf(three, { ...NO_SESSION, dashboard: 'cards' }, true, { address: 'scene:rave', href: '/scenes/rave?view=top' });
	assert.equal(known.go, null);
	assert.equal(known.dashboard, false);
	assert.equal(known.tabs.front, 'scene:rave');
	assert.deepEqual(known.tabs.tabs.map((t) => t.address), ['toon:catpope', 'character:priestess', 'scene:rave']);
	assert.equal(known.tabs.tabs[2].href, '/scenes/rave?view=top');
	const fresh = startOf(three, NO_SESSION, true, { address: 'chapter:catpope:3', href: '/catpope/chapter/3' });
	assert.equal(fresh.tabs.front, 'chapter:catpope:3');
	assert.deepEqual(fresh.tabs.tabs.map((t) => t.address), ['toon:catpope', 'character:priestess', 'chapter:catpope:3', 'scene:rave']);
});

test('the first member known takes the store; the same member changes nothing', () => {
	const first = switchMember(NO_RESUME, three, 'm1');
	assert.equal(first.store.member, 'm1');
	assert.equal(first.tabs, three);
	assert.equal(switchMember(first.store, three, 'm1').store, first.store);
});

test('another member on the same browser: each gets their own tabs and state back', () => {
	const mine: ResumeStore = { ...NO_RESUME, member: 'm1', dashboard: 'cards', full: { w: 'moods' } };
	const toM2 = switchMember(mine, three, 'm2');
	assert.equal(toM2.store.member, 'm2');
	assert.deepEqual(toM2.tabs, EMPTY);
	assert.equal(toM2.store.dashboard, false);
	assert.deepEqual(toM2.store.full, {});
	const m2tabs: TabsState = { tabs: [{ address: 'scene:a', href: '/scenes/a', label: 'A' }], front: 'scene:a' };
	const back = switchMember(toM2.store, m2tabs, 'm1');
	assert.equal(back.store.member, 'm1');
	assert.deepEqual(back.tabs, three);
	assert.equal(back.store.dashboard, 'cards');
	assert.deepEqual(back.store.full, { w: 'moods' });
	assert.deepEqual(back.store.others.m2.tabs, m2tabs);
	assert.equal(back.store.others.m1, undefined);
});

test('at most a few other members are kept', () => {
	let store: ResumeStore = { ...NO_RESUME, member: 'm0' };
	for (let i = 1; i <= MAX_OTHERS + 3; i++) store = switchMember(store, EMPTY, `m${i}`).store;
	assert.equal(Object.keys(store.others).length, MAX_OTHERS);
	assert.ok(!('m0' in store.others));
});

test('full screen per workspace: set, changed, cleared', () => {
	const on = withFull(NO_RESUME, 'creator-host-character', 'moods');
	assert.deepEqual(on.full, { 'creator-host-character': 'moods' });
	assert.equal(withFull(on, 'creator-host-character', 'moods'), on);
	assert.deepEqual(withFull(on, 'creator-host-character', null).full, {});
	assert.equal(withFull(NO_RESUME, 'x', null), NO_RESUME);
});

test('scroll positions: kept rounded, 0 forgets, the newest last, a limit', () => {
	const a = withScroll(NO_RESUME, stageKey('toon:catpope'), { top: 120.6 });
	assert.deepEqual(a.scroll, { 'stage:toon:catpope': { top: 121 } });
	assert.equal(withScroll(a, 'stage:toon:catpope', { top: 121 }), a);
	assert.deepEqual(withScroll(a, 'stage:toon:catpope', { top: 0 }).scroll, {});
	const p = withScroll(a, panelKey('creator-host-toon', 'library', 'library'), { top: 40, path: [0, 1] });
	assert.deepEqual(p.scroll['panel:creator-host-toon/library/library'], { top: 40, path: [0, 1] });
	assert.equal(stageKey(null), 'stage:home');
	let many = NO_RESUME;
	for (let i = 0; i < MAX_SCROLLS + 5; i++) many = withScroll(many, `stage:s${i}`, { top: 10 });
	assert.equal(Object.keys(many.scroll).length, MAX_SCROLLS);
	assert.ok(!('stage:s0' in many.scroll));
	assert.ok('stage:s0' in withScroll(NO_RESUME, 'stage:s0', { top: 1 }).scroll);
});

test('an item gone or not the member’s is known; anything unchecked stays', () => {
	const lists = { character: ['character:priestess'], scene: ['scene:rave'] };
	assert.equal(stillThere('character:priestess', lists, null), true);
	assert.equal(stillThere('character:ghost', lists, null), false);
	assert.equal(stillThere('scene:gone', lists, null), false);
	assert.equal(stillThere('rig:x', lists, null), null);
	assert.equal(stillThere('characters:all', lists, null), true);
	assert.equal(stillThere('catalog:items', lists, null), true);
	assert.equal(stillThere('toon:catpope', lists, null), null);
	assert.equal(stillThere('toon:catpope', lists, ['catpope']), true);
	assert.equal(stillThere('chapter:secret:2', lists, ['catpope']), false);
	assert.equal(stillThere('storyboard:catpope:3', lists, ['catpope']), true);
});

test('tabs without those gone: the front one hands over', () => {
	const next = withoutGone(three, ['character:priestess', 'nothing:here']);
	assert.deepEqual(next.tabs.map((t) => t.address), ['toon:catpope', 'scene:rave']);
	assert.equal(next.front, 'scene:rave');
	assert.deepEqual(withoutGone(three, []), three);
	assert.deepEqual(withoutGone(three, three.tabs.map((t) => t.address)), EMPTY);
});
