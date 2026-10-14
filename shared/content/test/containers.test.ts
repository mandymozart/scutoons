// Containers, scores, ageing, kept recipe inputs and inherited variants (agents/research/SCORES.md).   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { ageLook, breaksAt, fitsIn, hardCapacity, moveInCheck, readerPart, validateCatalog, type ItemDef } from '../src/index.ts';

const base = { toon: null, name: { en: 'x' }, description: {}, kind: 'virtual', icon: 'x.png', stackable: false, starter: false, grantable: ['story'], slot: null, skills: [], edition_id: null, product: null, variants: null } as const;
const def = (id: string, extra: Partial<ItemDef>): ItemDef => ({ ...base, id, category: 'collectible', ...extra }) as ItemDef;
const colours = [{ id: 'black', name: { en: 'Black' } }, { id: 'red', name: { en: 'Red' } }];

const score = def('cloud9_en', { category: 'score', score: { language: 'en', musicxml: 'scutoons/scores/cloud9-en.musicxml', pdf: 'scutoons/scores/cloud9-en.pdf', parts: { choir: { musicxml: 'scores/c/choir.musicxml', pdf: 'scores/c/choir.pdf' } } } });
const small = def('songbook_small', { category: 'tool', holds: { categories: ['score'], max: 12 }, variants: colours });
const diy = def('songbook_diy', { category: 'tool', holds: { categories: ['score'], max: 25, overfill: { days: 3, max: 99, breaks_into: 'songbook_diy_broken' } }, ages: { tint_days: 14, spots_days: 30, full_days: 90 }, variants: colours });
const broken = def('songbook_diy_broken', { category: 'tool', variants: colours });
const trolley = def('dj_trolley', { category: 'tool', holds: { items: ['awakening_vinyl'], max: 60 } });
const vinyl = def('awakening_vinyl', { category: 'media' });
const plastic = def('plastic_sheet', { category: 'consumable', stackable: true, variants: colours });
const drill = def('drill', { category: 'tool' });
const zip = def('zip_tie', { category: 'consumable', stackable: true });

test('an item fits by its category or its id – never a container in a container', () => {
	assert.equal(fitsIn(small, score), true);
	assert.equal(fitsIn(trolley, vinyl), true);
	assert.equal(fitsIn(trolley, score), false);
	assert.equal(fitsIn(small, vinyl), false);
	assert.equal(fitsIn(small, diy), false, 'no nesting');
	assert.equal(fitsIn(vinyl, score), false, 'not a container');
});

test('capacity: a hard limit, or an overfill that warns and then breaks', () => {
	assert.equal(moveInCheck(small, 11), 'ok');
	assert.equal(moveInCheck(small, 12), 'full');
	assert.equal(moveInCheck(diy, 24), 'ok');
	assert.equal(moveInCheck(diy, 25), 'overfill');
	assert.equal(moveInCheck(diy, 99), 'full');
	assert.equal(hardCapacity(diy), 99);
	assert.equal(hardCapacity(def('bag', { holds: { categories: ['food'] } })), Infinity);
	const day = 86_400_000;
	assert.equal(breaksAt(diy, 26, 1000), 1000 + 3 * day);
	assert.equal(breaksAt(diy, 25, 1000), null, 'back to 25 – saved');
	assert.equal(breaksAt(small, 12, 1000), null);
});

test('ageing: a tint after 2 weeks, spots after a month, full at 3 months', () => {
	const day = 86_400_000;
	assert.deepEqual(ageLook(diy.ages, 0, 10 * day), { tint: 0, spots: 0 });
	const month = ageLook(diy.ages, 0, 45 * day);
	assert.ok(month.tint > 0 && month.tint < 1 && month.spots > 0 && month.spots < month.tint);
	assert.deepEqual(ageLook(diy.ages, 0, 200 * day), { tint: 1, spots: 1 });
	assert.deepEqual(ageLook(null, 0, 200 * day), { tint: 0, spots: 0 });
});

const catalog = (items: ItemDef[], recipes: unknown[] = []) => ({ slots: [], items, recipes });
const messages = (c: unknown) => validateCatalog(c).map((p) => `${p.level}: ${p.message}`);

test('the song book catalog is valid', () => {
	const recipes = [
		{ id: 'songbook_diy', inputs: [{ item: 'drill', qty: 1, keep: true }, { item: 'zip_tie', qty: 4 }], output: { item: 'songbook_diy', qty: 1 } },
		{ id: 'songbook_diy_colour', inputs: [{ item: 'songbook_diy', qty: 1 }, { item: 'plastic_sheet', qty: 1 }], output: { item: 'songbook_diy', qty: 1, variant_from: 'plastic_sheet' } }
	];
	assert.deepEqual(messages(catalog([score, small, diy, broken, trolley, vinyl, plastic, drill, zip], recipes)), []);
});

test('validation: holds, overfill, ages, score files, recipes', () => {
	const bad = messages(
		catalog(
			[
				def('a', { holds: {} as never }),
				def('b', { holds: { items: ['songbook_small', 'ghost'], max: 0 } }),
				small,
				def('c', { holds: { categories: ['score'], max: 5, overfill: { days: 3, max: 4, breaks_into: 'nope' } } }),
				def('d', { ages: { tint_days: 100, spots_days: 1, full_days: 90 } }),
				def('e', { score: { language: 'en', musicxml: 'x.pdf' } }),
				def('f', { category: 'score', score: { language: 'en', musicxml: 'x.musicxml', pdf: 'x.png' } }),
				drill,
				plastic,
				def('book', { variants: [colours[0]] })
			],
			[
				{ id: 'r1', inputs: [{ item: 'drill', qty: 1, keep: true }], output: { item: 'book', qty: 1 } },
				{ id: 'r2', inputs: [{ item: 'plastic_sheet', qty: 1 }], output: { item: 'book', qty: 1, variant_from: 'drill' } },
				{ id: 'r3', inputs: [{ item: 'plastic_sheet', qty: 1 }], output: { item: 'book', qty: 1, variant_from: 'plastic_sheet' } }
			]
		)
	);
	const expect = [
		'error: a: holds needs categories or items',
		'error: b: a container can\'t hold a container ("songbook_small")',
		'warning: b: holds an unknown item "ghost"',
		'error: b: holds.max must be a whole number ≥ 1',
		'error: c: overfill.max must be above holds.max',
		'error: c: breaks into an unknown item "nope"',
		'error: d: ages must be { tint_days, spots_days, full_days } with both starts before full_days',
		'error: e: score must be { language, musicxml: a .musicxml / .mxl path, pdf? }',
		'warning: e: an item with score files should have the category "score"',
		'error: f: score.pdf must be a .pdf path',
		'error: r1: a recipe must use up at least one input',
		'error: r2: variant_from must name one of the inputs',
		'warning: r3: book has no variant red of plastic_sheet'
	];
	for (const m of expect) assert.ok(bad.includes(m), `missing: ${m}\n${bad.join('\n')}`);
});

test('score parts: the reader sees the choir when there is one; parts are checked', () => {
	assert.equal(readerPart(score.score), 'choir');
	assert.equal(readerPart({ parts: null }), null);
	assert.equal(readerPart(null), null);
	const bad = messages(catalog([def('s', { category: 'score', score: { language: 'en', musicxml: 'a.musicxml', parts: { Choir: { musicxml: 'x.pdf' } } as never } })]));
	assert.ok(bad.includes('error: s: part name "Choir" – lower-case letters, digits, _'), bad.join('\n'));
	assert.ok(bad.includes('error: s: part Choir needs a .musicxml / .mxl path'), bad.join('\n'));
});
