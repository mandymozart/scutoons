// `npm test -w scutoons-creator`: the Assignment tab's catalog lists, the core's requests and the rig's draft store
// (lib/editor/rig-assign.ts, #283).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { ApiError } from '@scutoons/api';
import { bundleRequest, coverageRequest, createRigStore, moduleIdsOf, policyIdFor, rigLists, templateRequest, type CatalogEntryLike } from '../src/lib/editor/rig-assign.ts';

const entry = (id: string, content: unknown, state: Partial<CatalogEntryLike> = {}): CatalogEntryLike => ({ id, live: true, draft: false, removed: false, content, ...state });

const LISTS = rigLists(
	[entry('drone', { id: 'drone', kind: 'pad', name: { en: 'Drone', de: 'Bordun', x: 3 } }), entry('cathedral_voice', { id: 'cathedral_voice', kind: 'voice' }), entry('gone', { id: 'gone' }, { removed: true })],
	[entry('drone.base', { id: 'drone.base', instrument: 'drone' }), entry('broken', null)],
	[entry('character.catpope', {}, { draft: true, published: 3 } as Partial<CatalogEntryLike>), entry('character.boy', {}, { live: false, draft: true })]
);

test('the lists: removed and unreadable entries gone, the pickers by id, the documents as stored, the rigs’ states only', () => {
	assert.deepEqual(LISTS.instruments, [{ id: 'cathedral_voice', kind: 'voice', name: undefined }, { id: 'drone', kind: 'pad', name: { en: 'Drone', de: 'Bordun' } }]);
	assert.deepEqual(LISTS.presets, [{ id: 'drone.base', instrument: 'drone', name: undefined }]);
	assert.equal(LISTS.instrumentDocs.length, 2);
	assert.deepEqual(LISTS.rigs, [{ id: 'character.catpope', live: true, draft: true, removed: false }, { id: 'character.boy', live: false, draft: true, removed: false }]);
});

test('the core’s requests: the characters with their personas’ ids; the rig with the catalog’s documents', () => {
	const req = JSON.parse(coverageRequest([{ id: 'catpope', personas: [{ id: 'regular' }, { id: 'dj' }] }, { id: 'boy' }], LISTS));
	assert.deepEqual(req.characters, [{ id: 'catpope', personas: ['regular', 'dj'] }, { id: 'boy', personas: [] }]);
	assert.equal(req.rigs.length, 2);
	const rig = { subject: 'character.boy', track: [] };
	assert.deepEqual(JSON.parse(bundleRequest(rig, LISTS)), { rig, instruments: LISTS.instrumentDocs, presets: LISTS.presetDocs, modules: [] });
});

test('a code rig: the module documents go along (#412), removed ones gone; the template asks the same catalog', () => {
	const lists = rigLists([], [entry('drone.base', { id: 'drone.base', instrument: 'drone' })], [], [entry('catpope_rig', { id: 'catpope_rig', files: {}, descriptor: { name: 'catpope_rig' } }), entry('gone', { id: 'gone' }, { removed: true })]);
	const rig = { subject: 'character.catpope', track: [], policy: { module: 'catpope_rig' } };
	assert.deepEqual(JSON.parse(bundleRequest(rig, lists)).modules, [{ id: 'catpope_rig', files: {}, descriptor: { name: 'catpope_rig' } }]);
	assert.deepEqual(JSON.parse(templateRequest('mine', rig, lists)), { module: 'mine', rig, instruments: [], presets: lists.presetDocs });
	assert.deepEqual(moduleIdsOf(lists), ['catpope_rig']);
});

test('the suggested policy id is a module id', () => {
	assert.equal(policyIdFor('character.catpope'), 'catpope_rig');
	assert.equal(policyIdFor('character.catpope.dj'), 'catpope_dj_rig');
	assert.equal(policyIdFor('item.incense_burner'), 'incense_burner_rig');
	assert.match(policyIdFor(`character.${'Long-name'.repeat(10)}`), /^[a-z0-9][a-z0-9_-]{0,31}$/);
});

test('the store: a missing rig is null; a save builds on the loaded version; publish publishes the saved one', async () => {
	const calls: string[] = [];
	let stored: { id: number; content: unknown } | null = null;
	const store = createRigStore({
		getDraft: (async (key: string) => {
			calls.push(`get ${key}`);
			if (!stored) throw new ApiError(404, 'not_found', 'none');
			return { version: { id: stored.id }, content: stored.content };
		}) as never,
		putDraft: (async (key: string, content: unknown, base: number | null) => {
			calls.push(`put ${key} on ${base}`);
			stored = { id: (stored?.id ?? 0) + 1, content };
			return { version: { id: stored.id } };
		}) as never,
		publishVersion: (async (key: string, version: number) => {
			calls.push(`publish ${key} v${version}`);
			return {};
		}) as never
	});
	assert.deepEqual(await store.load('character.boy'), { ok: true, rig: null });
	assert.deepEqual(await store.publish('character.boy'), { ok: false, message: 'rig:character.boy' }, 'nothing saved: nothing to publish');
	const rig = { subject: 'character.boy', track: [{ id: 'drone', instrument: 'drone' }] };
	assert.deepEqual(await store.save('character.boy', rig), { ok: true });
	assert.deepEqual(await store.save('character.boy', rig), { ok: true });
	assert.deepEqual(await store.publish('character.boy'), { ok: true });
	assert.deepEqual(await store.load('character.boy'), { ok: true, rig });
	assert.deepEqual(calls, ['get rig:character.boy', 'put rig:character.boy on null', 'put rig:character.boy on 1', 'publish rig:character.boy v2', 'get rig:character.boy']);
});
