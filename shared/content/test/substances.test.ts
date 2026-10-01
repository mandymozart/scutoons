// Substances & reader stats (agents/ITEMS.md "Substances"): decay, apply, effects, testo bands, validation – against the
// real server catalog.   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
	activeEffects,
	applySubstance,
	characterTesto,
	endEffect,
	isSubstance,
	statLevel,
	statLevels,
	testoBand,
	validateCatalog,
	validateCharacters,
	type Catalog,
	type ReaderState,
	type RegistryCharacter
} from '../src/index.ts';

const catalog: Catalog = JSON.parse(readFileSync(new URL('../../../server/config/items.json', import.meta.url), 'utf8'));
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
