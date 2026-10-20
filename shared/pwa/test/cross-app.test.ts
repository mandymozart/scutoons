// One SCUTOONS app opens another (src/cross-app.ts, research/PWA-APPS.md §3a): on Android inside an installed app the
// link goes out as an intent – the system hands it to the other installed app:
//   npm test -w @scutoons/pwa
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { intentsNeeded, intentUrl } from '../src/cross-app.ts';

const ANDROID = 'Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Mobile Safari/537.36';
const IPHONE = 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.5 Mobile/15E148 Safari/604.1';
const DESKTOP = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36';

test('an https address as an Android intent: the system picks the installed app, the browser is the fallback', () => {
	assert.equal(
		intentUrl('https://creator.scutoons.com/catpope?x=1'),
		'intent://creator.scutoons.com/catpope?x=1#Intent;scheme=https;action=android.intent.action.VIEW;category=android.intent.category.BROWSABLE;launchFlags=0x10000000;S.browser_fallback_url=https%3A%2F%2Fcreator.scutoons.com%2Fcatpope%3Fx%3D1;end'
	);
	assert.match(intentUrl('https://scutoons.com/'), /^intent:\/\/scutoons\.com\/#Intent;scheme=https;/);
	assert.match(intentUrl('http://localhost:10000/guide#script'), /^intent:\/\/localhost:10000\/guide#Intent;scheme=http;.*fallback_url=http%3A%2F%2Flocalhost%3A10000%2Fguide;end$/, 'the fragment is dropped');
	assert.equal(intentUrl('/relative'), '/relative');
	assert.equal(intentUrl('mailto:a@b.c'), 'mailto:a@b.c');
});

test('only on Android inside an installed app', () => {
	assert.equal(intentsNeeded(ANDROID, true), true);
	assert.equal(intentsNeeded(ANDROID, false), false, 'a browser tab: the plain link');
	assert.equal(intentsNeeded(IPHONE, true), false, 'iOS: home-screen apps cannot open each other');
	assert.equal(intentsNeeded(DESKTOP, true), false, 'desktop Chrome captures the plain link itself');
});
