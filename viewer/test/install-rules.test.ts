// When the "Install SCUTOONS as an app" bar shows (lib/install-rules.ts, agents/research/PWA.md "Install prompt";
// owner 2026-10-16: "a prompt to install for Android first"):
//   npm test -w viewer
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { EMPTY_MEMORY, SNOOZE_DAYS, canInstall, readMemory, shouldOffer, snoozed } from '../src/lib/install-rules.ts';

const DAY = 24 * 60 * 60 * 1000;
const now = Date.UTC(2026, 9, 16);
const chrome = { available: true, standalone: false, framed: false };
const memory = (change = {}) => ({ ...EMPTY_MEMORY, visits: 1, ...change });

test('never on the first visit: from the 2nd visit or after a finished chapter', () => {
	assert.equal(shouldOffer(chrome, memory(), now), false, 'first visit, nothing read');
	assert.equal(shouldOffer(chrome, memory({ visits: 2 }), now), true, 'came back');
	assert.equal(shouldOffer(chrome, memory({ chapters: 1 }), now), true, 'finished a chapter on the first visit');
});

test('only where the browser can install: the event, not installed, not framed', () => {
	const back = memory({ visits: 3 });
	assert.equal(shouldOffer({ ...chrome, available: false }, back, now), false, 'iOS / Firefox: no event');
	assert.equal(shouldOffer({ ...chrome, standalone: true }, back, now), false, 'inside the installed app');
	assert.equal(shouldOffer({ ...chrome, framed: true }, back, now), false, "the creator's Reader frame");
	assert.equal(shouldOffer(chrome, { ...back, installed: true }, now), false, 'appinstalled: never again');
});

test('"Not now" hides the bar for 30 days; the account entry stays', () => {
	const back = memory({ visits: 5, snoozedAt: now - 2 * DAY });
	assert.equal(snoozed(back, now), true);
	assert.equal(shouldOffer(chrome, back, now), false);
	assert.equal(canInstall(chrome, back), true, 'account page: "Install app" still there');
	assert.equal(shouldOffer(chrome, back, now - 2 * DAY + SNOOZE_DAYS * DAY), true, 'after 30 days it comes back');
	assert.equal(shouldOffer(chrome, { ...back, snoozedAt: now + DAY }, now), true, 'a date in the future (clock changed) does not block forever');
});

test('the stored memory: broken or foreign values read as empty', () => {
	assert.deepEqual(readMemory(null), EMPTY_MEMORY);
	assert.deepEqual(readMemory('{not json'), EMPTY_MEMORY);
	assert.deepEqual(readMemory('"x"'), EMPTY_MEMORY);
	assert.deepEqual(readMemory('{"visits":-3,"chapters":"2","snoozedAt":"yesterday","installed":1}'), EMPTY_MEMORY);
	assert.deepEqual(readMemory('{"visits":2.7,"chapters":1,"snoozedAt":5,"installed":true}'), { visits: 2, chapters: 1, snoozedAt: 5, installed: true });
});
