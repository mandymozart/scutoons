// Season addresses, labels and frozen positions (BACKLOG 10.41).
//   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { chapterAt, chapterAtPosition, chapterLabel, chapterPlace, chapterPosition, countedChapters, frozenSequenceProblems, validateNeutral } from '../src/index.ts';

const seq = (...numbers: number[]) => numbers.map((number) => ({ number }));
// season 1 = the prologue + 1–5, season 2 = 6–8
const seasons = [
	{ number: 2, sequence: seq(6, 7, 8) },
	{ number: 1, sequence: seq(0, 1, 2, 3, 4, 5) }
];

test('positions: the prologue is not counted, every other stored entry is', () => {
	assert.deepEqual(countedChapters(seq(0, 1, 2)), [1, 2]);
	assert.deepEqual(countedChapters([{ number: '3' }, null, { number: 0 }, { number: 4 }]), [3, 4]);
	assert.equal(chapterAtPosition(seq(0, 1, 2, 3), 2), 2);
	assert.equal(chapterAtPosition(seq(0, 1, 2, 3), 0), null);
	assert.equal(chapterAtPosition(seq(0, 1, 2, 3), 4), null);
	assert.equal(chapterPosition(seq(0, 1, 2, 3), 0), null);
	assert.equal(chapterPosition(seq(0, 3, 1), 1), 2);
});

test('season/S/chapter/K ↔ chapter/N', () => {
	assert.equal(chapterAt(seasons, 1, 2), 2);
	assert.equal(chapterAt(seasons, 2, 1), 6);
	assert.equal(chapterAt(seasons, 2, 4), null);
	assert.equal(chapterAt(seasons, 3, 1), null);
	assert.deepEqual(chapterPlace(seasons, 6), { season: 2, position: 1 });
	assert.deepEqual(chapterPlace(seasons, 0), { season: 1, position: null });
	assert.equal(chapterPlace(seasons, 42), null);
});

test('labels: the season only with two or more seasons, the position, never the absolute number', () => {
	assert.deepEqual(chapterLabel(seasons, 7), { prologue: false, season: 2, chapter: 2 });
	assert.deepEqual(chapterLabel(seasons, 3), { prologue: false, season: 1, chapter: 3 });
	assert.deepEqual(chapterLabel(seasons, 0), { prologue: true, season: null, chapter: 0 });
	assert.deepEqual(chapterLabel([seasons[1]], 3), { prologue: false, season: null, chapter: 3 });
	// a chapter no season lists keeps its number
	assert.deepEqual(chapterLabel([seasons[1]], 9), { prologue: false, season: null, chapter: 9 });
});

const published = [
	{ number: 0, status: { de: 'published', en: 'published' } },
	{ number: 1, status: { de: 'published', en: 'published' } },
	{ number: 2, status: { de: 'coming-soon', en: 'draft' } },
	{ number: 3, status: { de: 'draft', en: 'draft' } },
	{ number: 4, status: { de: 'draft', en: 'draft' } }
];

test('frozen positions: appending, status changes, the prologue and drafts behind the listed ones are fine', () => {
	assert.deepEqual(frozenSequenceProblems(published, seq(0, 1, 2, 3, 4, 5)), []);
	assert.deepEqual(frozenSequenceProblems(published, seq(0, 1, 2, 4, 3)), []);
	assert.deepEqual(frozenSequenceProblems(published, seq(1, 0, 2, 3)), []);
	assert.deepEqual(frozenSequenceProblems(published, seq(0, 1, 2)), []);
	assert.deepEqual(frozenSequenceProblems(null, seq(2, 1)), []);
});

test('frozen positions: a listed chapter moved, removed or something inserted before it is an error', () => {
	const moved = frozenSequenceProblems(published, seq(0, 2, 1, 3, 4));
	assert.equal(moved.length, 2);
	assert.deepEqual(moved[0].path, ['sequence', 2]);
	assert.match(moved[0].message, /chapter 1 moved from position 1 to 2/);
	const removed = frozenSequenceProblems(published, seq(0, 1, 3, 4));
	assert.equal(removed.length, 1);
	assert.match(removed[0].message, /chapter 2 .*removed/);
	// a draft inserted before a coming-soon chapter shifts it
	const inserted = frozenSequenceProblems(published, seq(0, 1, 9, 2, 3, 4));
	assert.deepEqual(inserted.map((p) => p.path), [['sequence', 3]]);
	// listed in one locale is enough; a plain string status (older documents) too
	assert.equal(frozenSequenceProblems([{ number: 1, status: 'coming-soon' }, { number: 2 }], seq(2, 1)).length, 2);
});

test('the validator reports frozen positions as errors of the season (Problems panel)', () => {
	const languages = { source: 'de', languages: ['de', 'en'] };
	const doc = { title: { de: 'S', en: 'S' }, description: { de: 'd', en: 'e' }, sequence: [published[0], published[2], published[1]] };
	const errors = validateNeutral('season', doc, languages, { file: 'season:t:1', publishedSequence: published }).filter((p) => p.level === 'error');
	assert.equal(errors.length, 2);
	assert.ok(errors.every((p) => p.locale === null && p.path[0] === 'sequence'));
	// never published: no check
	assert.equal(validateNeutral('season', doc, languages, { file: 'season:t:1' }).filter((p) => p.message.includes('position')).length, 0);
});
