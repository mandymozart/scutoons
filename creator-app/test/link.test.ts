import { test } from 'node:test';
import assert from 'node:assert/strict';
import table from '../../server/schema/tables/deck-link.json' with { type: 'json' };
import { goesOutside, linkIn, readConfig, readSchemes, sameOrigin } from '../src/link.ts';

const schemes = readSchemes(table);

test('the schemes come from the one link definition', () => {
	assert.deepEqual(schemes, { creator: 'scutoons-creator', deck: 'scutoons-deck' });
	assert.throws(() => readSchemes({ scheme: 'scutoons-deck' }));
});

test('a link among the arguments of a start (Windows, Linux)', () => {
	const exe = 'C:\\Users\\x\\AppData\\Local\\Programs\\scutoons-creator\\scutoons-creator.exe';
	assert.equal(linkIn([exe, 'scutoons-creator://scene/scene:avatar-home'], schemes.creator), 'scutoons-creator://scene/scene:avatar-home');
	assert.equal(linkIn([exe, '--allow-file-access-from-files', 'SCUTOONS-CREATOR://catalog/'], schemes.creator), 'SCUTOONS-CREATOR://catalog/');
	assert.equal(linkIn([exe, '.', 'scutoons-deck://show'], schemes.creator), null);
	assert.equal(linkIn([exe], schemes.creator), null);
});

test('the configuration: https, http only on this machine, the environment over the file', () => {
	assert.deepEqual(readConfig({ creatorUrl: 'https://creator.scutoons.com', deckDownloadUrl: '' }), { creatorUrl: 'https://creator.scutoons.com/', deckDownloadUrl: null });
	assert.equal(readConfig({ creatorUrl: 'https://creator.scutoons.com' }, { SCUTOONS_CREATOR_URL: 'http://127.0.0.1:10600' }).creatorUrl, 'http://127.0.0.1:10600/');
	assert.equal(readConfig({ creatorUrl: 'https://creator.scutoons.com', deckDownloadUrl: 'https://example.org/deck' }).deckDownloadUrl, 'https://example.org/deck');
	assert.equal(readConfig({ creatorUrl: 'https://creator.scutoons.com', deckDownloadUrl: 'http://example.org/deck' }).deckDownloadUrl, null);
	assert.equal(readConfig({ creatorUrl: 'https://creator.scutoons.com', deckDownloadUrl: 'file:///C:/deck.exe' }).deckDownloadUrl, null);
	assert.throws(() => readConfig({ creatorUrl: 'http://creator.scutoons.com' }));
	assert.throws(() => readConfig({}));
});

test('what stays in the window and what goes to the system', () => {
	const base = 'https://creator.scutoons.com/';
	assert.equal(sameOrigin('https://creator.scutoons.com/scenes/avatar-home', base), true);
	assert.equal(sameOrigin('https://scutoons.com/feilchen', base), false);
	assert.equal(sameOrigin('not a url', base), false);
	assert.equal(goesOutside('scutoons-deck://stage/scene:avatar-home', schemes), true);
	assert.equal(goesOutside('https://scutoons.com/feilchen', schemes), true);
	assert.equal(goesOutside('mailto:hello@scutoons.com', schemes), true);
	assert.equal(goesOutside('file:///C:/Windows/notepad.exe', schemes), false);
	assert.equal(goesOutside('ms-settings:privacy', schemes), false);
});
