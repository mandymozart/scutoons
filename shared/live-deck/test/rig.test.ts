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

test('a knob goes to live-core, to a noise the page plays, or nowhere (the plumbing)', () => {
	assert.deepEqual(knobTarget(catpope, 'voice_voice.ratio'), { engine: 'voice_voice.ratio' });
	assert.equal(knobTarget(catpope, 'mix1.gain1'), null, 'live-core sums by itself');
	assert.deepEqual(knobTarget(RIGS.incense_burner, 'air_hiss.gain'), { page: 'air_hiss', port: 'gain' });
	const strip = { ...catpope, engine: { ...catpope.engine, aliases: { 'voice_voice.gain': 'voice_voice_strip.a' } } };
	assert.deepEqual(knobTarget(strip, 'voice_voice.gain'), { engine: 'voice_voice_strip.a' });
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
	d.bands = { ...d.bands, level: 0.4, bass: 0.2 };
	assert.equal(d.read('fft.level'), 0.4);
	assert.equal(d.read('mix1.out'), 0.4);
	d.useRig(null);
	assert.deepEqual(Object.keys(d.params), Object.keys(PARAMS));
	assert.deepEqual(d.paths.sort(), Object.keys(STREAMED).sort());
	d.close();
});
