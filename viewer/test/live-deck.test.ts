// The browser deck's route helpers (#198, src/lib/live-deck.ts): the relay from the address, the character, the nick.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { characterOf, DEFAULT_CHARACTER, mayJoin, nickOf, preferOf, relayOf } from '../src/lib/live-deck.ts';
import type { Me } from '@scutoons/content';

const q = (s: string) => new URLSearchParams(s);
const me = (name: string | null, character?: string) =>
	({ user: { display_name: name }, avatar: { slots: {}, body: character ? { character, persona: 'regular', look: 'default' } : null } }) as unknown as Me;

test('the relay: a whole session URL, or a host and a code; nothing else', () => {
	assert.equal(relayOf(q('relay=ws://localhost:7070/live/CATH')), 'ws://localhost:7070/live/CATH');
	assert.equal(relayOf(q('relay=wss://relay.example/&code=A B')), 'wss://relay.example/live/A%20B');
	assert.equal(relayOf(q('relay=ws://localhost:7070')), undefined, 'a host without a code');
	assert.equal(relayOf(q('relay=http://localhost:7070/live/CATH')), undefined);
	assert.equal(relayOf(q('code=CATH')), undefined, 'no production relay yet');
	assert.equal(relayOf(q('')), undefined);
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
