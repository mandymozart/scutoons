// The scene browser's logic (#490): rows, the facts in words and what a Publish answered (422 problems, 503 unreachable).
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { type SceneListEntry } from '@scutoons/api';
import { entityLabel } from '../../shared/ui/src/entity-label.ts';
import { fillWords, labelIn, publishOutcome, sceneHref, sceneRows, sceneTitle } from '../src/lib/editor/scenes.ts';
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

test('the facts in words', () => {
	assert.equal(fillWords({ kind: ['character', 'persona'], ref: [], parent: 'character:catpope' }, { of: 'of', below: 'below' }), 'character | persona below character:catpope');
	assert.equal(fillWords({ kind: ['item'], ref: ['item:incense'], parent: null }, { of: 'aus', below: 'unter' }), 'item aus item:incense');
	assert.equal(labelIn({ en: 'where', de: 'wo' }, 'de'), 'wo');
	assert.equal(labelIn({ en: 'where' }, 'de'), 'where');
	assert.equal(labelIn(null, 'de'), null);
	assert.equal(sceneHref('avatar-home'), '/scenes/avatar-home');
});

test('Publish: the scene\'s line of the one Publish – the core\'s problems, the check not reachable, a skip', () => {
	const key = 'scene:nave';
	const refused = (code: string, reason: string) => publishOutcome(key, { published: ['rig:character.catpope'], refused: [{ key, code, reason }] });
	assert.deepEqual(publishOutcome(key, { published: ['rig:character.catpope', key], refused: [] }), { kind: 'published' });
	assert.equal(publishOutcome(key, { published: [], refused: [] }), null, 'cancelled or nothing to publish');
	assert.deepEqual(refused('invalid_scene', 'holder catpope: no rig; set: unknown'), { kind: 'problems', problems: ['holder catpope: no rig', 'set: unknown'] });
	assert.deepEqual(refused('no_build_service', 'not set up'), { kind: 'unreachable' });
	assert.deepEqual(refused('dependency', 'needs rig character.catpope, which was refused'), { kind: 'error', message: 'needs rig character.catpope, which was refused' });
});

test('a scene with a draft: named and linked to its own page (#493)', () => {
	assert.equal(placeHref('scene:avatar-home'), '/scenes/avatar-home');
	assert.equal(placeHref('scene:nave'), sceneHref('nave'));
	assert.equal(entityLabel('scene:nave'), 'Scene nave');
	assert.equal(entityLabel('scene:nave', { scene: (id) => `Szene: ${id}` }), 'Szene: nave');
});
