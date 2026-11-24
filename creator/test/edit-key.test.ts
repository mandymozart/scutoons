// `npm test -w scutoons-creator`: the editor's address (lib/editor/edit-key.ts, EDITOR.md §4).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { editHref, parseEditKey, rigHome, selectedModule, workspaceOf } from '../src/lib/editor/edit-key.ts';

test('a rig key names its character; a module key its id; the rest is not editable yet', () => {
	assert.deepEqual(parseEditKey('rig:character.catpope'), { kind: 'rig', key: 'rig:character.catpope', subject: 'character.catpope', character: 'catpope' });
	assert.equal(parseEditKey('rig:character.catpope.dj')?.kind === 'rig' && (parseEditKey('rig:character.catpope.dj') as { character: string }).character, 'catpope');
	assert.deepEqual(parseEditKey('module:walk'), { kind: 'module', key: 'module:walk', id: 'walk' });
	assert.deepEqual(parseEditKey('rig:item.drum-machine'), { kind: 'rig', key: 'rig:item.drum-machine', subject: 'item.drum-machine', character: '' }, 'an item\'s rig names no character');
	for (const bad of ['', 'walk', 'module:', 'module:Walk', 'module:a b', 'rig:item.', 'rig:item.a.b', 'rig:character.', 'chapter:catpope:1', ':x']) assert.equal(parseEditKey(bad), null, bad);
});

test('the workspace defaults to sound for rigs and modules', () => {
	const rig = parseEditKey('rig:character.catpope')!;
	assert.equal(workspaceOf(rig, null), 'sound');
	assert.equal(workspaceOf(rig, 'sound'), 'sound');
	assert.equal(workspaceOf(rig, 'scenes'), 'sound');
	assert.equal(workspaceOf(parseEditKey('module:walk')!, null), 'sound');
});

test('the Rust tab shows the module itself, or a rig page\'s selection', () => {
	assert.equal(selectedModule(parseEditKey('module:walk')!, 'module:other'), 'walk');
	assert.equal(selectedModule(parseEditKey('rig:character.catpope')!, 'module:walk'), 'walk');
	assert.equal(selectedModule(parseEditKey('rig:character.catpope')!, null), null);
	assert.equal(selectedModule(parseEditKey('rig:character.catpope')!, 'rig:character.x'), null);
});

test('editHref builds the address', () => {
	assert.equal(editHref('rig:character.catpope', { w: 'sound' }), '/edit/rig:character.catpope?w=sound');
	assert.equal(editHref('rig:character.catpope', { w: 'sound', sel: 'module:walk' }), '/edit/rig:character.catpope?w=sound&sel=module%3Awalk');
	assert.equal(editHref('module:walk'), '/edit/module:walk');
});

test("a character's rig lands on the character's Rig tab (lap 2 C4); an item's rig and a module have no such page", () => {
	assert.equal(rigHome(parseEditKey('rig:character.catpope')!), '/characters?id=catpope&tab=rig');
	assert.equal(rigHome(parseEditKey('rig:character.catpope.dj')!), '/characters?id=catpope&persona=dj&tab=rig');
	assert.equal(rigHome(parseEditKey('rig:item.drum-machine')!), null);
	assert.equal(rigHome(parseEditKey('module:walk')!), null);
});
