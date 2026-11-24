// A shot hands over by a link (src/lib/editor/storyboard-handover.ts): the slice's, the deck's, the XR experience's.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readAppLink } from '@scutoons/api';
import { deckTarget, shotDeckLink, sliceHref, xrHref } from '../src/lib/editor/storyboard-handover.ts';

test('the slice: the chapter workspace asked for the shot', () => {
	assert.equal(sliceHref('catpope', 3, 'sb-25-1'), '/catpope/chapter/3?shot=sb-25-1');
});

test('the deck: the made scene, else the shot\'s own address for the deck to make one', () => {
	assert.deepEqual(deckTarget({ deck: 'scene:sb-catpope-3-25-1' }, 'catpope', 3, 'sb-25-1'), { address: 'scene:sb-catpope-3-25-1', made: true });
	assert.deepEqual(deckTarget(undefined, 'catpope', 3, 'sb-25-1'), { address: 'shot:catpope:3:sb-25-1', made: false });
	assert.deepEqual(deckTarget({ deck: 'nope' }, 'catpope', 3, 'sb-25-1'), { address: 'shot:catpope:3:sb-25-1', made: false });
	const link = shotDeckLink('stage', 'shot:catpope:3:sb-25-1');
	assert.equal(link, 'scutoons-deck://stage/shot:catpope:3:sb-25-1');
	assert.deepEqual(readAppLink(link!), { app: 'deck', workspace: 'stage', address: 'shot:catpope:3:sb-25-1' });
	assert.equal(shotDeckLink(null, 'shot:catpope:3:sb-25-1'), null);
});

test('XR: the experience the shot was made into', () => {
	assert.equal(xrHref({ xr: 'experience:poster-catpope' }), '/experiences/poster-catpope');
	assert.equal(xrHref({}), null);
	assert.equal(xrHref(undefined), null);
});
