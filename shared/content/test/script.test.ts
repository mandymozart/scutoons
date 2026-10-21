// The screenplays: a season's and a chapter's (entities `season-script:` / `chapter-script:<toon>:<n>:<l>`, agents/DATA.md
// "Script", research/KANON.md §3).   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { chapterScriptEntities, entities, entityKeyFor, isLocalizedEntity, parseEntityKey, scriptOutline, validateFile, validateLocales } from '../src/index.ts';

const problems = (data: unknown) => validateFile('season-script', data, 'de', { file: 'season-script:catpope:1' } as never).map((p) => `${p.level} ${p.path.join('.')}`);

test('script entities: per season or chapter, per language; the toon script is gone', () => {
	assert.equal(entities.seasonScript('catpope', 1), 'season-script:catpope:1');
	assert.equal(isLocalizedEntity('season-script:catpope:1'), true);
	assert.equal(entityKeyFor('season-script:catpope:1', 'de'), 'season-script:catpope:1:de');
	assert.deepEqual(parseEntityKey('season-script:catpope:1:en'), { kind: 'season-script', toon: 'catpope', number: 1, locale: 'en' });
	assert.equal(parseEntityKey('script:catpope:de'), null, 'the toon script became season 1 (2026-10-21)');
	assert.deepEqual(chapterScriptEntities('toptoy', 3, 1), ['chapter-script:toptoy:3', 'season-script:toptoy:1']);
	assert.deepEqual(chapterScriptEntities('toptoy', 3, null), ['chapter-script:toptoy:3']);
});

test('script validation: { text, meta? }', () => {
	assert.deepEqual(problems({ text: '' }), []);
	assert.deepEqual(problems({ text: '    # 1. Kapitel\n', meta: { title: 'T', author: 'A', status: 'ENTWURF', date: 'November 2024', basedOn: 'B', basedOnFrom: 'M' } }), []);
	assert.deepEqual(problems({}), ['error text']);
	assert.deepEqual(problems({ text: 3 }), ['error text']);
	assert.deepEqual(problems('text'), ['error ']);
	assert.deepEqual(problems({ text: '', meta: 'x' }), ['error meta']);
	assert.deepEqual(problems({ text: '', meta: { title: 1, layout: {} } }), ['error meta.title', 'warning meta.layout']);
});

test('script translations: partial is fine, empty is a note', () => {
	const files = { de: { text: 'A\nB' }, en: { text: 'A' } };
	assert.deepEqual(validateLocales('season-script', 'season-script:catpope:1', files, 'de'), []);
	assert.deepEqual(
		validateLocales('season-script', 'season-script:catpope:1', { ...files, en: { text: '  ' } }, 'de').map((p) => `${p.level} ${p.locale} ${p.path.join('.')}`),
		['info en text']
	);
});

const SCRIPT = [
	'    # 0. Kapitel',
	'',
	'    # ALPTRAUM',
	'    # (Transformation Exorcismus)',
	'',
	'0   BOOK',
	'',
	'    Action.',
	'1   INT./EXT. VERSCHIEDENE ORTE - TAG & NACHT',
	'---',
	'    # 1. Lied',
	'',
	'    # ROAR LITTLE KITTY      ',
	'',
	'    1. Strophe  You feel like you',
	'2   NOT A SCENE (song lyrics)',
	'---',
	'3   NOT A SCENE (no chapter after the song)',
	'                            # 3. Lied',
	'---',
	'    # 1. Chapter',
	'',
	'    # DENIAL',
	'',
	'2   INT. SCHLAFZIMMER - NACHT',
	'',
	'                            PLF',
	'                        (schrie)',
	'                MAMA! WTF?!'
].join('\n');

test('script outline: chapters, scenes, songs with their lines (screenplay editor rules)', () => {
	assert.deepEqual(scriptOutline(SCRIPT), [
		{ kind: 'chapter', line: 1, number: 0, heading: '', title: 'ALPTRAUM', subtitle: '(Transformation Exorcismus)' },
		{ kind: 'scene', line: 6, number: 0, heading: 'BOOK' },
		{ kind: 'scene', line: 9, number: 1, heading: 'INT./EXT. VERSCHIEDENE ORTE - TAG & NACHT' },
		{ kind: 'song', line: 11, number: 1, heading: '', title: 'ROAR LITTLE KITTY' },
		{ kind: 'song', line: 19, number: 3, heading: '' },
		{ kind: 'chapter', line: 21, number: 1, heading: '', title: 'DENIAL' },
		{ kind: 'scene', line: 25, number: 2, heading: 'INT. SCHLAFZIMMER - NACHT' }
	]);
	assert.deepEqual(scriptOutline(''), []);
});

test('a chapter script has no chapter markers: its scenes count from the first line', () => {
	const text = ['1   INT. KÜCHE - NACHT', '', 'PLF kocht.', '', '2   EXT. GARTEN - TAG', ''].join(String.fromCharCode(10));
	assert.deepEqual(scriptOutline(text).map((o) => o.kind), []);
	assert.deepEqual(scriptOutline(text, { chapter: true }).map((o) => `${o.kind} ${o.number} ${o.heading}`), ['scene 1 INT. KÜCHE - NACHT', 'scene 2 EXT. GARTEN - TAG']);
});
