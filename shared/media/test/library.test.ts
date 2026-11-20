// The Library's families (one tile per asset family) and its scope / filter / order rules.
//   npm test -w @scutoons/media
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { AssetRendition, AssetSummary } from '@scutoons/api';
import { expectedVersions, formatDuration, groupFamilies, parseVersionName } from '../src/library/families.ts';
import {
	beforeKinds,
	contextOf,
	filterFamilies,
	fits,
	folderTree,
	groupByKind,
	inContext,
	kindCounts,
	originCounts,
	scopeCounts,
	sortFamilies,
	todoCounts,
	toonOfAddress
} from '../src/library/filter.ts';

let next = 1;
function file(path: string, extra: Partial<AssetRendition> = {}): AssetRendition {
	return { id: next++, label: 'x', quality: null, path, size: 100, mime: null, width: null, height: null, duration: null, origin: 'imported', ...extra };
}
function asset(slug: string, kind: AssetSummary['kind'], files: AssetRendition[], extra: Partial<AssetSummary> = {}): AssetSummary {
	return { id: `a-${slug}`, slug, kind, folder: slug.split('/')[0], title: slug, tags: [], version: 1, path: files[0].path, width: null, height: null, duration: null, size: 0, usages: 1, updated_at: null, renditions: files, ...extra };
}

test('parseVersionName: the markers in use', () => {
	assert.deepEqual(parseVersionName('catpope/s1/c1/s1c1-key-visual.jpg'), { base: 'catpope/s1/c1/s1c1-key-visual', label: 'source', ext: 'jpg', kind: 'image' });
	assert.equal(parseVersionName('catpope/s1/c1/s1c1-key-visual.2x.webp').label, '2x');
	assert.equal(parseVersionName('catpope/s1/c1/s1c1-key-visual.3x.webp').base, 'catpope/s1/c1/s1c1-key-visual');
	assert.equal(parseVersionName('catpope/s1/c1/s1c1-key-visual.480p.mp4').label, '480p');
	assert.equal(parseVersionName('catpope/s1/c1/s1c1-key-visual.900w.mp4').label, '900w');
	assert.equal(parseVersionName('catpope/s1/s1-trailer-loop.poster.webp').label, 'poster');
	assert.deepEqual(parseVersionName('catpope/s1/c1/s1c1-bells.128.mp3'), { base: 'catpope/s1/c1/s1c1-bells', label: '128k', ext: 'mp3', kind: 'audio' });
	// a number is a bitrate only on audio; a dot in a folder is no marker
	assert.equal(parseVersionName('catpope/items/v.128.webp').label, 'source');
	assert.equal(parseVersionName('catpope/v1.2/x.webp').base, 'catpope/v1.2/x');
	assert.equal(parseVersionName('characters/catpope/catpope-regular.glb').kind, 'model');
});

test('groupFamilies: a video, its stills and its poster are one family, its versions in order', () => {
	const families = groupFamilies([
		asset('catpope/s1/c1/s1c1-key-visual', 'video', [
			file('catpope/s1/c1/s1c1-key-visual.480p.mp4', { width: 854, height: 480 }),
			file('catpope/s1/c1/s1c1-key-visual.1080p.mp4', { width: 1920, height: 1080, duration: 12 }),
			file('catpope/s1/c1/s1c1-key-visual.720p.mp4', { width: 1280, height: 720, duration: 12 })
		]),
		asset('catpope/s1/c1/s1c1-key-visual-still', 'image', [file('catpope/s1/c1/s1c1-key-visual.jpg'), file('catpope/s1/c1/s1c1-key-visual.webp'), file('catpope/s1/c1/s1c1-key-visual.2x.webp')], { usages: 2 })
	]);
	assert.equal(families.length, 1);
	const [f] = families;
	assert.equal(f.kind, 'video');
	assert.equal(f.name, 's1c1-key-visual');
	assert.equal(f.folder, 'catpope/s1/c1');
	assert.deepEqual(f.versions.map((v) => v.label), ['source', '1080p', '720p', '480p', '1x', '2x']);
	assert.equal(f.assets.length, 2);
	assert.equal(f.usages, 3);
	assert.equal(f.duration, 12);
	assert.equal(f.main, 'catpope/s1/c1/s1c1-key-visual.720p.mp4');
	assert.equal(f.preview, 'catpope/s1/c1/s1c1-key-visual.webp');
	assert.deepEqual(f.missing, ['poster']);
});

test('formatDuration', () => {
	assert.equal(formatDuration(null), '');
	assert.equal(formatDuration(12.4), '0:12');
	assert.equal(formatDuration(130), '2:10');
	assert.equal(formatDuration(3725), '1:02:05');
});

test('expectedVersions: video levels up to the source, the audio levels, a chapter image 2x / 3x', () => {
	const [video] = groupFamilies([asset('t/v', 'video', [file('t/v.mp4', { width: 1280, height: 720 })])]);
	assert.deepEqual(expectedVersions(video), ['480p', '720p', 'poster']);
	assert.deepEqual(video.missing, ['480p', '720p', 'poster']);
	const [audio] = groupFamilies([asset('t/s1/c1/a', 'audio', [file('t/s1/c1/a.128.mp3'), file('t/s1/c1/a.320.mp3')])]);
	assert.deepEqual(audio.missing, ['96k']);
	const [slice] = groupFamilies([asset('t/s1/c2/s1c2-slice-1', 'image', [file('t/s1/c2/s1c2-slice-1.webp')])]);
	assert.deepEqual(slice.missing, ['2x', '3x']);
	const [vector] = groupFamilies([asset('t/vectors/dot', 'image', [file('t/vectors/dot.svg')])]);
	assert.deepEqual(vector.missing, []);
});

const fam = groupFamilies([
	asset('catpope/s1/c1/s1c1-slice-10', 'image', [file('catpope/s1/c1/s1c1-slice-10.webp', { width: 390, height: 624 })]),
	asset('catpope/s1/c1/s1c1-slice-2', 'image', [file('catpope/s1/c1/s1c1-slice-2.webp'), file('catpope/s1/c1/s1c1-slice-2.2x.webp'), file('catpope/s1/c1/s1c1-slice-2.3x.webp')], { usages: 4, updated_at: '2026-11-02' }),
	asset('catpope/s1/c1/s1c1-bells', 'audio', [file('catpope/s1/c1/s1c1-bells.128.mp3', { origin: 'uploaded' })], { usages: 0 }),
	asset('catpope/s1/c2/s1c2-slice-1', 'image', [file('catpope/s1/c2/s1c2-slice-1.webp')]),
	asset('characters/catpope/catpope_avatar-angry', 'image', [file('characters/catpope/catpope_avatar-angry.webp', { width: 512, height: 512 })], { usages: 0 }),
	asset('characters/catpope-human/catpope-human_avatar-neutral', 'image', [file('characters/catpope-human/catpope-human_avatar-neutral.webp', { width: 500, height: 520 })]),
	asset('characters/catpopes/x', 'image', [file('characters/catpopes/x.webp')]),
	asset('scutoons/items/camera', 'image', [file('scutoons/items/camera.webp', { origin: 'generated' })], { updated_at: '2026-11-10' })
]);

test('contextOf: each workspace narrows to its folder', () => {
	assert.equal(contextOf(null), null);
	assert.equal(contextOf('guide:style'), null);
	const chapter = contextOf('chapter:catpope:1')!;
	assert.deepEqual(fam.filter((f) => inContext(f, chapter)).map((f) => f.name), ['s1c1-bells', 's1c1-slice-10', 's1c1-slice-2']);
	assert.equal(contextOf('storyboard:catpope:1')!.pattern, chapter.pattern);
	const character = contextOf('character:catpope')!;
	assert.deepEqual(fam.filter((f) => inContext(f, character)).map((f) => f.name).sort(), ['catpope-human_avatar-neutral', 'catpope_avatar-angry']);
	assert.equal(character.folder, 'characters/catpope/');
	assert.deepEqual(fam.filter((f) => inContext(f, contextOf('item:camera'))).map((f) => f.name), ['camera']);
	assert.equal(fam.filter((f) => inContext(f, contextOf('season:catpope:1'))).length, 4);
	assert.equal(toonOfAddress('chapter:catpope:3'), 'catpope');
	assert.equal(toonOfAddress('character:catpope'), null);
});

test('scopeCounts / kindCounts / todo / origin', () => {
	assert.deepEqual(scopeCounts(fam, contextOf('chapter:catpope:1'), 'catpope'), { context: 3, toon: 4, all: 8, unused: 2 });
	assert.deepEqual(scopeCounts(fam, null, null), { context: 8, toon: 8, all: 8, unused: 2 });
	assert.deepEqual(kindCounts(fam), { image: 7, video: 0, audio: 1, model: 0, other: 0 });
	assert.deepEqual(todoCounts(fam), { missing: 3, unused: 2 });
	assert.deepEqual(originCounts(fam), { uploaded: 1, imported: 6, generated: 1 });
});

test('filterFamilies: scope, kind, search, to do, folder, origin', () => {
	const base = { context: contextOf('chapter:catpope:1'), toon: 'catpope' };
	assert.equal(filterFamilies(fam, { ...base, scope: 'context', kind: 'image' }).length, 2);
	assert.equal(filterFamilies(fam, { ...base, scope: 'toon', kind: 'all' }).length, 4);
	assert.deepEqual(filterFamilies(fam, { ...base, scope: 'unused', kind: 'all' }).map((f) => f.name), ['s1c1-bells', 'catpope_avatar-angry']);
	assert.deepEqual(filterFamilies(fam, { ...base, scope: 'all', kind: 'all', query: 'slice 2' }).map((f) => f.name), ['s1c1-slice-2', 's1c2-slice-1']);
	assert.deepEqual(filterFamilies(fam, { ...base, scope: 'all', kind: 'all', query: '3x' }).map((f) => f.name), ['s1c1-slice-2']);
	assert.equal(filterFamilies(fam, { ...base, scope: 'all', kind: 'all', todo: 'missing' }).length, 3);
	assert.equal(filterFamilies(fam, { ...base, scope: 'all', kind: 'all', folder: 'characters' }).length, 3);
	assert.equal(filterFamilies(fam, { ...base, scope: 'all', kind: 'all', origin: 'generated' })[0].name, 'camera');
	assert.equal(beforeKinds(fam, { ...base, scope: 'context' }).length, 3);
});

test('sortFamilies: natural names, newest, most used, biggest', () => {
	const chapter = fam.filter((f) => f.folder === 'catpope/s1/c1' && f.kind === 'image');
	assert.deepEqual(sortFamilies(chapter, 'name').map((f) => f.name), ['s1c1-slice-2', 's1c1-slice-10']);
	assert.equal(sortFamilies(fam, 'updated')[0].name, 'camera');
	assert.equal(sortFamilies(fam, 'uses')[0].name, 's1c1-slice-2');
	assert.equal(sortFamilies(fam, 'size')[0].name, 's1c1-slice-2');
});

test('groupByKind and folderTree', () => {
	assert.deepEqual(groupByKind(fam).map((g) => [g.kind, g.items.length]), [['image', 7], ['audio', 1]]);
	const tree = folderTree(fam);
	assert.deepEqual(tree.map((n) => [n.name, n.count]), [['catpope', 4], ['characters', 3], ['scutoons', 1]]);
	const s1 = tree[0].children[0];
	assert.deepEqual([s1.path, s1.children.map((c) => [c.name, c.count])], ['catpope/s1', [['c1', 3], ['c2', 1]]]);
	assert.deepEqual(folderTree(fam, 'catpope').map((n) => n.path), ['catpope/s1']);
});

test('fits: only what the slot takes; a square slot takes square pictures', () => {
	const slot = { kinds: ['image'] as const, square: true };
	const byName = (n: string) => fam.find((f) => f.name === n)!;
	assert.equal(fits(byName('catpope_avatar-angry'), slot), true);
	assert.equal(fits(byName('catpope-human_avatar-neutral'), slot), false);
	assert.equal(fits(byName('s1c1-slice-10'), slot), false);
	assert.equal(fits(byName('s1c1-bells'), slot), false);
	assert.equal(fits(byName('s1c2-slice-1'), slot), true);
	assert.equal(fits(byName('s1c1-bells'), null), true);
});
