// `npm test -w shared/live-deck` (#198): the deck's state without a browser – one state for the page's controls and the
// being's 3D buttons, the streamed paths, the character switch, the image sniffing for the set's textures.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { FRAME, LiveCore, analysisOf, clockOf, type ClockOp, type ClockState } from '@scutoons/live-core';
import { Deck, deviceKind, portraitsOf } from '../src/deck.ts';
import { sniff } from '../src/space.ts';
import { DECK, MOODS, OWN, OWN_PARAMS, OWN_STREAMED, SILENT, stateOf } from './own.ts';

const API = 'https://api.example/api';

/** a deck on the core's state (the cathedral test scene as its own: knobs and a sound to check; `stateOf({ silent: true })` = the deck as every platform has it), as the page's wasm gives it */
function deckOf(character = 'yellowmouse', state = stateOf()): Deck {
	const d = new Deck({ character, nick: 'guest', api: API });
	d.useState(state);
	return d;
}

test('a deck without a relay plays alone; its paths are what it streams', () => {
	const d = deckOf();
	assert.equal(d.link, null);
	assert.deepEqual(d.paths.sort(), Object.keys(OWN_STREAMED).sort());
	// without sound only the knobs have values; the outputs are what the engine plays (#320)
	for (const p of d.paths) assert.equal(Number.isFinite(d.read(p)), p in OWN_PARAMS, p);
	d.close();
});

test('one state: the being turns a value, the page hears whose hand it was; unknown paths are ignored', () => {
	const d = deckOf();
	const heard: [string, number, string][] = [];
	d.onValue = (p, v, hand) => heard.push([p, v, hand]);
	d.setParam('voice.index', 5.5, 'being');
	d.setParam('voice.ratio', 2);
	d.setParam('nobody.gain', 1, 'being');
	d.setParam('voice.gain', NaN, 'being');
	assert.deepEqual(heard, [
		['voice.index', 5.5, 'being'],
		['voice.ratio', 2, 'page']
	]);
	assert.equal(d.read('voice.index'), 5.5, 'what the being turned is what the deck streams');
	assert.equal(d.value['voice.gain'], OWN_PARAMS['voice.gain'].value);
});

test('#360: the deck plays the scene the core gave it – nothing of it before, its graph and knobs after (the cathedral test scene)', () => {
	const d = new Deck({ character: 'yellowmouse', nick: 'guest', api: API });
	assert.deepEqual(d.params, {}, 'no scene of the page: no sliders before the wasm gave one');
	assert.deepEqual(d.graph().nodes, []);
	d.useState(stateOf());
	assert.deepEqual(Object.keys(d.params), Object.keys(OWN_PARAMS));
	assert.equal(d.rig, null, 'no rig plays');
	const g = d.graph();
	assert.deepEqual(g.nodes.map((n) => n.id), OWN.nodes.map((n) => n.id), 'the being on the wire: the scene the X1 plays');
	assert.ok(g.nodes.some((n) => n.id === 'seq' && n.kind === 'seq') && !g.nodes.some((n) => n.kind === 'walk'), "the cathedral's arp, no page melody");
	assert.equal(g.character, 'yellowmouse', "the deck's own scene wears the deck's character");
	for (let k = 0; k <= 5; k++) d.tick(10 + k * 0.05);
	assert.ok(Math.abs(d.time - 0.25) < 1e-9);
});

test('#425: no rig, no sound – the deck on the core\'s silent scene: the being in its room, no audio stream; a character without a rig changes nothing', () => {
	const state = stateOf({ silent: true });
	const d = deckOf('yellowmouse', state);
	assert.equal(d.rig, null);
	assert.deepEqual(d.graph().nodes.map((n) => n.id), SILENT.nodes.map((n) => n.id), 'the picture: what the core played');
	assert.ok(!d.paths.some((p) => p.startsWith('fft.')) && !d.graph().nodes.some((n) => n.kind === 'audio_out'), 'nothing sounds, nothing is analysed');
	assert.equal(state.room(), DECK.silent.room, "the room is the core's");
	const t = d.play('nobody', '', true);
	assert.equal(t.rig, false);
	assert.equal(d.rig, null, 'a character without a rig stays silent');
	assert.ok(t.note.includes('silent'), t.note);
	assert.deepEqual(d.graph().nodes.map((n) => n.id), SILENT.nodes.map((n) => n.id));
});

test('#367: each frame the level, the clock and the hand go to the core’s ladder; the mood is what it says', () => {
	const s = stateOf({ moods: MOODS, said: ['neutral', '', 'happy'] });
	const d = deckOf('yellowmouse', s);
	const said: string[] = [];
	d.onMood = (m) => said.push(m);
	d.bands = { ...d.bands, level: 0.5 };
	d.touch = 1;
	d.tick(10);
	d.tick(10.25);
	d.tick(10.75);
	assert.deepEqual(said, ['neutral', 'happy'], 'only a turn is a mood change');
	assert.equal(d.mood, 'happy');
	assert.deepEqual(s.calls.filter((c) => c[0] === 'step')[2], ['step', 0.5, 0, false, 1, 0.5, []], 'no clock yet: no tempo; dt unclamped (the core clamps)');
});

test('another character: its portraits go to the core (its ladder), the face is the core’s pick', async () => {
	const realFetch = globalThis.fetch;
	globalThis.fetch = (async (url: string) => {
		assert.match(String(url), /scene-content\?characters=catpope$/);
		const portraits = Object.fromEntries(['neutral', 'happy', 'surprised', 'sad'].map((m) => [m, { path: `characters/catpope/catpope_portrait-${m}.webp` }]));
		return new Response(JSON.stringify({ characters: { catpope: { portraits } } }));
	}) as typeof fetch;
	try {
		const s = stateOf();
		const d = deckOf('yellowmouse', s);
		await d.pickCharacter('catpope');
		assert.deepEqual(s.calls.at(-1), ['set_portraits', ['neutral', 'happy', 'surprised', 'sad']]);
		assert.equal(d.face('sad')?.path, 'characters/catpope/catpope_portrait-sad.webp', 'the face the core picks');
	} finally {
		globalThis.fetch = realFetch;
	}
	globalThis.fetch = (async () => {
		throw new Error('offline');
	}) as typeof fetch;
	try {
		assert.deepEqual(await portraitsOf(API, 'x'), {}, 'offline: no portraits, no throw');
	} finally {
		globalThis.fetch = realFetch;
	}
});

test("the set's images: PNG, JPEG, else WebP", () => {
	assert.equal(sniff(new Uint8Array([0x89, 0x50, 0x4e, 0x47])), 'image/png');
	assert.equal(sniff(new Uint8Array([0xff, 0xd8, 0xff])), 'image/jpeg');
	assert.equal(sniff(new Uint8Array([0x52, 0x49, 0x46, 0x46])), 'image/webp');
});

test('#188: the being turns the cutoff as its base – the engine’s LFO keeps swaying on top, the deck streams what is heard', async () => {
	const d = deckOf();
	const sets: [string, number[], string | undefined][] = [];
	let heard = NaN;
	d.core = {
		load: async () => ({}),
		set: (p: string, v: number[], o: { mode?: string } = {}) => void sets.push([p, [...v], o.mode]),
		value: (p: string) => (p === 'incense.cutoff' ? heard : NaN)
	} as never;
	d.play('');
	await Promise.resolve();
	await Promise.resolve();
	sets.length = 0;
	d.setParam('incense.cutoff', 600, 'being');
	assert.deepEqual(sets, [['incense.cutoff', [600], 'base']], 'the knob: the base');
	assert.equal(d.read('incense.cutoff'), 600, 'nothing heard yet: the knob');
	heard = 1350;
	assert.equal(d.read('incense.cutoff'), 1350, 'the sway on top of it, as the engine played it');
});

test('#365: a touch screen is a phone to the core’s lead', () => {
	assert.equal(deviceKind(true), 'phone');
	assert.equal(deviceKind(false), 'desktop');
});

test('#370: the picker and the MIDI clock out go to the core; a restart hands the core its own last word back', async () => {
	const ops: ClockOp[][] = [];
	let frame: ((c: ClockState) => void) | null = null;
	const realStart = LiveCore.start;
	LiveCore.start = (async () => {
		const mine: ClockOp[] = [];
		ops.push(mine);
		return {
			onAnalysis: () => {},
			onSignal: () => {},
			onMidiOut: () => {},
			onClock: (cb: (c: ClockState) => void) => (frame = cb),
			clockOp: (o: ClockOp) => mine.push(o),
			load: async () => ({}),
			stop: async () => {}
		} as unknown as LiveCore;
	}) as typeof LiveCore.start;
	try {
		const d = deckOf();
		const files = { worker: {} as Worker, wasm: '', worklet: '' };
		await d.start(files);
		assert.deepEqual(ops[0], [], "a fresh deck: the core's own start (the show clock)");
		assert.deepEqual(d.state?.clock_modes(), ['show', 'own', 'lead', 'midi'], "the choices are the core's");
		d.clockMode(2);
		d.clockOut(true);
		assert.deepEqual(ops[0], [{ op: 'mode', mode: 2 }, { op: 'out', on: true }]);
		// a hand on the clock moved it: the core's frame says its own clock, the MIDI clock out on
		const said = clockOf([0, 0, 120, 4, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0]);
		frame!(said);
		assert.equal(d.clock, said);
		await d.stop();
		assert.equal(d.clock, null);
		await d.start(files);
		assert.deepEqual(ops[1], [{ op: 'mode', mode: 1 }, { op: 'out', on: true }], 'what the core said last, not what the page picked');
	} finally {
		LiveCore.start = realStart;
	}
});

test('#298: stop ends the sound and leaves the deck as it was – values kept, no core, quiet bands', async () => {
	const d = deckOf();
	let stopped = 0;
	d.core = { stop: async () => void stopped++ } as unknown as NonNullable<Deck['core']>;
	d.bands = { ...analysisOf(new Float32Array(FRAME)), level: 0.7, bass: 0.5, mid: 0.2, high: 0.1, reduction: 3 };
	d.setParam('voice.index', 4.5);
	await d.stop();
	assert.equal(stopped, 1);
	assert.equal(d.core, null);
	assert.equal(d.bands.level, 0);
	assert.equal(d.value['voice.index'], 4.5);
	await d.stop();
	assert.equal(stopped, 1, 'a second stop is nothing');
	d.setParam('voice.index', 5);
	d.tick(1);
});
