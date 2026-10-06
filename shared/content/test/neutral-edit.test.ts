// Language-neutral documents in the creator (agents/NEUTRAL.md phase 2): the conversion plan the Convert page and
// scripts/dev/neutralize.mjs share, the text walker, validation of a neutral document once + per-locale completeness,
// accepted warnings with their locale, the previews' helpers.
//   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	acceptedWarningKey,
	forEachText,
	isAcceptedWarning,
	neutralizeChapter,
	planConversion,
	previewText,
	resolveSlice,
	validateNeutral,
	visibleDifferences,
	withAcceptedWarning,
	type ChapterDoc,
	type GroupDoc,
	type SeasonDoc
} from '../src/index.ts';

const L = { source: 'de', languages: ['de', 'en'] };
const rect = { x: 10, y: 10, width: 50 };

const chapterDoc = (over: Partial<ChapterDoc> = {}): ChapterDoc =>
	({
		title: { de: 'Eins', en: 'One' },
		author: 'M',
		number: 1,
		release: { de: '2025-10-04', en: '2025-10-04' },
		description: { de: 'Beschreibung', en: 'Description' },
		thumbnail: 't.webp',
		groups: [
			{
				label: 'g',
				renderer: { type: 'vertical' },
				slices: [{ image: 'a.webp', width: 390, height: 625, dialog: [{ id: 'd1', rect, text: { de: 'Hallo', en: 'Hello' } }] }],
				events: [
					{ type: 'showMessage', params: { text: { de: 'Na, du?', en: 'Well?' } } },
					{ type: 'addChoice', params: { label: { de: 'Klar', en: 'Yes' } } },
					{ type: 'showChoices' }
				]
			}
		],
		...over
	}) as unknown as ChapterDoc;

const lines = (problems: { level: string; locale: string | null; path: (string | number)[]; message: string }[]) =>
	problems.map((p) => `${p.level} ${p.locale ?? '-'} ${p.path.join('.')}`);

test('forEachText: every locale-map leaf with its path and kind', () => {
	const seen: string[] = [];
	forEachText('chapter', chapterDoc(), (path, _value, leaf) => seen.push(`${leaf} ${path.join('.')}`));
	assert.deepEqual(seen, [
		'title title',
		'release release',
		'description description',
		'dialog groups.0.slices.0.dialog.0.text',
		'message groups.0.events.0.params.text',
		'choice groups.0.events.1.params.label'
	]);
	const season: string[] = [];
	forEachText('season', { title: { de: 'S' }, sequence: [{ type: 'chapter', number: 1, status: { de: 'published' } }] }, (path, _v, leaf) => season.push(`${leaf} ${path.join('.')}`));
	assert.deepEqual(season, ['title title', 'status sequence.0.status']);
});

test('validateNeutral: a complete chapter has no problems', () => {
	assert.deepEqual(lines(validateNeutral('chapter', chapterDoc(), L, { file: 'chapter:x:1', seasonStatus: 'published' })), []);
});

test('validateNeutral: missing / untranslated / long texts per locale, the structure once', () => {
	const doc = chapterDoc();
	(doc.title as Record<string, string>).en = '';
	const group = doc.groups[0] as GroupDoc;
	(group.slices![0].dialog![0].text as Record<string, string>).en = 'Hello there, this is far too long';
	(group.events![0] as any).params.text = { de: 'Na, du?' };
	(group.events![1] as any).params.label = { de: 'Ja', en: 'Ja' };
	delete (group.slices![0] as any).image;
	const problems = validateNeutral('chapter', doc, L, { file: 'chapter:x:1', seasonStatus: 'published' });
	assert.deepEqual(lines(problems), [
		'error - groups.0.slices.0.image',
		'warning en title',
		'warning en groups.0.slices.0.dialog.0.text',
		'warning en groups.0.events.0.params.text',
		'info en groups.0.events.1.params.label'
	]);
	assert.match(problems.find((p) => p.path.join('.') === 'groups.0.events.0.params.text')!.message, /^missing en text for "Na, du\?"$/);
	assert.match(problems[1].message, /^missing en title$/);
});

test('validateNeutral: shapes of the locale maps, a source without text, season statuses per locale', () => {
	const doc = chapterDoc({ title: 'Eins' as never, release: { de: '2025-10-04', en: 'soon' } });
	(doc.description as Record<string, string>).de = '';
	const problems = lines(validateNeutral('chapter', doc, L, { file: 'c', seasonStatus: null }));
	assert.ok(problems.includes('error - title'), 'a plain string title is not a locale map');
	assert.ok(problems.includes('error en release'));
	assert.ok(problems.includes('warning de description'), 'the source has no text, en has one');
	assert.ok(problems.includes('error - status'), 'not in the season');
	const season = { title: { de: 'S', en: 'S1' }, description: { de: 'P', en: 'Plot' }, number: 1, sequence: [{ type: 'chapter', number: 1, status: { de: 'published', en: 'later' } }] } as unknown as SeasonDoc;
	assert.deepEqual(lines(validateNeutral('season', season, L, { file: 's' })), ['error en sequence.0.status']);
});

test('validateNeutral: a scene (group)', () => {
	const scene = { label: 'scene-1', renderer: { type: 'vertical' }, slices: [], events: [{ type: 'showMessage', params: { text: { de: 'Hi' } } }] } as unknown as GroupDoc;
	assert.deepEqual(lines(validateNeutral('group', scene, L, { file: 'scene:x:1:scene-1' })), ['warning en events.0.params.text']);
});

test('accepted warnings in a neutral document keep their locale', () => {
	const doc = chapterDoc();
	const warning = { path: ['title'], message: 'missing en title' };
	const en = { ...warning, scope: 'en' };
	assert.match(acceptedWarningKey(en, doc), /^en:title#/);
	assert.equal(acceptedWarningKey(warning, doc).startsWith('title#'), true, 'without a scope: the per-locale form');
	const list = withAcceptedWarning(en, doc, true);
	const accepted = { ...doc, accepted_warnings: list };
	assert.equal(isAcceptedWarning(en, accepted), true);
	assert.equal(isAcceptedWarning({ ...warning, scope: 'fr' }, accepted), false, 'another locale is not accepted');
	(accepted.title as Record<string, string>).en = 'One!';
	assert.equal(isAcceptedWarning(en, accepted), false, 'the value changed: the warning is back');
});

test('planConversion: published + newer draft, unchanged steps, the toon order', () => {
	const de = { title: 'Eins', number: 1, language: 'de', release: '2025-10-04', groups: [] };
	const en = { ...de, title: 'One', language: 'en' };
	const enDraft = { ...en, title: 'One!' };
	const input = { kind: 'chapter' as const, source: 'de', languages: ['de', 'en'], published: { en: { id: 2, content: en }, de: { id: 1, content: de } }, newest: { de: { id: 1, content: de }, en: { id: 3, content: enDraft } } };
	const steps = planConversion(input);
	assert.deepEqual(
		steps.map((s) => [s.status, s.versions, s.unchanged, (s.doc as ChapterDoc).title]),
		[
			['published', 'de#1 en#2', false, { de: 'Eins', en: 'One' }],
			['draft', 'de#1 en#3', false, { de: 'Eins', en: 'One!' }]
		]
	);
	const again = planConversion({ ...input, neutralPublished: JSON.parse(JSON.stringify(steps[0].doc)), neutralNewest: JSON.parse(JSON.stringify(steps[1].doc)) });
	assert.deepEqual(again.map((s) => s.unchanged), [true, true], 'idempotent');
	const never = planConversion({ ...input, published: {} });
	assert.deepEqual(never.map((s) => s.status), ['draft'], 'never published: a draft only');
});

test('visibleDifferences: what readers see, not the deprecated fields', () => {
	const de = { title: 'Eins', number: 1, language: 'de', release: '2025-10-04', groups: [{ label: 'g', interaction: 'x', slices: [{ image: 'a.webp', dialog: [{ rect, text: 'Hallo', size: 4 }] }] }] };
	const en = { ...de, language: 'en', groups: [{ label: 'g', slices: [{ image: 'a.webp', dialog: [{ rect, text: '', size: 5 }] }] }] };
	const { report } = neutralizeChapter({ de, en } as never, 'de');
	assert.deepEqual(
		visibleDifferences(report).map((r) => `${r.kind} ${r.path}`),
		['text-missing groups.0.slices.0.dialog.0.text', 'differs groups.0.slices.0.dialog.0.size']
	);
	assert.equal(report.some((r) => r.path === 'groups.0.interaction'), true, 'the report itself has it');
});

test('resolveSlice / previewText: the creator canvas and list lines', () => {
	const slice = (chapterDoc().groups[0] as GroupDoc).slices![0];
	assert.equal(resolveSlice(slice, 'en', L).dialog![0].text, 'Hello');
	assert.equal(resolveSlice(slice, 'fr', L).dialog![0].text, 'Hallo', 'falls back to the source');
	assert.equal(previewText({ de: ['a', 'b'] }, ['en', 'de']), 'a b');
	assert.equal(previewText('plain'), 'plain');
	assert.equal(previewText({ en: '' }), '');
});
