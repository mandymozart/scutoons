// Where a stored file's row leads (lib/editor/place-href.ts – the dashboard's "Not live yet", the review list; #493) and
// what it is called (shared/ui entity-label.ts).
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { entityLabel } from '../../shared/ui/src/entity-label.ts';
import { placeHref } from '../src/lib/editor/place-href.ts';

test('every kind of stored file leads to the page that edits it', () => {
	assert.equal(placeHref('chapter:catpope:2'), '/catpope/chapter/2');
	assert.equal(placeHref('season:catpope:1'), '/catpope/season/1');
	assert.equal(placeHref('storyboard:catpope:3'), '/catpope/chapter/3/storyboard');
	assert.equal(placeHref('chapter-script:catpope:2'), '/catpope/chapter/2/script');
	assert.equal(placeHref('item:camera'), '/catalog/items?id=camera');
	assert.equal(placeHref('place:cathedral'), '/map');
	assert.equal(placeHref('character:catpope'), '/characters');
	assert.equal(placeHref('scene:avatar-home'), '/scenes/avatar-home');
	assert.equal(placeHref('module:walk'), '/edit/module:walk');
	assert.equal(placeHref('rig:character.catpope'), '/edit/rig:character.catpope');
	assert.equal(placeHref('rig:character.catpope.dj'), '/edit/rig:character.catpope.dj');
});

test('a module, a rig and the other catalog documents are named with their id, never by the bare kind', () => {
	assert.equal(entityLabel('module:walk'), 'Module walk');
	assert.equal(entityLabel('rig:character.catpope'), 'Rig character.catpope');
	assert.equal(entityLabel('instrument:organ'), 'Instrument organ');
	assert.equal(entityLabel('home-scene:cave'), 'Home-scene cave');
	assert.equal(entityLabel('item:camera'), 'Item camera');
	assert.equal(entityLabel('module:walk', { catalog: (kind, id) => `${kind === 'module' ? 'Modul' : kind} ${id}` }), 'Modul walk');
});
