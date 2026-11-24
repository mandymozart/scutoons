import { test } from 'node:test';
import assert from 'node:assert/strict';
import vectors from '../../../server/schema/tables/deck-link.vectors.json' with { type: 'json' };
import { DECK_CARDS, deckLink, readDeckState } from '../src/deck-link.ts';

test('the links every language builds and reads (deck-link.vectors.json)', () => {
	for (const v of vectors.links) assert.equal(deckLink(v.workspace, v.address), v.link, `${v.workspace} ${v.address}`);
});

test('no link for a workspace or an address that is not one', () => {
	for (const v of vectors.refused) assert.equal(deckLink(v.workspace, v.address), null, `${v.workspace} ${v.address}`);
});

test("the deck's device row as read", () => {
	for (const v of vectors.states) assert.deepEqual(readDeckState(v.state), v.read, JSON.stringify(v.state));
});

test("the deck's cards in the deck's order", () => {
	assert.deepEqual(DECK_CARDS, ['stage', 'rigging', 'show']);
});
