// Wardrobe (agents/ITEMS.md "Wardrobe"): the bodies, the basic dress, the catalog validation – against the
// test fixtures (server/tests/fixtures: the catalog and the character registry of 2026-10-04).   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
	avatarBodies,
	basicDress,
	unlockedBodies,
	validateCatalog,
	catalogFromSeed,
	type Catalog,
	type CharacterRegistry
} from '../src/index.ts';

const json = (rel: string) => JSON.parse(readFileSync(new URL(rel, import.meta.url), 'utf8'));
const catalog: Catalog = catalogFromSeed(json('../../../server/tests/fixtures/catalog/items.json'));
const registry: CharacterRegistry = json('../../../server/tests/fixtures/content/scutoons/_/characters.json');
const item = (id: string) => catalog.items.find((i) => i.id === id)!;

test('wardrobe: the catalog validates against the registry', () => {
	const problems = validateCatalog(catalog, registry);
	assert.deepEqual(problems.filter((p) => p.level !== 'info'), []);
	for (const id of ['scutoons_tshirt', 'scutoons_swim_pants', 'scutoons_shorts', 'scutoons_sandals']) {
		assert.equal(item(id).starter, true, `${id} is a starter item`);
		assert.equal(item(id).category, 'wearable');
	}
});

test('wardrobe: validation errors', () => {
	const bad = structuredClone(catalog);
	bad.items.find((i) => i.id === 'scutoons_shorts')!.wear = { 'toptoy/torso-left': 'characters/toptoy/wear/x.webp', 'Cat Pope': 'x.webp', yellowmouse: 'x.gif' };
	bad.items.find((i) => i.id === 'camera')!.wear = { catpope: 'characters/catpope/wear/camera.webp' };
	const errors = validateCatalog(bad, registry).filter((p) => p.level === 'error').map((p) => p.message);
	assert.ok(errors.some((m) => m.includes('torso slot, the item is worn on legs')), errors.join('\n'));
	assert.ok(errors.some((m) => m.includes('must be <character> or <character>/<slot instance>')));
	assert.ok(errors.some((m) => m.includes('.webp/.png')));
	assert.ok(errors.some((m) => m.includes('only wearables')));
	const twinsAsOne = structuredClone(catalog);
	twinsAsOne.items.find((i) => i.id === 'scutoons_tshirt')!.wear = { toptoy: 'characters/toptoy/wear/scutoons_tshirt.webp' };
	assert.ok(validateCatalog(twinsAsOne, registry).some((p) => p.level === 'warning' && p.message.includes('several torso slots')));
	// legs may be empty (owner 2026-10-05: no naked A-pose) – no starter for legs is fine
	const noLegs = structuredClone(catalog);
	for (const i of noLegs.items) if (i.slot === 'legs') i.starter = false;
	assert.ok(!validateCatalog(noLegs, registry).some((p) => p.message.includes('"legs" slot')));
});

test('items: a 3D model is a .glb path, on the item or a variant (FEATURES 10.102)', () => {
	const ok = structuredClone(catalog);
	const camera = ok.items.find((i) => i.id === 'camera')!;
	camera.model = 'catpope/items/camera.model.glb';
	camera.variants = [{ id: 'red', name: { de: 'Rot', en: 'Red' }, model: 'catpope/items/camera_red.model.glb' }];
	assert.deepEqual(validateCatalog(ok, registry).filter((p) => p.level === 'error' && p.message.includes('model')), []);
	camera.model = 'catpope/items/camera.gltf';
	camera.variants[0].model = 'catpope/items/camera_red.png';
	const errors = validateCatalog(ok, registry).filter((p) => p.level === 'error').map((p) => p.message);
	assert.ok(errors.includes('camera: model must be a .glb path'), errors.join(' | '));
	assert.ok(errors.includes('camera/red: model must be a .glb path'), errors.join(' | '));
});

test('wardrobe: the basic dress per body (shorts where they have an overlay, else the swim pants)', () => {
	const starters = catalog.items.filter((i) => i.starter && i.slot);
	assert.deepEqual(basicDress(starters, 'catpope').map((i) => i.id), ['scutoons_tshirt', 'scutoons_shorts', 'scutoons_sandals']);
	assert.deepEqual(basicDress(starters, 'yellowmouse').map((i) => i.id), ['scutoons_tshirt', 'scutoons_swim_pants', 'scutoons_sandals']);
	assert.deepEqual(basicDress(starters, 'toptoy').map((i) => i.id), ['scutoons_tshirt', 'scutoons_swim_pants', 'scutoons_sandals']);
	assert.equal(item('scutoons_swim_pants').default_worn, undefined, 'the swim pants are the safety, not the basic dress');
	const bad = structuredClone(catalog);
	bad.items.find((i) => i.id === 'camera')!.default_worn = true;
	assert.ok(validateCatalog(bad, registry).some((p) => p.level === 'error' && p.message.includes('default_worn')));
});

test('wardrobe: the bodies are the looks with an A-pose (owner 2026-10-12; the naked look is gone)', () => {
	const all = unlockedBodies(avatarBodies(registry), [{ character: 'catpope', looks: [] }, { character: 'yellowmouse', looks: [] }]);
	assert.deepEqual(all.map((b) => `${b.character}/${b.persona}/${b.look}`), ['catpope/regular/default', 'yellowmouse/regular/default']);
	assert.equal(all[0].path, 'characters/catpope/catpope_apose-swim.webp');
	// no image anchors any more: a slot instance is id, type (and a label for twins)
	assert.deepEqual(all[0].slots[0], { id: 'head', slot: 'head' });
	const toptoy = avatarBodies(registry).find((b) => b.character === 'toptoy')!;
	assert.equal(toptoy.slots.length, 18);
	assert.deepEqual(Object.keys(toptoy.slots[0]).sort(), ['id', 'label', 'slot']);
});
