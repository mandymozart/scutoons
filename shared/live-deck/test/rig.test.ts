// `npm test -w shared/live-deck` (#191, #368): the browser deck playing a character's rig – the answers are deck-space's
// own (test/fixtures/deck.json, written and kept current by `cargo test -p deck-space deck_fixture`): the sliders, the
// being on the wire, the streamed paths, the knob targets and a mood's turn are the core's; the deck applies them.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Deck } from '../src/deck.ts';
import { CATPOPE, DECK, MOODS, OWN, OWN_PARAMS, OWN_STREAMED, stateOf } from './own.ts';

const API = 'https://api.example/api';
const paths = (sliders: { path: string }[]) => sliders.map((s) => s.path);

test("a rig's knobs become the deck's sliders, in the page's locale (the core's)", () => {
	const p = Object.fromEntries(DECK.catpope.sliders.map((s) => [s.path, s]));
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
	assert.deepEqual(p['voice_melody.spread'], { path: 'voice_melody.spread', min: 0, max: 1, value: 0, label: 'spread' }, 'the melody (#236): held by default');
	assert.equal(DECK.catpope.targets['voice_melody.spread'], 'voice_melody.spread', "live-core's walk row");
	assert.deepEqual(p['voice_voice.ratio'], { path: 'voice_voice.ratio', min: 0.5, max: 4, value: 1.5, label: 'FM ratio', log: true });
	assert.equal(p['voice_voice.gain'].log, undefined);
	assert.equal(DECK.catpope.sliders_de.find((s) => s.path === 'drone_drone.gain')?.label, 'Bordun');
});

test('the being on the wire is the flattened scene, its knobs exposed; it streams the knobs and the analyser', () => {
	const g = DECK.catpope.graph;
	assert.equal(g.character, 'catpope');
	assert.ok(g.nodes.some((n) => n.id === 'voice_voice' && n.kind === 'fm'));
	assert.ok(g.links.some((l) => l.from === 'mix1.out' && l.to === 'fft.in'));
	assert.deepEqual(g.expose, paths(DECK.catpope.sliders));
	assert.deepEqual(g.moods, MOODS);
	const s = Object.fromEntries(CATPOPE.streamed);
	assert.deepEqual(s['voice_voice.ratio'], { input: [0.5, 4] });
	assert.equal(s['fft.level'], 'control');
	assert.equal(s['mix1.out'], 'audio');
});

test('a knob goes to live-core under its own path (#320: the whole scene plays there), or nowhere for a kind it lacks', () => {
	const t = DECK.catpope.targets;
	assert.equal(t['voice_voice.ratio'], 'voice_voice.ratio');
	assert.equal(t['mix1.gain'], 'mix1.gain', 'the mixers are the engine’s too');
	assert.equal(t['nobody.gain'], '');
	assert.equal(DECK.own.targets['music.gain'], '', 'a kind the browser does not carry yet (chapter_music)');
	assert.deepEqual(CATPOPE.engine.page, [], 'catpope plays whole');
});

test('a mood changes values only (the core’s diff)', () => {
	const t = DECK.catpope.laughing.turn!;
	assert.equal(t.reload, false);
	assert.deepEqual(new Map(t.values), new Map([['voice_voice.ratio', 3], ['voice_voice.index', 4]]));
});

test('the deck plays a rig and back: sliders, stream and wire follow; a mood moves the sliders by its hand', () => {
	const d = new Deck({ character: 'catpope', nick: 'guest', api: API });
	d.useState(stateOf({ said: [DECK.catpope.laughing] }));
	d.play('catpope', '', true);
	assert.equal(d.rig, d.played);
	assert.deepEqual(Object.keys(d.params), paths(DECK.catpope.sliders));
	assert.equal(d.value['voice_voice.ratio'], 1.5);
	assert.equal(d.value['voice.ratio'], undefined, "the deck's own sliders are gone");
	assert.deepEqual(d.paths.sort(), CATPOPE.streamed.map(([p]) => p).sort());
	assert.equal(d.graph().nodes.length, CATPOPE.nodes.length);
	const heard: [string, number, string][] = [];
	d.onValue = (p, v, hand) => heard.push([p, v, hand]);
	d.tick(1);
	assert.deepEqual(heard.sort(), [
		['voice_voice.index', 4, 'mood'],
		['voice_voice.ratio', 3, 'mood']
	]);
	assert.equal(d.mood, 'laughing');
	assert.equal(d.read('voice_voice.index'), 4);
	assert.ok(Number.isNaN(d.read('fft.level')), 'no sound: nothing heard');
	d.play('');
	assert.equal(d.rig, null);
	assert.deepEqual(Object.keys(d.params), Object.keys(OWN_PARAMS));
	assert.deepEqual(d.paths.sort(), Object.keys(OWN_STREAMED).sort());
	d.close();
});

test("#282: the deck hands on the core's wiring of what plays – the rig's, then its own scene's again", () => {
	const d = new Deck({ character: 'catpope', nick: 'guest', api: API });
	assert.deepEqual(d.wiring(), { nodes: [], links: [] }, 'no core yet: no graph');
	d.useState(stateOf());
	d.play('catpope', '', true);
	const w = d.wiring();
	assert.deepEqual(w, DECK.catpope.wiring);
	const id = (i: number) => w.nodes[i].id;
	assert.ok(w.links.some((l) => id(l.from) === 'voice_melody' && id(l.to) === 'voice_voice' && l.value === 'voice_melody.cv' && l.type === 'audio'));
	assert.ok(w.links.some((l) => l.value === 'drone_drone.out' && l.out === 'out'), 'a bare node is its first output (the core resolved it)');
	assert.ok(w.nodes.every((n) => n.played), 'catpope plays whole');
	d.play('');
	assert.deepEqual(d.wiring(), DECK.own.wiring);
	assert.ok(DECK.own.wiring.nodes.some((n) => !n.played), 'a node the browser does not carry yet');
	d.close();
});

test("the rig's corner sticks reach the deck; no rig restores the deck's own", () => {
	const d = new Deck({ character: 'catpope', nick: 'guest', api: API });
	assert.equal(d.sticks, null);
	assert.deepEqual(d.params, {}, 'no scene from the core yet: no controls');
	d.useState(stateOf({ said: [DECK.catpope.laughing] }));
	assert.deepEqual(d.sticks, OWN.sticks);
	assert.ok('voice.ratio' in d.params, "the deck's own scene: its knobs");
	d.play('catpope', '', true);
	assert.deepEqual(d.sticks, [
		['voice_voice.ratio', 'voice_voice.index'],
		['voice_melody.spread', 'drone_drone.freq']
	]);
	assert.deepEqual(Object.keys(d.params), paths(DECK.catpope.sliders), "a rig's controls are its knobs alone");
	d.tick(1);
	assert.equal(d.sticks?.[0][0], 'voice_voice.ratio', 'a mood keeps them');
	d.play('');
	assert.deepEqual(d.sticks, OWN.sticks);
	assert.ok('voice.ratio' in d.params);
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
	// the deck's own scene playing (it landed): every knob went out as its input's base; the tempo is the core's clock's (#321)
	d.core = core as never;
	d.useState(stateOf());
	core.land();
	await Promise.resolve();
	assert.equal(core.extra, 0, 'no page tempo with the scene');
	assert.equal(core.scene, OWN.engine.scene, "live-core loads the core's part of the X1's scene");
	assert.ok(sent(core).includes('voice.ratio') && sent(core).includes('seq.rate'), 'the knobs, the arp’s too');
	assert.ok(!sent(core).some((p) => p.startsWith('music.')), 'a node the browser does not carry takes none');
	assert.ok(core.sets.every(([, mode]) => mode === 'base'), 'a knob turns the base (#188)');
	core.sets.length = 0;
	// the switch: the rig's scene is loading – a frame and a knob turn meanwhile must not throw
	d.play('catpope', '', true);
	assert.doesNotThrow(() => d.tick(1.02));
	assert.doesNotThrow(() => d.setParam('voice_voice.ratio', 2.5, 'being'));
	assert.equal(core.sets.length, 0, 'nothing reaches the engine while its scene is the old one');
	// a second switch before the first landed: the first is ignored when it lands, the second plays
	d.play('');
	core.land();
	await Promise.resolve();
	await Promise.resolve();
	assert.ok(core.scene.includes('id = "voice"') && !core.scene.includes('voice_voice'), 'the last scene asked for');
	assert.ok(sent(core).includes('voice.ratio'), "the deck's own scene plays again");
	assert.ok(!sent(core).some((p) => p.startsWith('voice_')), "the overtaken rig's values never went out");
	d.close();
});

test('#274: the first start – a frame and a knob turn before the scene landed throw nothing; it plays once it has', async () => {
	const d = new Deck({ character: 'catpope', nick: 'guest', api: API });
	const core = fakeCore();
	// as Deck.start does: the core is there before its scene is (the page's frame loop died on this once)
	d.core = core as never;
	d.useState(stateOf());
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
	d.useState(stateOf());
	d.play('catpope', '', true);
	core.land();
	await Promise.resolve();
	core.heard.set('fft.level', 0.4).set('mix1.out', 0.3).set('voice_voice.index', 2.5);
	assert.equal(d.read('fft.level'), 0.4);
	assert.equal(d.read('mix1.out'), 0.3, 'an audio output: its block peak');
	assert.equal(d.read('voice_voice.index'), 2.5, 'an input: base plus what drives it');
	assert.equal(d.read('voice_voice.ratio'), 1.5, 'not heard yet: the knob');
});