// Unit tests of the media standard's rules (media-standard.ts – BACKLOG 10.64); the encoding needs a browser canvas.
//   npm test -w @scutoons/media
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { checkMedia, densityFromWidth, densityLevel, densityPath, densityPlan, IMAGE_MAX_BYTES, mediaKind, splitMediaName, standardName } from '../src/media-standard.ts';

test('checkMedia: PNG / JPEG convert to WebP, WebP / SVG / GIF are fine, a big WebP is too large', () => {
	assert.deepEqual(checkMedia({ type: 'image/png', name: 'a.png' }), { ok: false, advice: 'PNG → WebP', convertible: true });
	assert.deepEqual(checkMedia({ type: 'image/jpeg', name: 'a.jpg' }), { ok: false, advice: 'JPEG → WebP', convertible: true });
	assert.equal(checkMedia({ type: 'image/webp', name: 'a.webp', size: 2000 }).ok, true);
	assert.equal(checkMedia({ type: 'image/svg+xml', name: 'a.svg' }).ok, true);
	assert.deepEqual(checkMedia({ type: 'image/webp', name: 'a.webp', size: IMAGE_MAX_BYTES + 1 }), { ok: false, advice: 'too large', convertible: false });
});

test('checkMedia: the type from the extension when there is none (registry paths)', () => {
	assert.equal(checkMedia({ type: '', name: 'catpope/s1/c1/s1c1-slice-3.PNG' }).convertible, true);
	assert.equal(mediaKind({ type: '', name: 'x.webm' }), 'video');
});

test('checkMedia: video – MP4 / WebM fine, others need ffmpeg (not convertible here)', () => {
	assert.equal(checkMedia({ type: 'video/mp4', name: 'v.mp4' }).ok, true);
	assert.deepEqual(checkMedia({ type: 'video/quicktime', name: 'v.mov' }), { ok: false, advice: '→ MP4 (H.264) / WebM', convertible: false });
});

test('standardName: .png / .jpg → .webp, the rest unchanged', () => {
	assert.equal(standardName('cathedral.png'), 'cathedral.webp');
	assert.equal(standardName('Key.JPEG'), 'Key.webp');
	assert.equal(standardName('a.webp'), 'a.webp');
	assert.equal(standardName('a.svg'), 'a.svg');
});

test('splitMediaName: stem, density marker, extension', () => {
	assert.deepEqual(splitMediaName('catpope/s1/c1/s1c1-slice-26.3x.jpg'), { stem: 's1c1-slice-26', density: 3, ext: 'jpg' });
	assert.deepEqual(splitMediaName('slice-26@2x.PNG'), { stem: 'slice-26', density: 2, ext: 'png' });
	assert.deepEqual(splitMediaName('old-church-plaster.2x.png'), { stem: 'old-church-plaster', density: 2, ext: 'png' });
	assert.deepEqual(splitMediaName('old-church.png'), { stem: 'old-church', density: null, ext: 'png' });
	assert.deepEqual(splitMediaName('box 1x.webp'), { stem: 'box', density: 1, ext: 'webp' });
});

test('densityPath: 1x has no suffix', () => {
	assert.equal(densityPath('catpope/map/places/cathedral', 1), 'catpope/map/places/cathedral.webp');
	assert.equal(densityPath('catpope/map/places/cathedral', 3), 'catpope/map/places/cathedral.3x.webp');
});

test('densityFromWidth: by the 1x width, 1x … 3x', () => {
	assert.equal(densityFromWidth(390, 390), 1);
	assert.equal(densityFromWidth(779, 390), 2);
	assert.equal(densityFromWidth(1170, 390), 3);
	assert.equal(densityFromWidth(1024, 256), 3);
	assert.equal(densityFromWidth(100, 256), 1);
});

test('densityLevel: low / medium / high', () => {
	assert.deepEqual([densityLevel(1), densityLevel(2), densityLevel(3)], ['low', 'medium', 'high']);
});

test('densityPlan: every density up to the highest, from its own file else the smallest bigger one', () => {
	const plan = (ds: number[]) => densityPlan(ds.map((density, i) => ({ density: density as 1 | 2 | 3, i }))).map((p) => [p.density, p.source.i]);
	assert.deepEqual(plan([3]), [[1, 0], [2, 0], [3, 0]]);
	assert.deepEqual(plan([1, 3]), [[1, 0], [2, 1], [3, 1]]);
	assert.deepEqual(plan([3, 2]), [[1, 1], [2, 1], [3, 0]]);
	assert.deepEqual(plan([2]), [[1, 0], [2, 0]]);
	assert.deepEqual(plan([]), []);
});
