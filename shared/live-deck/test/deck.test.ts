// `npm test -w shared/live-deck` (#198): the deck's state without a browser – one state for the page's controls and the
// being's 3D buttons, the streamed paths, the character switch, the image sniffing for the set's textures.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Deck, defaultLead, portraitsOf } from '../src/deck.ts';
import { sniff } from '../src/space.ts';
import { LADDER, PARAMS, STREAMED } from '../src/scene.ts';

const API = 'https://api.example/api';

test('a deck without a relay plays alone; its paths are what it streams', () => {
	const d = new Deck({ character: 'yellowmouse', nick: 'guest', api: API });
	assert.equal(d.link, null);
	assert.deepEqual(d.paths.sort(), Object.keys(STREAMED).sort());
	for (const p of d.paths) assert.ok(Number.isFinite(d.read(p)) || p === 'seq.cv', p);
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

test('the LFO sways the incense cutoff the being shows on its linked button', () => {
	const d = new Deck({ character: 'yellowmouse', nick: 'guest', api: API });
	d.setParam('sway.rate', 1);
	d.setParam('sway.depth', 1);
	// frames of 50 ms (a frame's step is capped at 100 ms)
	for (let k = 0; k <= 5; k++) d.tick(10 + k * 0.05);
	assert.ok(Math.abs(d.read('sway.out') - 1) < 1e-6, 'a quarter turn of a 1 Hz LFO');
	assert.equal(d.read('incense.cutoff'), Math.min(12000, d.value['incense.cutoff'] * 2 ** 1.5));
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

test('#188: the being turns the cutoff as its base – the LFO keeps swaying on top', () => {
	const d = new Deck({ character: 'yellowmouse', nick: 'guest', api: API });
	d.setParam('sway.rate', 1);
	d.setParam('sway.depth', 1);
	for (let k = 0; k <= 5; k++) d.tick(10 + k * 0.05);
	d.setParam('incense.cutoff', 600, 'being');
	assert.equal(d.value['incense.cutoff'], 600, 'the knob: the base');
	assert.ok(Math.abs(d.read('incense.cutoff') - 600 * 2 ** 1.5) < 1e-6, 'the sway on top of it');
});

test('#188: a phone keeps more sound ahead of the speaker', () => {
	assert.equal(defaultLead(true), 3072);
	assert.equal(defaultLead(false), 2048);
});

test('#298: stop ends the sound and leaves the deck as it was – values kept, no core, quiet bands', async () => {
	const d = new Deck({ character: 'yellowmouse', nick: 'guest', api: API });
	let stopped = 0;
	d.core = { stop: async () => void stopped++ } as unknown as NonNullable<Deck['core']>;
	d.bands = { level: 0.7, bass: 0.5, mid: 0.2, high: 0.1, spectrum: new Float32Array(4), reduction: 3 };
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
