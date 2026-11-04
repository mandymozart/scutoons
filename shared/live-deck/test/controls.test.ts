// `npm test -w shared/live-deck` (#209, #369): the controls' bridge without a browser, against the core's answers
// (test/fixtures/controls.json – deck-space's controls on the deck's own scene; the logic is tested in Rust): the deck's
// knobs handed in as the core answered them, a stick moving the deck from its values, a knob's position and its drags
// and keys, the panel's line, MIDI learn saving per deck, Web MIDI's wiring. And the two drawing helpers.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Hands, arcPoint, deflection, mappingKey, type Store } from '../src/controls.ts';
import { connectMidi, midiSupported, type MidiAccessLike, type MidiInputLike, type MidiOutputLike } from '../src/midi.ts';
import { answer, controlsOf } from './controls-core.ts';
import { OWN_PARAMS } from './own.ts';

const near = (a: number, b: number, eps = 1e-9) => assert.ok(Math.abs(a - b) < eps, `${a} ≈ ${b}`);

test('a stick: the thumb from where it landed, in radii, y up; a knob\'s arc spans 270°', () => {
	assert.deepEqual(deflection([100, 100], [125, 75], 50), [0.5, 0.5]);
	assert.deepEqual(deflection([0, 0], [300, -400], 50), [6, 8], 'the core keeps it inside the circle');
	const [x0, y0] = arcPoint(0, 10, 12);
	const [x1, y1] = arcPoint(1, 10, 12);
	near(x0, 12 - 10 * Math.SQRT1_2, 1e-6);
	near(y0, 12 + 10 * Math.SQRT1_2, 1e-6);
	near(x1, 12 + 10 * Math.SQRT1_2, 1e-6);
	near(y1, y0, 1e-6);
	near(arcPoint(0.5, 10, 12)[1], 2, 1e-6);
});

function fakeDeck() {
	const value: Record<string, number> = Object.fromEntries(Object.entries(OWN_PARAMS).map(([k, s]) => [k, s.value]));
	return { value, touch: 0 as 0 | 1, setParam(p: string, v: number) { value[p] = v; } };
}

test('Hands: the deck\'s knobs go to the core as it answered them; a stick moves the deck by the core\'s answers', () => {
	const c = controlsOf();
	const deck = fakeDeck();
	const h = new Hands(c, deck, OWN_PARAMS);
	const [, sliders] = c.calls[0] as [string, { path: string }[], string[]];
	assert.deepEqual(sliders.map((s) => s.path), Object.keys(OWN_PARAMS), 'every knob, with its path, in the deck\'s order');
	h.select('being');
	const [px, py] = h.sticks(0);
	near(h.norm(px), answer('norm', px, OWN_PARAMS[px].value));
	h.stickDown(0);
	assert.equal(deck.touch, 1);
	h.stickMove(0, 0.5, 0);
	const [tx, ty] = answer<number[]>('stick_move', 0, 0.5, 0);
	assert.equal(deck.value[px], answer('value_at', px, tx));
	assert.equal(deck.value[py], answer('value_at', py, ty));
	h.stickUp(0);
	assert.equal(deck.touch, 0);
	assert.equal(h.norm('nowhere.x'), answer('norm', 'nowhere.x', NaN), 'not the deck\'s knob: the core says 0');
	h.useParams(OWN_PARAMS, [
		['voice.ratio', 'voice.index'],
		['', 'voice.ratio']
	]);
	assert.deepEqual(c.calls.at(-1)?.[2], ['voice.ratio', 'voice.index', '', 'voice.ratio'], 'a rig\'s pairs, flat');
});

test('a knob: its position, a drag and a key are the core\'s; the panel\'s line and the sticks\' labels too', () => {
	const c = controlsOf();
	const path = Object.keys(OWN_PARAMS)[0];
	const v = OWN_PARAMS[path].value;
	assert.equal(c.norm(path, v), answer('norm', path, v));
	const up = c.turn(path, v, -100, 200, false);
	assert.ok(up !== undefined && up >= v, 'up turns it up');
	assert.equal(c.step(path, v, 'Enter', false), undefined, 'not a key that turns it');
	new Hands(c, fakeDeck(), OWN_PARAMS).select('being');
	assert.match(c.selection_line('Being', 'Master'), /^Being · L .+ \/ .+ · R .+ \/ .+$/);
	assert.equal(c.stick_labels(0).length, 2);
});

test('Hands: MIDI learn saves the deck\'s mappings at once, a bad stored line is told', () => {
	const c = controlsOf();
	const deck = fakeDeck();
	const saved = new Map<string, string>([[mappingKey('plf'), 'cc 1 9 nowhere.x\n']]);
	const store: Store = { get: (k) => saved.get(k) ?? null, set: (k, v) => saved.set(k, v) };
	const h = new Hands(c, deck, OWN_PARAMS, store);
	h.load('plf');
	assert.deepEqual(h.problems, ['cc 1 9 nowhere.x']);
	const path = Object.keys(OWN_PARAMS).find((p) => answer('value_at', p, 1) !== undefined)!;
	h.learn(path);
	assert.deepEqual(h.midi(new Uint8Array([0xb0, 7, 10])), { path, learned: true });
	assert.equal(saved.get(mappingKey('plf')), `cc 1 7 ${path}\n`, 'saved per deck at once');
	assert.deepEqual(h.midi(new Uint8Array([0xb0, 7, 127])), { path, learned: false });
	assert.equal(deck.value[path], answer('value_at', path, 1), 'the CC\'s 0..1 on the knob\'s range – the core\'s');
	assert.equal(h.midi(new Uint8Array([0x90, 60, 1])), null);
	assert.equal(h.mapped(path), 'CC 7 · 1');
	h.unmap(path);
	assert.equal(saved.get(mappingKey('plf')), '');
	assert.equal(mappingKey(''), 'scutoons.live-deck.midi.guest');
});

test('Web MIDI: every input\'s messages, inputs plugged in later, closed cleanly', async () => {
	assert.equal(midiSupported(undefined), false);
	assert.equal(midiSupported({ requestMIDIAccess: () => {} }), true);
	const inputs: MidiInputLike[] = [{ name: 'nanoKONTROL', onmidimessage: null }];
	const access: MidiAccessLike = { inputs: { forEach: (f) => inputs.forEach(f) }, onstatechange: null };
	const got: number[][] = [];
	const lists: string[][] = [];
	const link = await connectMidi((d) => got.push([...d]), (l) => lists.push(l), async () => access);
	inputs[0].onmidimessage!({ data: new Uint8Array([0xb0, 1, 2]) });
	inputs.push({ name: 'LPD8', onmidimessage: null });
	access.onstatechange!({});
	inputs[1].onmidimessage!({ data: new Uint8Array([0xb1, 3, 4]) });
	inputs[1].onmidimessage!({ data: null });
	assert.deepEqual(got, [[0xb0, 1, 2], [0xb1, 3, 4]]);
	assert.deepEqual(lists, [['nanoKONTROL', 'LPD8']]);
	assert.deepEqual(link.inputs(), ['nanoKONTROL', 'LPD8']);
	link.close();
	assert.equal(inputs[0].onmidimessage, null);
});

test('#321 Web MIDI out: the clock out\'s bytes to every output, due at the page\'s ms on Web MIDI\'s own clock', async () => {
	const sent: [string, number[], number | undefined][] = [];
	const out = (name: string): MidiOutputLike => ({ name, send: (d, t) => sent.push([name, d, t]) });
	const access: MidiAccessLike = { inputs: { forEach: () => {} }, outputs: { forEach: (f) => [out('UCX II'), out('loopMIDI')].forEach(f) }, onstatechange: null };
	const link = await connectMidi(() => {}, undefined, async () => access);
	link.send([0xf8], performance.timeOrigin + 1234.5);
	link.send([0xf2, 32, 0], performance.timeOrigin + 2000);
	assert.deepEqual(sent, [
		['UCX II', [0xf8], 1234.5],
		['loopMIDI', [0xf8], 1234.5],
		['UCX II', [0xf2, 32, 0], 2000],
		['loopMIDI', [0xf2, 32, 0], 2000]
	]);
	// a browser without outputs (the narrow shape): nothing, no throw
	const bare = await connectMidi(() => {}, undefined, async () => ({ inputs: { forEach: () => {} }, onstatechange: null }));
	bare.send([0xf8], 0);
});
