// Cover lettering layouts (owner 2026-10-02, the creator's cover editor; agents/DATA.md "Cover lettering layout").
//   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { coverLetteringOn, coverLines, coverLineText, defaultCoverLines, estimateCoverFit, validateFile, type CoverLine } from '../src/index.ts';

const problems = (kind: string, data: object) => validateFile(kind, data, 'de', { file: 'x' } as never).map((p) => `${p.level} ${p.path.join('.')}`);
// covers belong to the season (owner 2026-10-02)
const entry = (cover: unknown) => problems('season', { title: 'S', description: 'D', language: 'de', number: 1, sequence: [], cover });
const line = (extra: Partial<CoverLine> = {}): CoverLine => ({ text: 'season', x: 50, y: 30, size: 12, maxWidth: 84, ...extra });

test('lettering: off, true (the default layout) and a layout per format', () => {
	assert.equal(coverLetteringOn(undefined), false);
	assert.equal(coverLetteringOn(false), false);
	assert.equal(coverLetteringOn(true), true);
	assert.equal(coverLetteringOn({}), true);
	assert.equal(coverLetteringOn([]), false);
	// true = the default layout; an object without the format = the default too; an empty list = nothing on that format
	assert.equal(coverLines(true, 'square'), null);
	assert.equal(coverLines({ portrait: [line()] }, 'square'), null);
	assert.deepEqual(coverLines({ square: [line()] }, 'square'), [line()]);
	assert.deepEqual(coverLines({ square: [] }, 'square'), []);
	assert.equal(coverLines(false, 'square'), null);
	// the default layout: the toon's name small over the season's title big in quotes, both tilted
	const lines = defaultCoverLines('square');
	assert.deepEqual(lines.map((l) => [l.text, l.quotes === true]), [['toon', false], ['season', true]]);
	assert.ok(lines[0].size < lines[1].size && lines[0].y < lines[1].y);
	assert.ok(lines.every((l) => l.rotate === -5));
});

test('a line\'s text comes from the text fields or its own text per language – never from the image', () => {
	const texts = { toon: 'Katzenpapst', season: ' Das Erwachen ' };
	assert.equal(coverLineText(line({ text: 'toon' }), texts, 'de'), 'Katzenpapst');
	assert.equal(coverLineText(line({ quotes: true }), texts, 'de'), '"Das Erwachen"');
	const custom = line({ text: 'custom', custom: { de: 'Staffel 1', en: 'Season 1', ko: '' } });
	assert.equal(coverLineText(custom, texts, 'en'), 'Season 1');
	assert.equal(coverLineText(custom, texts, 'ko'), 'Staffel 1', 'a missing translation shows the first text there is');
	assert.equal(coverLineText(line({ quotes: true }), { toon: '', season: '' }, 'de'), '', 'no text → no quotes');
});

test('the fit estimate shrinks long texts into their box (German, English, Korean)', () => {
	assert.equal(estimateCoverFit('"DAS ERWACHEN"', 12, 84), 1);
	const long = estimateCoverFit('"DAS ERWACHEN DER GROSSEN KATZE"', 12.5, 84);
	assert.ok(long < 1 && long > 0.35);
	assert.ok(estimateCoverFit('"각성"', 12.5, 84) === 1);
	assert.equal(estimateCoverFit('x'.repeat(500), 12.5, 84), 0.35, 'never below the minimum (the reader wraps then)');
});

test('validation: true stays valid, layouts are checked line by line', () => {
	assert.deepEqual(entry({ square: 'images/c.webp', lettering: true }), [], 'compatibility: lettering true');
	assert.deepEqual(entry({ square: 'images/c.webp', lettering: false }), []);
	assert.deepEqual(entry({ square: 'images/c.webp', lettering: { square: defaultCoverLines('square') } }), []);
	assert.deepEqual(entry({ square: 'images/c.webp', lettering: { square: [] } }), [], 'an empty layout = no lettering');
	assert.deepEqual(entry({ square: 'images/c.webp', lettering: 'yes' }), ['error cover.lettering']);
	assert.deepEqual(entry({ square: 'images/c.webp', lettering: [] }), ['error cover.lettering']);
	assert.deepEqual(entry({ lettering: { square: [line()] } }), ['warning cover.lettering', 'warning cover.lettering.square']);
	assert.deepEqual(entry({ square: 'images/c.webp', lettering: { square: line() } }), ['error cover.lettering.square']);
	assert.deepEqual(entry({ square: 'images/c.webp', lettering: { landscape: [] } }), ['warning cover.lettering.landscape']);
	const one = (l: object) => entry({ square: 'images/c.webp', lettering: { square: [l] } });
	assert.deepEqual(one({ ...line(), text: 'image' }), ['error cover.lettering.square.0.text']);
	assert.deepEqual(one({ ...line(), x: 120, size: 0, maxWidth: 'wide' }), ['error cover.lettering.square.0.x', 'error cover.lettering.square.0.size', 'error cover.lettering.square.0.maxWidth']);
	assert.deepEqual(one({ text: 'toon' }), ['x', 'y', 'size', 'maxWidth'].map((k) => `error cover.lettering.square.0.${k}`), 'geometry is required');
	assert.deepEqual(one(line({ rotate: -5, skew: -4, align: 'left', color: 'cyan', outline: 'ink', quotes: true })), []);
	assert.deepEqual(one(line({ rotate: 400 })), ['error cover.lettering.square.0.rotate']);
	assert.deepEqual(one({ ...line(), color: '#ff00ff' }), ['error cover.lettering.square.0.color'], 'colours from the palette only');
	assert.deepEqual(one({ ...line(), align: 'justify' }), ['error cover.lettering.square.0.align']);
	// custom texts: per language; a toon language without it is a warning
	assert.deepEqual(one(line({ text: 'custom', custom: { de: 'Staffel 1', en: 'Season 1', ko: '시즌 1' } })), []);
	assert.deepEqual(one(line({ text: 'custom', custom: { de: 'Staffel 1', en: '' } })), ['warning cover.lettering.square.0.custom'], 'an empty translation');
	assert.deepEqual(one({ ...line(), text: 'custom', custom: 'Staffel 1' }), ['error cover.lettering.square.0.custom']);
	assert.deepEqual(one(line({ custom: { de: 'x' } })), ['warning cover.lettering.square.0.custom'], 'custom on a title line is unused');
});
