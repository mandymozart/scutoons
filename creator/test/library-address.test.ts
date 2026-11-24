// The Library's file address and the core's families (lib/editor/library-address.ts).
import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { AssetFamily } from '@scutoons/media';
import { familiesByPath, familyAddress, pathOf } from '../src/lib/editor/library-address.ts';

const version = (path: string, label: string, asset: string) => ({ path, label, asset }) as AssetFamily['versions'][number];
const trailer = {
	key: 'catpope/s1/s1-trailer-loop',
	kind: 'video',
	main: 'catpope/s1/s1-trailer-loop.720p.mp4',
	assets: ['261', '262'],
	versions: [
		version('catpope/s1/s1-trailer-loop.480p.mp4', '480p', '261'),
		version('catpope/s1/s1-trailer-loop.720p.mp4', '720p', '261'),
		version('catpope/s1/s1-trailer-loop.1920w.mp4', '1920w', '261'),
		version('catpope/s1/s1-trailer-loop.poster.webp', 'poster', '262')
	]
} as AssetFamily;
const registered = new Map([
	['261', 'catpope/s1/s1-trailer-loop.1920w.mp4'],
	['262', 'catpope/s1/s1-trailer-loop.poster.webp']
]);

test('a file address names its path', () => {
	assert.equal(pathOf('video:catpope/s1/a.720p.mp4'), 'catpope/s1/a.720p.mp4');
	assert.equal(pathOf('image:x/y:z.webp'), 'x/y:z.webp');
});

test('any file of a family finds it – the registered one, a level, a poster filed as its own asset', () => {
	const by = familiesByPath([trailer]);
	assert.equal(by.get(pathOf('video:catpope/s1/s1-trailer-loop.1920w.mp4')), trailer);
	assert.equal(by.get('catpope/s1/s1-trailer-loop.720p.mp4'), trailer);
	assert.equal(by.get(pathOf('image:catpope/s1/s1-trailer-loop.poster.webp')), trailer);
	assert.equal(by.get('catpope/s1/other.mp4'), undefined);
});

test('a family’s address is the server’s: the registered file of the asset holding its main', () => {
	assert.equal(familyAddress(trailer, registered), 'video:catpope/s1/s1-trailer-loop.1920w.mp4');
	assert.equal(familyAddress(trailer, new Map()), 'video:catpope/s1/s1-trailer-loop.720p.mp4');
});
