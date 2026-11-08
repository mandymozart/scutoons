// The Assignment tab's document edits (#283): pure, on catpope's rig as the core writes it.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { addTrack, emptyRig, faceTrack, moodPreset, moodRows, ownedBy, ownedInputs, policyOf, presetsOf, removeTrack, setMoodPreset, setPolicy, setTrack, trackId, type CatalogPreset, type RigDoc, type RigFace } from '../src/assignment.ts';

const PRESETS: CatalogPreset[] = [
	{ id: 'cathedral_voice.base', instrument: 'cathedral_voice' },
	{ id: 'cathedral_voice.low', instrument: 'cathedral_voice' },
	{ id: 'cathedral_voice.bright', instrument: 'cathedral_voice' },
	{ id: 'drone.base', instrument: 'drone' }
];

const catpope = (): RigDoc => ({
	subject: 'character.catpope',
	track: [
		{ id: 'voice', instrument: 'cathedral_voice', preset: 'cathedral_voice.base' },
		{ id: 'drone', instrument: 'drone', preset: 'drone.base' }
	],
	sticks: [['voice_voice.ratio', 'voice_voice.index'], ['voice_melody.spread', 'drone_drone.freq']],
	moods: {
		sad: { presets: { voice: 'cathedral_voice.low' }, values: { 'drone:drone.gain': 0.14 } },
		happy: { presets: { voice: 'cathedral_voice.bright' } }
	}
});

test('a new rig has its subject and no tracks; a track gets a free id and its instrument’s first preset', () => {
	const rig = addTrack(emptyRig('character.boy'), 'cathedral_voice', PRESETS);
	assert.deepEqual(rig, { subject: 'character.boy', track: [{ id: 'cathedral_voice', instrument: 'cathedral_voice', preset: 'cathedral_voice.base' }] });
	const two = addTrack(rig, 'cathedral_voice', PRESETS);
	assert.equal(two.track[1].id, 'cathedral_voice2');
	assert.deepEqual(addTrack(emptyRig('x'), 'kazoo', PRESETS).track[0], { id: 'kazoo', instrument: 'kazoo' }, 'no preset: none named');
	assert.equal(trackId(emptyRig('x'), 'a.b:c'), 'a_b_c', 'no : or . in a track id');
});

test('the preset picker lists the instrument’s presets only', () => {
	assert.deepEqual(presetsOf('drone', PRESETS).map((p) => p.id), ['drone.base']);
});

test('a track’s gain or preset changes alone; the fields the form does not edit travel along', () => {
	const rig = setTrack(catpope(), 'drone', { gain: 0.5 });
	assert.equal(rig.track[1].gain, 0.5);
	assert.deepEqual(rig.sticks, catpope().sticks);
	assert.deepEqual(rig.moods, catpope().moods);
	assert.equal(setTrack(catpope(), 'drone', { preset: '' }).track[1].preset, undefined, 'none: the instrument bare');
	const same = catpope();
	assert.equal(setTrack(same, 'nope', { gain: 2 }), same, 'an unknown track: unchanged');
});

test('another instrument drops the presets of the old one – the track’s and the moods’', () => {
	const rig = setTrack(catpope(), 'voice', { instrument: 'drone' });
	assert.deepEqual(rig.track[0], { id: 'voice', instrument: 'drone' });
	assert.equal(rig.moods?.happy, undefined, 'happy only named the voice: gone');
	assert.deepEqual(rig.moods?.sad, { values: { 'drone:drone.gain': 0.14 } });
});

test('a mood’s preset per track: set, then back to the track’s own; an empty mood is dropped', () => {
	let rig = setMoodPreset(catpope(), 'thinking', 'voice', 'cathedral_voice.low');
	assert.equal(moodPreset(rig, 'thinking', 'voice'), 'cathedral_voice.low');
	assert.equal(moodPreset(rig, 'thinking', 'drone'), null);
	rig = setMoodPreset(rig, 'thinking', 'voice', null);
	assert.equal(rig.moods?.thinking, undefined);
	rig = setMoodPreset(rig, 'happy', 'voice', null);
	assert.deepEqual(Object.keys(rig.moods ?? {}), ['sad']);
	rig = setMoodPreset(rig, 'sad', 'voice', null);
	assert.deepEqual(rig.moods?.sad, { values: { 'drone:drone.gain': 0.14 } }, 'the values stay');
});

test('a removed track takes its variants and routes with it', () => {
	const rig = removeTrack({ ...catpope(), route: [{ from: 'voice:voice.out', to: 'being:hue' }, { from: 'drone:mix.out', to: 'being:glow' }] }, 'voice');
	assert.deepEqual(rig.track.map((t) => t.id), ['drone']);
	assert.deepEqual(rig.route, [{ from: 'drone:mix.out', to: 'being:glow' }]);
	assert.deepEqual(rig.moods, { sad: { values: { 'drone:drone.gain': 0.14 } } });
	assert.equal(removeTrack(catpope(), 'drone').moods?.sad?.values, undefined);
});

test('the mood rows: the rig’s own first, then the offered ones it has none for', () => {
	assert.deepEqual(moodRows(catpope(), ['calm', 'happy', 'thinking']), ['sad', 'happy', 'calm', 'thinking']);
	assert.deepEqual(moodRows(emptyRig('x'), []), []);
});

test('a policy on the rig: named, then replaced; what it holds stays', () => {
	assert.equal(policyOf(catpope()), null);
	assert.equal(policyOf(null), null);
	const rig = setPolicy(catpope(), 'catpope_rig');
	assert.equal(policyOf(rig), 'catpope_rig');
	assert.deepEqual(setPolicy({ ...rig, policy: { module: 'old', seed: 7, values: { heat: 0.3 } } }, 'next').policy, { module: 'next', seed: 7, values: { heat: 0.3 } });
	assert.deepEqual(rig.moods, catpope().moods, 'the tables stay');
});

test('the face’s owners, as the core says: only what it marks owned locks a row', () => {
	const set = { output: 'policy.voice_index', mode: 'set', owned: true };
	const add = { output: 'policy.drone_lift', mode: 'add', owned: false };
	const face: RigFace = {
		subject: 'character.catpope',
		personas: [],
		moods: [],
		tracks: [
			{ id: 'voice', instrument: 'v', preset_driven: { output: 'policy.voice_preset', mode: 'set', owned: true }, addressable: [{ path: 'voice:voice.index', input: 'voice_voice.index', min: 0, max: 40, driven: set }, { path: 'voice:voice.ratio', input: 'voice_voice.ratio', min: 0, max: 8 }] },
			{ id: 'drone', instrument: 'd', gain_driven: add, addressable: [{ path: 'drone:drone.freq', input: 'drone_drone.freq', min: 20, max: 4000, log: true, driven: add }] }
		]
	};
	assert.equal(ownedBy(faceTrack(face, 'voice')?.preset_driven)?.output, 'policy.voice_preset');
	assert.equal(ownedBy(faceTrack(face, 'drone')?.gain_driven), null, 'an add layers over the form');
	assert.equal(faceTrack(null, 'voice'), undefined);
	assert.deepEqual(ownedInputs(face), [{ path: 'voice:voice.index', by: set }]);
	assert.deepEqual(ownedInputs(null), []);
});