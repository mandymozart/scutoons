// Containers, scores, ageing, kept recipe inputs and inherited variants (agents/research/SCORES.md).   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { ageLook, breaksAt, canFill, canStack, fitsIn, hardCapacity, moveInCheck, packBlock, readerPart, roomIn, validateCatalog, validateContainers, type ItemDef } from '../src/index.ts';

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

test('furniture is filled where it is set up, a carried container anywhere', () => {
	const shelf = def('record_shelf', { category: 'furniture', furniture: { size: 20, image: 'shelf.webp' }, holds: { items: ['awakening_vinyl'], max: 50 } });
	assert.equal(canFill(shelf, {}), false, 'carried: only carried');
	assert.equal(canFill(shelf, { home: { id: 'h1', x: 10, y: 80, flip: false } }), true, 'set up in a home');
	assert.equal(canFill(trolley, {}), true, 'the DJ trolley is no furniture');
	assert.equal(canFill(vinyl, {}), false, 'not a container');
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
		'error: a: holds needs categories, items or formats',
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

// research/PACKING.md §3 (owner 2026-10-16): the sound carrier's format decides – CDs, LPs, tapes
const lp = def('lp', { category: 'media', format: 'vinyl' } as Partial<ItemDef>);
const cd = def('cd', { category: 'media', format: 'cd' } as Partial<ItemDef>);
const tape = def('tape', { category: 'media', format: 'tape' } as Partial<ItemDef>);
const cdCase = def('cd_case', { category: 'bag', holds: { formats: ['cd'], max: 25 } });
const djTrolley = def('dj_trolley', { category: 'bag', holds: { formats: ['vinyl'], max: 50 } });
const cdTower = def('cd_rack', { category: 'furniture', furniture: { size: 12, image: 'x.webp' }, holds: { formats: ['cd'], max: 50 } });

test('formats: CDs into the CD case, LPs into the DJ trolley, never across', () => {
	assert.equal(fitsIn(cdCase, cd), true);
	assert.equal(fitsIn(cdCase, lp), false);
	assert.equal(fitsIn(djTrolley, lp), true);
	assert.equal(fitsIn(djTrolley, tape), false);
	assert.equal(fitsIn(cdTower, cd), true);
	assert.equal(fitsIn(djTrolley, drill), false, 'no format – no fit');
});

test('the pack mode: why a target is dimmed – the rule, then at home, then full', () => {
	assert.equal(packBlock(djTrolley, lp, 10), null);
	assert.equal(packBlock(cdCase, lp, 0), 'kind');
	assert.equal(packBlock(cdTower, cd, 0), 'at_home', 'furniture is filled at home');
	assert.equal(packBlock(cdTower, lp, 0), 'kind', 'the rule first');
	assert.equal(packBlock(djTrolley, lp, 50), 'full');
	assert.equal(packBlock(diy, score, 25), null, 'an overfill container still takes it (after a warning)');
	assert.equal(packBlock(drill, lp, 0), 'kind', 'not a container');
	assert.equal(roomIn(djTrolley, 47), 3);
	assert.equal(roomIn(djTrolley, 60), 0);
	assert.equal(roomIn(def('sack', { holds: { categories: ['food'] } }), 5), Infinity);
});

test('validation: holds.formats', () => {
	const ok = messages(catalog([cdCase, cd]));
	assert.ok(!ok.some((m) => m.includes('cd_case')), ok.join(' | '));
	const bad = messages(catalog([def('box', { holds: { formats: ['lp12' as never] } })]));
	assert.ok(bad.some((m) => m.startsWith('error') && m.includes('holds.formats must be a list of vinyl, tape, cd')), bad.join(' | '));
});

test('two of the same kind go onto one field by hand – not pieces of their own, not across places', () => {
	const cross = def('cucumber_cross', { category: 'tool' });
	const e = (id: string, extra: object = {}) => ({ id, item_id: 'cucumber_cross', variant: null, code: null, edition_number: null, in: null, home: null, ...extra });
	assert.equal(canStack(e('a'), e('b'), cross), true);
	assert.equal(canStack(e('a'), e('a'), cross), false, 'not itself');
	assert.equal(canStack(e('a'), e('b', { variant: 'red' }), cross), false, 'another colour');
	assert.equal(canStack(e('a'), e('b', { code: 'X1' }), cross), false, 'a piece with a code stays apart');
	assert.equal(canStack(e('a'), e('b', { edition_number: 3 }), cross), false, 'a numbered piece stays apart');
	assert.equal(canStack(e('a'), e('b', { in: 'bag' }), cross), false, 'one loose, one in a bag');
	assert.equal(canStack(e('a', { in: 'bag' }), e('b', { in: 'bag' }), cross), true, 'both in the same bag');
	assert.equal(canStack(e('a'), e('b'), small), false, 'containers keep their contents apart');
	assert.equal(canStack(e('a'), e('b'), diy), false, 'an ageing item keeps its age');
});

// research/COLLAGE.md §4.1: a score's collage, bound to the file it was placed on
test('score collage: pieces on the score host, a file of its own; another file warns', () => {
	const piece = (extra: object = {}) => ({ id: 'piece-1', src: 'catpope/vectors/wreath.svg', ratio: 1, rect: { x: 50, y: -2, width: 12 }, anchor: 'c', at: { bar: '12' }, ...extra });
	const files = { language: 'en', musicxml: 'scores/s/full.musicxml', parts: { choir: { musicxml: 'scores/s/choir.musicxml' } } };
	const withCollage = (score: object) => messages(catalog([def('s', { category: 'score', score: score as never })]));
	assert.deepEqual(withCollage({ ...files, collage: { file: 'scores/s/full.musicxml', pieces: [piece({ behind: true })] } }), []);
	assert.deepEqual(withCollage({ ...files, collage: null }), []);
	assert.deepEqual(withCollage({ ...files, parts: { choir: { ...files.parts.choir, collage: { file: 'scores/s/choir.musicxml', pieces: [piece()] } } } }), []);
	assert.deepEqual(withCollage({ ...files, collage: { file: 'scores/s/full.musicxml', pieces: [piece({ at: { staff: 2 } })] } }), [
		'error: s: at.bar must be the bar number as the file writes it ("12", "6a")'
	]);
	assert.deepEqual(withCollage({ ...files, collage: { file: 'scores/s/old.musicxml', pieces: [piece()] } }), ['warning: s: the collage was made on another file – check the bars']);
	assert.deepEqual(withCollage({ ...files, parts: { choir: { ...files.parts.choir, collage: { file: 'scores/s/full.musicxml', pieces: [piece({ at: undefined })] } } } }), [
		'warning: s (choir): the collage was made on another file – check the bars',
		'error: s (choir): a score piece needs at { bar } (the bar it sticks to)'
	]);
	assert.deepEqual(withCollage({ ...files, collage: { pieces: {} } }), [
		'error: s: collage.file must be the scores/ .musicxml path the pieces were placed on',
		'error: s: collage.pieces must be a list'
	]);
	// a file outside the vault's scores/ – the server's CatalogValidator refuses it too (422 on Publish)
	assert.deepEqual(withCollage({ ...files, collage: { file: 'other/x.musicxml', pieces: [piece()] } }), [
		'error: s: collage.file must be the scores/ .musicxml path the pieces were placed on'
	]);
	const paths = validateCatalog(catalog([def('s', { category: 'score', score: { ...files, collage: { file: files.musicxml, pieces: [piece({ style: { blend: 'nope' } })] } } as never })])).map((p) => p.path.join('.'));
	assert.deepEqual(paths, ['items.0.score.collage.pieces.0.style.blend']);
});

// research/COLLAGE.md §12.5: custom presets are judged only against a presets list (the creator's score stage has one)
test('score collage: custom presets – validateContainers with and without presets', () => {
	const piece = (id: string, preset: string) => ({ id, src: 'catpope/vectors/wreath.svg', ratio: 1, rect: { x: 50, y: -2, width: 12 }, anchor: 'c', at: { bar: '12' }, preset });
	const score = { language: 'en', musicxml: 'scores/s/full.musicxml', collage: { file: 'scores/s/full.musicxml', pieces: [piece('a', 'night'), piece('b', 'old'), piece('c', 'nope'), piece('d', 'burn')] } };
	const items = [def('s', { category: 'score', score: score as never })];
	const run = (presets?: { id: string; name: string; removed?: boolean }[]) => {
		const out: string[] = [];
		validateContainers({ items }, (level, path, message) => out.push(`${level} ${path.join('.')}: ${message}`), presets);
		return out;
	};
	assert.deepEqual(run(), [], 'no list: only code looks are judged (the catalog, the admin)');
	assert.deepEqual(run([{ id: 'night', name: 'Night' }, { id: 'old', name: 'Old', removed: true }]), [
		'warning items.0.score.collage.pieces.1.preset: s: the preset "Old" was deleted – the piece still draws it; pick another',
		'warning items.0.score.collage.pieces.2.preset: s: unknown preset "nope" – the piece is drawn without it'
	]);
	assert.deepEqual(messages(catalog(items)), [], 'validateCatalog passes no list');
});
