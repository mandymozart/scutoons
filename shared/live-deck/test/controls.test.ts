// `npm test -w shared/live-deck` (#209): the controls' bridge without a browser – the stick's deflection, the knob's
// turn and arc, Hands on a stand-in for deck-space's DeckControls (the logic itself is tested in Rust:
// deck/space/src/controls.rs), Web MIDI's wiring.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Hands, arcPoint, deflection, knobTurn, mappingKey, type DeckControlsView, type Store } from '../src/controls.ts';
import { connectMidi, midiSupported, type MidiAccessLike, type MidiInputLike, type MidiOutputLike } from '../src/midi.ts';
import { MELODY, PARAMS } from '../src/scene.ts';

const near = (a: number, b: number, eps = 1e-9) => assert.ok(Math.abs(a - b) < eps, `${a} ≈ ${b}`);

test('a stick: the thumb from where it landed, in radii, y up, inside the circle', () => {
	assert.deepEqual(deflection([100, 100], [100, 100], 50), [0, -0]);
	assert.deepEqual(deflection([100, 100], [125, 75], 50), [0.5, 0.5]);
	const [x, y] = deflection([0, 0], [300, -400], 50);
	near(x, 0.6);
	near(y, 0.8);
});

test('a rotary knob: drag up turns it up, Shift is fine, it stops at its ends; the arc spans 270°', () => {
	near(knobTurn(0.5, -100, 200), 1);
	near(knobTurn(0.5, 50, 200), 0.25);
	near(knobTurn(0.5, 50, 200, true), 0.475);
	assert.equal(knobTurn(0.9, -1000, 200), 1);
	const [x0, y0] = arcPoint(0, 10, 12);
	const [x1, y1] = arcPoint(1, 10, 12);
	near(x0, 12 - 10 * Math.SQRT1_2, 1e-6);
	near(y0, 12 + 10 * Math.SQRT1_2, 1e-6);
	near(x1, 12 + 10 * Math.SQRT1_2, 1e-6);
	near(y1, y0, 1e-6);
	const [, top] = arcPoint(0.5, 10, 12);
	near(top, 2, 1e-6);
});

/** a stand-in for deck-space's DeckControls: the being's left pair, MIDI learn of one CC */
function fakeControls() {
	const calls: string[] = [];
	let learning = '';
	let bound = '';
	let last = { value: 0, learned: false };
	let start = [0, 0];
	const c: DeckControlsView = {
		set_deck: (paths, sticks) => calls.push(`deck ${paths.join(',')} | ${sticks.join(',')}`),
		select: (n) => calls.push(`select ${n}`),
		selection: () => 'being',
		stick_paths: () => ['voice.ratio', 'voice.index'],
		stick_down: (_s, x, y) => (start = [x, y]),
		stick_move: (_s, dx, dy) => [start[0] + dx * 0.6, start[1] + dy * 0.6],
		stick_driving: () => ['voice.ratio', 'voice.index'],
		stick_up: () => calls.push('up'),
		assign: () => {},
		learn: (p) => (learning = p),
		learning: () => learning,
		midi: (m) => {
			if (m[0] !== 0xb0) return '';
			if (learning) {
				bound = learning;
				learning = '';
				last = { value: 0, learned: true };
				return bound;
			}
			last = { value: m[2] / 127, learned: false };
			return bound;
		},
		value: () => last.value,
		learned: () => last.learned,
		mapped: (p) => (p === bound ? 'CC 7 · 1' : ''),
		unmap: () => (bound = ''),
		to_text: () => (bound ? `cc 1 7 ${bound}\n` : ''),
		load_text: (t) => (t.includes('nowhere') ? ['cc 1 9 nowhere.x'] : [])
	};
	return { c, calls };
}

function fakeDeck() {
	const value: Record<string, number> = Object.fromEntries(Object.entries({ ...PARAMS, ...MELODY }).map(([k, s]) => [k, s.value]));
	return { value, touch: 0 as 0 | 1, setParam(p: string, v: number) { value[p] = v; } };
}

test('Hands: a stick moves the deck from its values, MIDI learn saves the deck\'s mappings, a bad stored line is told', () => {
	const { c } = fakeControls();
	const deck = fakeDeck();
	const saved = new Map<string, string>([[mappingKey('plf'), 'cc 1 9 nowhere.x\n']]);
	const store: Store = { get: (k) => saved.get(k) ?? null, set: (k, v) => saved.set(k, v) };
	const h = new Hands(c, deck, { ...PARAMS, ...MELODY }, store);
	h.load('plf');
	assert.deepEqual(h.problems, ['cc 1 9 nowhere.x']);
	h.select('being');
	assert.equal(h.selection, 'being');
	const ratio = h.norm('voice.ratio');
	h.stickDown(0);
	assert.equal(deck.touch, 1);
	h.stickMove(0, 0.5, 0);
	near(h.norm('voice.ratio'), ratio + 0.3, 1e-9);
	h.stickUp(0);
	assert.equal(deck.touch, 0);
	h.learn('drone.gain');
	assert.deepEqual(h.midi(new Uint8Array([0xb0, 7, 10])), { path: 'drone.gain', learned: true });
	assert.equal(saved.get(mappingKey('plf')), 'cc 1 7 drone.gain\n', 'saved per deck at once');
	assert.deepEqual(h.midi(new Uint8Array([0xb0, 7, 127])), { path: 'drone.gain', learned: false });
	near(deck.value['drone.gain'], PARAMS['drone.gain'].max);
	assert.equal(h.midi(new Uint8Array([0x90, 60, 1])), null);
	assert.equal(h.mapped('drone.gain'), 'CC 7 · 1');
	h.unmap('drone.gain');
	assert.equal(saved.get(mappingKey('plf')), '');
	assert.equal(mappingKey(''), 'scutoons.live-deck.midi.guest');
});

test('Web MIDI: every input\'s messages, inputs plugged in later, closed cleanly', async () => {
	assert.equal(midiSupported(undefined), false);
	assert.equal(midiSupported({ requestMIDIAccess: () => {} }), true);
	const inputs: MidiInputLike[] = [{ name: 'nanoKONTROL', onmidimessage: null }];
	const access: MidiAccessLike = { inputs: { forEach: (f) => inputs.forEach(f) }, onstatechange: null };
	const got: number[][] = [];
	let lists: string[][] = [];
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
	lists = [];
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

test('Hands: a rig gives the controls its knobs and sticks, null the fixed patch\'s defaults', () => {
	const { c, calls } = fakeControls();
	const h = new Hands(c, fakeDeck(), { ...PARAMS, ...MELODY });
	const rig = { 'a_x.p': { min: 0, max: 1, value: 0, label: 'p' }, 'a_x.q': { min: 0, max: 1, value: 0, label: 'q' } };
	h.useParams(rig, [
		['a_x.p', 'a_x.q'],
		['', 'a_x.p']
	]);
	assert.equal(calls.at(-1), 'deck a_x.p,a_x.q | a_x.p,a_x.q,,a_x.p');
	assert.equal(h.norm('voice.ratio'), 0, 'the fixed patch\'s knobs are not the deck\'s now');
	h.useParams({ ...PARAMS, ...MELODY }, null);
	assert.ok(String(calls.at(-1)).endsWith(' | '), 'no pairs: the defaults');
	assert.ok(h.norm('voice.ratio') >= 0);
});
