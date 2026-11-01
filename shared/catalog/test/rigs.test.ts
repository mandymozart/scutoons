// The rig validator (#269) against what the server and deck-space accept: the core rigs as scene-content answers them
// (server/tests/fixtures/rigs/scene-content.json, the contract fixture deck-space's tests read too) pass; what
// rig/tests.rs refuses is refused here with the server's words.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { validateInstrument, validatePreset, validateRig, lineage, fold, knobPaths, type Doc, type Lookup } from '../src/rigs/validate.ts';
import { checkSchema, RIG_SCHEMA } from '../src/rigs/schema.ts';
import { buildGraph, checkEvents, controlInputs, audioOutputs } from '../src/rigs/graph.ts';

const fixture = JSON.parse(readFileSync(new URL('../../../server/tests/fixtures/rigs/scene-content.json', import.meta.url), 'utf8'));
const instruments: Record<string, Doc> = Object.fromEntries(Object.entries(fixture.instruments as Record<string, { data: Doc }>).map(([id, e]) => [id, e.data]));
const presets: Record<string, Doc> = Object.fromEntries(Object.entries(fixture.presets as Record<string, { data: Doc }>).map(([id, e]) => [id, e.data]));
const I: Lookup = (id) => instruments[id] ?? null;
const P: Lookup = (id) => presets[id] ?? null;
const rigs: { id: string; data: Doc }[] = [...Object.values(fixture.characters as Record<string, any>), ...Object.values(fixture.items as Record<string, any>)]
	.filter((e) => e.rig)
	.map((e) => ({ id: e.rig.id, data: e.rig.data }));

const clone = <T>(v: T): T => structuredClone(v);

test('the core rigs, their instruments and presets pass as the server stores them', () => {
	assert.ok(Object.keys(instruments).length >= 3 && rigs.length >= 2, 'the fixture holds the core set');
	for (const [id, i] of Object.entries(instruments)) assert.deepEqual(validateInstrument(i), [], id);
	for (const [id, p] of Object.entries(presets)) assert.deepEqual(validatePreset(p, I, P), [], id);
	for (const r of rigs) assert.deepEqual(validateRig(r.data, r.id, I, P), [], r.id);
});

test('the schema knows the node kinds, the scene-only kinds and the knob limit', () => {
	assert.equal(RIG_SCHEMA.nodes.fm.inputs.ratio, 'control');
	assert.equal(RIG_SCHEMA.nodes.fm.outputs.out, 'audio');
	assert.ok(RIG_SCHEMA.scene_only.includes('audio_out'));
	assert.equal(RIG_SCHEMA.event_knobs, 4);
});

test('the schema check words structure like the server: unknown fields, required fields, types', () => {
	const who = 'instrument x';
	assert.deepEqual(checkSchema(RIG_SCHEMA.instrument, RIG_SCHEMA.instrument, { id: 'x', kind: 'lead', node: [], out: 'a', colour: 1 }, who), [`${who}: unknown field "colour" (id, kind, name, node, link, out, expose, rights)`.replace('(id, kind, name, node, link, out, expose, rights)', `(${Object.keys((RIG_SCHEMA.instrument as any).properties).join(', ')})`)]);
	assert.deepEqual(checkSchema(RIG_SCHEMA.instrument, RIG_SCHEMA.instrument, { id: 'x', kind: 'lead', node: [] }, who), [`${who}: out is required`]);
	assert.deepEqual(checkSchema(RIG_SCHEMA.instrument, RIG_SCHEMA.instrument, { id: 'x', kind: 'organ', node: [], out: 'a' }, who), [`${who}: kind must be one of drum, bass, lead, pad, voice, sampler, texture, fx`]);
	assert.deepEqual(checkSchema(RIG_SCHEMA.rig, RIG_SCHEMA.rig, { subject: 's', track: 'voice' }, 'rig s'), ['rig s: track must be a list of { id, instrument, gain?, mute?, pan?, preset? }']);
	assert.deepEqual(checkSchema(RIG_SCHEMA.rig, RIG_SCHEMA.rig, { subject: 's', track: [{ id: 'v', instrument: 'i', gain: 'loud' }] }, 'rig s'), ['rig s: track[0]: gain must be a number']);
	assert.deepEqual(checkSchema(RIG_SCHEMA.rig, RIG_SCHEMA.rig, { subject: 's', track: [], sticks: [['a', 'b']] }, 'rig s'), ['rig s: sticks must have 2 entries']);
	assert.deepEqual(checkSchema(RIG_SCHEMA.preset, RIG_SCHEMA.preset, { id: 'p', instrument: 'i', rights: { lent: ['sing'] } }, 'preset p'), ['preset p: rights: lent[0] must be one of play, tweak, derive, copy']);
	// a number past f32, a null where serde reads a default
	assert.deepEqual(checkSchema(RIG_SCHEMA.rig, RIG_SCHEMA.rig, { subject: 's', track: [{ id: 'v', instrument: 'i', gain: 1e39 }] }, 'rig s'), ['rig s: track[0]: gain must be a number']);
	assert.deepEqual(checkSchema(RIG_SCHEMA.rig, RIG_SCHEMA.rig, { subject: 's', track: [{ id: 'v', instrument: 'i', preset: null, gain: null }] }, 'rig s'), ['rig s: track[0]: gain must be a number – leave it out instead of null']);
});

test('an instrument is a graph without scene-only kinds, an audio out and control inputs on show', () => {
	const voice = clone(instruments.cathedral_voice);
	assert.deepEqual(controlInputs([{ id: 'v', kind: 'fm', params: {} }]), ['v.freq', 'v.ratio', 'v.index', 'v.gain']);
	assert.deepEqual(audioOutputs([{ id: 'v', kind: 'fm', params: {} }, { id: 'l', kind: 'lfo', params: {} }]), ['v.out']);
	const bad = clone(voice);
	bad.node.push({ id: 'spk', kind: 'audio_out' });
	assert.deepEqual(validateInstrument(bad), ['instrument cathedral_voice: a audio_out node belongs to the scene, not to an instrument (spk)']);
	const out = { ...clone(voice), out: 'melody' };
	assert.deepEqual(validateInstrument(out), ["instrument cathedral_voice: out 'melody' is not an audio output"]);
	const expose = clone(voice);
	expose.expose.push({ path: 'voice.cv' });
	assert.deepEqual(validateInstrument(expose), ['instrument cathedral_voice: voice.cv is not a control input of fm']);
	const twice = clone(voice);
	twice.link.push({ from: 'melody.cv', to: 'voice.cv' });
	assert.deepEqual(validateInstrument(twice), ['instrument cathedral_voice: link melody.cv → voice.cv: the input already has a link']);
	const typed = clone(voice);
	typed.link = [{ from: 'melody.cv', to: 'voice.freq' }];
	assert.deepEqual(validateInstrument(typed), ['instrument cathedral_voice: link melody.cv → voice.freq: a audio output cannot feed a control input']);
	const unknown = clone(voice);
	unknown.node[0].kind = 'organ';
	assert.deepEqual(validateInstrument(unknown), ["instrument cathedral_voice: node 'voice': unknown kind 'organ'"]);
	const nul = clone(voice);
	nul.node[0].freq = null;
	assert.deepEqual(validateInstrument(nul), ['instrument cathedral_voice: node voice: freq has a null (TOML has none – leave it out)']);
	assert.deepEqual(validateInstrument({ ...clone(voice), id: 'Cathedral' }), ['instrument Cathedral: id must be lower-case letters, digits, _ or -']);
});

test('a graph with a cycle over ordering links is refused, a control loop through audio is not', () => {
	const cyc = buildGraph('g', [{ id: 'a', kind: 'lfo', params: {} }, { id: 'b', kind: 'lfo', params: {} }], [{ from: 'a', to: 'b.rate' }, { from: 'b', to: 'a.rate' }]);
	assert.deepEqual(cyc, { problems: ['g: the graph has a cycle'] });
	// an fft (audio) feeding a control back into an osc's gain never orders (Link::orders)
	const ok = buildGraph('g', [{ id: 'o', kind: 'osc', params: {} }, { id: 'f', kind: 'fft', params: {} }], [{ from: 'o', to: 'f.in' }, { from: 'f.level', to: 'o.gain' }]);
	assert.ok('kinds' in ok);
});

test('an events node must hold a patch that builds: generators, edges, out, at most 4 knobs', () => {
	const ok = { generator: [{ id: 'w', kind: 'random_walk' }], out: 'w.notes', knobs: ['w.spread'] };
	assert.deepEqual(checkEvents('e', ok), []);
	assert.deepEqual(checkEvents('e', { ...ok, knobs: ['w.spread', 'w.spread', 'w.spread', 'w.spread', 'w.spread'] }), ['e: at most 4 knobs (a, b, c, d), not 5']);
	assert.deepEqual(checkEvents('e', { ...ok, knobs: ['w.loudness'] }), ["e: w has no param 'loudness'"]);
	assert.deepEqual(checkEvents('e', { generator: [{ id: 'w', kind: 'random_walk' }] }), ['e: out is required (<generator>.<port> the host plays)']);
	assert.deepEqual(checkEvents('e', { generator: [{ id: 'a', kind: 'arp' }, { id: 'b', kind: 'arp' }], edge: [{ from: 'a.notes', to: 'b.notes' }, { from: 'b.notes', to: 'a.notes' }], out: 'a.notes' }), ['e: the generators loop without a delayed edge']);
	assert.deepEqual(checkEvents('e', { generator: [{ id: 'a', kind: 'arp' }, { id: 'b', kind: 'arp' }], edge: [{ from: 'a.notes', to: 'b.notes' }, { from: 'b.notes', to: 'a.notes', delayed: true }], out: 'a.notes' }), []);
});

test('a lineage runs root first; loops, strangers and orphans are refused', () => {
	const chain = lineage('cathedral_voice.bright', P);
	assert.ok('chain' in chain);
	assert.deepEqual(chain.chain.map((p) => p.id), ['cathedral_voice.base', 'cathedral_voice.bright']);
	assert.deepEqual(fold(chain.chain).values, { 'voice.ratio': 3.0 });
	const loop: Lookup = (id) => ({ a: { id: 'a', instrument: 'i', parent: 'b' }, b: { id: 'b', instrument: 'i', parent: 'a' } })[id] ?? null;
	assert.deepEqual(lineage('a', loop), { problem: 'preset a: its lineage loops at "a"' });
	const stranger: Lookup = (id) => ({ a: { id: 'a', instrument: 'i', parent: 'b' }, b: { id: 'b', instrument: 'j' } })[id] ?? null;
	assert.deepEqual(lineage('a', stranger), { problem: 'preset a (i) derives from "b" of another instrument (j)' });
	assert.deepEqual(lineage('a', (id) => (id === 'a' ? { id: 'a', instrument: 'i', parent: 'gone' } : null)), { problem: 'preset a: unknown preset "gone" (its parent)' });
});

test('a preset sets unlinked control inputs of its instrument only', () => {
	const bright = clone(presets['cathedral_voice.bright']);
	assert.deepEqual(validatePreset({ ...bright, values: { 'voice.cv': 1 } }, I, P), ['preset cathedral_voice.bright: voice.cv is not a control input of fm']);
	assert.deepEqual(validatePreset({ ...bright, values: { 'voice.loud': 1 } }, I, P), ['preset cathedral_voice.bright: voice.loud is not a control input of fm']);
	// voice.cv is linked from the melody: a value on a linked input needs an unlink first
	const linked = { ...bright, link: [{ from: 'melody.cv', to: 'voice.freq' }] };
	assert.deepEqual(validatePreset(linked, I, P), ['preset cathedral_voice.bright (with its instrument): link melody.cv → voice.freq: a audio output cannot feed a control input']);
	assert.deepEqual(validatePreset({ ...bright, unlink: ['voice.nothing'] }, I, P), ['preset cathedral_voice.bright: no input voice.nothing to re-route']);
	assert.deepEqual(validatePreset({ ...bright, instrument: 'drone' }, I, P), ['preset cathedral_voice.bright (drone) derives from "cathedral_voice.base" of another instrument (cathedral_voice)']);
	assert.deepEqual(validatePreset({ ...bright, instrument: 'kazoo' }, I, P), ['preset cathedral_voice.bright: unknown instrument "kazoo"']);
	assert.deepEqual(validatePreset({ ...bright, id: 'Bright!' }, I, P), ['preset Bright!: id is not a valid preset id']);
	assert.deepEqual(validatePreset({ ...bright, mapping: [{ source: 'face:jawOpen', target: 'voice.gain' }] }, I, P), []);
	assert.deepEqual(validatePreset({ ...bright, mapping: [{ source: 'face:jawOpen', target: 'voice.cv' }] }, I, P), ['preset cathedral_voice.bright: voice.cv is not a control input of fm']);
	// a sway LFO linked into the index by a parent, freed by the child: the child's value is fine
	const lfo = { id: 'sway', kind: 'lfo', rate: 0.1, depth: 1.0 };
	const inst = { ...clone(instruments.cathedral_voice), id: 'swayed', node: [...clone(instruments.cathedral_voice.node), lfo], link: [...clone(instruments.cathedral_voice.link), { from: 'sway', to: 'voice.index' }] };
	const lib: Record<string, Doc> = { root: { id: 'root', instrument: 'swayed' }, free: { id: 'free', instrument: 'swayed', parent: 'root', unlink: ['voice.index'], values: { 'voice.index': 5 } }, stuck: { id: 'stuck', instrument: 'swayed', parent: 'root', values: { 'voice.index': 5 } } };
	const L: Lookup = (id) => lib[id] ?? null;
	const II: Lookup = (id) => (id === 'swayed' ? inst : null);
	assert.deepEqual(validatePreset(lib.free, II, L), []);
	assert.deepEqual(validatePreset(lib.stuck, II, L), ['preset stuck: voice.index is linked – a preset value needs an unlinked input (unlink it first)']);
});

test('a rig names known tracks, instruments, presets of the right instrument, knobs for its sticks', () => {
	const catpope = rigs.find((r) => r.id === 'character.catpope')!;
	const rig = clone(catpope.data);
	assert.deepEqual(knobPaths(rig, I), ['voice_voice.ratio', 'voice_voice.index', 'voice_voice.gain', 'voice_melody.spread', 'voice_melody.rate', 'voice_melody.glide', 'drone_drone.freq', 'drone_drone.gain']);
	assert.deepEqual(validateRig({ ...rig, subject: 'character.other' }, catpope.id, I, P), ['rig character.catpope: subject must be "character.catpope"']);
	assert.deepEqual(validateRig(rig, 'Catpope', I, P), ['rig Catpope: the id is a subject – character.<id>[.<persona>] or item.<id>', 'rig Catpope: subject must be "Catpope"']);
	const twice = clone(rig);
	twice.track.push({ ...twice.track[0] });
	assert.deepEqual(validateRig(twice, catpope.id, I, P), ["rig character.catpope: track 'voice' twice"]);
	const dotted = clone(rig);
	dotted.track[0].id = 'voice.1';
	assert.deepEqual(validateRig(dotted, catpope.id, I, P), ["rig character.catpope: a track id has no ':' or '.' (voice.1)", "rig character.catpope: mood happy: no track 'voice'", "rig character.catpope: mood laughing: no track 'voice'", "rig character.catpope: mood laughing: no track 'voice'", "rig character.catpope: mood sad: no track 'voice'", "rig character.catpope: mood surprised: no track 'voice'", "rig character.catpope: mood surprised: no track 'voice'", "rig character.catpope: mood thinking: no track 'voice'", "rig character.catpope: stick 'voice_voice.ratio' is not one of its knobs", "rig character.catpope: stick 'voice_voice.index' is not one of its knobs", "rig character.catpope: stick 'voice_melody.spread' is not one of its knobs"]);
	const wrong = clone(rig);
	wrong.track[1].preset = 'cathedral_voice.base';
	assert.deepEqual(validateRig(wrong, catpope.id, I, P), ["rig character.catpope: track 'drone' plays 'drone', preset 'cathedral_voice.base' is for 'cathedral_voice'"]);
	const unknown = clone(rig);
	unknown.track[1].instrument = 'kazoo';
	assert.deepEqual(validateRig(unknown, catpope.id, I, P), ['rig character.catpope: unknown instrument "kazoo"', "rig character.catpope: track 'drone' plays 'kazoo', preset 'drone.base' is for 'drone'", "rig character.catpope: stick 'drone_drone.freq' is not one of its knobs"]);
	const held = validateRig(rig, catpope.id, I, P, ['cathedral_voice']);
	assert.deepEqual(held, ['rig character.catpope: track drone: the instrument "drone" is not one held']);
	const mood = clone(rig);
	mood.moods.sad.values = { 'drone:drone.cv': 1, 'nobody:x.y': 1, drone: 1 };
	assert.deepEqual(validateRig(mood, catpope.id, I, P), ['rig character.catpope: mood sad: instrument drone: drone.cv is not a control input of osc', "rig character.catpope: mood sad: no track 'nobody'", "rig character.catpope: mood sad: 'drone' is not <track>:<node>.<port>"]);
	const route = { ...clone(rig), route: [{ from: 'voice:melody.cv', to: 'being:glow' }, { from: 'being:glow', to: 'drone:drone.gain' }, { from: 'ghost:a.b', to: 'drone:drone.gain' }, { from: 'noscope', to: 'drone:drone.gain' }] };
	assert.deepEqual(validateRig(route, catpope.id, I, P), ["rig character.catpope: no track 'being' (route being:glow)", "rig character.catpope: no track 'ghost' (route ghost:a.b)", "rig character.catpope: route end 'noscope' has no scope"]);
	const sticks = { ...clone(rig), sticks: [['voice_voice.ratio', ''], ['', 'drone_fifth.freq']] };
	assert.deepEqual(validateRig(sticks, catpope.id, I, P), ["rig character.catpope: stick 'drone_fifth.freq' is not one of its knobs"]);
	const muted = clone(rig);
	muted.track[1].mute = true;
	assert.deepEqual(validateRig(muted, catpope.id, I, P), ["rig character.catpope: stick 'drone_drone.freq' is not one of its knobs"]);
});
