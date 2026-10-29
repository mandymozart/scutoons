// Pairing a cyberdeck on the account page (#180, lib/account/decks.ts): the code as typed or scanned, and the body of
// the approval.
//   npm test -w viewer
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { approveBody, codeFromUrl, formatCode, normalizeCode } from '../src/lib/account/decks.ts';

test('a code is typed in any case, with or without the dash; the confusable letters are no code', () => {
	assert.equal(normalizeCode('K7Q-42M'), 'K7Q42M');
	assert.equal(normalizeCode(' k7q 42m '), 'K7Q42M');
	assert.equal(normalizeCode('K7Q42'), null);
	assert.equal(normalizeCode('K7Q-42MX'), null);
	assert.equal(normalizeCode('K0Q-42M'), null, 'no 0');
	assert.equal(normalizeCode('KIQ-42M'), null, 'no I');
	assert.equal(normalizeCode(null), null);
	assert.equal(formatCode('k7q42m'), 'K7Q-42M');
	assert.equal(formatCode('xx'), 'xx');
});

test('the QR link prefills the code', () => {
	assert.equal(codeFromUrl(new URL('https://scutoons.com/account/decks?deck=K7Q42M')), 'K7Q-42M');
	assert.equal(codeFromUrl(new URL('https://scutoons.com/account/decks?deck=nope')), '');
	assert.equal(codeFromUrl(new URL('https://scutoons.com/account/decks')), '');
});

test('the approval: the lifetime, idle only when chosen, a name only when given', () => {
	assert.deepEqual(approveBody('k7q-42m', 182, 0, ''), { code: 'K7Q42M', days: 182 });
	assert.deepEqual(approveBody('K7Q42M', 7, 1, '  Stage   left '), {
		code: 'K7Q42M',
		days: 7,
		idle_days: 1,
		name: 'Stage left'
	});
	assert.equal(approveBody('oops', 182, 0, ''), null);
});
