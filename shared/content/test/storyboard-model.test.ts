// @scutoons/content/storyboard (#119): the slice grid, the vocabulary, measures, validate / validateChapter, the
// chapter document, the render spec.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	S, SLICE_ASPECTS, UNIT, W, aspectSlices, chapterStoryboard, findShot, isIn, measureShot, measureVn, parseAspect, POSES,
	readSeconds, shotHeight, shotSpec, slicesOf, storyboardKey, summarize, validate, validateChapter, words,
	type ChapterStoryboard, type Shot, type StoryboardDoc
} from '../src/storyboard/index.ts';

const shot = (extra: Partial<Shot> = {}): Shot => ({
	id: 'sb-1-1', format: 'viewport', aspect: '1:1.6', view: 'real', action: { de: 'PLF steht im Schnee.' }, source: { type: 'schematic', want: 'photo' }, ...extra
});
const scene = (n: number, shots: Shot[], extra: Partial<StoryboardDoc> = {}): StoryboardDoc => ({
	storyboard: 2, toon: 'catpope', chapter: 3, source: { scene: n }, light: { key: 'top-left', source: 'sun' },
	cast: [{ ref: 'plf', character: 'catpope', persona: 'human' }], locations: [{ ref: 'village', place: 'mountain-village', time: 'snow' }],
	groups: [{ label: `scene-${n}`, renderer: { type: 'vertical' }, shots }], ...extra
});

test('grid: one slice is 390 × 624; whole slices within 0.5 %; aspects', () => {
	assert.equal(W, 390);
	assert.equal(S, 844);
	assert.equal(UNIT, 624);
	assert.equal(slicesOf(624), 1);
	assert.equal(slicesOf(625), 1); // the old cut, 0.16 % off
	assert.equal(slicesOf(1248), 2);
	assert.equal(slicesOf(1560), null);
	assert.equal(slicesOf(0), null);
	assert.deepEqual(SLICE_ASPECTS.map(aspectSlices), [1, 2, 3, 4, 5]);
	assert.equal(aspectSlices('9:16'), null);
	assert.deepEqual(parseAspect('1:4.8'), { w: 1, h: 4.8 });
	assert.equal(parseAspect('1/4'), null);
	assert.equal(shotHeight('1:4', 'vertical'), 1560);
	assert.equal(shotHeight('6:1', 'loop360'), S);
	assert.equal(shotHeight('nope'), S);
});

test('vocab: words count tokens with a letter or digit, German first; isIn is a type guard', () => {
	assert.equal(words({ en: 'one two', de: 'eins – zwei drei' }), 3);
	assert.equal(words({ en: 'one two' }), 2);
	assert.equal(words(undefined), 0);
	assert.equal(readSeconds(8), 0.8 + 2);
	assert.ok(isIn(POSES, 'pray'));
	assert.ok(!isIn(POSES, 'fly'));
});

test('measure: a viewport is 624 px; a 3:1 panorama is one screen high and looked at across its width', () => {
	const m = measureShot(shot({ dialog: [{ rect: { x: 0, y: 0 }, text: { de: 'eins zwei drei vier' } }] }));
	assert.equal(m.h, 624);
	assert.equal(m.words, 4);
	assert.equal(m.boxes, 1);
	assert.ok(Math.abs(m.seconds - (10 * 624 / 844 + 0.8 + 1 + 1.2 * 624 / 844)) < 1e-9);
	const p = measureShot(shot({ format: 'loop360', aspect: '3:1' }));
	assert.equal(p.h, S);
	assert.ok(Math.abs(p.panoScreens - 3 * S / W) < 1e-9);
	const vn = measureVn({ events: [{ type: 'showMessage', params: { text: 'a b' } }, { type: 'showChoices' }, { type: 'showMessage' }, { type: 'showMessage' }] });
	assert.deepEqual({ messages: vn.messages, words: vn.words, maxRun: vn.maxRun }, { messages: 3, words: 2, maxRun: 2 });
	const sum = summarize([{ sb: scene(1, [shot(), shot({ id: 'sb-1-2', format: 'loop360', aspect: '3:1' })]) }]);
	assert.equal(sum.total.shots, 2);
	assert.ok(Math.abs(sum.total.screens - (624 / 844 + 1)) < 1e-9);
});

test('validate: a clean shot passes; errors and warnings carry the file and the shot id', () => {
	assert.deepEqual(validate(scene(1, [shot()]), 'a/b/scene-1.json'), { errors: [], warnings: [] });
	const r = validate(scene(1, [shot({ format: 'vertical', aspect: '1:4', move: 'zoom' as never, source: { want: 'generated' }, figures: [{ cast: 'tiger' }] })]), 'a\\scene-1.json');
	assert.deepEqual(r.errors, ['scene-1.json sb-1-1: move "zoom"', 'scene-1.json sb-1-1: figure cast "tiger" not in cast']);
	assert.deepEqual(r.warnings, ['scene-1.json sb-1-1: vertical 1:4 is 2.5 slices – not whole slices: 1:3.2 or 1:4.8']);
	// no file: the old tool's leading space is kept
	assert.deepEqual(validate(scene(1, [shot({ view: undefined })])).warnings, [' sb-1-1: no view (real / belief / narration / memory) – whose perspective is this shot?']);
	assert.deepEqual(validate({ storyboard: 2 } as StoryboardDoc).errors, [': toon missing', ': source.scene missing', ': no groups']);
});

test('validate: rhythm – 5 verticals in a row, 4 viewports in a row, a wall of words', () => {
	const v = Array.from({ length: 5 }, (_, i) => shot({ id: `sb-1-${i + 1}`, format: 'vertical', aspect: '1:3.2' }));
	assert.deepEqual(validate(scene(1, v), 'x').warnings, ['x sb-1-5: 5 vertical shots in a row without a stop (viewport) – no rest']);
	const p = Array.from({ length: 4 }, (_, i) => shot({ id: `sb-1-${i + 1}` }));
	assert.deepEqual(validate(scene(1, p), 'x').warnings, ['x sb-1-4: 4 viewports in a row – flat']);
	const wall = shot({ dialog: [1, 2].map(() => ({ rect: { x: 0, y: 0 }, text: { de: Array(20).fill('Wort').join(' ') } })) });
	assert.deepEqual(validate(scene(1, [wall]), 'x').warnings, ['x sb-1-1: 40 words on 0.7 screens – a wall (> 35 per screen)']);
});

test('validateChapter: every scene named scene-<n>, plus the chapter-wide checks', () => {
	const doc = chapterStoryboard('catpope', 3, [scene(26, [shot({ id: 'sb-26-1' })]), scene(25, [shot({ id: 'sb-25-1' })])]);
	assert.deepEqual(doc.scenes.map((s) => s.source.scene), [25, 26]);
	assert.deepEqual(validateChapter(doc), { errors: [], warnings: [] });
	assert.equal(storyboardKey('catpope', 3), 'storyboard:catpope:3');
	assert.equal(findShot(doc, 'sb-26-1')?.scene.source.scene, 26);
	assert.equal(findShot(doc, 'sb-9-9'), null);
	const bad: ChapterStoryboard = {
		storyboard: 2, toon: 'catpope', chapter: 0,
		scenes: [scene(25, [shot({ id: 'sb-25-1' })]), scene(25, [shot({ id: 'sb-25-1' }), shot({ id: 'sb-25-2', aspect: '1:3' })], { toon: 'other', chapter: 4 })]
	};
	assert.deepEqual(validateChapter(bad).errors, [
		'chapter: chapter 0 – a chapter number ≥ 1',
		'scene-25: chapter 3 – the document is chapter 0',
		'scene-25: toon "other" – the chapter is catpope\'s',
		'scene-25: chapter 4 – the document is chapter 0',
		'scene-25: scene 25 twice'
	]);
	assert.deepEqual(validateChapter(bad).warnings, ['scene-25 sb-25-2: viewport 1:3 is 1.88 slices – a viewport is one slice, 1:1.6']);
	const twoScenes = chapterStoryboard('catpope', 3, [scene(25, [shot({ id: 'sb-25-1' })]), scene(26, [shot({ id: 'sb-25-1' })])]);
	assert.deepEqual(validateChapter(twoScenes).errors, ['scene-26 sb-25-1: duplicate id (also in scene-25)']);
	assert.deepEqual(validateChapter({ storyboard: 2, toon: 'x', chapter: 1 } as ChapterStoryboard).errors, ['chapter: scenes missing']);
});

test('shotSpec: canvas size, merged light, accent colour, cast with persona / look overrides', () => {
	const sb = scene(1, [], { light: { key: 'left', source: 'sun', accent: { color: 'red' } } });
	const s = shotSpec(sb, shot({ format: 'loop360', aspect: '3:1', light: { key: 'back' }, location: 'village', figures: [{ cast: 'plf', persona: 'cat', look: 'helmet' }, { cast: 'ghost' }], dialog: [{ rect: { x: 1, y: 2 }, text: { en: 'hi' } }] }), 2);
	assert.deepEqual([s.w, s.h, s.outW, s.outH], [2 * Math.round(3 * S), 2 * S, Math.round(3 * S), S]);
	assert.deepEqual(s.light, { key: 'back', source: 'sun', accent: { color: 'red' } });
	assert.equal(s.accent, '#c40000');
	assert.equal(s.location?.place, 'mountain-village');
	assert.deepEqual(s.figures[0]!.cast, { ref: 'plf', character: 'catpope', persona: 'cat', look: 'helmet' });
	assert.deepEqual(s.figures[1]!.cast, { ref: 'ghost' });
	assert.equal(s.dialog[0]!.text, 'hi');
	assert.equal(s.dialog[0]!.variant, 'narration');
	assert.equal(shotSpec(sb, shot({ aspect: 'x', light: { accent: { color: '#123' } } })).accent, '#123');
	assert.equal(shotSpec(sb, shot({ aspect: 'x' })).outH, Math.round(W * 16 / 9));
});
