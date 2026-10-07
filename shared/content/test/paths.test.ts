// toonAsset / toonAssetRef / rebaseToonAsset (paths.ts, BACKLOG 10.39): the one resolver of stored media paths
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { rebaseToonAsset, toonAsset, toonAssetRef } from '../src/paths.ts';

test('toonAsset: relative to the toon folder, ../ steps out, a leading / starts at the CDN root', () => {
	assert.equal(toonAsset('catpope', 's1/c1/x.webp'), 'catpope/s1/c1/x.webp');
	assert.equal(toonAsset('toptoy', '../catpope/s1/c2/s1c2-slice-53.webp'), 'catpope/s1/c2/s1c2-slice-53.webp');
	assert.equal(toonAsset('toptoy', '/catpope/s1/x.webp'), 'catpope/s1/x.webp');
	assert.equal(toonAsset('toptoy', '/characters/x.webp'), 'characters/x.webp');
	assert.equal(toonAsset('catpope', './s1//x.webp'), 'catpope/s1/x.webp');
	assert.equal(toonAsset('', 'scutoons/items/a.webp'), 'scutoons/items/a.webp');
	assert.equal(toonAsset('catpope', 'https://cdn.example/x.webp'), 'https://cdn.example/x.webp');
	assert.equal(toonAsset('catpope', ''), '');
	assert.equal(toonAsset('catpope', undefined), '');
});

test('toonAssetRef + rebaseToonAsset: a paste into another toon references the source file, never copies it', () => {
	assert.equal(toonAssetRef('catpope', 'catpope/s1/x.webp'), 's1/x.webp');
	assert.equal(toonAssetRef('toptoy', 'catpope/s1/x.webp'), '../catpope/s1/x.webp');
	assert.equal(rebaseToonAsset('s1/c1/a.webp', 'catpope', 'toptoy'), '../catpope/s1/c1/a.webp');
	assert.equal(rebaseToonAsset('../catpope/s1/c1/a.webp', 'toptoy', 'catpope'), 's1/c1/a.webp', 'back home: relative again');
	assert.equal(rebaseToonAsset('../catpope/a.webp', 'toptoy', 'sandbox'), '../catpope/a.webp');
	assert.equal(rebaseToonAsset('/characters/x.webp', 'toptoy', 'sandbox'), '../characters/x.webp');
	assert.equal(rebaseToonAsset('s1/a.webp', 'catpope', 'catpope'), 's1/a.webp');
});
