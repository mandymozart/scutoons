// The toon screenplay (entity `script:<toon>:<l>`, agents/DATA.md "Script").   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { entities, entityKeyFor, isLocalizedEntity, parseEntityKey, scriptOutline, validateFile, validateLocales } from '../src/index.ts';

const problems = (data: unknown) => validateFile('script', data, 'de', { file: 'script:catpope' } as never).map((p) => `${p.level} ${p.path.join('.')}`);

test('script entity: per toon, per language', () => {
	assert.equal(entities.script('catpope'), 'script:catpope');
	assert.equal(isLocalizedEntity('script:catpope'), true);
	assert.equal(entityKeyFor('script:catpope', 'de'), 'script:catpope:de');
	assert.deepEqual(parseEntityKey('script:catpope:en'), { kind: 'script', toon: 'catpope', locale: 'en' });
	assert.equal(parseEntityKey('script:catpope:1:de'), null, 'no chapter in a script key (scripts come per toon)');
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
	assert.deepEqual(validateLocales('script', 'script:catpope', files, 'de'), []);
	assert.deepEqual(
		validateLocales('script', 'script:catpope', { ...files, en: { text: '  ' } }, 'de').map((p) => `${p.level} ${p.locale} ${p.path.join('.')}`),
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
