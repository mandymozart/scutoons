// collage-presets.ts (agents/research/COLLAGE.md §12.4): what the preset grid offers, whether a piece differs from what
// it names, a new preset's values / name / id, the edits of the presets document `colorations`; collage-edit piecePath
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { COLLAGE_LOOKS, PRESET_ID, pieceStyle, type CollagePiece, type CollagePreset, type Colorations } from '@scutoons/content';
import {
	GRID_PRESETS,
	addPreset,
	dropPreset,
	ownDiffers,
	pieceLook,
	placeholderName,
	presetChoices,
	presetIdFor,
	presetName,
	presetValues,
	removePreset,
	renamePreset,
	restorePreset,
	setPieceLook,
	styleOfPreset,
	updatePreset
} from '../src/lib/editor/collage-presets.ts';
import { figurePath, piecePath } from '../src/lib/editor/collage-edit.ts';

const piece = (more: Partial<CollagePiece> = {}): CollagePiece => ({ id: 'piece-a', src: 'vectors/x.svg', ratio: 1, rect: { x: 10, y: 10, width: 30 }, ...more });
const night: CollagePreset = { id: 'night', name: 'Night', fill: '#1a2b6d', blend: 'multiply' };
const many = (n: number, more: Partial<CollagePreset> = {}): CollagePreset[] => Array.from({ length: n }, (_, i) => ({ id: `p${i + 1}`, name: `P ${i + 1}`, ...more }));

test('presetIdFor: a slug, never a code look\'s id or none, never a stored id (deleted ones too), ≤ 40, PRESET_ID', () => {
	assert.equal(presetIdFor('Night', []), 'night');
	assert.equal(presetIdFor('Negative', []), 'negative-2');
	assert.equal(presetIdFor('None', []), 'none-2');
	assert.equal(presetIdFor('Night', [{ ...night, removed: true }]), 'night-2', 'a deleted preset keeps its id');
	assert.equal(presetIdFor('Night', [night, { id: 'night-2', name: 'x' }]), 'night-3');
	assert.equal(presetIdFor('2 Nächte', []), 'p-2-naechte', 'a name starting with a digit gets a letter first');
	assert.equal(presetIdFor('!!!', []), 'x');
	const long = 'A very long preset name that goes on and on and on';
	const first = presetIdFor(long, []);
	const second = presetIdFor(long, [{ id: first, name: long }]);
	for (const id of [first, second, presetIdFor('Negative', []), presetIdFor('2 Nächte', [])]) {
		assert.ok(id.length <= 40, id);
		assert.match(id, PRESET_ID);
	}
	assert.notEqual(first, second);
	assert.ok(second.endsWith('-2'));
	for (const look of COLLAGE_LOOKS) assert.match(presetIdFor(look.id, []), PRESET_ID);
});

test('presetName: trimmed, at most 40, empty = the fallback, a shown name gets " 2" (case-insensitive)', () => {
	assert.equal(presetName('  Night  ', 'Preset 1', []), 'Night');
	assert.equal(presetName('   ', 'Preset 1', []), 'Preset 1');
	assert.equal(presetName('x'.repeat(50), 'Preset 1', []).length, 40);
	assert.equal(presetName('night', 'Preset 2', [night]), 'night 2');
	assert.equal(presetName('Night', 'Preset 3', [night, { id: 'n2', name: 'Night 2' }]), 'Night 3');
	assert.equal(presetName('Night', 'Preset 1', [{ ...night, removed: true }]), 'Night', 'a deleted preset is not shown');
	assert.equal(presetName('Night', 'Night', [night], { except: 'night' }), 'Night', 'a rename keeps its own name');
	assert.equal(presetName('negative', 'Preset 1', [], { also: ['As drawn', 'Negative'] }), 'negative 2', "a code look's name counts");
	// an emoji across the cut goes whole – half of it (a lone surrogate) makes the server refuse the presets document
	const lone = /[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/;
	const moon = `${'x'.repeat(37)}🌙`;
	for (const name of [presetName(`${'x'.repeat(39)}🌙`, 'Preset 1', []), presetName(moon, 'Preset 1', [{ id: 'm', name: moon }])]) assert.ok(name.length <= 40 && !lone.test(name), JSON.stringify(name));
	assert.equal(presetName(`${'x'.repeat(39)}🌙`, 'Preset 1', []), 'x'.repeat(39));
	assert.equal(presetName(`${'x'.repeat(38)}🌙`, 'Preset 1', []), `${'x'.repeat(38)}🌙`, 'a whole emoji at the end stays');
	const cut = presetName('y'.repeat(40), 'Preset 1', [{ id: 'y', name: 'y'.repeat(40) }]);
	assert.equal(cut.length, 40);
	assert.ok(cut.endsWith(' 2'));
	assert.equal(placeholderName([night, { ...night, id: 'gone', removed: true }], (n) => `Preset ${n}`), 'Preset 2');
});

test('presetValues: the piece as drawn – a deep copy, nulls dropped (an effect switched off stays off)', () => {
	const p = piece({ preset: 'sticker', style: { shadow: null, opacity: 0.5 } });
	const values = presetValues(p, []);
	assert.deepEqual(values, { opacity: 0.5, outline: { color: '#ffffff', width: 3 } });
	values.outline!.width = 9;
	assert.equal(pieceStyle(p).outline?.width, 3, 'a copy – the code look is untouched');
	const custom = presetValues(piece({ preset: 'night', style: { blend: 'screen' } }), [night]);
	assert.deepEqual(custom, { blend: 'screen', fill: '#1a2b6d' });
	assert.deepEqual(presetValues(piece(), []), {});
});

test('ownDiffers: false without own values or when they equal the preset, true when they change something', () => {
	assert.equal(ownDiffers(piece(), []), false);
	assert.equal(ownDiffers(piece({ preset: 'negative' }), []), false);
	assert.equal(ownDiffers(piece({ preset: 'negative', style: { fill: '#ffffff' } }), []), false, 'the same value as the look');
	assert.equal(ownDiffers(piece({ preset: 'sticker', style: { outline: { width: 3, color: '#ffffff' } } }), []), false, 'key order does not count');
	assert.equal(ownDiffers(piece({ preset: 'negative', style: { opacity: 0.5 } }), []), true);
	assert.equal(ownDiffers(piece({ preset: 'night', style: { fill: '#000000' } }), [night]), true);
	assert.equal(ownDiffers(piece({ style: { opacity: 0.4 } }), []), true, 'own values on As drawn');
	assert.equal(ownDiffers(piece({ preset: 'ink', style: { blur: null } }), []), false, "switching off what the look hasn't changes nothing");
});

test('presetChoices: live presets in stored order, the deleted one this piece names, the fold from GRID_PRESETS + 1', () => {
	const stored: unknown[] = [night, { id: 'dawn', name: 'Dawn', removed: true }, { id: 'Bad!', name: 'Bad' }, { id: 'night', name: 'Night again' }, null, { id: 'mist', name: '' }];
	const { choices, inGrid } = presetChoices(stored as CollagePreset[], undefined);
	assert.deepEqual(choices, [
		{ id: 'night', name: 'Night', removed: false },
		{ id: 'mist', name: 'mist', removed: false }
	], 'a malformed id, a repeated id, null are left out; no name = the id');
	assert.equal(inGrid, true);
	assert.deepEqual(presetChoices(stored as CollagePreset[], 'dawn').choices.map((c) => [c.id, c.removed]), [['night', false], ['dawn', true], ['mist', false]]);
	assert.equal(presetChoices(many(GRID_PRESETS), undefined).inGrid, true);
	assert.equal(presetChoices(many(GRID_PRESETS + 1), undefined).inGrid, false);
	assert.equal(presetChoices([...many(GRID_PRESETS), { id: 'gone', name: 'Gone', removed: true }], 'gone').inGrid, true, 'only live presets count');
	assert.deepEqual(presetChoices(null, 'night'), { choices: [], inGrid: true });
});

test('the presets document: add (newest last, the six keys only), update keeps id / name / removed, rename, delete = a tombstone', () => {
	const file: Colorations = { presets: [{ ...night }] };
	addPreset(file, { id: 'dawn', name: 'Dawn', fill: '#ff8800', outline: { color: '#ffffff', width: 2 }, ...({ junk: 1 } as object) });
	assert.deepEqual(file.presets[1], { id: 'dawn', name: 'Dawn', fill: '#ff8800', outline: { color: '#ffffff', width: 2 } });
	removePreset(file, 'night');
	const old = styleOfPreset(file, 'night');
	assert.deepEqual(old, { fill: '#1a2b6d', blend: 'multiply' });
	assert.equal(updatePreset(file, 'night', { opacity: 0.5, shadow: { color: '#111111', x: 1, y: 1, blur: 2 } }), true);
	assert.deepEqual(file.presets[0], { id: 'night', name: 'Night', removed: true, opacity: 0.5, shadow: { color: '#111111', x: 1, y: 1, blur: 2 } });
	updatePreset(file, 'night', old!);
	assert.deepEqual(pieceStyle({ preset: 'night' }, file.presets), { blend: 'multiply', fill: '#1a2b6d' }, "an Update's Undo puts the old values back");
	restorePreset(file, 'night');
	assert.equal(file.presets[0].removed, undefined);
	renamePreset(file, 'night', 'Deep night');
	assert.equal(file.presets[0].name, 'Deep night');
	assert.equal(file.presets[0].id, 'night', 'the id stays');
	assert.equal(dropPreset(file, 'dawn'), true);
	assert.deepEqual(file.presets.map((p) => p.id), ['night']);
	assert.equal(updatePreset(file, 'nope', {}), false);
	const empty = {} as Colorations;
	addPreset(empty, { id: 'a', name: 'A' });
	assert.deepEqual(empty, { presets: [{ id: 'a', name: 'A' }] });
});

test("a piece's look as it was (copy) and back – what a toast's Undo gives the piece", () => {
	const p = piece({ preset: 'negative', style: { opacity: 0.5 } });
	const was = pieceLook(p);
	p.preset = 'night';
	delete p.style;
	setPieceLook(p, was);
	assert.deepEqual([p.preset, p.style], ['negative', { opacity: 0.5 }]);
	assert.notEqual(p.style, was.style, 'a copy');
	setPieceLook(p, { preset: undefined, style: {} });
	assert.equal('preset' in p || 'style' in p, false);
});

test('piecePath finds a piece by id in the list the path points into; figurePath only a character picture', () => {
	const file = { pieces: [piece({ id: 'a' }), piece({ id: 'b', figure: { id: 'catpope', view: 'portrait' } })] };
	assert.deepEqual(piecePath(file, ['pieces', 0], 'b'), ['pieces', 1]);
	assert.equal(piecePath(file, ['pieces', 0], 'gone'), undefined);
	assert.deepEqual(figurePath(file, ['pieces', 0], 'b'), ['pieces', 1]);
	assert.equal(figurePath(file, ['pieces', 1], 'a'), undefined);
	assert.equal(piecePath({}, ['pieces', 0], 'a'), undefined);
});
