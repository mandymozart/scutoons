// `npm test -w shared/live-deck` (#198): the deck's state without a browser – one state for the page's controls and the
// being's 3D buttons, the streamed paths, the character switch, the image sniffing for the set's textures.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Deck, defaultLead, portraitsOf } from '../src/deck.ts';
import { sniff } from '../src/space.ts';
import { FIXED_SCENE, LADDER, MELODY, OUTPUTS, PARAMS, STREAMED } from '../src/scene.ts';

const API = 'https://api.example/api';

test('a deck without a relay plays alone; its paths are what it streams', () => {
	const d = new Deck({ character: 'yellowmouse', nick: 'guest', api: API });
	assert.equal(d.link, null);
	assert.deepEqual(d.paths.sort(), Object.keys(STREAMED).sort());
	// without sound only the knobs have values; the outputs are what the engine plays (#320)
	for (const p of d.paths) assert.equal(Number.isFinite(d.read(p)), !(p in OUTPUTS), p);
	d.close();
});

test('one state: the being turns a value, the page hears whose hand it was; unknown paths are ignored', () => {
	const d = new Deck({ character: 'yellowmouse', nick: 'guest', api: API });
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
	assert.equal(d.value['voice.gain'], PARAMS['voice.gain'].value);
});

test('#320: the fixed scene is a real scene – every slider is one of its paths, the melody a walk, the sway an engine LFO', () => {
	const ids = [...FIXED_SCENE.matchAll(/^id = "([^"]+)"$/gm)].map((m) => m[1]);
	for (const p of [...Object.keys(PARAMS), ...Object.keys(MELODY), ...Object.keys(OUTPUTS)]) assert.ok(ids.includes(p.split('.')[0]), p);
	assert.match(FIXED_SCENE, /id = "melody"\nkind = "walk"\nseed = 84\n/);
	assert.match(FIXED_SCENE, /from = "sway.out"\nto = "incense.cutoff"\nscale = 2500\.0\noffset = 1800\.0/, 'the unipolar sway lifts the cutoff above its base');
	assert.doesNotMatch(FIXED_SCENE, /Math|random/);
	const d = new Deck({ character: 'yellowmouse', nick: 'guest', api: API });
	for (let k = 0; k <= 5; k++) d.tick(10 + k * 0.05);
	assert.ok(Math.abs(d.time - 0.25) < 1e-9);
});

test('another character: its portraits, its ladder, the being on the wire anew', async () => {
	const realFetch = globalThis.fetch;
	globalThis.fetch = (async (url: string) => {
		assert.match(String(url), /scene-content\?characters=catpope$/);
		const portraits = Object.fromEntries(['neutral', 'happy', 'surprised', 'sad'].map((m) => [m, { path: `characters/catpope/catpope_portrait-${m}.webp` }]));
		return new Response(JSON.stringify({ characters: { catpope: { portraits } } }));
	}) as typeof fetch;
	try {
		const d = new Deck({ character: 'yellowmouse', nick: 'guest', api: API });
		await d.pickCharacter('catpope');
		assert.deepEqual(d.moods.moods, ['neutral', 'happy', 'surprised']);
		const g = d.graph();
		assert.equal(g.character, 'catpope');
		assert.deepEqual(g.moods, ['neutral', 'happy', 'surprised', 'sad'], 'the ladder first, then every other mood it has');
		assert.ok(LADDER.includes(d.mood));
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
	const d = new Deck({ character: 'yellowmouse', nick: 'guest', api: API });
	const sets: [string, number[], string | undefined][] = [];
	let heard = NaN;
	d.core = {
		load: async () => ({}),
		set: (p: string, v: number[], o: { mode?: string } = {}) => void sets.push([p, [...v], o.mode]),
		value: (p: string) => (p === 'incense.cutoff' ? heard : NaN)
	} as never;
	d.useRig(null);
	await Promise.resolve();
	await Promise.resolve();
	sets.length = 0;
	d.setParam('incense.cutoff', 600, 'being');
	assert.deepEqual(sets, [['incense.cutoff', [600], 'base']], 'the knob: the base');
	assert.equal(d.read('incense.cutoff'), 600, 'nothing heard yet: the knob');
	heard = 1350;
	assert.equal(d.read('incense.cutoff'), 1350, 'the sway on top of it, as the engine played it');
});

test('#188: a phone keeps more sound ahead of the speaker', () => {
	assert.equal(defaultLead(true), 3072);
	assert.equal(defaultLead(false), 2048);
});

test('#298: stop ends the sound and leaves the deck as it was – values kept, no core, quiet bands', async () => {
	const d = new Deck({ character: 'yellowmouse', nick: 'guest', api: API });
	let stopped = 0;
	d.core = { stop: async () => void stopped++ } as unknown as NonNullable<Deck['core']>;
	d.bands = { level: 0.7, bass: 0.5, mid: 0.2, high: 0.1, spectrum: new Float32Array(4), reduction: 3, outputs: new Float32Array(2), values: new Float32Array(0) };
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
