// Wardrobe (agents/ITEMS.md "Wardrobe"): overlays, the base image, the catalog validation – against the
// test fixtures (server/tests/fixtures: the catalog and the character registry of 2026-10-04).   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
	avatarBodies,
	basicDress,
	missingWearOverlays,
	unlockedBodies,
	validateCatalog,
	catalogFromSeed,
	wardrobeBase,
	wearConventionPath,
	wearOverlay,
	WEAR_ORDER,
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

test('wardrobe: overlays per body and slot instance, convention paths', () => {
	assert.equal(wearOverlay(item('scutoons_swim_pants'), 'catpope', 'legs'), 'characters/catpope/wear/scutoons_swim_pants.webp');
	assert.equal(wearOverlay(item('scutoons_swim_pants'), 'toptoy', 'legs-right'), 'characters/toptoy/wear/scutoons_swim_pants-legs-right.webp');
	assert.equal(wearOverlay(item('scutoons_tshirt'), 'toptoy', 'torso-left'), null);
	assert.equal(wearOverlay(item('dago_hoodie'), 'catpope', 'torso'), null, 'hoodies keep the icon at the anchor');
	assert.equal(wearConventionPath('toptoy', 'scutoons_tshirt', { id: 'torso-left', slot: 'torso' }), 'characters/toptoy/wear/scutoons_tshirt-torso-left.webp');
	assert.equal(wearConventionPath('catpope', 'scutoons_tshirt', { id: 'torso', slot: 'torso' }), 'characters/catpope/wear/scutoons_tshirt.webp');
	assert.deepEqual(WEAR_ORDER.slice(0, 3), ['legs', 'feet', 'torso']);
	const missing = missingWearOverlays({ items: catalog.items.filter((i) => i.id.startsWith('scutoons_')) }, registry).map((m) => `${m.item}@${m.character}/${m.instance}`);
	assert.deepEqual(missing.sort(), [
		'scutoons_sandals@toptoy/feet-left',
		'scutoons_sandals@toptoy/feet-right',
		'scutoons_sandals@yellowmouse/feet',
		'scutoons_shorts@toptoy/legs-left',
		'scutoons_shorts@toptoy/legs-right',
		'scutoons_shorts@yellowmouse/legs',
		'scutoons_tshirt@toptoy/torso-left',
		'scutoons_tshirt@toptoy/torso-right',
		'scutoons_tshirt@yellowmouse/torso'
	]);
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

test('wardrobe: the base image is always the swim look (the legacy naked look is ignored)', () => {
	// the fixture registry still has the removed `naked` looks (production data of 2026-10-04)
	const all = unlockedBodies(avatarBodies(registry), [{ character: 'catpope', looks: ['swim', 'naked'] }, { character: 'yellowmouse', looks: ['swim'] }]);
	assert.deepEqual(all.map((b) => `${b.character}/${b.look}`), ['catpope/swim', 'yellowmouse/swim']);
	const cat = all.find((b) => b.character === 'catpope')!;
	assert.equal(wardrobeBase(cat)?.path, 'characters/catpope/catpope_apose-swim.webp');
	assert.equal(wardrobeBase(null), null);
});
