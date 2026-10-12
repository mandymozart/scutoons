// Unit tests of the optimisation tool's audit (media-audit.ts – BACKLOG 10.64).
//   npm test -w @scutoons/media
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { auditAsset, auditAssets, fixFindings, nextFreeName, type AuditAsset, type FixDeps, type StandardFinding } from '../src/media-audit.ts';

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
	assert.deepEqual([f.fix, f.target, f.targetSize], ['twin', 'catpope/s1/c1/s1c1-key-visual.webp', 200_000]);
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

const finding = (path: string, fix: StandardFinding['fix'], target?: string): StandardFinding => ({ asset: path, slug: path, kind: 'image', path, size: 1000, usages: 1, advice: 'PNG → WebP', fix, target, targetSize: fix === 'twin' ? 300 : undefined });
const relinkResult = (map: Record<string, string>) => ({ status: 'success' as const, map, content: [], catalog: [], manual: [], usages: Object.keys(map).length });

test('fixFindings: converts to new files, takes a free name when one is taken, relinks everything in ONE pass', async () => {
	const uploaded: string[] = [];
	const relinks: Record<string, string>[] = [];
	const deps: FixDeps = {
		read: async (path) => new File(['x'.repeat(1000)], path, { type: 'image/png' }),
		encode: async () => new Blob(['y'.repeat(200)]),
		upload: async (_blob, path) => {
			if (path === 'a/taken.webp') throw Object.assign(new Error('exists'), { code: 'exists' });
			uploaded.push(path);
		},
		relink: async (map) => (relinks.push(map), relinkResult(map))
	};
	const report = await fixFindings([finding('a/one.png', 'convert', 'a/one.webp'), finding('a/taken.png', 'convert', 'a/taken.webp'), finding('a/twin.jpg', 'twin', 'a/twin.webp')], deps);
	assert.deepEqual(uploaded, ['a/one.webp', 'a/taken-2.webp']);
	assert.deepEqual(relinks, [{ 'a/one.png': 'a/one.webp', 'a/taken.png': 'a/taken-2.webp', 'a/twin.jpg': 'a/twin.webp' }]);
	assert.deepEqual(report.rows.map((r) => [r.result, r.target, r.after]), [['converted', 'a/one.webp', 200], ['converted', 'a/taken-2.webp', 200], ['relinked', 'a/twin.webp', 300]]);
	assert.equal(report.relink?.usages, 3);
});

test('fixFindings: a failed file is reported and left out of the relink; a failed relink is reported', async () => {
	const deps: FixDeps = {
		read: async (path) => {
			if (path === 'a/broken.png') throw new Error('404');
			return new File(['x'], path, { type: 'image/png' });
		},
		encode: async () => new Blob(['y']),
		upload: async () => {},
		relink: async () => {
			throw new Error('missing_target');
		}
	};
	const report = await fixFindings([finding('a/broken.png', 'convert', 'a/broken.webp'), finding('a/ok.png', 'convert', 'a/ok.webp')], deps);
	assert.deepEqual(report.rows.map((r) => [r.path, r.result, r.error]), [['a/broken.png', 'failed', '404'], ['a/ok.png', 'converted', undefined]]);
	assert.deepEqual([report.relink, report.error], [null, 'missing_target']);
});

test('fixFindings: nothing fixable – no relink at all', async () => {
	let called = false;
	const report = await fixFindings([finding('a/clip.mov', null)], { read: async () => new File([], ''), encode: async () => new Blob(), upload: async () => {}, relink: async (m) => ((called = true), relinkResult(m)) });
	assert.deepEqual([report.rows, report.relink, called], [[], null, false]);
});
