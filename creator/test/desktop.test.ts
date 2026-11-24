// The installed creator (lib/shell/desktop.ts): its bridge and where a creator link leads.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { creatorLinkHref, desktopBridge } from '../src/lib/shell/desktop.ts';

const entries: Record<string, string> = { catalog: '/catalog/items', scene: '/scenes/avatar-home' };
const entryOf = (kind: string) => entries[kind] ?? null;

test("a creator link opens its item's page", () => {
	assert.equal(creatorLinkHref('scutoons-creator://scene/scene:avatar-home', entryOf), '/scenes/avatar-home');
	assert.equal(creatorLinkHref('scutoons-creator://chapter/chapter:feilchen:3', entryOf), '/feilchen/chapter/3');
	assert.equal(creatorLinkHref('scutoons-creator://character/character:catpope', entryOf), '/characters?id=catpope');
});

test('a workspace alone opens its entry, Home when it has none', () => {
	assert.equal(creatorLinkHref('scutoons-creator://catalog', entryOf), '/catalog/items');
	assert.equal(creatorLinkHref('scutoons-creator://scene/', entryOf), '/scenes/avatar-home');
	assert.equal(creatorLinkHref('scutoons-creator://toon', entryOf), '/');
});

test("the deck's link, another workspace, a broken link: nothing", () => {
	assert.equal(creatorLinkHref('scutoons-deck://stage/scene:avatar-home', entryOf), null);
	assert.equal(creatorLinkHref('scutoons-creator://help', entryOf), null);
	assert.equal(creatorLinkHref('scutoons-creator://scene/avatar-home', entryOf), null);
	assert.equal(creatorLinkHref('https://creator.scutoons.com/scenes/avatar-home', entryOf), null);
});

test('only the installed creator has the bridge', () => {
	assert.equal(desktopBridge({}), null);
	assert.equal(desktopBridge({ scutoonsCreatorApp: { onLink: 1 } }), null);
	const bridge = { hasDeckDownload: false, onLink: () => {}, deckMissing: () => true, openDeckDownload: () => {} };
	assert.equal(desktopBridge({ scutoonsCreatorApp: bridge }), bridge);
});
