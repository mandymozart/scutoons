// A shot hands over (src/storyboard/handover.ts): its asset address, what it was made into, the deck's shot address, the
// sources' deck in Unreal's place, the slice placement – and the generated schema holds the same patterns.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
	SHOT_ASSET, SHOT_MADE, fileShotAsset, readShotAddress, readShotAsset, readSource, shotAddress, shotRows, sliceOfShot,
	sliceSpotFor, validate, type Shot, type StoryboardDoc
} from '../src/storyboard/index.ts';

const shot = (extra: Partial<Shot> = {}): Shot => ({
	id: 'sb-1-1', format: 'viewport', aspect: '1:1.6', view: 'real', action: { de: 'PLF steht im Schnee.' }, source: { type: 'schematic', want: 'photo' }, ...extra
});
const scene = (shots: Shot[]): StoryboardDoc => ({
	storyboard: 2, toon: 'catpope', chapter: 3, source: { scene: 1 }, light: { key: 'top-left', source: 'sun' },
	groups: [{ label: 'scene-1', renderer: { type: 'vertical' }, shots }]
});
const errors = (s: Shot) => validate(scene([s]), 'scene-1').errors;

test('asset: a file of the kinds a slot takes or a generator, read back', () => {
	assert.deepEqual(readShotAsset('image:catpope/s1/c3/a.webp'), { at: 'file', kind: 'image', path: 'catpope/s1/c3/a.webp', address: 'image:catpope/s1/c3/a.webp' });
	assert.deepEqual(readShotAsset('generator:luma'), { at: 'generator', id: 'luma', address: 'generator:luma' });
	for (const bad of ['model:a.glb', 'image:', 'image:a b.png', 'generator:Luma', 'a.webp', 7, null]) assert.equal(readShotAsset(bad), null, String(bad));
	assert.equal(fileShotAsset('video', 'cat/t.mp4'), 'video:cat/t.mp4');
	assert.equal(fileShotAsset('audio', 'cat/t.mp3'), null);
});

test('validate: the asset and the made pointers, old storyboards unchanged', () => {
	assert.deepEqual(errors(shot()), []);
	assert.deepEqual(errors(shot({ asset: 'generator:runway', made: { deck: 'scene:sb-catpope-3-1-1', xr: 'experience:poster-catpope' } })), []);
	assert.deepEqual(errors(shot({ asset: 'catpope/a.webp' })), ['scene-1 sb-1-1: asset "catpope/a.webp" – a Library address: image:<path>, video:<path> or generator:<id>']);
	assert.deepEqual(errors(shot({ made: { deck: 'experience:x' } })), ['scene-1 sb-1-1: made.deck "experience:x" – scene:<id>']);
	assert.deepEqual(errors(shot({ made: { slice: 'x' } as never })), ['scene-1 sb-1-1: made.slice – deck | xr']);
});

test('sources: the deck in Unreal\'s place; an old "unreal" reads as deck', () => {
	assert.equal(readSource('deck'), 'deck');
	assert.equal(readSource('unreal'), 'deck');
	assert.equal(readSource('toString'), undefined);
	assert.deepEqual(errors(shot({ source: { want: 'unreal' } })), []);
	const tall = (want: string) => validate(scene([shot({ format: 'vertical', aspect: '1:8', source: { want: want as never } })]), 'scene-1').warnings.filter((w) => w.includes('beyond what we can generate'));
	assert.equal(tall('unreal').length, 0);
	assert.equal(tall('deck').length, 0);
	assert.match(tall('photo')[0], /source\.want deck \/ photo stitch/);
	assert.equal(shotRows([scene([shot({ source: { want: 'unreal' } })])])[0].want, 'deck');
});

test('the shot\'s own address for the deck', () => {
	assert.equal(shotAddress('catpope', 3, 'sb-25-1'), 'shot:catpope:3:sb-25-1');
	assert.deepEqual(readShotAddress('shot:catpope:3:sb-25-1'), { toon: 'catpope', chapter: 3, shot: 'sb-25-1' });
	assert.deepEqual(readShotAddress('shot:catpope:0:sb-1-1'), { toon: 'catpope', chapter: 0, shot: 'sb-1-1' }, 'a prologue');
	assert.equal(readShotAddress('shot:catpope:03:sb-25-1'), null);
	assert.equal(readShotAddress('scene:x'), null);
});

test('slices: the one cut from a shot, and where a new one goes', () => {
	const chapter = { groups: [{ slices: [{ shot: 'sb-1-1' }, { shot: 'sb-1-1' }, {}] }, { slices: [{ shot: 'sb-1-4' }] }] };
	const order = ['sb-1-1', 'sb-1-2', 'sb-1-3', 'sb-1-4', 'sb-1-5'];
	assert.deepEqual(sliceOfShot(chapter, 'sb-1-4'), { group: 1, slice: 0 });
	assert.equal(sliceOfShot(chapter, 'sb-1-2'), null);
	assert.deepEqual(sliceSpotFor(chapter, order, 'sb-1-2'), { group: 0, slice: 2 }, 'after the last slice of the shot before');
	assert.deepEqual(sliceSpotFor({ groups: [{ slices: [{}] }, { slices: [{ shot: 'sb-1-4' }] }] }, order, 'sb-1-2'), { group: 1, slice: 0 }, 'before the next shot\'s');
	assert.deepEqual(sliceSpotFor(chapter, order, 'sb-1-5'), { group: 1, slice: 1 });
	assert.deepEqual(sliceSpotFor({ groups: [{ slices: [{}] }] }, order, 'sb-9-9'), { group: 0, slice: 1 }, 'not in the order: the end');
	const scenes = { groups: [{ id: 'scene-1', slices: [{}, {}] }, { id: 'scene-2', slices: [{}] }] };
	assert.deepEqual(sliceSpotFor(scenes, order, 'sb-1-2', 'scene-1'), { group: 0, slice: 2 }, 'no shot has a slice: the end of its scene\'s group');
	assert.deepEqual(sliceSpotFor(scenes, order, 'sb-1-2', 'scene-9'), { group: 1, slice: 1 });
	assert.equal(sliceSpotFor({ groups: [] }, order, 'sb-1-1'), null);
});

test('the generated schema holds the patterns handover.ts checks', () => {
	const schema = JSON.parse(readFileSync(new URL('../schema/storyboard.schema.json', import.meta.url), 'utf8'));
	const defs = schema.definitions;
	assert.equal(defs.Shot.properties.asset.pattern, SHOT_ASSET.source);
	assert.equal(defs.ShotMade.properties.deck.pattern, SHOT_MADE.deck.source);
	assert.equal(defs.ShotMade.properties.xr.pattern, SHOT_MADE.xr.source);
});
