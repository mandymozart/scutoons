// "Save chapter to device" (lib/sw/saved-rules.ts, agents/research/PWA.md §2 "Phase 3"): the files per quality level,
// the stand-ins between levels, the size shown first, the data policy, Range answers, the access rules.
//   npm test -w viewer
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	accessEnded,
	accessUntilOf,
	savedWhole,
	aliasesFor,
	autoSaveAllowed,
	autoSaveVideos,
	bytesByKind,
	saveKindOf,
	dataWarnings,
	documentUrls,
	filesFor,
	isSavedCacheName,
	isSavedDocRequest,
	isStaleSavedCacheName,
	maySaveStreamed,
	offerWithoutVideos,
	parseRange,
	planSave,
	roundMb,
	savedCacheName,
	savedReadable,
	toPurge,
	type ChapterFile,
	type ChapterFiles
} from '../src/lib/sw/saved-rules.ts';
import {
	kindOfUrl,
	planRefresh,
	planSeason,
	refreshDue,
	refreshMode,
	REFRESH_EVERY_MS,
	seasonState,
	seasonTargets,
	SMALL_REFRESH_BYTES,
	type SeasonChapter
} from '../src/lib/sw/saved-update-rules.ts';

const MB = 1024 * 1024;
const FILES: ChapterFile[] = [
	{ path: 't/s1/one.webp', size: 100, kind: 'image', levels: ['low'], group: 0 },
	{ path: 't/s1/one@2x.webp', size: 200, kind: 'image', levels: ['medium'], group: 0 },
	{ path: 't/s1/one@3x.webp', size: 300, kind: 'image', levels: ['high'], group: 0 },
	{ path: 't/s1/two.webp', size: 50, kind: 'image' },
	{ path: 't/s1/m.96.mp3', size: 1 * MB, kind: 'audio', levels: ['low', 'medium'], group: 1 },
	{ path: 't/s1/m.320.mp3', size: 3 * MB, kind: 'audio', levels: ['high'], group: 1 },
	{ path: 't/s1/v.480p.mp4', size: 4 * MB, kind: 'video', levels: ['low'], group: 2 },
	{ path: 't/s1/v.720p.mp4', size: 12 * MB, kind: 'video', levels: ['medium', 'high'], group: 2 },
	{ path: 't/s1/unknown.mp3', size: null, kind: 'audio' }
];

test('cache names: one per chapter and version; never a phase 2 cache', () => {
	assert.equal(savedCacheName('catpope', 1), 'scutoons-chapter-catpope-1-v1');
	assert.ok(isSavedCacheName('scutoons-chapter-catpope-1-v1'));
	for (const other of ['scutoons-media-v1', 'scutoons-library-v1', 'scutoons-shell-0.1.0-abc']) assert.ok(!isSavedCacheName(other), other);
	assert.ok(isStaleSavedCacheName('scutoons-chapter-catpope-1-v0'));
	assert.ok(!isStaleSavedCacheName('scutoons-chapter-catpope-1-v1'));
});

test('the files of a level: its variants + what every level uses; without videos on request', () => {
	assert.deepEqual(filesFor(FILES, 'low').map((f) => f.path), ['t/s1/one.webp', 't/s1/two.webp', 't/s1/m.96.mp3', 't/s1/v.480p.mp4', 't/s1/unknown.mp3']);
	assert.deepEqual(filesFor(FILES, 'high').map((f) => f.path), ['t/s1/one@3x.webp', 't/s1/two.webp', 't/s1/m.320.mp3', 't/s1/v.720p.mp4', 't/s1/unknown.mp3']);
	assert.ok(!filesFor(FILES, 'medium', false).some((f) => f.kind === 'video'));
});

test('the size before a byte moves: registry sizes, unknown ones estimated; the "without videos" offer', () => {
	const medium = planSave(FILES, 'medium');
	assert.equal(medium.bytes, 200 + 50 + 1 * MB + 12 * MB + 2 * MB);
	assert.equal(medium.videoBytes, 12 * MB);
	assert.ok(offerWithoutVideos(medium), '12 MB of videos in 15 MB');
	const lean = planSave(FILES, 'medium', false);
	assert.equal(lean.bytes, 200 + 50 + 1 * MB + 2 * MB);
	assert.equal(lean.videoBytes, 12 * MB, 'what leaving them out saves');
	assert.ok(!offerWithoutVideos(planSave(FILES, 'low')), '4 MB of videos: no offer');
	assert.equal(roundMb(18.4 * MB), '18');
	assert.equal(roundMb(2.26 * MB), '2.3');
	assert.equal(roundMb(300 * 1024), '< 1');
});

test('stand-ins: another level of a saved media answers with the saved file; unsaved media none', () => {
	const saved = filesFor(FILES, 'medium');
	assert.deepEqual(aliasesFor(FILES, saved), {
		't/s1/one.webp': 't/s1/one@2x.webp',
		't/s1/one@3x.webp': 't/s1/one@2x.webp',
		't/s1/m.320.mp3': 't/s1/m.96.mp3',
		't/s1/v.480p.mp4': 't/s1/v.720p.mp4'
	});
	const lean = filesFor(FILES, 'medium', false);
	assert.ok(!('t/s1/v.480p.mp4' in aliasesFor(FILES, lean)), 'saved without videos: the video stays online-only');
});

test('the data policy: mobile data warned from 20 MB, data saver always said', () => {
	assert.deepEqual(dataWarnings(25 * MB, { type: 'cellular' }), { saveData: false, cellular: true });
	assert.deepEqual(dataWarnings(5 * MB, { type: 'cellular' }), { saveData: false, cellular: false });
	assert.deepEqual(dataWarnings(25 * MB, { type: 'wifi' }), { saveData: false, cellular: false });
	assert.deepEqual(dataWarnings(1 * MB, { saveData: true }), { saveData: true, cellular: false });
	assert.deepEqual(dataWarnings(25 * MB, undefined), { saveData: false, cellular: false });
});

test('the documents a saved chapter opens with: exactly the app\'s public library / catalog URLs', () => {
	assert.deepEqual(documentUrls('https://api.scutoons.com/api/', 'catpope', 1, 'de'), [
		'https://api.scutoons.com/api/library/toons/catpope/chapters/1?locale=de',
		'https://api.scutoons.com/api/library/toons/catpope?locale=de',
		'https://api.scutoons.com/api/library/toons/catpope/cast',
		'https://api.scutoons.com/api/library/toons/catpope/credits',
		'https://api.scutoons.com/api/library/toons?locale=de',
		'https://api.scutoons.com/api/items/',
		'https://api.scutoons.com/api/editions/'
	]);
	const API = 'https://api.scutoons.com/api';
	const req = (url: string, headers: Record<string, string> = {}, credentials = 'same-origin') => ({ url, method: 'GET', credentials, headers: new Headers(headers) });
	assert.ok(isSavedDocRequest(req(`${API}/items/`), API));
	assert.ok(isSavedDocRequest(req(`${API}/editions/`), API));
	assert.ok(!isSavedDocRequest(req(`${API}/items/`, { Authorization: 'Bearer x' }), API), 'never with a session');
	assert.ok(!isSavedDocRequest(req(`${API}/items/`, { 'X-Device-Id': 'd' }), API));
	assert.ok(!isSavedDocRequest(req(`${API}/items/`, {}, 'include'), API));
	assert.ok(!isSavedDocRequest(req(`${API}/inventory/`), API));
	assert.ok(!isSavedDocRequest(req(`${API}/items/?x=1`), API));
});

test('streamed files a saved chapter may hold before the index is read: audio, video, fonts on the CDN', () => {
	const CDN = 'https://cdn.scutoons.com';
	assert.ok(maySaveStreamed(new URL(`${CDN}/catpope/s1/c1/a.128.mp3`), CDN));
	assert.ok(maySaveStreamed(new URL(`${CDN}/catpope/s1/c1/kv.900w.mp4`), CDN));
	assert.ok(maySaveStreamed(new URL(`${CDN}/scutoons/fonts/x.woff2`), CDN));
	assert.ok(!maySaveStreamed(new URL(`${CDN}/catpope/s1/c1/a.webp`), CDN), 'images: the media cache path');
	assert.ok(!maySaveStreamed(new URL(`${CDN}/vault/a.mp3?token=x&expires=1`), CDN), 'signed URLs never');
	assert.ok(!maySaveStreamed(new URL('https://vault.example/a.mp3'), CDN));
});

test('Range: one range from the whole file (206), suffix and open ranges, 416 past the end', () => {
	assert.equal(parseRange(null, 1000), null);
	assert.deepEqual(parseRange('bytes=0-', 1000), { start: 0, end: 999 });
	assert.deepEqual(parseRange('bytes=0-1', 1000), { start: 0, end: 1 });
	assert.deepEqual(parseRange('bytes=500-2000', 1000), { start: 500, end: 999 }, 'clamped to the file');
	assert.deepEqual(parseRange('bytes=-100', 1000), { start: 900, end: 999 });
	assert.deepEqual(parseRange('bytes=-5000', 1000), { start: 0, end: 999 });
	assert.equal(parseRange('bytes=1000-', 1000), 'unsatisfiable');
	assert.equal(parseRange('bytes=5-2', 1000), 'unsatisfiable');
	assert.equal(parseRange('bytes=0-1,5-9', 1000), null, 'multi-range: the whole file');
	assert.equal(parseRange('items=0-1', 1000), null);
});

test('access: free chapters always; gated ones until the access end they were saved with', () => {
	const now = Date.parse('2026-10-16T12:00:00Z');
	assert.ok(accessEnded('2026-10-01 00:00:00', now));
	assert.ok(!accessEnded('2027-01-01T00:00:00Z', now));
	assert.ok(!accessEnded(null, now));
	assert.ok(savedReadable({ gated: false, accessUntil: '2020-01-01 00:00:00' }, now));
	assert.ok(savedReadable({ gated: true, accessUntil: null }, now));
	assert.ok(!savedReadable({ gated: true, accessUntil: '2026-10-01 00:00:00' }, now));
	assert.equal(accessUntilOf([]), null);
	assert.equal(accessUntilOf(['2026-12-01 00:00:00', null]), null, 'a grant without an end');
	assert.equal(accessUntilOf(['2026-12-01 00:00:00', '2027-02-01T00:00:00Z']), '2027-02-01T00:00:00Z', 'the latest');
});

test('purge on start: by the known access, offline by the saved end; free chapters never', () => {
	const now = Date.parse('2026-10-16T12:00:00Z');
	const saved = [
		{ id: 'free', gated: false, accessUntil: null },
		{ id: 'open', gated: true, accessUntil: null },
		{ id: 'ended', gated: true, accessUntil: '2026-10-01 00:00:00' },
		{ id: 'later', gated: true, accessUntil: '2027-01-01 00:00:00' }
	];
	const ids = (list: { id: string }[]) => list.map((m) => m.id);
	assert.deepEqual(ids(toPurge(saved, { known: true, valid: false }, now)), ['open', 'ended', 'later'], 'access ended: every gated save');
	assert.deepEqual(ids(toPurge(saved, { known: true, valid: true }, now)), [], 'renewed: kept, even past the saved end');
	assert.deepEqual(ids(toPurge(saved, { known: false, valid: false }, now)), ['ended'], 'offline: the saved end decides');
});

test('auto save: off never, data saver never, wifi only where the browser says so, always everywhere', () => {
	assert.equal(autoSaveAllowed('off', { type: 'wifi' }), false);
	assert.equal(autoSaveAllowed('always', { type: 'cellular', saveData: true }), false);
	assert.equal(autoSaveAllowed('wifi', { type: 'wifi' }), true);
	assert.equal(autoSaveAllowed('wifi', { type: 'ethernet' }), true);
	assert.equal(autoSaveAllowed('wifi', { type: 'cellular' }), false);
	// Safari / Firefox: no connection info – only "always" saves there
	assert.equal(autoSaveAllowed('wifi', undefined), false);
	assert.equal(autoSaveAllowed('always', undefined), true);
	assert.equal(autoSaveAllowed('always', { type: 'cellular' }), true);
});

test('auto save: the videos stay unless they are the bulk', () => {
	assert.equal(autoSaveVideos({ files: [], bytes: 20 * MB, videoBytes: 2 * MB }), true);
	assert.equal(autoSaveVideos({ files: [], bytes: 50 * MB, videoBytes: 40 * MB }), false);
});
test('progress per kind: pictures, music, videos, the rest', () => {
	assert.equal(saveKindOf('image'), 'image');
	assert.equal(saveKindOf('font'), 'data');
	assert.equal(saveKindOf('archive'), 'data');
	assert.deepEqual(
		bytesByKind([
			{ path: 'a.webp', size: 100, kind: 'image' },
			{ path: 'b.webp', size: 50, kind: 'image' },
			{ path: 'c.mp3', size: 1000, kind: 'audio' },
			{ path: 'd.mp4', size: null, kind: 'video' },
			{ path: 'e.json', size: 7, kind: 'other' }
		]),
		{ image: 150, audio: 1000, video: 5 * MB, data: 7 }
	);
});

// ── a new version of a saved chapter; a whole season (lib/sw/saved-update-rules.ts) ─────────────────────────────────────
const CDN = 'https://cdn.example/';
const url = (path: string) => CDN + path;
const LIST = (files: ChapterFile[], version = 'v1', chapter = 1): ChapterFiles => ({ toon: 't', chapter, locale: 'de', version, gated: false, files });
const SAVED = { version: 'v1', level: 'medium' as const, videos: true, urls: [...filesFor(FILES, 'medium').map((f) => url(f.path)), CDN + 'fonts/letter.woff2'] };

test('refresh: the same version and files → nothing to do', () => {
	const plan = planRefresh(SAVED, LIST(FILES), url);
	assert.equal(plan.changed, false);
	assert.deepEqual(plan.add, []);
	assert.deepEqual(plan.drop, []);
	assert.equal(plan.bytes, 0);
});

test('refresh: only the changed files are downloaded, the gone ones dropped, the fonts kept', () => {
	// a swapped picture (a new name – immutable CDN paths) and a new slice; the 720p video is gone
	const next = [
		...FILES.filter((f) => f.path !== 't/s1/one@2x.webp' && f.kind !== 'video'),
		{ path: 't/s1/one.v2@2x.webp', size: 220, kind: 'image', levels: ['medium'], group: 0 },
		{ path: 't/s1/three.webp', size: 70, kind: 'image' }
	] as ChapterFile[];
	const plan = planRefresh(SAVED, LIST(next, 'v2'), url);
	assert.equal(plan.changed, true);
	assert.deepEqual(plan.add.map((f) => f.path), ['t/s1/one.v2@2x.webp', 't/s1/three.webp']);
	assert.deepEqual(plan.drop, [url('t/s1/one@2x.webp'), url('t/s1/v.720p.mp4')]);
	assert.equal(plan.bytes, 290);
	assert.ok(plan.urls.includes(CDN + 'fonts/letter.woff2'), 'the lettering fonts stay');
	assert.ok(!plan.urls.includes(url('t/s1/v.720p.mp4')));
	// saved without the videos: a changed video is not fetched, the new version's documents still come
	const leanSaved = { ...SAVED, videos: false, urls: filesFor(FILES, 'medium', false).map((f) => url(f.path)) };
	const lean = planRefresh(leanSaved, LIST(FILES.map((f) => (f.kind === 'video' ? { ...f, path: f.path + '.v2' } : f)), 'v2'), url);
	assert.deepEqual(lean.add, []);
	assert.equal(lean.changed, true);
	// the same files, only the version (a text fix): changed, nothing to download
	const text = planRefresh(SAVED, LIST(FILES, 'v2'), url);
	assert.deepEqual([text.changed, text.bytes], [true, 0]);
});

test('refresh: background when small or the setting would save; never with data saver / tier none', () => {
	assert.equal(refreshMode(200 * 1024, 'off', undefined, 'normal'), 'auto', 'a text fix: always');
	assert.equal(refreshMode(SMALL_REFRESH_BYTES + 1, 'off', undefined, 'normal'), 'ask');
	assert.equal(refreshMode(5 * MB, 'always', { type: 'cellular' }, 'normal'), 'auto');
	assert.equal(refreshMode(5 * MB, 'wifi', { type: 'cellular' }, 'normal'), 'ask');
	assert.equal(refreshMode(5 * MB, 'wifi', { type: 'wifi' }, 'generous'), 'auto');
	assert.equal(refreshMode(10, 'always', { saveData: true }, 'none'), 'ask', 'data saver: the reader taps');
	assert.equal(refreshMode(10, 'always', { type: 'wifi' }, 'none'), 'ask', 'tier none');
});

test('refresh: asked at most every 30 minutes', () => {
	assert.ok(refreshDue(undefined, 1000));
	assert.ok(!refreshDue(1000, 1000 + REFRESH_EVERY_MS - 1));
	assert.ok(refreshDue(1000, 1000 + REFRESH_EVERY_MS));
});

test('refresh: a stored URL counts as its kind', () => {
	assert.equal(kindOfUrl(url('a/b.WEBP')), 'image');
	assert.equal(kindOfUrl(url('a/b.m4a')), 'audio');
	assert.equal(kindOfUrl(url('a/b.webm')), 'video');
	assert.equal(kindOfUrl(url('fonts/x.woff2')), 'data');
	assert.equal(kindOfUrl('https://api.example/api/library/toons/t/chapters/1?locale=de'), 'data');
});

const SEASON: SeasonChapter[] = [
	{ number: 0, readable: true },
	{ number: 1, readable: true },
	{ number: 2, readable: true },
	{ number: 3, readable: false }
];

test('season: the readable chapters not saved yet, in order; the button state', () => {
	assert.deepEqual(seasonTargets(SEASON, (n) => n === 1), [0, 2]);
	assert.deepEqual(seasonTargets(SEASON, () => false), [0, 1, 2]);
	assert.equal(seasonState(SEASON, (n) => n === 1), 'partial');
	assert.equal(seasonState(SEASON, (n) => n < 3), 'saved', 'the coming-soon one does not count');
	assert.equal(seasonState(SEASON, () => false), 'partial');
	assert.equal(seasonState([{ number: 3, readable: false }], () => false), 'none');
});

test('season: the size of all its chapters; "without videos" judged on the whole season', () => {
	const small: ChapterFile[] = [{ path: 'a.webp', size: 2 * MB, kind: 'image' }];
	const video: ChapterFile[] = [
		{ path: 'b.webp', size: 1 * MB, kind: 'image' },
		{ path: 'v.mp4', size: 11 * MB, kind: 'video' }
	];
	const plan = planSeason([LIST(small, 'x', 1), LIST(video, 'y', 2)], 'low');
	assert.deepEqual(plan.chapters.map((c) => c.chapter), [1, 2]);
	assert.equal(plan.bytes, 14 * MB);
	assert.equal(plan.leanBytes, 3 * MB);
	assert.ok(plan.offerLean, '11 MB of videos in 14 MB');
	assert.ok(!planSeason([LIST(small), LIST(small, 'x', 2)], 'low').offerLean);
});
test('a saved chapter is whole only with its videos', () => {
	assert.equal(savedWhole({ videos: true }), true);
	assert.equal(savedWhole({ videos: false }), false);
	assert.equal(savedWhole(undefined), false);
});
