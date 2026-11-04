import { test } from 'node:test';
import assert from 'node:assert/strict';
import { deckOptions, folder } from '../src/options.ts';

test('folders end with a slash', () => {
	assert.equal(folder('/live'), '/live/');
	assert.equal(folder('/live/'), '/live/');
});

test('the deck gets no relay unless one is asked for; the nick is trimmed to 24', () => {
	const base = { api: 'https://api/x', cdn: 'https://cdn', wasmBase: '/live/', character: 'catpope' };
	const alone = deckOptions(base, () => undefined);
	assert.equal(alone.relay, undefined);
	assert.equal(alone.nick, 'deck');
	assert.equal(alone.cdn, 'https://cdn/');
	const joined = deckOptions({ ...base, relay: 'ws://127.0.0.1:7070/live/CATH', nick: '  ' + 'x'.repeat(40) }, () => undefined);
	assert.equal(joined.relay, 'ws://127.0.0.1:7070/live/CATH');
	assert.equal(joined.nick.length, 24);
});
