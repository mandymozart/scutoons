// The rig editor's form helpers (#269): what the forms offer and write.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { descendants, inherited, instrumentControls, instrumentOutputs, instrumentPorts, routeEnds, linkedInputs, lookupOf, newTrackId, parentOptions, presetsOf, renameKey, rightsOf, sticksOf, trackPaths, valueRows, withRight, withStick, withValue, withVariantPreset, withVariantValue } from '../src/rigs/edit.ts';
import { fold, type Doc } from '../src/rigs/validate.ts';

const fixture = JSON.parse(readFileSync(new URL('../../../server/tests/fixtures/rigs/scene-content.json', import.meta.url), 'utf8'));
const instruments = Object.entries(fixture.instruments as Record<string, { data: Doc }>).map(([id, e]) => ({ id, data: e.data }));
const presets = Object.entries(fixture.presets as Record<string, { data: Doc }>).map(([id, e]) => ({ id, data: e.data }));
const I = lookupOf(instruments);
const P = lookupOf(presets);
const voice = I('cathedral_voice')!;

test('an instrument offers its control inputs and audio outputs', () => {
	assert.deepEqual(instrumentControls(voice), ['voice.freq', 'voice.ratio', 'voice.index', 'voice.gain', 'melody.spread', 'melody.rate', 'melody.glide']);
	assert.deepEqual(instrumentOutputs(voice), ['voice.out', 'melody.cv']);
	assert.deepEqual(instrumentControls(null), []);
	assert.deepEqual(instrumentPorts(voice).inputs, ['voice.cv', 'voice.freq', 'voice.ratio', 'voice.index', 'voice.gain', 'melody.spread', 'melody.rate', 'melody.glide']);
	const ends = routeEnds((fixture.characters.catpope as any).rig.data, I);
	assert.deepEqual(ends.from, ['voice:voice.out', 'voice:melody.cv', 'drone:drone.out', 'drone:fifth.out', 'drone:mix.out']);
	assert.ok(ends.to.includes('drone:mix.in3') && ends.to.includes('being:glow'));
});

test('a preset may derive from presets of its instrument, never from itself or one below it', () => {
	assert.deepEqual(presetsOf(presets, 'drone').map((p) => p.id), ['drone.base']);
	assert.deepEqual([...descendants(presets, 'cathedral_voice.base')].sort(), ['cathedral_voice.base', 'cathedral_voice.bright', 'cathedral_voice.low']);
	assert.deepEqual(parentOptions(presets, P('cathedral_voice.base')!), []);
	assert.deepEqual(parentOptions(presets, P('cathedral_voice.bright')!), ['cathedral_voice.base', 'cathedral_voice.low']);
	assert.deepEqual(parentOptions(presets, { id: 'new', instrument: 'cathedral_voice' }), ['cathedral_voice.base', 'cathedral_voice.bright', 'cathedral_voice.low']);
});

test('the value rows show what a preset sets, what it inherits and what is linked', () => {
	const low = P('cathedral_voice.low')!;
	const parent = inherited(low, P);
	assert.ok(parent);
	assert.deepEqual(parent.values, {});
	const rows = valueRows(voice, low, parent);
	const ratio = rows.find((r) => r.path === 'voice.ratio')!;
	assert.deepEqual(ratio, { path: 'voice.ratio', own: 0.5, inherits: undefined, base: 1.5, linked: false, exposed: true });
	// the melody drives voice.cv (audio) – no control input is linked in this instrument
	assert.ok(rows.every((r) => !r.linked));
	// a grandchild inherits the folded chain
	const grand = { id: 'g', instrument: 'cathedral_voice', parent: 'cathedral_voice.low' };
	assert.equal(valueRows(voice, grand, inherited(grand, P)).find((r) => r.path === 'voice.index')!.inherits, 1.0);
	assert.equal(inherited({ id: 'x', instrument: 'cathedral_voice' }, P), null);
	assert.equal(inherited({ id: 'x', instrument: 'cathedral_voice', parent: 'gone' }, P), null);
});

test('linked inputs follow the lineage’s unlink and link', () => {
	const swayed = { ...voice, node: [...voice.node, { id: 'sway', kind: 'lfo', rate: 0.1 }], link: [...voice.link, { from: 'sway', to: 'voice.index' }] };
	assert.deepEqual([...linkedInputs(swayed, fold([{ id: 'a', instrument: 'swayed' }]), null)], ['voice.cv', 'voice.index']);
	const parent = fold([{ id: 'root', instrument: 'swayed', unlink: ['voice.index'] }]);
	assert.deepEqual([...linkedInputs(swayed, fold([{ id: 'c', instrument: 'swayed', parent: 'root' }]), parent)], ['voice.cv']);
	const relinked = fold([{ id: 'c', instrument: 'swayed', parent: 'root', link: [{ from: 'sway', to: 'voice.ratio' }] }]);
	assert.deepEqual([...linkedInputs(swayed, relinked, parent)], ['voice.cv', 'voice.ratio']);
});

test('a value is written to the preset or taken out of it', () => {
	const p = { id: 'x', instrument: 'cathedral_voice', values: { 'voice.ratio': 2 } };
	assert.deepEqual(withValue(p, 'voice.index', 4), { id: 'x', instrument: 'cathedral_voice', values: { 'voice.ratio': 2, 'voice.index': 4 } });
	assert.deepEqual(withValue(p, 'voice.ratio', null), { id: 'x', instrument: 'cathedral_voice' });
	assert.deepEqual(withValue({ id: 'x', instrument: 'i', values: { voice: 1 } }, 'voice.out', 2), { id: 'x', instrument: 'i', values: { 'voice.out': 2 } });
});

test('a rig’s tracks, variants and sticks are edited in place of their maps', () => {
	const rig = (fixture.characters.catpope as any).rig.data as Doc;
	assert.equal(newTrackId(rig), 'track1');
	assert.equal(newTrackId({ track: [{ id: 'track1' }, { id: 'track2' }] }), 'track3');
	assert.deepEqual(trackPaths(rig, I).slice(0, 3), ['voice:voice.freq', 'voice:voice.ratio', 'voice:voice.index']);
	assert.deepEqual(withVariantPreset({ presets: { voice: 'a' } }, 'drone', 'b'), { presets: { voice: 'a', drone: 'b' } });
	assert.deepEqual(withVariantPreset({ presets: { voice: 'a' }, values: { x: 1 } }, 'voice', ''), { values: { x: 1 } });
	assert.deepEqual(withVariantValue({}, 'drone:drone.gain', 0.1), { values: { 'drone:drone.gain': 0.1 } });
	assert.deepEqual(withVariantValue({ values: { a: 1 } }, 'a', null), {});
	assert.deepEqual(Object.keys(renameKey({ sad: 1, happy: 2 }, 'sad', 'blue')), ['blue', 'happy']);
	assert.deepEqual(renameKey({ sad: 1, happy: 2 }, 'sad', 'happy'), { sad: 1, happy: 2 });
	assert.deepEqual(sticksOf(rig), ['voice_voice.ratio', 'voice_voice.index', 'voice_melody.spread', 'drone_drone.freq']);
	assert.deepEqual(sticksOf({}), ['', '', '', '']);
	assert.deepEqual(withStick({ subject: 's', track: [] }, 3, 'a_b.c').sticks, [['', ''], ['', 'a_b.c']]);
	assert.deepEqual(withStick({ subject: 's', track: [], sticks: [['x', ''], ['', '']] }, 0, ''), { subject: 's', track: [] });
});

test('rights read with their defaults and go back to none when they are the defaults again', () => {
	assert.deepEqual(rightsOf({}), { lent: ['play', 'tweak'], owned: ['play', 'tweak', 'derive', 'copy'], core: ['play', 'tweak', 'derive'] });
	const lentDerive = withRight({ id: 'x' }, 'lent', 'derive', true);
	assert.deepEqual(lentDerive.rights, { lent: ['play', 'tweak', 'derive'], owned: ['play', 'tweak', 'derive', 'copy'], core: ['play', 'tweak', 'derive'] });
	assert.deepEqual(withRight(lentDerive, 'lent', 'derive', false), { id: 'x' });
	assert.deepEqual(rightsOf({ rights: { lent: [] } }).lent, []);
});
