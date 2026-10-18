// When the "Install SCUTOONS as an app" bar shows (lib/install-rules.ts, agents/research/PWA.md "Install prompt";
// owner 2026-10-16: "a prompt to install for Android first"):
//   npm test -w viewer
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	EMPTY_MEMORY,
	SNOOZE_DAYS,
	canInstall,
	forcedOffer,
	installWay,
	isAndroidFirefox,
	isIosSafari,
	isSafari,
	manualWay,
	readMemory,
	shouldOffer,
	snoozed
} from '../src/lib/install-rules.ts';

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

const IPHONE_SAFARI = 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.5 Mobile/15E148 Safari/604.1';
const IPAD_DESKTOP_MODE = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.5 Safari/605.1.15';
const IPHONE_CHROME = 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/138.0.7204.156 Mobile/15E148 Safari/604.1';
const IPHONE_INSTAGRAM = 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148 Instagram 390.0.0.0';
const ANDROID_CHROME = 'Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Mobile Safari/537.36';

test('the iOS how-to: Safari on iPhone / iPad only', () => {
	assert.equal(isIosSafari(IPHONE_SAFARI), true);
	assert.equal(isIosSafari(IPAD_DESKTOP_MODE, 5), true, 'iPadOS asks as a Mac with touch');
	assert.equal(isIosSafari(IPAD_DESKTOP_MODE, 0), false, 'a real Mac');
	assert.equal(isIosSafari(IPHONE_CHROME), false);
	assert.equal(isIosSafari(IPHONE_INSTAGRAM), false);
	assert.equal(isIosSafari(ANDROID_CHROME), false);
});

const ANDROID_FIREFOX = 'Mozilla/5.0 (Android 14; Mobile; rv:143.0) Gecko/143.0 Firefox/143.0';
const DESKTOP_FIREFOX = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:143.0) Gecko/20100101 Firefox/143.0';
const MAC_CHROME = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36';
const way = (change = {}, mem = memory()) =>
	installWay({ available: false, standalone: false, framed: false, iosSafari: false, androidFirefox: false, ...change }, mem);

test('Firefox on Android installs by hand; Safari (also on a Mac) evicts after 7 days', () => {
	assert.equal(isAndroidFirefox(ANDROID_FIREFOX), true);
	assert.equal(isAndroidFirefox(DESKTOP_FIREFOX), false);
	assert.equal(isAndroidFirefox(ANDROID_CHROME), false);
	assert.equal(isSafari(IPHONE_SAFARI), true);
	assert.equal(isSafari(IPAD_DESKTOP_MODE, 0), true, 'Safari on a Mac');
	assert.equal(isSafari(MAC_CHROME), false);
	assert.equal(isSafari(IPHONE_CHROME), false, 'Chrome on iOS: its own storage rules are not ours to explain');
	assert.equal(isSafari(ANDROID_FIREFOX), false);
});

test('the account page says what this browser can do', () => {
	assert.equal(way({ available: true }), 'available');
	assert.equal(way({ available: true }, memory({ installed: true })), 'unsupported', 'installed on this browser: open it from the home screen');
	assert.equal(way({ iosSafari: true }), 'ios');
	assert.equal(way({ androidFirefox: true }), 'android-firefox');
	assert.equal(way({}), 'unsupported');
	assert.equal(way({ standalone: true, available: true }), 'installed');
	assert.equal(way({ framed: true, iosSafari: true }), 'hidden');
	assert.equal(manualWay('ios') && manualWay('android-firefox'), true);
	assert.equal(manualWay('available') || manualWay('unsupported'), false);
});

test('the bar by hand (no install event): the same moment and snooze as Chrome’s', () => {
	const manual = { available: false, standalone: false, framed: false, manual: true };
	assert.equal(shouldOffer(manual, memory(), now), false, 'never on the first visit');
	assert.equal(shouldOffer(manual, memory({ visits: 2 }), now), true);
	assert.equal(shouldOffer(manual, memory({ visits: 2, snoozedAt: now - DAY }), now), false);
	assert.equal(shouldOffer({ ...manual, standalone: true }, memory({ visits: 2 }), now), false);
	assert.equal(shouldOffer({ ...manual, framed: true }, memory({ visits: 2 }), now), false);
	assert.equal(shouldOffer({ ...manual, manual: false }, memory({ visits: 2 }), now), false, 'no way to install: no bar');
});

test('?install shows the bar on every browser – not in the app, not in the creator frame', () => {
	for (const w of ['available', 'ios', 'android-firefox', 'unsupported'] as const) assert.equal(forcedOffer(w), true, w);
	assert.equal(forcedOffer('installed'), false);
	assert.equal(forcedOffer('hidden'), false);
});
