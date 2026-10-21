// unused_files (issue 152): the admin's Unused list folded into groups (a tile set as one, else by folder) with totals,
// and the file list paged.
import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import type { AssetUnusedList } from '@scutoons/api';
import { summarizeUnused } from '../src/unused.ts';

const totals = { rows: 3, assets: 4, files: 5, bytes: 1110 };
const list = {
	status: 'success',
	items: [
		{
			id: 'tiles:catpope/map/terrain/plaster/v1',
			slug: 'catpope/map/terrain/plaster/v1',
			kind: 'image',
			all_unused: true,
			files: [
				{ path: 'catpope/map/terrain/plaster/v1/1/0/0.webp', what: 'current', size: 100 },
				{ path: 'catpope/map/terrain/plaster/v1/2/1/0.webp', what: 'current', size: 200 }
			],
			file_count: 2,
			size: 300,
			assets: 2,
			tiles: 2
		},
		{
			id: 'a1',
			slug: 'catpope/s1/c1/s1c1-slice-1',
			kind: 'image',
			all_unused: true,
			files: [
				{ path: 'catpope/s1/c1/s1c1-slice-1.webp', what: 'current', size: 10 },
				{ path: 'catpope/s1/c1/s1c1-slice-1.2x.webp', what: 'current', size: 800 }
			],
			file_count: 2,
			size: 810,
			assets: 1,
			tiles: null
		},
		{
			id: 'a2',
			slug: 'catpope/items/camera',
			kind: 'image',
			all_unused: false,
			files: [{ path: 'catpope/items/old/camera.webp', what: 'old', size: null }],
			file_count: 1,
			size: 0,
			assets: 1,
			tiles: null
		}
	],
	pagination: { total: 3, page: 1, pageSize: 3 },
	totals,
	matching: totals
} as unknown as AssetUnusedList;

test('summarizeUnused: a tile set is one group, other files by folder, biggest first, totals as the server says', () => {
	const s = summarizeUnused(list);
	assert.deepEqual(s.totals, totals);
	assert.equal(s.files, undefined);
	assert.deepEqual(s.groups, [
		{ group: 'catpope/s1/c1', tiles: false, files: 2, bytes: 810, what: { current: 2 }, whole_assets: 2 },
		{ group: 'catpope/map/terrain/plaster/v1', tiles: true, files: 2, bytes: 300, what: { current: 2 }, whole_assets: 2 },
		{ group: 'catpope/items/old', tiles: false, files: 1, bytes: 0, what: { old: 1 }, whole_assets: 0 }
	]);
});

test('summarizeUnused: files sorted by path, paged with offset / limit', () => {
	const s = summarizeUnused(list, { files: true, offset: 1, limit: 2 });
	assert.equal(s.files?.total, 5);
	assert.equal(s.files?.more, true);
	assert.deepEqual(
		s.files?.items.map((f) => f.path),
		['catpope/map/terrain/plaster/v1/1/0/0.webp', 'catpope/map/terrain/plaster/v1/2/1/0.webp']
	);
	assert.deepEqual(s.files?.items[0], { path: 'catpope/map/terrain/plaster/v1/1/0/0.webp', size: 100, what: 'current', group: 'catpope/map/terrain/plaster/v1', whole_asset: true });
	const last = summarizeUnused(list, { files: true, offset: 4 });
	assert.deepEqual([last.files?.items.length, last.files?.more], [1, false]);
});
