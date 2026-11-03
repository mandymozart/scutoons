// `npm test -w shared/live-deck` (#191): the browser deck playing a character's rig – the answers are deck-space's
// own (test/fixtures/rigs.json, written and kept current by `cargo test -p deck-space rig_fixture`).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { Deck } from '../src/deck.ts';
import { knobLabel, knobTarget, rigDiff, rigGraph, rigParams, rigStreamed } from '../src/rig.ts';
import { PARAMS, STREAMED } from '../src/scene.ts';
import type { RigPlayed } from '../src/space.ts';

const RIGS = JSON.parse(readFileSync(new URL('./fixtures/rigs.json', import.meta.url), 'utf8')) as Record<string, RigPlayed>;
const catpope = RIGS.catpope;
const API = 'https://api.example/api';

test("a rig's knobs become the deck's sliders, in the page's locale", () => {
	const p = rigParams(catpope);
	assert.deepEqual(Object.keys(p), [
		'voice_voice.ratio',
		'voice_voice.index',
		'voice_voice.gain',
		'voice_melody.spread',
		'voice_melody.rate',
		'voice_melody.glide',
		'drone_drone.freq',
		'drone_drone.gain'
	]);
	assert.deepEqual(p['voice_melody.spread'], { min: 0, max: 1, value: 0, label: 'spread' }, 'the melody (#236): held by default');
	assert.deepEqual(knobTarget(catpope, 'voice_melody.spread'), { engine: 'voice_melody.spread' }, "live-core's walk row");
	assert.deepEqual(p['voice_voice.ratio'], { min: 0.5, max: 4, value: 1.5, label: 'FM ratio', log: true });
	assert.equal(p['voice_voice.gain'].log, undefined);
	assert.equal(rigParams(catpope, 'de')['drone_drone.gain'].label, 'Bordun');
	assert.equal(knobLabel({ path: 'x.y', min: 0, max: 1, value: 0 }), 'x.y');
});

test('the being on the wire is the flattened scene, its knobs exposed; it streams the knobs and the analyser', () => {
	const g = rigGraph(catpope, ['neutral', 'happy']);
	assert.equal(g.character, 'catpope');
	assert.ok(g.nodes.some((n) => n.id === 'voice_voice' && n.kind === 'fm'));
	assert.ok(g.links.some((l) => l.from === 'mix1.out' && l.to === 'fft.in'));
	assert.deepEqual(g.expose, Object.keys(rigParams(catpope)));
	assert.deepEqual(g.moods, ['neutral', 'happy']);
	const s = rigStreamed(catpope);
	assert.deepEqual(s['voice_voice.ratio'], { input: [0.5, 4] });
	assert.equal(s['fft.level'], 'control');
	assert.equal(s['mix1.out'], 'audio');
});

test('a knob goes to live-core under its own path (#320: the whole scene plays there), or nowhere for a kind it lacks', () => {
	assert.deepEqual(knobTarget(catpope, 'voice_voice.ratio'), { engine: 'voice_voice.ratio' });
	assert.deepEqual(knobTarget(catpope, 'mix1.gain1'), { engine: 'mix1.gain1' }, 'the mixers are the engine’s too');
	assert.deepEqual(knobTarget(RIGS.incense_burner, 'air_hiss.gain'), { engine: 'air_hiss.gain' }, 'the noise is the engine’s');
	assert.equal(knobTarget(catpope, 'nobody.gain'), null);
	const music = { ...catpope, nodes: [...catpope.nodes, { id: 'track', kind: 'chapter_music' }], engine: { ...catpope.engine, page: [{ id: 'track', kind: 'chapter_music' }] } };
	assert.equal(knobTarget(music, 'track.gain'), null, 'a kind the browser does not carry yet');
	assert.deepEqual(catpope.engine.page, [], 'catpope plays whole');
});

test("a mood changes values only – a new routing reloads the engine", () => {
	const d = rigDiff(catpope, RIGS.catpope_laughing);
	assert.equal(d.reload, false);
	assert.deepEqual(new Map(d.values), new Map([['voice_voice.ratio', 3], ['voice_voice.index', 4]]));
	const rerouted = { ...catpope, links: [...catpope.links, { from: 'fft.level', to: 'voice_voice.index', scale: 1, offset: 0 }] };
	assert.equal(rigDiff(catpope, rerouted).reload, true);
});

test('the deck plays a rig and back: sliders, stream and wire follow; a mood moves the sliders by its hand', () => {
	const d = new Deck({ character: 'catpope', nick: 'guest', api: API });
	d.useRig(catpope);
	assert.deepEqual(Object.keys(d.params), Object.keys(rigParams(catpope)));
	assert.equal(d.value['voice_voice.ratio'], 1.5);
	assert.equal(d.value['voice.ratio'], undefined, "the fixed patch's sliders are gone");
	assert.ok('melody.spread' in d.value, 'the melody stays the page\'s');
	assert.deepEqual(d.paths.sort(), Object.keys(rigStreamed(catpope)).sort());
	assert.equal(d.graph().nodes.length, catpope.nodes.length);
	const heard: [string, number, string][] = [];
	d.onValue = (p, v, hand) => heard.push([p, v, hand]);
	d.moodRig(RIGS.catpope_laughing);
	assert.deepEqual(heard.sort(), [
		['voice_voice.index', 4, 'mood'],
		['voice_voice.ratio', 3, 'mood']
	]);
	assert.equal(d.read('voice_voice.index'), 4);
	assert.ok(Number.isNaN(d.read('fft.level')), 'no sound: nothing heard');
	d.useRig(null);
	assert.deepEqual(Object.keys(d.params), Object.keys(PARAMS));
	assert.deepEqual(d.paths.sort(), Object.keys(STREAMED).sort());
	d.close();
});

test("the rig's corner sticks reach the deck; no rig restores the fixed patch's", () => {
	const d = new Deck({ character: 'catpope', nick: 'guest', api: API });
	assert.equal(d.sticks, null);
	assert.ok('voice.ratio' in d.controlSpecs && 'melody.spread' in d.controlSpecs, 'the fixed patch: sliders + melody');
	d.useRig(catpope);
	assert.deepEqual(d.sticks, [
		['voice_voice.ratio', 'voice_voice.index'],
		['voice_melody.spread', 'drone_drone.freq']
	]);
	assert.deepEqual(d.controlSpecs, d.params, "a rig's controls are its knobs alone");
	d.moodRig(RIGS.catpope_laughing);
	assert.equal(d.sticks?.[0][0], 'voice_voice.ratio', 'a mood keeps them');
	d.useRig(null);
	assert.equal(d.sticks, null);
	assert.ok('voice.ratio' in d.controlSpecs);
	d.close();
});

/** live-core as the deck sees it: a scene that loads later (the worker's round trip), a `set` that throws for a path
 *  the loaded scene lacks (the real one does), the heard values by path */
function fakeCore() {
	let settle: (() => void)[] = [];
	const sets: [string, string][] = [];
	const heard = new Map<string, number>();
	const core = {
		scene: '',
		/** arguments past the scene (none since #321: the clock is the core's) */
		extra: 0,
		loads: 0,
		sets,
		heard,
		load(scene: string, ...rest: unknown[]) {
			this.loads++;
			this.extra = rest.length;
			return new Promise<void>((ok) =>
				settle.push(() => {
					this.scene = scene;
					ok();
				})
			);
		},
		set(param: string, _v: number[], o: { mode?: string } = {}) {
			const node = param.split('.')[0];
			if (!this.scene.includes(`id = "${node}"`)) throw new Error(`live-core: no node '${node}'`);
			sets.push([param, o.mode ?? 'force']);
			return true;
		},
		value: (path: string) => heard.get(path) ?? NaN,
		/** the pending loads land, oldest first */
		land() {
			const s = settle;
			settle = [];
			for (const f of s) f();
		},
		onAnalysis() {
			return () => {};
		},
		stats: () => ({ underruns: 0 })
	};
	return core;
}

const sent = (core: ReturnType<typeof fakeCore>) => core.sets.map(([p]) => p);

test('a character switch while the sound loads: no throw out of tick or setParam, the newest scene wins (S22 crash)', async () => {
	const d = new Deck({ character: 'catpope', nick: 'guest', api: API });
	const core = fakeCore();
	// the fixed scene playing (it landed): every knob went out as its input's base; the tempo is the core's clock's (#321)
	d.core = core as never;
	d.useRig(null);
	core.land();
	await Promise.resolve();
	assert.equal(core.extra, 0, 'no page tempo with the scene');
	assert.ok(sent(core).includes('voice.ratio') && sent(core).includes('melody.spread'), 'the knobs and the melody’s');
	assert.ok(core.sets.every(([, mode]) => mode === 'base'), 'a knob turns the base (#188)');
	core.sets.length = 0;
	// the switch: the rig's scene is loading – a frame and a knob turn meanwhile must not throw
	d.useRig(catpope);
	assert.doesNotThrow(() => d.tick(1.02));
	assert.doesNotThrow(() => d.setParam('voice_voice.ratio', 2.5, 'being'));
	assert.equal(core.sets.length, 0, 'nothing reaches the engine while its scene is the old one');
	// a second switch before the first landed: the first is ignored when it lands, the second plays
	d.useRig(null);
	core.land();
	await Promise.resolve();
	await Promise.resolve();
	assert.ok(core.scene.includes('id = "voice"') && !core.scene.includes('voice_voice'), 'the last scene asked for');
	assert.ok(sent(core).includes('voice.ratio'), 'the fixed scene plays again');
	assert.ok(!sent(core).some((p) => p.startsWith('voice_')), "the overtaken rig's values never went out");
	d.close();
});

test('#274: the first start – a frame and a knob turn before the scene landed throw nothing; it plays once it has', async () => {
	const d = new Deck({ character: 'catpope', nick: 'guest', api: API });
	const core = fakeCore();
	// as Deck.start does: the core is there before its scene is (the page's frame loop died on this once)
	d.core = core as never;
	d.useRig(null);
	assert.doesNotThrow(() => d.tick(0.5));
	assert.doesNotThrow(() => d.setParam('voice.ratio', 2));
	assert.equal(core.sets.length, 0, 'nothing reaches an engine without a scene');
	core.land();
	await Promise.resolve();
	assert.ok(core.sets.some(([p]) => p === 'voice.ratio'), 'the knobs go out once its scene landed');
});

test('#320: what the deck streams is what the engine played – the heard block’s values, else the knob', async () => {
	const d = new Deck({ character: 'catpope', nick: 'guest', api: API });
	const core = fakeCore();
	d.core = core as never;
	d.useRig(catpope);
	core.land();
	await Promise.resolve();
	core.heard.set('fft.level', 0.4).set('mix1.out', 0.3).set('voice_voice.index', 2.5);
	assert.equal(d.read('fft.level'), 0.4);
	assert.equal(d.read('mix1.out'), 0.3, 'an audio output: its block peak');
	assert.equal(d.read('voice_voice.index'), 2.5, 'an input: base plus what drives it');
	assert.equal(d.read('voice_voice.ratio'), 1.5, 'not heard yet: the knob');
});