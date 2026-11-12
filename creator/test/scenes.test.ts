// The scene browser's logic (#490): rows, filters (toon incl. "no toon", one search, Draft / Published), the facts in
// words and what a Publish answered (422 problems, 503 unreachable).
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { ApiError, type SceneListEntry } from '@scutoons/api';
import { entityLabel } from '../../shared/ui/src/entity-label.ts';
import { NO_TOON, fillWords, filterRows, inPublishBatch, labelIn, publishOutcome, rowHint, sceneHref, sceneRows, sceneTitle, sceneToons } from '../src/lib/editor/scenes.ts';
import { placeHref } from '../src/lib/editor/place-href.ts';

const nave = {
	format: 2,
	id: 'nave',
	version: 3,
	toons: ['catpope'],
	holder: [{ id: 'catpope', ref: 'character:catpope', rig: { ref: 'rig:character.catpope' } }],
	stage: { name: 'Nave' },
	node: [{ id: 'space', kind: 'space', set: 'cathedral', spot: 'spot_altar' }]
};
const stage = {
	format: 2,
	id: 'stage',
	toons: [],
	stage: { name: 'Stage' },
	location: { id: 'where', fill: { kind: ['location'] }, label: { en: 'where', de: 'wo' } },
	holder: [
		{ id: 'left', spot: 'spot_left', fill: { kind: ['character'] }, enter: 'join' },
		{ id: 'crowd', fill: { kind: ['item'], ref: ['item:incense'] }, take: 'many', arrange: { kind: 'cluster', spacing: 0.5 }, max: 8 }
	]
};
const crypt = { format: 2, id: 'crypt', toons: ['catpope', 'toptoy'], location: 'location:crypt', holder: [] };
const e = (doc: { id: string }, latest: number, published: number | null): SceneListEntry => ({ key: `scene:${doc.id}`, content: doc as never, latest, published, same_as_published: false });
const rows = sceneRows([e(stage, 4, null), e(nave, 6, 5), e(crypt, 9, 9), { key: 'scene:catpope:2:scene-14:de', content: null, latest: 1, published: 1 }]);

test('rows: deck scenes by name, with their state and facts (an old chapter group key is no scene)', () => {
	assert.deepEqual(rows.map((r) => [r.id, r.state]), [['crypt', 'published'], ['nave', 'draft'], ['stage', 'unpublished']]);
	assert.equal(sceneTitle(rows[0].facts), 'crypt', 'no stage name: the id');
	const n = rows[1].facts;
	assert.deepEqual(n.where, { kind: 'set', set: 'cathedral' });
	assert.equal(n.spot, 'spot_altar');
	assert.equal(n.stamp, 3);
	assert.deepEqual(n.holders[0], { id: 'catpope', ref: 'character:catpope', fill: null, many: null, spot: null, enter: 'start', rig: 'rig:character.catpope', label: null });
	const zone = rows[2].facts.holders[1];
	assert.deepEqual(zone.many, { arrange: 'cluster', spacing: 0.5, max: 8 });
	assert.deepEqual(zone.fill, { kind: ['item'], ref: ['item:incense'], parent: null });
	assert.deepEqual(rows[2].facts.where, { kind: 'placeholder', id: 'where', fill: ['location'], label: { en: 'where', de: 'wo' }, preview: null });
});

test('filters: a toon, no toon, the state, one search', () => {
	const ids = (filter: Partial<Parameters<typeof filterRows>[1]>) => filterRows(rows, { query: '', toon: '', state: 'all', ...filter }).map((r) => r.id);
	assert.deepEqual(ids({}), ['crypt', 'nave', 'stage']);
	assert.deepEqual(ids({ toon: 'toptoy' }), ['crypt']);
	assert.deepEqual(ids({ toon: NO_TOON }), ['stage']);
	assert.deepEqual(ids({ state: 'published' }), ['crypt']);
	assert.deepEqual(ids({ state: 'draft' }), ['nave', 'stage'], 'Draft = not published yet, or a newer draft waits');
	assert.deepEqual(ids({ query: 'cathedral' }), ['nave'], 'the set');
	assert.deepEqual(ids({ query: 'incense' }), ['stage'], 'a placeholder\'s filter');
	assert.deepEqual(ids({ query: 'CATPOPE' }), ['nave'], 'a filler, case-blind');
	assert.deepEqual(ids({ query: 'crypt', toon: 'catpope' }), ['crypt']);
});

test('the toon filter offers the scenes\' toons and "no toon"', () => {
	assert.deepEqual(sceneToons(rows), { toons: ['catpope', 'toptoy'], noToon: true });
	assert.deepEqual(sceneToons(rows.slice(0, 2)), { toons: ['catpope', 'toptoy'], noToon: false });
});

test('the row\'s hint and the facts in words', () => {
	assert.equal(rowHint(rows[1].facts), 'cathedral · spot_altar · catpope');
	assert.equal(rowHint(rows[2].facts), '‹where› · ‹left› · ‹crowd›');
	assert.equal(rowHint(rows[0].facts, (ref) => ref.toUpperCase()), 'LOCATION:CRYPT');
	assert.equal(fillWords({ kind: ['character', 'persona'], ref: [], parent: 'character:catpope' }, { of: 'of', below: 'below' }), 'character | persona below character:catpope');
	assert.equal(fillWords({ kind: ['item'], ref: ['item:incense'], parent: null }, { of: 'aus', below: 'unter' }), 'item aus item:incense');
	assert.equal(labelIn({ en: 'where', de: 'wo' }, 'de'), 'wo');
	assert.equal(labelIn({ en: 'where' }, 'de'), 'where');
	assert.equal(labelIn(null, 'de'), null);
	assert.equal(sceneHref('avatar-home'), '/scenes/avatar-home');
});

test('Publish: the core\'s problems, the check not reachable, other errors', () => {
	assert.deepEqual(publishOutcome(null), { kind: 'published' });
	assert.deepEqual(publishOutcome(new ApiError(422, 'invalid_scene', 'problems', { problems: ['holder catpope: no rig', 'set: unknown'] })), { kind: 'problems', problems: ['holder catpope: no rig', 'set: unknown'] });
	assert.deepEqual(publishOutcome(new ApiError(422, 'invalid_content', 'Not a scene document.', {})), { kind: 'problems', problems: ['Not a scene document.'] });
	assert.deepEqual(publishOutcome(new ApiError(503, 'build_unavailable', 'down')), { kind: 'unreachable' });
	assert.deepEqual(publishOutcome(new ApiError(403, 'review_required', 'A review first.')), { kind: 'error', message: 'A review first.' });
	assert.deepEqual(publishOutcome(new Error('x')), { kind: 'error', message: 'x' });
});

test('a scene with a draft: not in the batch Publish, named and linked to its own page (#493)', () => {
	assert.equal(inPublishBatch('scene:nave'), false);
	assert.equal(inPublishBatch('scene:avatar-home'), false);
	for (const key of ['chapter:catpope:2', 'place:cathedral', 'character:catpope', 'item:camera', 'colorations', 'toon:catpope']) assert.equal(inPublishBatch(key), true, key);
	assert.equal(placeHref('scene:avatar-home'), '/scenes/avatar-home');
	assert.equal(placeHref('scene:nave'), sceneHref('nave'));
	assert.equal(entityLabel('scene:nave'), 'Scene nave');
	assert.equal(entityLabel('scene:nave', { scene: (id) => `Szene: ${id}` }), 'Szene: nave');
});
