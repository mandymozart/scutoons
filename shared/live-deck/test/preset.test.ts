// `npm test -w shared/live-deck` (#270): "save as preset" from the deck's knobs – which track a knob belongs to, what
// changed against the played preset, the body of POST /api/rigs/presets (only the changed values, the played preset as
// the parent). On deck-space's own answers (test/fixtures/rigs.json).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { deriveBody, knobTrack, presetChanges, saveTracks, savedLine } from '../src/preset.ts';
import { rigParams } from '../src/rig.ts';
import type { RigPlayed } from '../src/space.ts';

const RIGS = JSON.parse(readFileSync(new URL('./fixtures/rigs.json', import.meta.url), 'utf8')) as Record<string, RigPlayed>;
const catpope = RIGS.catpope;
/** the knobs as played */
const played = () => Object.fromEntries(Object.entries(rigParams(catpope)).map(([k, s]) => [k, s.value]));

test('a knob belongs to the longest track its path starts with', () => {
	assert.deepEqual(knobTrack(catpope, 'voice_voice.ratio'), { track: 'voice', port: 'voice.ratio' });
	assert.deepEqual(knobTrack(catpope, 'voice_melody.spread'), { track: 'voice', port: 'melody.spread' });
	assert.deepEqual(knobTrack(catpope, 'drone_drone.freq'), { track: 'drone', port: 'drone.freq' });
	assert.equal(knobTrack(catpope, 'mix1.gain1'), null, 'the plumbing is no track');
	assert.deepEqual(knobTrack(RIGS.incense_burner, 'air_hiss.gain'), { track: 'air', port: 'hiss.gain' }, 'a node id with an underscore');
	const twins: RigPlayed = { ...catpope, tracks: [{ track: 'voice', preset: 'a' }, { track: 'voice_voice', preset: 'b' }] };
	assert.deepEqual(knobTrack(twins, 'voice_voice_osc.ratio'), { track: 'voice_voice', port: 'osc.ratio' }, 'the longest id wins');
	assert.deepEqual(knobTrack(twins, 'voice_voice.ratio'), { track: 'voice', port: 'voice.ratio' }, "a track's node `voice` – the `_` after the id decides");
});

test('the tracks a preset can be saved for play a preset and show knobs', () => {
	const t = saveTracks(catpope);
	assert.deepEqual(
		t.map((s) => [s.track, s.parent, s.knobs.map((k) => k.path)]),
		[
			['voice', 'cathedral_voice.base', ['voice_voice.ratio', 'voice_voice.index', 'voice_voice.gain', 'voice_melody.spread', 'voice_melody.rate', 'voice_melody.glide']],
			['drone', 'drone.base', ['drone_drone.freq', 'drone_drone.gain']]
		]
	);
	const bare: RigPlayed = { ...catpope, tracks: [{ track: 'voice' }, { track: 'drone', preset: 'drone.base' }] };
	assert.deepEqual(saveTracks(bare).map((s) => s.track), ['drone'], 'no preset, no parent');
});

test('what changed: the knobs moved away from the played value, as the preset stores them', () => {
	const v = played();
	assert.deepEqual(presetChanges(catpope, 'voice', v), [], 'untouched');
	v['voice_voice.index'] = 4;
	v['drone_drone.gain'] = 0.2;
	v['voice_voice.ratio'] = 1.5; // set to what it was
	const c = presetChanges(catpope, 'voice', v);
	assert.deepEqual(c.map((x) => [x.path, x.from, x.to]), [['voice.index', 2, 4]], "only this track's moved knobs");
	assert.equal(c[0].knob.label?.en, 'FM index');
	assert.deepEqual(presetChanges(catpope, 'drone', v).map((x) => x.path), ['drone.gain']);
	v['voice_voice.gain'] = NaN;
	assert.deepEqual(presetChanges(catpope, 'voice', v).map((x) => x.path), ['voice.index'], 'a broken value is no change');
});

test('the request: the played preset as the parent, only the changed values, a trimmed name', () => {
	const v = played();
	v['voice_voice.index'] = 4;
	v['voice_voice.ratio'] = 3;
	assert.deepEqual(deriveBody(catpope, 'voice', v, '  Vespers   bright ', 'de'), {
		parent: 'cathedral_voice.base',
		name: { de: 'Vespers bright' },
		values: { 'voice.index': 4, 'voice.ratio': 3 }
	});
	assert.deepEqual(deriveBody(catpope, 'drone', v), { parent: 'drone.base', values: {} }, 'nothing moved: still a derivative (the server keeps its lineage)');
	assert.equal(deriveBody(catpope, 'nobody', v), null);
	const bare: RigPlayed = { ...catpope, tracks: [{ track: 'voice' }] };
	assert.equal(deriveBody(bare, 'voice', v), null, 'a track without a preset has no parent');
	assert.equal(deriveBody(catpope, 'voice', v, 'x'.repeat(120))!.name!.en.length, 100);
});

test('the status line after a save', () => {
	assert.equal(savedLine({ id: 'm:01ARZ3NDEKTSV4RRFFQ69G5FAV', instrument: 'cathedral_voice', name: { de: 'Vesper' }, values: { 'voice.index': 4 } }, 'de'), 'SAVED VESPER · 1 VALUE');
	assert.equal(savedLine({ id: 'm:01ARZ3NDEKTSV4RRFFQ69G5FAV', instrument: 'cathedral_voice' }), 'SAVED G5FAV · 0 VALUES'.replace('G5FAV', '9G5FAV'));
});
