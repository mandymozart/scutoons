// Collage pieces: looks, custom presets, style merge, the bake, sizes, weight, names, problems (agents/research/COLLAGE.md
// §2, §12).   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
	BLEND_MODES,
	bakePresets,
	COLLAGE_LOOKS,
	collageLook,
	collageProblems,
	colorationProblems,
	customPreset,
	emptyColorations,
	heavyWeight,
	PIECE_STYLE_KEYS,
	pieceHeight,
	pieceKind,
	pieceName,
	pieceStyle,
	PRESET_ID,
	type CollagePiece,
	type CollagePreset,
	type PresetList
} from '../src/index.ts';

const piece = (extra: Partial<CollagePiece> = {}): CollagePiece => ({ id: 'piece-abcdefg', src: 'vectors/wreath.svg', ratio: 0.5, rect: { x: 10, y: 10, width: 30 }, ...extra });
const problems = (pieces: unknown, host: 'slice' | 'score' = 'slice', presets?: PresetList) => {
	const out: string[] = [];
	collageProblems(pieces, host, (level, path, message) => out.push(`${level} ${path.join('.')}: ${message}`), presets);
	return out;
};
const where = (pieces: unknown, host: 'slice' | 'score' = 'slice', presets?: PresetList) => problems(pieces, host, presets).map((p) => p.split(':')[0]);

test('the 16 blend modes and the ten code looks (each valid as a piece style)', () => {
	assert.equal(BLEND_MODES.length, 16);
	assert.deepEqual(COLLAGE_LOOKS.map((l) => l.id), ['ink', 'red', 'cyan', 'negative', 'burn', 'acid', 'sticker', 'glow', 'ghost', 'fade']);
	for (const { id, ...style } of COLLAGE_LOOKS) assert.deepEqual(problems([piece({ style })]), [], id);
	assert.equal(collageLook('negative')?.blend, 'difference');
	assert.equal(collageLook('nope'), undefined);
	assert.equal(collageLook(3), undefined);
});

test('pieceStyle: the look under the own keys, a key replaced whole, null = off', () => {
	assert.deepEqual(pieceStyle(piece()), {});
	assert.deepEqual(pieceStyle(piece({ preset: 'negative' })), { fill: '#ffffff', blend: 'difference' });
	// a whole shadow replaces the look's, null switches the outline off; the id never leaks
	assert.deepEqual(pieceStyle(piece({ preset: 'sticker', style: { outline: null, shadow: { color: '#ff0000', x: 0, y: 4, blur: 1 } } })), {
		shadow: { color: '#ff0000', x: 0, y: 4, blur: 1 }
	});
	// an own value over a look, undefined = the look's
	assert.deepEqual(pieceStyle(piece({ preset: 'negative', style: { blend: 'normal', fill: undefined } })), { fill: '#ffffff', blend: 'normal' });
	// an unknown look = none
	assert.deepEqual(pieceStyle(piece({ preset: 'gone', style: { opacity: 0.4 } })), { opacity: 0.4 });
	assert.deepEqual(pieceStyle(null), {});
});

test('pieceStyle: only the six style keys, own ones – a stored __proto__ key sets no prototype', () => {
	const stored = JSON.parse('{"id":"piece-abcdefg","src":"vectors/wreath.svg","ratio":0.5,"rect":{"x":10,"y":10,"width":30},"style":{"__proto__":{"fill":"url(x)","blend":"plus-lighter"},"id":"x"}}');
	const style = pieceStyle(stored);
	assert.deepEqual(style, {});
	assert.equal(Object.getPrototypeOf(style), Object.prototype);
	assert.equal((style as Record<string, unknown>).fill, undefined);
	assert.deepEqual(problems([stored]), []);
	// unknown keys are dropped, the known ones kept
	assert.deepEqual(pieceStyle(piece({ style: { opacity: 0.5, id: 'x' } as never })), { opacity: 0.5 });
});

test('pieceStyle reads every style key of the piece and of its preset, missing ones too (a $state proxy then tracks a key added later)', () => {
	const spy = <T extends object>(object: T, read: Set<PropertyKey>) => new Proxy(object, { get: (target, key, receiver) => (read.add(key), Reflect.get(target, key, receiver)) });
	const [own, theirs] = [new Set<PropertyKey>(), new Set<PropertyKey>()];
	const preset = spy({ id: 'night', name: 'Night', fill: '#1a2b6d' }, theirs) as CollagePreset;
	const style = pieceStyle(piece({ preset: 'night', style: spy({ blur: { amount: 2 } }, own) }), [preset]);
	assert.deepEqual(style, { fill: '#1a2b6d', blur: { amount: 2 } });
	for (const key of PIECE_STYLE_KEYS) assert.ok(own.has(key), `the piece's ${key}`);
	for (const key of PIECE_STYLE_KEYS) assert.ok(theirs.has(key), `the preset's ${key}`);
});

test('pieceHeight: the width in % of the box, a file of ratio w ÷ h, a box of h ÷ w', () => {
	assert.equal(pieceHeight(30, 0.5, 1.6), 37.5);
	// a square file on the 390 × 625 slice
	assert.equal(Math.round(pieceHeight(50, 1, 625 / 390) * 100) / 100, 31.2);
});

test('heavyWeight: outline 1, shadow 1, blur 1, a progressive blur 2', () => {
	assert.equal(heavyWeight(undefined), 0);
	assert.equal(heavyWeight({ fill: '#111111', blend: 'multiply' }), 0);
	assert.equal(heavyWeight(pieceStyle({ preset: 'sticker' })), 2);
	assert.equal(heavyWeight(pieceStyle({ preset: 'ghost' })), 1);
	assert.equal(heavyWeight(pieceStyle({ preset: 'fade' })), 2);
	assert.equal(heavyWeight({ outline: { color: '#fff', width: 0 }, blur: { amount: 0 } }), 0);
});

test('kind and name are derived from the file / the figure', () => {
	assert.equal(pieceKind(piece()), 'shape');
	assert.equal(pieceKind(piece({ src: '../characters/catpope/regular/default_portrait-happy.webp' })), 'image');
	assert.equal(pieceKind(piece({ src: 'x.webp', figure: { id: 'catpope', mood: 'happy', view: 'portrait' } })), 'character');
	assert.equal(pieceName(piece()), 'wreath');
	assert.equal(pieceName(piece({ src: 'images/1x/bg.2x.webp' })), 'bg');
	assert.equal(pieceName(piece({ figure: { id: 'catpope', mood: 'happy', view: 'full' } })), 'catpope · happy');
	assert.equal(pieceName(piece({ figure: { id: 'catpope', view: 'avatar' } })), 'catpope');
});

test('problems: a good slice piece has none; nothing / an empty list is fine, null is an error', () => {
	assert.deepEqual(problems(undefined), []);
	assert.deepEqual(problems([]), []);
	assert.deepEqual(problems([piece({ anchor: 'c', rotation: -12, preset: 'burn', style: { opacity: 0.7, blur: { amount: 4, from: { x: 0, y: 0 }, to: { x: 100, y: 100 } } } })]), []);
	assert.deepEqual(where({}), ['error ']);
	// null is no list (the server's schema refuses `pieces: null`)
	assert.deepEqual(problems(null), ['error : pieces must be a list']);
	assert.deepEqual(where([3]), ['error 0']);
});

test('problems: ids, file, ratio, rect, anchor, rotation, figure', () => {
	assert.deepEqual(where([piece({ id: '' }), piece(), piece()]), ['error 0.id', 'error 2.id']);
	assert.deepEqual(where([piece({ src: 'vectors/wreath' }), piece({ id: 'b', src: 'a.mp3' })]), ['error 0.src', 'error 1.src']);
	assert.deepEqual(where([piece({ ratio: 0 })]), ['error 0.ratio']);
	assert.deepEqual(where([piece({ rect: { x: 1, y: 2, width: 0 } })]), ['error 0.rect.width']);
	assert.deepEqual(where([piece({ rect: { x: 1, y: 2, width: 10, height: 10 } as never })]), ['error 0.rect.height']);
	assert.deepEqual(where([piece({ rect: { x: '1', y: 2 } as never })]), ['error 0.rect.x', 'error 0.rect.width']);
	assert.deepEqual(where([piece({ anchor: 'middle' as never, rotation: '9' as never })]), ['error 0.anchor', 'error 0.rotation']);
	assert.deepEqual(where([piece({ figure: { id: 'catpope', view: 'bust' as never } })]), ['error 0.figure.view']);
});

test('problems: style values', () => {
	const style = (s: object) => where([piece({ style: s as never })]);
	assert.deepEqual(style({ blend: 'plus-lighter' }), ['error 0.style.blend']);
	assert.deepEqual(style({ opacity: 1.2 }), ['error 0.style.opacity']);
	assert.deepEqual(style({ fill: 'red' }), ['error 0.style.fill']);
	assert.deepEqual(style({ fill: null, outline: null, shadow: null, blur: null }), []);
	assert.deepEqual(style({ fill: { stops: [{ at: 0, color: '#fff' }] } }), ['error 0.style.fill.stops']);
	assert.deepEqual(style({ fill: { angle: 90, stops: [{ at: 50, color: '#fff' }, { at: 120, color: '#000' }] } }), ['error 0.style.fill.stops.1.at']);
	assert.deepEqual(style({ fill: { stops: [{ at: 60, color: '#fff' }, { at: 20, color: '#000', opacity: 2 }] } }), ['error 0.style.fill.stops.1.at', 'error 0.style.fill.stops.1.opacity']);
	assert.deepEqual(style({ outline: { color: '#ffffff', width: -1 } }), ['error 0.style.outline.width']);
	assert.deepEqual(style({ shadow: { color: '#000', x: 0, y: 'a', blur: 2, opacity: 3 } }), ['error 0.style.shadow.y', 'error 0.style.shadow.opacity']);
	assert.deepEqual(style({ blur: { amount: 3, from: { x: 0, y: 0 } } }), ['error 0.style.blur']);
	assert.deepEqual(style({ blur: { amount: 3, from: { x: 0 }, to: { x: 0, y: 1 } } }), ['error 0.style.blur.from']);
	assert.deepEqual(where([piece({ style: 'ink' as never })]), ['error 0.style']);
	// effect sizes past the limits warn (% of the piece)
	assert.deepEqual(style({ outline: { color: '#fff', width: 9 }, shadow: { color: '#000', x: 0, y: 0, blur: 26 }, blur: { amount: 21 } }), [
		'warning 0.style.outline.width',
		'warning 0.style.shadow.blur',
		'warning 0.style.blur.amount'
	]);
});

test('problems: an unknown look (against a presets list), too many pieces, too heavy for phones', () => {
	assert.deepEqual(where([piece({ preset: 'mystery' })], 'slice', []), ['warning 0.preset']);
	const many = Array.from({ length: 13 }, (_, i) => piece({ id: `p${i}` }));
	assert.deepEqual(where(many), ['warning ']);
	assert.deepEqual(where(many.slice(0, 12)), []);
	// sticker (2) × 3 = 6 > 4 on a slice, fine on a score (20)
	const heavy = Array.from({ length: 3 }, (_, i) => piece({ id: `h${i}`, preset: 'sticker', at: { bar: '1' } }));
	assert.match(problems(heavy.map(({ at: _at, ...p }) => p))[0], /^warning : outlines, shadows and blurs weigh 6/);
	assert.deepEqual(problems(heavy, 'score'), []);
});

test('problems: hosts – at / behind only on scores, at { bar } required there', () => {
	assert.deepEqual(where([piece({ at: { bar: '3' }, behind: true })]), ['error 0.at', 'error 0.behind']);
	const score = (extra: Partial<CollagePiece>) => where([piece({ anchor: 'c', ...extra })], 'score');
	assert.deepEqual(score({ at: { bar: '6a', nth: 2, staff: 1, page: false }, behind: true }), []);
	assert.deepEqual(score({}), ['error 0.at']);
	assert.deepEqual(score({ at: { bar: '' } }), ['error 0.at.bar']);
	assert.deepEqual(score({ at: { bar: 12 as never } }), ['error 0.at.bar']);
	assert.deepEqual(score({ at: { bar: '1', nth: 0, staff: 1.5, page: 'yes' as never }, behind: 'no' as never }), ['error 0.at.nth', 'error 0.at.staff', 'error 0.at.page', 'error 0.behind']);
});

// ── custom presets (research/COLLAGE.md §12) ────────────────────────────────────────────────────────────────────

const night: CollagePreset = { id: 'night', name: 'Night', fill: '#1a2b6d', blend: 'multiply', shadow: { color: '#000000', x: 2, y: 2, blur: 4 } };

test('PRESET_ID: a slug that is never a code look, nor none – the same pattern as the schema', () => {
	for (const id of [...COLLAGE_LOOKS.map((l) => l.id), 'none', 'Night', '1x', '', '-night', 'night!', 'a'.repeat(41)]) assert.equal(PRESET_ID.test(id), false, id);
	for (const id of ['night', 'negative-2', 'none-2', 'inky', 'a', 'a'.repeat(40)]) assert.equal(PRESET_ID.test(id), true, id);
	const schema = JSON.parse(readFileSync(new URL('../schema/colorations.schema.json', import.meta.url), 'utf8'));
	assert.equal(schema.definitions.PresetId.pattern, PRESET_ID.source);
	const server = JSON.parse(readFileSync(new URL('../../../server/schema/content/colorations.schema.json', import.meta.url), 'utf8'));
	assert.deepEqual(server, schema, 'the API carries the same schema');
	assert.deepEqual(PIECE_STYLE_KEYS, ['blend', 'opacity', 'fill', 'outline', 'shadow', 'blur']);
	assert.deepEqual(emptyColorations(), { presets: [] });
});

test('collageLook / customPreset: code looks first, then the first stored entry of an id (deleted ones too)', () => {
	assert.equal(collageLook('negative', [{ id: 'negative', name: 'N', fill: '#ff0000' }])?.fill, '#ffffff', 'data never redefines a built-in');
	assert.equal(collageLook('night'), undefined, 'no list: code looks only');
	assert.equal(collageLook('night', [night])?.fill, '#1a2b6d');
	assert.equal(collageLook('night', [night, { id: 'night', name: 'Again', fill: '#ffffff' }])?.fill, '#1a2b6d', 'the first wins');
	assert.equal(collageLook('old', [{ id: 'old', name: 'Old', removed: true, blend: 'screen' }])?.blend, 'screen');
	assert.equal(customPreset('Night!', [{ id: 'Night!', name: 'x', fill: '#000' }]), undefined, 'a malformed id is none');
	assert.equal(customPreset('negative', [{ id: 'negative', name: 'N', fill: '#ff0000' }]), undefined);
	assert.equal(customPreset('night', [null, 3, 'night', night] as never)?.name, 'Night', 'entries that are no objects are skipped');
	assert.equal(customPreset('night', null), undefined);
	assert.equal(customPreset(3, [night]), undefined);
});

test('pieceStyle with presets: a custom preset under the own keys, as a code look', () => {
	assert.deepEqual(pieceStyle(piece({ preset: 'night' }), [night]), { fill: '#1a2b6d', blend: 'multiply', shadow: { color: '#000000', x: 2, y: 2, blur: 4 } });
	assert.deepEqual(pieceStyle(piece({ preset: 'night', style: { shadow: null, opacity: 0 } }), [night]), { fill: '#1a2b6d', blend: 'multiply', opacity: 0 });
	assert.deepEqual(pieceStyle(piece({ preset: 'night', style: { opacity: 0.4 } })), { opacity: 0.4 }, 'without the list: drawn without it');
	// a stored preset's other keys (its id, name, a __proto__) are never style
	const stored = JSON.parse('[{"id":"odd","name":"Odd","removed":false,"__proto__":{"fill":"url(x)"},"blur":{"amount":2}}]');
	const style = pieceStyle(piece({ preset: 'odd' }), stored);
	assert.deepEqual(style, { blur: { amount: 2 } });
	assert.equal(Object.getPrototypeOf(style), Object.prototype);
});

type BakeCase = { name: string; piece: CollagePiece; baked: CollagePiece };
const fixture = JSON.parse(readFileSync(new URL('./fixtures/collage-bake.json', import.meta.url), 'utf8')) as { presets: CollagePreset[]; cases: BakeCase[] };

test('bakePresets: the golden fixture (the server CollagePresetsTest reads the same file)', () => {
	const { presets, cases } = fixture;
	const pieces = cases.map((c) => c.piece);
	const before = JSON.stringify(pieces);
	const presetsBefore = JSON.stringify(presets);
	const baked = bakePresets(pieces, presets);
	assert.equal(JSON.stringify(pieces), before, 'the input is untouched');
	assert.equal(baked.length, cases.length);
	cases.forEach((c, i) => {
		assert.equal(JSON.stringify(baked[i]), JSON.stringify(c.baked), c.name);
		assert.deepEqual(pieceStyle(baked[i]), pieceStyle(c.piece, presets), `${c.name}: drawn the same`);
		assert.deepEqual(pieceStyle(baked[i], presets), pieceStyle(c.piece, presets), `${c.name}: drawn the same with the list`);
		if (JSON.stringify(c.piece) === JSON.stringify(c.baked)) assert.equal(baked[i], c.piece, `${c.name}: passed through as it is`);
		else assert.equal('preset' in baked[i], false, `${c.name}: no preset left`);
	});
	// the baked values are copies: changing one changes neither the preset nor the piece
	const dusk = baked[cases.findIndex((c) => c.piece.preset === 'dusk')];
	(dusk.style!.fill as { stops: { color: string }[] }).stops[0].color = '#000000';
	assert.equal(JSON.stringify(presets), presetsBefore);
	assert.equal(JSON.stringify(pieces), before);
});

test('bakePresets: nothing to bake = the same list back', () => {
	const pieces = [piece({ preset: 'negative' }), piece({ id: 'b', preset: 'nope' }), piece({ id: 'c' })];
	assert.equal(bakePresets(pieces, [night]), pieces);
	assert.equal(bakePresets(pieces, []), pieces);
	assert.equal(bakePresets(pieces, undefined), pieces);
	assert.equal(bakePresets(undefined, [night]), undefined);
	const mixed = [piece({ preset: 'night' }), pieces[0]];
	const baked = bakePresets(mixed, [night]);
	assert.notEqual(baked, mixed);
	assert.equal(baked[1], mixed[1], 'an untouched piece is the same object');
});

test('collageProblems with presets: known fine, deleted / unknown warn; without a list only code looks are judged', () => {
	const old: CollagePreset = { id: 'old', name: 'Old', removed: true, outline: { color: '#ffffff', width: 2 } };
	const list = [night, old];
	assert.deepEqual(problems([piece({ preset: 'night' }), piece({ id: 'b', preset: 'ink' })], 'slice', list), []);
	assert.deepEqual(problems([piece({ preset: 'old' })], 'slice', list), ['warning 0.preset: the preset "Old" was deleted – the piece still draws it; pick another']);
	assert.deepEqual(problems([piece({ preset: 'nope' })], 'slice', list), ['warning 0.preset: unknown preset "nope" – the piece is drawn without it']);
	assert.deepEqual(problems([piece({ preset: 'nope' }), piece({ id: 'b', preset: 'night' })]), [], 'no list: not judged');
	assert.deepEqual(where([piece({ preset: 3 as never })]), ['error 0.preset'], 'a preset is always a string');
	// a preset's outline + shadow weigh 2 – three of them on a slice: 6 > 4
	const shiny: CollagePreset = { id: 'shiny', name: 'Shiny', outline: { color: '#ffffff', width: 2 }, shadow: { color: '#000000', x: 1, y: 1, blur: 1 } };
	const heavy = (presets?: PresetList) => problems(Array.from({ length: 3 }, (_, i) => piece({ id: `h${i}`, preset: 'shiny' })), 'slice', presets);
	assert.match(heavy([shiny])[0], /^warning : outlines, shadows and blurs weigh 6/);
	assert.deepEqual(heavy(), [], 'without the list the preset is not drawn, so it weighs nothing');
});

test('colorationProblems: the document, ids, names, removed, style values', () => {
	const doc = (...presets: unknown[]) => {
		const out: string[] = [];
		colorationProblems({ presets }, (level, path, message) => out.push(`${level} ${path.join('.')}: ${message}`));
		return out;
	};
	const at = (...presets: unknown[]) => doc(...presets).map((p) => p.split(':')[0]);
	assert.deepEqual(doc(), []);
	assert.deepEqual(doc(night, { id: 'old', name: 'Old', removed: true }, { id: 'dusk', name: 'Dusk', fill: { stops: [{ at: 0, color: '#fff' }, { at: 100, color: '#000' }] } }), []);
	const shell: string[] = [];
	for (const bad of [null, [], {}, { presets: {} }]) colorationProblems(bad, (level, path, message) => shell.push(`${level} ${path.join('.')}: ${message}`));
	assert.deepEqual(shell, Array(4).fill('error : the presets document is { presets: [...] }'));
	assert.deepEqual(at(3), ['error presets.0']);
	assert.deepEqual(doc({ id: 'negative', name: 'N' }, { id: 'none', name: 'As drawn' }), [
		'error presets.0.id: "negative" is reserved for a built-in look',
		'error presets.1.id: "none" is reserved for a built-in look'
	]);
	assert.deepEqual(at({ id: 'Night', name: 'N' }, { id: '1x', name: 'X' }, { name: 'No id' }), ['error presets.0.id', 'error presets.1.id', 'error presets.2.id']);
	assert.deepEqual(doc(night, { ...night, name: 'Other' }), ['error presets.1.id: preset id "night" is used twice']);
	assert.deepEqual(at({ id: 'a', name: '' }, { id: 'b', name: '  ' }, { id: 'c' }), ['error presets.0.name', 'error presets.1.name', 'error presets.2.name']);
	assert.deepEqual(at({ id: 'a', name: 'A', removed: 'yes' }), ['error presets.0.removed']);
	assert.deepEqual(at({ id: 'a', name: 'A', fill: '#zzz', blend: 'plus-lighter', opacity: 2 }), ['error presets.0.blend', 'error presets.0.opacity', 'error presets.0.fill']);
	// two shown presets with one name warn (case and spaces aside); a deleted one never
	assert.deepEqual(doc(night, { id: 'night-2', name: ' night ' }), ['warning presets.1.name: two presets are called "night"']);
	assert.deepEqual(doc({ ...night, removed: true }, { id: 'night-2', name: 'Night' }), []);
	assert.deepEqual(at({ id: 'a', name: 'A', blur: { amount: 21 } }), ['warning presets.0.blur.amount']);
});
