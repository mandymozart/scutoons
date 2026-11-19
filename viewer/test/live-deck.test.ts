// The browser deck's route helpers (#198, src/lib/live-deck.ts): the relay from the address, the character, the nick.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { characterOf, codeOf, DEFAULT_CHARACTER, DEFAULT_CODE, mayJoin, nickOf, preferOf, relayOf, rigOf } from '../src/lib/live-deck.ts';
import { cameraHref, cameraKey, cameraName, heldBefore, keepHeld, listHref, type Store } from '../src/lib/live-cameras.ts';
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
	// #438: the QUIC door, for the core to read from the URL
	assert.equal(relayOf(q(''), LIVE, 'A', 'udp.scutoons.com:443'), 'wss://live.scutoons.com/live/A?quic=udp.scutoons.com%3A443', 'LIVE_QUIC');
	assert.equal(relayOf(q(''), LIVE, 'A'), 'wss://live.scutoons.com/live/A', 'no door named: WebSocket only');
	assert.equal(relayOf(q('relay=ws://localhost:7070/live/CATH'), LIVE, 'X', 'udp.scutoons.com:443'), 'ws://localhost:7070/live/CATH', 'a local relay is not the hosted door');
	assert.equal(relayOf(q('relay=ws://localhost:7070/live/CATH&quic=localhost:7070&quichash=ab'), LIVE), 'ws://localhost:7070/live/CATH?quic=localhost%3A7070&quichash=ab');
	assert.equal(relayOf(q('relay=ws://localhost:7070/live/CATH&quic='), LIVE), 'ws://localhost:7070/live/CATH?quic=', 'the relay is its own door');
	// a venue's relay under our name – the deck's LAN IP with dashes (bunny DNS answers it) – with the wildcard certificate (#201): wss on the deck's port
	const venue = 'wss://192-168-1-116.lan.scutoons.com:7700/live/CATH';
	assert.equal(relayOf(q(`relay=${encodeURIComponent(venue)}`), LIVE, 'X'), venue, 'the QR code / link of #201');
	assert.equal(relayOf(q(`relay=${venue}`), LIVE), venue, 'unencoded too');
	assert.equal(relayOf(q('relay=wss://192-168-1-116.lan.scutoons.com:7700'), LIVE, 'A7K2'), 'wss://192-168-1-116.lan.scutoons.com:7700/live/A7K2', 'the name + the path code');
});

test("the character: the address, the member's avatar, the default", () => {
	assert.equal(characterOf(q('character=catpope'), me('Tilman', 'billthefrog')), 'catpope');
	assert.equal(characterOf(q(''), me('Tilman', 'billthefrog')), 'billthefrog');
	assert.equal(characterOf(q(''), me('Tilman')), DEFAULT_CHARACTER);
	assert.equal(characterOf(q(''), null), DEFAULT_CHARACTER);
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

test('the live camera pages: the key made once and kept, the camera held across a reload, the session in every link', () => {
	const m = new Map<string, string>();
	const s: Store = { getItem: (k) => m.get(k) ?? null, setItem: (k, v) => void m.set(k, v), removeItem: (k) => void m.delete(k) };
	let n = 0;
	const make = () => `k${++n}`;
	assert.equal(cameraKey(s, make), 'k1');
	assert.equal(cameraKey(s, make), 'k1', 'the same phone after a reload');
	assert.equal(cameraKey(null, make), 'k2', 'no storage: a key for the page');
	assert.equal(heldBefore(s, 'scene:duet'), null);
	keepHeld(s, 'scene:duet', 'c4');
	assert.equal(heldBefore(s, 'scene:duet'), 'c4');
	assert.equal(heldBefore(s, 'scene:other'), null, 'only the camera it held');
	keepHeld(s, null, null);
	assert.equal(heldBefore(s, 'scene:duet'), null, 'Leave forgets it');
	const search = q('live=H7K2&relay=ws://192.168.1.4:7071&lang=de');
	assert.equal(cameraHref('scene:duet > shot:close', search), '/live/cameras/scene%3Aduet%20%3E%20shot%3Aclose?live=H7K2&relay=ws%3A%2F%2F192.168.1.4%3A7071');
	assert.equal(listHref(q('live=H7K2')), '/live/cameras?live=H7K2');
	assert.equal(cameraName({ id: 'scene:duet', label: ' ' }), 'scene:duet');
	assert.equal(cameraName({ id: 'scene:duet', label: 'Duet close' }), 'Duet close');
});
