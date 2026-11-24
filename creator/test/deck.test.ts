// The deck from the creator (lib/shell/deck.ts): the switcher's deck cards and the hook for a machine without a deck.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readDeckState } from '@scutoons/api';
import { deckCards, deckMissing, fillDeckHooks, onDeckLink } from '../src/lib/shell/deck.ts';

test("the deck's cards in the deck's order, each on its last item", () => {
	const state = readDeckState({ app: 'deck', v: 1, workspace: 'show', last: { stage: 'scene:avatar-home', show: 'show:new show' } });
	assert.deepEqual(deckCards(state), [
		{ id: 'deck-stage', workspace: 'stage', item: 'scene:avatar-home', href: 'scutoons-deck://stage/scene:avatar-home' },
		{ id: 'deck-rigging', workspace: 'rigging', item: null, href: 'scutoons-deck://rigging' },
		{ id: 'deck-show', workspace: 'show', item: 'show:new show', href: 'scutoons-deck://show/show:new%20show' }
	]);
});

test('no row of the deck: its cards with nothing open', () => {
	assert.deepEqual(
		deckCards(null).map((c) => [c.item, c.href]),
		[
			[null, 'scutoons-deck://stage'],
			[null, 'scutoons-deck://rigging'],
			[null, 'scutoons-deck://show']
		]
	);
});

test('the web creator just tries the link; the installed one offers the download when there is no deck', () => {
	let prevented = 0;
	let offered = 0;
	const event = { preventDefault: () => prevented++ };
	assert.equal(deckMissing(), false);
	onDeckLink(event);
	assert.deepEqual([prevented, offered], [0, 0]);
	fillDeckHooks({ missing: () => true, offer: () => offered++ });
	onDeckLink(event);
	assert.deepEqual([prevented, offered], [1, 1]);
	fillDeckHooks({ missing: () => false });
});
