// What the service worker may cache (lib/sw/cache-rules.ts, agents/research/PWA.md §2 "Phase 2"): public CDN images and
// public library reads only – nothing private ever qualifies; the media cache's LRU cap per data tier.
//   npm test -w viewer
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	evictions,
	isCacheableAnswer,
	isLibraryRequest,
	isMediaRequest,
	mediaCap,
	MEDIA_CAPS,
	tierFromConnection,
	type RequestLike
} from '../src/lib/sw/cache-rules.ts';

const CDN = 'https://cdn.scutoons.com';
const API = 'https://api.scutoons.com/api';

function req(url: string, init: Partial<Omit<RequestLike, 'headers'>> & { headers?: Record<string, string> } = {}): RequestLike {
	const headers = new Headers(init.headers ?? {});
	return { url, method: init.method ?? 'GET', mode: init.mode ?? 'no-cors', credentials: init.credentials ?? 'same-origin', cache: init.cache ?? 'default', headers };
}

test('media: images on the public CDN, whole, without a query', () => {
	for (const path of ['catpope/s1/c1/s1c1-01.webp', 'catpope/s1/c1/s1c1-key-visual.webp', 'characters/cat_front-neutral.png', 'items/icons/x.svg', 'catpope/map/terrain/paint/v2/3-4.jpg'])
		assert.ok(isMediaRequest(req(`${CDN}/${path}`), CDN), path);
	// an <img> without crossorigin: no-cors + credentials include (the browser's default)
	assert.ok(isMediaRequest(req(`${CDN}/catpope/s1/c1/s1c1-01.webp`, { mode: 'no-cors', credentials: 'include' }), CDN));
});

test('media: never audio / video, a range, a no-store probe, the vault, a signed or other-host URL, credentials', () => {
	const no = [
		req(`${CDN}/catpope/s1/c1/s1c1-key-visual.480p.mp4`),
		req(`${CDN}/catpope/s1/c1/music.128.mp3`),
		req(`${CDN}/catpope/s1/c1/a.webp`, { headers: { Range: 'bytes=0-100' } }),
		req(`${CDN}/catpope/s1/c1/a.webp`, { cache: 'no-store' }),
		req(`${CDN}/scores/x.webp?token=HS256-abc&expires=1`),
		req('https://vault.scutoons.com/albums/x/cover.webp?token=HS256-abc&expires=1'),
		req('https://scutoons.com/x.webp'),
		req(`${CDN}/a.webp`, { mode: 'cors', credentials: 'include' }),
		req(`${CDN}/a.webp`, { headers: { Authorization: 'Bearer x' } }),
		req(`${CDN}/a.webp`, { method: 'POST' })
	];
	for (const r of no) assert.equal(isMediaRequest(r, CDN), false, `${r.method} ${r.url} ${r.cache}`);
});

test('library: the public client\'s reads of /library/ only', () => {
	for (const path of ['/library/toons?locale=de', '/library/toons/catpope?locale=de', '/library/toons/catpope/chapters/1?locale=de', '/library/toons/catpope/cast?chapter=1', '/library/map?locale=de'])
		assert.ok(isLibraryRequest(req(API + path, { mode: 'cors' }), API), path);
});

test('library: nothing private – auth, session, me, inventory, progress, codes, purchases, signed URLs, drafts, credentials', () => {
	const no = [
		'/auth/session',
		'/me/',
		'/inventory/',
		'/progress/catpope',
		'/access/check',
		'/codes/ABC',
		'/claim',
		'/orders/',
		'/items/x/score/pdf',
		'/albums/a/tracks/1',
		'/library/toons/catpope/chapters/1?draft=1',
		'/items/',
		'/events/'
	].map((path) => req(API + path, { mode: 'cors' }));
	no.push(req(`${API}/library/toons`, { mode: 'cors', credentials: 'include' }));
	no.push(req(`${API}/library/toons`, { mode: 'cors', headers: { Authorization: 'Bearer t' } }));
	no.push(req(`${API}/library/toons`, { mode: 'cors', headers: { 'X-Device-Id': 'd' } }));
	no.push(req(`${API}/library/toons`, { mode: 'cors', method: 'POST' }));
	no.push(req('https://evil.example/api/library/toons', { mode: 'cors' }));
	for (const r of no) assert.equal(isLibraryRequest(r, API), false, `${r.method} ${r.url} ${r.credentials}`);
});

test('answers: whole public 200s only, never opaque, private or no-store', () => {
	const answer = (status: number, type: string, control: string | null) => ({ status, type, headers: new Headers(control ? { 'Cache-Control': control } : {}) });
	assert.ok(isCacheableAnswer(answer(200, 'cors', 'public, max-age=2592000'), 'media'));
	assert.ok(isCacheableAnswer(answer(200, 'cors', null), 'media'));
	assert.ok(isCacheableAnswer(answer(200, 'cors', 'public, max-age=60'), 'library'));
	assert.equal(isCacheableAnswer(answer(200, 'opaque', null), 'media'), false, 'opaque: ~7 MB quota each');
	assert.equal(isCacheableAnswer(answer(206, 'cors', null), 'media'), false);
	assert.equal(isCacheableAnswer(answer(404, 'cors', 'no-cache'), 'media'), false);
	assert.equal(isCacheableAnswer(answer(200, 'cors', 'private, no-store'), 'library'), false, 'a draft answer');
	assert.equal(isCacheableAnswer(answer(200, 'cors', 'no-store'), 'library'), false);
	assert.equal(isCacheableAnswer(answer(200, 'cors', null), 'library'), false, 'library: only what says public');
});

test('caps: 150 MB normal / generous, less on light, nothing new on none; never over a quarter of the quota', () => {
	assert.equal(mediaCap('normal'), 150 * 1024 * 1024);
	assert.equal(mediaCap('generous'), MEDIA_CAPS.normal);
	assert.ok(mediaCap('light') < mediaCap('normal') && mediaCap('light') > 0);
	assert.equal(mediaCap('none'), 0);
	assert.equal(mediaCap('normal', 100 * 1024 * 1024), 25 * 1024 * 1024);
	assert.equal(tierFromConnection({ saveData: true }), 'none');
	assert.equal(tierFromConnection({ effectiveType: '2g' }), 'none');
	assert.equal(tierFromConnection({ effectiveType: '3g' }), 'light');
	assert.equal(tierFromConnection(undefined), 'normal');
});

test('eviction: least recently used first, down to 90 % of the cap', () => {
	const e = (url: string, size: number, used: number) => ({ url, size, used, fetched: 0 });
	const entries = [e('a', 40, 3), e('b', 40, 1), e('c', 40, 2), e('d', 40, 4)];
	assert.deepEqual(evictions(entries, 200), [], 'fits');
	assert.deepEqual(evictions(entries, 100), ['b', 'c'], '160 → 80 ≤ 90');
	assert.deepEqual(evictions(entries, 160), [], 'exactly the cap');
	assert.deepEqual(evictions(entries, 150), ['b'], '160 > 150 → 120 ≤ 135');
});
