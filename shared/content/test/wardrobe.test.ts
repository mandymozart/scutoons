// Wardrobe (agents/ITEMS.md "Wardrobe"): overlays, the cover rule, the base image, the catalog validation – against the
// real server catalog (server/config/items.json) and the seed registry.   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
	avatarBodies,
	coverSensitive,
	missingWearOverlays,
	unlockedBodies,
	validateCatalog,
	wardrobeBase,
	wearConventionPath,
	wearOverlay,
	WEAR_ORDER,
	type Catalog,
	type CharacterRegistry
} from '../src/index.ts';

const json = (rel: string) => JSON.parse(readFileSync(new URL(rel, import.meta.url), 'utf8'));
const catalog: Catalog = json('../../../server/config/items.json');
const registry: CharacterRegistry = json('../seed/scutoons/_/characters.json');
const item = (id: string) => catalog.items.find((i) => i.id === id)!;

test('wardrobe: the catalog validates against the registry', () => {
	const problems = validateCatalog(catalog, registry);
	assert.deepEqual(problems.filter((p) => p.level !== 'info'), []);
	for (const id of ['scutoons_tshirt', 'scutoons_swim_pants', 'scutoons_shorts']) {
		assert.equal(item(id).starter, true, `${id} is a starter item`);
		assert.equal(item(id).category, 'wearable');
	}
	// the swim pants come before the shorts (the cover rule takes the first owned legs item)
	assert.ok(catalog.items.indexOf(item('scutoons_swim_pants')) < catalog.items.indexOf(item('scutoons_shorts')));
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
	const noCover = structuredClone(catalog);
	for (const i of noCover.items) if (i.slot === 'legs') i.starter = false;
	assert.ok(validateCatalog(noCover, registry).some((p) => p.message.includes('no starter item for the "legs" slot')));
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
		'scutoons_shorts@toptoy/legs-left',
		'scutoons_shorts@toptoy/legs-right',
		'scutoons_shorts@yellowmouse/legs',
		'scutoons_tshirt@toptoy/torso-left',
		'scutoons_tshirt@toptoy/torso-right',
		'scutoons_tshirt@yellowmouse/torso'
	]);
});

test('wardrobe: the cover rule refills legs, swim pants first, never twice', () => {
	const single = [{ id: 'torso', slot: 'torso' as const }, { id: 'legs', slot: 'legs' as const }];
	const owned = [
		{ id: 'pants', slot: 'legs' as const },
		{ id: 'shorts', slot: 'legs' as const },
		{ id: 'shirt', slot: 'torso' as const }
	];
	assert.deepEqual(coverSensitive({ torso: 'shirt', legs: null }, single, owned), { worn: { torso: 'shirt', legs: 'pants' }, restored: ['legs'] });
	assert.deepEqual(coverSensitive({ legs: 'shorts' }, single, owned).restored, []);
	assert.deepEqual(coverSensitive({ torso: null }, single, []).restored, [], 'nothing to wear: stays empty (safe base)');
	const twins = [{ id: 'legs-left', slot: 'legs' as const }, { id: 'legs-right', slot: 'legs' as const }];
	assert.deepEqual(coverSensitive({ 'legs-left': 'pants', 'legs-right': null }, twins, owned).worn, { 'legs-left': 'pants', 'legs-right': 'shorts' });
	assert.deepEqual(coverSensitive({ 'legs-left': 'pants', 'legs-right': null }, twins, owned.slice(0, 1)).restored, []);
});

test('wardrobe: the base image is naked only when allowed', () => {
	const all = unlockedBodies(avatarBodies(registry), [{ character: 'catpope', looks: ['swim', 'naked'] }, { character: 'yellowmouse', looks: ['swim'] }]);
	const cat = all.find((b) => b.character === 'catpope' && b.look === 'swim')!;
	assert.equal(wardrobeBase(all, cat, { hide: true, covered: true })?.naked, true, 'covered: naked base + the overlays');
	assert.equal(wardrobeBase(all, cat, { hide: true, covered: false })?.path, cat.path, 'hidden + uncovered: the safe swim look');
	assert.equal(wardrobeBase(all, cat, { hide: false, covered: false })?.naked, true, 'setting off: naked allowed');
	const mouse = all.find((b) => b.character === 'yellowmouse')!;
	assert.equal(wardrobeBase(all, mouse, { hide: false, covered: false })?.naked, false, 'naked look not unlocked: swim');
	assert.equal(wardrobeBase(all, null, { hide: true, covered: true }), null);
});
