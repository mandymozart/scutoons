// Unit tests of the optimisation tool's audit (media-audit.ts – BACKLOG 10.64).
//   npm test -w @scutoons/media
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { auditAsset, auditAssets, nextFreeName, type AuditAsset } from '../src/media-audit.ts';

const file = (path: string, mime: string, size: number, usages = 1) => ({ path, size, mime, label: 'original', usages });
const asset = (files: ReturnType<typeof file>[]): AuditAsset => ({
	id: files[0].path,
	slug: files[0].path.replace(/\.[^.]+$/, ''),
	kind: files[0].mime.startsWith('video') ? 'video' : 'image',
	path: files[0].path,
	usages: files.reduce((n, f) => n + f.usages, 0),
	renditions: files
});

test('auditAsset: a used WebP within the limit is on the standard', () => {
	assert.deepEqual(auditAsset(asset([file('catpope/s1/c1/s1c1-slice-1.webp', 'image/webp', 90_000)])), []);
});

test('auditAsset: a used PNG converts to a new WebP beside it', () => {
	const [f] = auditAsset(asset([file('catpope/items/camera.png', 'image/png', 400_000, 3)]));
	assert.deepEqual([f.fix, f.target, f.advice, f.usages], ['convert', 'catpope/items/camera.webp', 'PNG → WebP', 3]);
});

test('auditAsset: an unused PNG master beside its used WebP is fine (owner data: toptoy_apose-naked)', () => {
	assert.deepEqual(auditAsset(asset([file('characters/toptoy/toptoy_apose-naked.png', 'image/png', 1_101_054, 0), file('characters/toptoy/toptoy_apose-naked.webp', 'image/webp', 182_136, 1)])), []);
});

test('auditAsset: a used JPEG whose WebP twin exists – relink only', () => {
	const [f] = auditAsset(asset([file('catpope/s1/c1/s1c1-key-visual.jpg', 'image/jpeg', 800_000, 2), file('catpope/s1/c1/s1c1-key-visual.webp', 'image/webp', 200_000, 0)]));
	assert.deepEqual([f.fix, f.target], ['twin', 'catpope/s1/c1/s1c1-key-visual.webp']);
});

test('auditAsset: no size limit; a video off the standard – listed, nothing to do here', () => {
	assert.deepEqual(auditAsset(asset([file('a/big.webp', 'image/webp', 30_000_000)])), []);
	assert.equal(auditAsset(asset([file('a/clip.mov', 'video/quicktime', 9_000_000)]))[0].fix, null);
	assert.deepEqual(auditAsset(asset([file('a/clip.mp4', 'video/mp4', 9_000_000)])), []);
});

test('auditAsset: without per-file counts the source stands for the asset', () => {
	const a = asset([file('a/x.png', 'image/png', 10)]);
	a.renditions = a.renditions.map(({ usages: _, ...r }) => r);
	assert.equal(auditAsset(a)[0].fix, 'convert');
});

test('auditAssets: the most used first, then the biggest', () => {
	const list = auditAssets([asset([file('a/small.png', 'image/png', 10, 1)]), asset([file('a/big.png', 'image/png', 5_000, 1)]), asset([file('a/hot.png', 'image/png', 1, 9)])]);
	assert.deepEqual(list.map((f) => f.path), ['a/hot.png', 'a/big.png', 'a/small.png']);
});

test('nextFreeName: -2, -3 before the extension', () => {
	assert.equal(nextFreeName('catpope/items/camera.webp', 2), 'catpope/items/camera-2.webp');
});
