// The browser deck's route helpers (#198, src/lib/live-deck.ts): the relay from the address, the character, the nick.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { characterOf, codeOf, DEFAULT_CHARACTER, DEFAULT_CODE, mayJoin, nickOf, preferOf, relayOf, rigOf, sticksLine } from '../src/lib/live-deck.ts';
import type { Me } from '@scutoons/content';

const q = (s: string) => new URLSearchParams(s);
const me = (name: string | null, character?: string) =>
	({ user: { display_name: name }, avatar: { slots: {}, body: character ? { character, persona: 'regular', look: 'default' } : null } }) as unknown as Me;

const LIVE = 'wss://live.scutoons.com';

test('the session code: the path, ?code=, else the default', () => {
	assert.equal(codeOf(q('code=X'), 'A7K2'), 'A7K2');
	assert.equal(codeOf(q('code=X'), undefined), 'X');
	assert.equal(codeOf(q(''), ' '), DEFAULT_CODE);
	assert.equal(DEFAULT_CODE, 'CATH');
});

test('the relay: the hosted one by default; ?relay= a whole session URL or a host with the code', () => {
	assert.equal(relayOf(q(''), LIVE), 'wss://live.scutoons.com/live/CATH', 'no address needed (#199)');
	assert.equal(relayOf(q(''), `${LIVE}/`, 'A7K2'), 'wss://live.scutoons.com/live/A7K2', 'the path code');
	assert.equal(relayOf(q('code=A B'), LIVE), 'wss://live.scutoons.com/live/A%20B');
	assert.equal(relayOf(q('relay=ws://localhost:7070/live/CATH'), LIVE, 'X'), 'ws://localhost:7070/live/CATH', 'a whole URL wins');
	assert.equal(relayOf(q('relay=wss://relay.example/&code=A B'), LIVE), 'wss://relay.example/live/A%20B');
	assert.equal(relayOf(q('relay=ws://localhost:7070'), LIVE, 'DECK'), 'ws://localhost:7070/live/DECK', 'a host + the path code');
	assert.equal(relayOf(q('relay=ws://localhost:7070'), LIVE), 'ws://localhost:7070/live/CATH');
	assert.equal(relayOf(q('relay=http://localhost:7070/live/CATH'), LIVE), 'wss://live.scutoons.com/live/CATH', 'not ws: ignored');
	assert.equal(relayOf(q(''), 'ws://127.0.0.1:7071'), 'ws://127.0.0.1:7071/live/CATH', 'VITE_LIVE_URL');
	// a venue's relay under our name – the deck's LAN IP with dashes (bunny DNS answers it) – with the wildcard certificate (#201): wss on the deck's port
	const venue = 'wss://192-168-1-116.lan.scutoons.com:7700/live/CATH';
	assert.equal(relayOf(q(`relay=${encodeURIComponent(venue)}`), LIVE, 'X'), venue, 'the QR code / link of #201');
	assert.equal(relayOf(q(`relay=${venue}`), LIVE), venue, 'unencoded too');
	assert.equal(relayOf(q('relay=wss://192-168-1-116.lan.scutoons.com:7700'), LIVE, 'A7K2'), 'wss://192-168-1-116.lan.scutoons.com:7700/live/A7K2', 'the name + the path code');
});

test("the character: the address, the member's avatar, the device, the default", () => {
	assert.equal(characterOf(q('character=catpope'), me('Tilman', 'billthefrog'), 'x'), 'catpope');
	assert.equal(characterOf(q(''), me('Tilman', 'billthefrog'), 'x'), 'billthefrog');
	assert.equal(characterOf(q(''), me('Tilman'), 'x'), 'x');
	assert.equal(characterOf(q(''), null, null), DEFAULT_CHARACTER);
});

test("the nick: the member's name, else the guest's, at most 24 characters", () => {
	assert.equal(nickOf(me('  Tilman '), 'Guest deck'), 'Tilman');
	assert.equal(nickOf(me(null), 'Guest deck'), 'Guest deck');
	assert.equal(nickOf(null, 'Gast-Deck'), 'Gast-Deck');
	assert.equal(nickOf(me('x'.repeat(40)), 'g').length, 24);
});

test('WebGL2 on request; everyone may join for now', () => {
	assert.equal(preferOf(q('gl=1')), 'webgl2');
	assert.equal(preferOf(q('')), 'auto');
	assert.equal(mayJoin(null), true);
});

test('the rig is opt-in while the phone check is open (#191)', () => {
	assert.equal(rigOf(new URLSearchParams('rig=1')), true);
	assert.equal(rigOf(new URLSearchParams('')), false);
	assert.equal(rigOf(new URLSearchParams('rig=0')), false);
});

test('#209: the sticks line names the selection and what each stick drives', () => {
	const labels: Record<string, string> = { 'voice.ratio': 'FM ratio', 'voice.index': 'FM index', 'space.hue': 'hue' };
	const words = { being: 'Being', master: 'Master' };
	const of = (p: string) => labels[p];
	assert.equal(sticksLine('being', of, ['voice.ratio', 'voice.index'], ['x.y', ''], words), 'Being · L FM ratio / FM index · R x.y / –');
	assert.equal(sticksLine('knob:voice.index', of, ['voice.ratio', 'voice.index'], [], words), 'FM index · L FM ratio / FM index · R –');
	assert.equal(sticksLine('', of, ['space.hue', ''], [], words), 'Master · L hue / – · R –');
});
