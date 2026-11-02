// The Kanon texts and the season / chapter scripts (research/KANON.md, #80).   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	entities,
	entityKeyFor,
	isLocalizedEntity,
	kanonHeadings,
	kanonProblems,
	kanonSection,
	kanonSubjectSegment,
	kanonTemplate,
	parseEntityKey,
	parseKanonSubject,
	validateFile,
	validateLocales
} from '../src/index.ts';

test('season and chapter scripts: per toon and number, per language', () => {
	assert.equal(entities.seasonScript('catpope', 1), 'season-script:catpope:1');
	assert.equal(entityKeyFor(entities.chapterScript('toptoy', 0), 'de'), 'chapter-script:toptoy:0:de');
	assert.deepEqual(parseEntityKey('season-script:catpope:1:de'), { kind: 'season-script', toon: 'catpope', number: 1, locale: 'de' });
	assert.deepEqual(parseEntityKey('chapter-script:toptoy:0'), { kind: 'chapter-script', toon: 'toptoy', number: 0 });
	// seasons start at 1, chapters at 0
	assert.equal(parseEntityKey('season-script:catpope:0:de'), null);
	const ok = validateFile('season-script', { text: '# 1. Kapitel\n', meta: { title: 'X' } }, 'de', { file: 'season-script:catpope:1' } as never);
	assert.deepEqual(ok, []);
	assert.ok(validateFile('chapter-script', { text: 3 }, 'de', { file: 'chapter-script:toptoy:1' } as never).some((p) => p.level === 'error'));
});

test('rig documents (#258): instruments, presets with dots, rigs by subject – one segment', () => {
	assert.deepEqual(parseEntityKey('rig:character.catpope'), { kind: 'rig', id: 'character.catpope' });
	assert.deepEqual(parseEntityKey('preset:cathedral_voice.bright'), { kind: 'preset', id: 'cathedral_voice.bright' });
	assert.deepEqual(parseEntityKey('instrument:drone'), { kind: 'instrument', id: 'drone' });
	assert.deepEqual(parseEntityKey('module:walk'), { kind: 'module', id: 'walk' }); // #281
	assert.equal(parseEntityKey('rig:character:catpope'), null);
});

test('kanon keys: one subject segment, then the locale', () => {
	assert.equal(isLocalizedEntity('kanon:character.catpope'), true);
	assert.equal(entityKeyFor(entities.kanon('place.cathedral'), 'en'), 'kanon:place.cathedral:en');
	assert.deepEqual(parseEntityKey('kanon:character.catpope.human.helmet:de'), { kind: 'kanon', id: 'character.catpope.human.helmet', locale: 'de' });
	assert.deepEqual(parseEntityKey('kanon:toon.toptoy:de'), { kind: 'kanon', id: 'toon.toptoy', toon: 'toptoy', locale: 'de' });
	assert.deepEqual(parseEntityKey('kanon:place.cathedral'), { kind: 'kanon', id: 'place.cathedral' });
	for (const bad of ['kanon:toon.scutoons:de', 'kanon:place.a.b:de', 'kanon:character.a.b.c.d:de', 'kanon:item.x:de', 'kanon:character.catpope:de:x', 'kanon:place.Cathedral']) {
		assert.equal(parseEntityKey(bad), null, bad);
	}
	const s = parseKanonSubject('character.catpope.human');
	assert.deepEqual(s, { kind: 'character', character: 'catpope', persona: 'human' });
	assert.equal(kanonSubjectSegment(s!), 'character.catpope.human');
});

test('kanon sections: headings in de and en, a section body, the rules as warnings', () => {
	const text = 'Intro.\n\n## Vorgeschichte\nBorn in a cave.\n\n## look\nPale, big ears.\n';
	assert.deepEqual(kanonHeadings(text, 'character').map((h) => [h.line, h.id]), [[3, 'backstory'], [6, 'look']]);
	assert.equal(kanonSection(text, 'character', 'look'), 'Pale, big ears.');
	assert.equal(kanonSection(text, 'character', 'who'), '');
	assert.deepEqual(kanonProblems(text, 'character'), []);
	const bad = '## Look\n\n## Backstory\n\n## Look\n\n## Hobbies\n';
	assert.deepEqual(kanonProblems(bad, 'character').map((p) => p.line), [3, 5, 7]);
	assert.equal(kanonTemplate('place', 'de'), '## Vorgeschichte\n\n## Szenerie\n\n## Regeln\n\n');
	const warnings = validateFile('kanon', { text: bad }, 'en', { file: 'kanon:character.catpope' } as never);
	assert.equal(warnings.filter((p) => p.level === 'warning').length, 3);
	assert.ok(validateFile('kanon', { text: null }, 'en', { file: 'kanon:place.cathedral' } as never).some((p) => p.level === 'error'));
	// an empty translation is worth a note, a partial one is not
	const notes = validateLocales('kanon', 'kanon:place.cathedral', { de: { text: 'x' }, en: { text: '' } }, 'de');
	assert.equal(notes.length, 1);
});
