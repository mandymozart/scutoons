// Substances & reader stats (agents/ITEMS.md "Substances"): decay, apply, effects, testo bands, validation – against the
// fixture catalog (server/tests/fixtures/catalog).   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
	activeEffects,
	applySubstance,
	characterTesto,
	endEffect,
	isSubstance,
	matchScan,
	scanPasses,
	scoreScan,
	scanTargets,
	statLevel,
	statLevels,
	testoBand,
	validateCatalog,
	catalogFromSeed,
	validateCharacters,
	type Catalog,
	type ReaderState,
	type RegistryCharacter
} from '../src/index.ts';

// the fixture catalog + the code parts it does not carry (slots, stats) = the served shape
const catalog: Catalog = catalogFromSeed(JSON.parse(readFileSync(new URL('../../../server/tests/fixtures/catalog/items.json', import.meta.url), 'utf8')));
const item = (id: string) => catalog.items.find((i) => i.id === id)!;
const stats = catalog.stats!;
const T0 = Date.parse('2026-10-01T12:00:00Z');
const DAY = 86_400_000;

test('substances: the catalog validates; testo, the testo camera, psychedelics, semaglutide', () => {
	assert.deepEqual(validateCatalog(catalog).filter((p) => p.level === 'error'), []);
	for (const id of ['testo', 'psilocybin', 'lsd', 'semaglutide']) assert.ok(isSubstance(item(id)), id);
	assert.ok(!isSubstance(item('camera')));
	assert.deepEqual(catalog.recipes.find((r) => r.id === 'testo_camera')?.inputs.map((i) => i.item).sort(), ['camera', 'testo']);
	assert.equal((item('testo_camera').skills[0] as any).grants, 'testo');
	assert.deepEqual(item('testo').grantable, ['story', 'scan']);
});

test('substances: decay toward the baseline (half-life), apply, cap', () => {
	const def = stats.testo;
	assert.equal(statLevel(def, null, T0), 50);
	assert.equal(statLevel(def, { level: 90, updated_at: new Date(T0).toISOString() }, T0 + 3 * DAY), 70);
	let s: ReaderState = { stats: {}, effects: [] };
	s = applySubstance(s, item('testo'), stats, T0);
	assert.equal(s.stats.testo.level, 70);
	s = applySubstance(applySubstance(s, item('testo'), stats, T0), item('testo'), stats, T0);
	assert.equal(s.stats.testo.level, 100, 'capped');
	assert.equal(statLevel(def, s.stats.testo, T0 + 3 * DAY), 75);
	s = applySubstance(s, item('semaglutide'), stats, T0);
	assert.equal(s.stats.appetite.level, 30);
	assert.deepEqual(statLevels(catalog, s, T0 + 3 * DAY).map((x) => `${x.id}:${x.level}`), ['testo:75', 'appetite:40']);
});

test('appetite: weed raises it, eating (cucumber −5, tuna can −30) lowers it, clamped 0–100', () => {
	assert.equal(item('weed').category, 'substance');
	assert.deepEqual(item('weed').effects, [{ stat: 'appetite', delta: 25 }]);
	assert.equal(item('weed').effect ?? null, null, 'no reader effect: only the appetite');
	for (const id of ['cucumber', 'tuna_can']) {
		assert.equal(item(id).category, 'food', id);
		assert.ok(isSubstance(item(id)) && item(id).skills.some((s) => s.type === 'apply'), id);
	}
	for (const id of ['weed', 'tuna_can']) assert.deepEqual(item(id).grantable, ['story', 'scan'], id);
	assert.ok(catalog.recipes.some((r) => r.inputs.some((i) => i.item === 'cucumber')), 'the cucumber stays a recipe ingredient');
	let s: ReaderState = { stats: {}, effects: [] };
	s = applySubstance(s, item('weed'), stats, T0);
	assert.equal(s.stats.appetite.level, 75);
	s = applySubstance(applySubstance(s, item('weed'), stats, T0), item('weed'), stats, T0);
	assert.equal(s.stats.appetite.level, 100, 'capped');
	s = applySubstance(s, item('cucumber'), stats, T0);
	assert.equal(s.stats.appetite.level, 95);
	s = applySubstance(s, item('tuna_can'), stats, T0);
	assert.equal(s.stats.appetite.level, 65);
	for (let i = 0; i < 3; i++) s = applySubstance(s, item('tuna_can'), stats, T0);
	assert.equal(s.stats.appetite.level, 0, 'floored at 0');
	assert.equal(statLevel(stats.appetite, s.stats.appetite, T0 + 3 * DAY), 25, 'and it drifts back up (half-life)');
});

test('substances: a timed effect runs, expires, can be ended', () => {
	let s = applySubstance({ stats: {}, effects: [] }, item('psilocybin'), stats, T0);
	assert.equal(s.effects[0].kind, 'psychedelic');
	assert.equal(activeEffects(s.effects, T0 + 19 * 60_000).length, 1);
	assert.equal(activeEffects(s.effects, T0 + 21 * 60_000).length, 0);
	s = applySubstance(s, item('lsd'), stats, T0);
	assert.equal(s.effects.length, 1, 'the same kind replaces the running one');
	assert.equal(Date.parse(s.effects[0].until), T0 + 45 * 60_000);
	assert.deepEqual(endEffect(s, 'psychedelic').effects, []);
});

test('testo: no male/female – a level 0–100 on characters and personas, bands as the hook', () => {
	assert.deepEqual([0, 33, 34, 66, 67, 100].map(testoBand), ['testo-low', 'testo-low', 'testo-mid', 'testo-mid', 'testo-high', 'testo-high']);
	const c = { id: 'x', name: { en: 'X' }, color: 'red', testo: 40, tendencies: ['psychedelics'], personas: [{ id: 'regular', testo: 80, looks: [{ id: 'default', default: true, images: {} }] }] } as unknown as RegistryCharacter;
	assert.equal(characterTesto(c, 'regular'), 80);
	assert.equal(characterTesto({ ...c, personas: [] }), 40);
	assert.equal(validateCharacters({ characters: [c] }, { languages: ['en'] }).filter((p) => /testo|tendencies/.test(p.message)).length, 0);
	const bad = { ...c, testo: 120, tendencies: ['Psychedelics!'] } as unknown as RegistryCharacter;
	const msgs = validateCharacters({ characters: [bad] }, { languages: ['en'] }).filter((p) => p.level === 'error').map((p) => p.message);
	assert.ok(msgs.some((m) => m.includes('testo must be a number 0–100')));
	assert.ok(msgs.some((m) => m.includes('tendencies')));
});

test('substances: validation errors', () => {
	const bad = structuredClone(catalog);
	bad.items.find((i) => i.id === 'testo')!.effects = [{ stat: 'mood', delta: 5 }];
	bad.items.find((i) => i.id === 'lsd')!.skills = [];
	bad.items.find((i) => i.id === 'psilocybin')!.effect = { kind: 'psychedelic', duration_min: 0 };
	bad.stats!.appetite.half_life_days = 0;
	const msgs = validateCatalog(bad).filter((p) => p.level === 'error').map((p) => p.message);
	assert.ok(msgs.some((m) => m.includes('unknown stat "mood"')));
	assert.ok(msgs.some((m) => m.includes('needs the "apply" skill')));
	assert.ok(msgs.some((m) => m.includes('duration_min > 0')));
	assert.ok(msgs.some((m) => m.includes('stat "appetite"')));
});

test('Check It camera: the multi-label scan maps model labels to substances (game recognition only)', () => {
	const cam = item('checkit_camera');
	assert.equal(cam.name.de, 'Check It Kamera');
	assert.deepEqual(cam.grantable, ['story']);
	assert.deepEqual(catalog.recipes.find((r) => r.id === 'checkit_camera')?.inputs.map((i) => `${i.item}×${i.qty}`).sort(), ['camera×1', 'testo×1']);
	const skill = cam.skills[0] as any;
	assert.deepEqual(scanTargets(skill).map((t) => t.grants), ['testo', 'semaglutide', 'psilocybin', 'lsd']);
	const hit = (className: string, probability = 0.8) => matchScan([{ className, probability }], skill);
	assert.equal(hit('pill bottle')?.target.grants, 'testo');
	assert.equal(hit('sunscreen, sunblock, sun blocker')?.target.grants, 'testo');
	assert.equal(hit('syringe')?.target.grants, 'semaglutide');
	assert.equal(hit('agaric')?.target.grants, 'psilocybin');
	assert.equal(hit('envelope')?.target.grants, 'lsd');
	assert.equal(hit('cucumber, cuke'), null);
	// the best prediction wins across targets
	assert.equal(matchScan([{ className: 'syringe', probability: 0.4 }, { className: 'mushroom', probability: 0.7 }], skill)?.target.grants, 'psilocybin');
	// every mapped item can be granted by a scan; a target granting a non-scannable item is an error
	const bad = structuredClone(catalog);
	(bad.items.find((i) => i.id === 'checkit_camera')!.skills[0] as any).map.push({ labels: ['banana'], grants: 'camera' });
	assert.ok(validateCatalog(bad).some((p) => p.message.includes('"camera" must be grantable by "scan"')));
});


test('scoreScan sums matching classes and ranks them; scanPasses: threshold or top-n with a minimum', () => {
	const skill = { labels: ['cucumber', 'zucchini'], grants: 'cucumber' };
	const rule = { threshold: 0.25, top: 3, min: 0.1 };
	// the owner's floor test (2026-10-02)
	const floor = scoreScan([{ className: 'slug', probability: 0.35 }, { className: 'quill, quill pen', probability: 0.14 }, { className: 'cucumber, cuke', probability: 0.13 }, { className: 'zucchini, courgette', probability: 0.05 }], skill);
	assert.equal(floor?.rank, 3);
	assert.ok(Math.abs((floor?.score ?? 0) - 0.18) < 1e-9);
	assert.equal(scanPasses(floor, rule), true);
	// far down the list and weak: no pass
	const weak = scoreScan([{ className: 'slug', probability: 0.4 }, { className: 'quill', probability: 0.2 }, { className: 'eel', probability: 0.1 }, { className: 'nail', probability: 0.06 }, { className: 'cucumber, cuke', probability: 0.05 }], skill);
	assert.equal(weak?.rank, 5);
	assert.equal(scanPasses(weak, rule), false);
	// top 3 but below the minimum: no pass
	assert.equal(scanPasses(scoreScan([{ className: 'slug', probability: 0.5 }, { className: 'cucumber', probability: 0.06 }], skill), rule), false);
	// strong alone: pass; nothing matching: null
	assert.equal(scanPasses(scoreScan([{ className: 'cucumber', probability: 0.3 }], skill), rule), true);
	assert.equal(scoreScan([{ className: 'slug', probability: 0.9 }], skill), null);
});

test('eating sobers up: each piece takes its share off every running effect (owner 2026-10-08)', () => {
	const now = Date.parse('2026-10-08T12:00:00Z');
	const state = { stats: {}, effects: [{ kind: 'psychedelic' as const, until: new Date(now + 16 * 60_000).toISOString() }] };
	const candy = { sober: 0.5 };
	const once = applySubstance(state, candy, {}, now);
	assert.equal(once.effects[0].until, new Date(now + 8 * 60_000).toISOString(), '16 minutes left → 8');
	const twice = applySubstance(once, candy, {}, now);
	assert.equal(twice.effects[0].until, new Date(now + 4 * 60_000).toISOString(), 'and 4');
	assert.ok(isSubstance(candy), 'an item that only sobers is used like a substance');
});
