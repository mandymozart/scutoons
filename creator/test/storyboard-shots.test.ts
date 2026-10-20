// The Storyboard page's shots and script (src/lib/editor/storyboard-shots.ts, storyboard-script.ts, #123): the toon
// order with each shot's place, the shot list's picking (the studio's rules), a field set / unset, the script scene.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { ChapterStoryboard } from '@scutoons/content/storyboard';
import { aspectOptions, optionsOf, pickShot, setField, shotEntries, shotOfMessage, splitMessage, svgSource, type Picking } from '../src/lib/editor/storyboard-shots.ts';
import { sceneLines } from '../src/lib/editor/storyboard-script.ts';

const shot = (id: string) => ({ id, format: 'viewport' as const, aspect: '9:16', action: { de: id } });
const doc: ChapterStoryboard = {
	storyboard: 2,
	toon: 'catpope',
	chapter: 3,
	scenes: [
		{ storyboard: 2, toon: 'catpope', source: { scene: 25 }, groups: [{ label: 'a', shots: [shot('sb-25-1'), shot('sb-25-2')] }, { label: 'b', shots: [shot('sb-25-3')] }] },
		{ storyboard: 2, toon: 'catpope', source: { scene: 26 }, groups: [{ label: 'c', shots: [shot('sb-26-1'), shot('sb-26-2')] }] }
	]
};
const order = shotEntries(doc).map((e) => e.id);
const state = (picked: string[], current: string | null, anchor: string | null = current): Picking => ({ picked, current, anchor });

test('the shots in toon order, each with its path in the document', () => {
	assert.deepEqual(order, ['sb-25-1', 'sb-25-2', 'sb-25-3', 'sb-26-1', 'sb-26-2']);
	const e = shotEntries(doc)[2];
	assert.deepEqual(e.path, ['scenes', 0, 'groups', 1, 'shots', 0]);
	assert.equal(e.scene.source.scene, 25);
	assert.deepEqual(shotEntries(null), []);
});

test('a plain click: only that shot, and it is the current one', () => {
	assert.deepEqual(pickShot(order, state(['sb-25-1', 'sb-25-2'], 'sb-25-1'), 'sb-26-1'), state(['sb-26-1'], 'sb-26-1'));
	// an unknown id changes nothing
	const s = state(['sb-25-1'], 'sb-25-1');
	assert.equal(pickShot(order, s, 'nope'), s);
});

test('Ctrl + click adds a shot (it becomes current) or takes it off (the current goes to its neighbour)', () => {
	const added = pickShot(order, state(['sb-25-1'], 'sb-25-1'), 'sb-26-1', { ctrlKey: true });
	assert.deepEqual(added, state(['sb-25-1', 'sb-26-1'], 'sb-26-1'));
	// taking off the current one: the next one in the set
	const off = pickShot(order, state(['sb-25-1', 'sb-25-3', 'sb-26-1'], 'sb-25-3'), 'sb-25-3', { ctrlKey: true });
	assert.deepEqual(off.picked, ['sb-25-1', 'sb-26-1']);
	assert.equal(off.current, 'sb-26-1');
	// the last one in the set: the one before it
	assert.equal(pickShot(order, state(['sb-25-1', 'sb-26-1'], 'sb-26-1'), 'sb-26-1', { metaKey: true }).current, 'sb-25-1');
	// taking off another one keeps the current
	assert.equal(pickShot(order, state(['sb-25-1', 'sb-26-1'], 'sb-26-1'), 'sb-25-1', { ctrlKey: true }).current, 'sb-26-1');
	// the only picked one stays
	const only = state(['sb-25-2'], 'sb-25-2');
	assert.equal(pickShot(order, only, 'sb-25-2', { ctrlKey: true }), only);
});

test('Shift + click picks the range from the last clicked one, which stays the anchor', () => {
	const r = pickShot(order, state(['sb-25-2'], 'sb-25-2'), 'sb-26-1', { shiftKey: true });
	assert.deepEqual(r, state(['sb-25-2', 'sb-25-3', 'sb-26-1'], 'sb-26-1', 'sb-25-2'));
	// a second Shift + click spans from the same anchor (upwards too)
	assert.deepEqual(pickShot(order, r, 'sb-25-1', { shiftKey: true }).picked, ['sb-25-1', 'sb-25-2']);
	// Ctrl + Shift adds the range to the set
	assert.deepEqual(pickShot(order, state(['sb-26-2', 'sb-25-1'], 'sb-25-1'), 'sb-25-3', { shiftKey: true, ctrlKey: true }).picked, ['sb-25-1', 'sb-25-2', 'sb-25-3', 'sb-26-2']);
});

test('setField sets, removes an empty value and prunes the objects it emptied', () => {
	const s: Record<string, any> = { id: 'x', light: { key: 'left' } };
	setField(s, ['light', 'height'], 'low');
	assert.deepEqual(s.light, { key: 'left', height: 'low' });
	setField(s, ['light', 'key'], '');
	setField(s, ['light', 'height'], null);
	assert.equal('light' in s, false);
	setField(s, ['light', 'accent', 'color'], 'red');
	assert.deepEqual(s.light, { accent: { color: 'red' } });
	setField(s, ['light', 'accent', 'color'], undefined);
	assert.equal('light' in s, false);
	// unsetting under a missing parent creates nothing
	setField(s, ['shot', 'size'], '');
	assert.equal('shot' in s, false);
	// a list item emptied stays in its place
	const f: Record<string, any> = { figures: [{ x: 1 }, { x: 2 }] };
	setField(f, ['figures', '0', 'x'], undefined);
	assert.deepEqual(f.figures, [{}, { x: 2 }]);
	// 0 and false are values
	setField(s, ['light', 'soft'], 0);
	assert.deepEqual(s.light, { soft: 0 });
});

test('options keep an old word that is not in the list', () => {
	assert.deepEqual(optionsOf(['a', 'b'], 'c').map((o) => o.value), ['a', 'b', 'c']);
	assert.deepEqual(optionsOf(['a', 'b'], 'a').map((o) => o.value), ['a', 'b']);
});

test('a validate message names its place and its shot', () => {
	const ids = new Set(order);
	assert.equal(shotOfMessage('scene-25 sb-25-3: no persona', ids), 'sb-25-3');
	assert.equal(shotOfMessage('scene-25: scene 25 twice', ids), null);
	assert.equal(shotOfMessage('chapter: toon missing', ids), null);
	assert.deepEqual(splitMessage('scene-25 sb-25-3: figure x: no persona'), { place: ['scene-25', 'sb-25-3'], text: 'figure x: no persona' });
	assert.deepEqual(splitMessage('plain'), { place: [], text: 'plain' });
});

test('the schematic as an image source', () => {
	assert.ok(svgSource('<svg a="1"/>').startsWith('data:image/svg+xml;charset=utf-8,%3Csvg'));
});

test('a scene of the script: its heading to the next marker, without the trailing gap', () => {
	const text = ['# 3. Kapitel', '# DIE PROBE', '', '25  INT. KÜCHE - NACHT', 'Mira sieht auf.', '', '26  EXT. HOF - NACHT', 'Regen.', '', '', '# 4. Kapitel', '27  INT. X'].join('\n');
	assert.deepEqual(sceneLines(text, 25), ['25  INT. KÜCHE - NACHT', 'Mira sieht auf.']);
	assert.deepEqual(sceneLines(text, 26), ['26  EXT. HOF - NACHT', 'Regen.']);
	assert.deepEqual(sceneLines(text, 27), ['27  INT. X']);
	assert.equal(sceneLines(text, 99), null);
	assert.equal(sceneLines('', 1), null);
});

test('the Aspect select offers whole slices for viewport / vertical, the free aspects for loop360 / strip', () => {
	const words = { slices: (n: number) => `${n} slices`, offGrid: 'not whole slices' };
	assert.deepEqual(aspectOptions('vertical', '1:4.8', words), [
		{ value: '1:1.6', label: '1:1.6 · 1 slices' },
		{ value: '1:3.2', label: '1:3.2 · 2 slices' },
		{ value: '1:4.8', label: '1:4.8 · 3 slices' },
		{ value: '1:6.4', label: '1:6.4 · 4 slices' },
		{ value: '1:8', label: '1:8 · 5 slices' }
	]);
	// an off-grid value stays selected and says so
	const off = aspectOptions('viewport', '9:16', words);
	assert.equal(off.length, 6);
	assert.deepEqual(off.at(-1), { value: '9:16', label: '9:16 – not whole slices' });
	// a panorama: the free aspects, no slice aspects
	const pano = aspectOptions('loop360', '6:1', words).map((o) => o.value);
	assert.ok(pano.includes('6:1') && pano.includes('9:16'));
	assert.ok(!pano.includes('1:4.8'));
	assert.deepEqual(aspectOptions('strip', '7:1', words).at(-1), { value: '7:1', label: '7:1' });
});
