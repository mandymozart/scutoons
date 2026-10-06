// Per-locale documents → one language-neutral document (agents/NEUTRAL.md "Migration"), and back: resolving the
// neutral document gives each locale's document again where the locales agreed.
//   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { neutralizeChapter, neutralizeGroup, neutralizeSeason, resolveChapter, resolveGroup, resolveSeason, type Chapter, type Group, type Season } from '../src/index.ts';

const L = { source: 'de', languages: ['de', 'en'] };
const slice = (text: unknown, extra: object = {}) => ({ image: 'a.webp', width: 390, height: 625, dialog: [{ id: 'd1', rect: { x: 1, y: 2, width: 50 }, text, size: 4 }], ...extra });
const chapter = (l: string, over: object = {}): Chapter =>
	({
		title: l === 'de' ? 'Eins' : 'One',
		author: 'M',
		number: 1,
		language: l,
		release: '2025-10-04',
		status: 'published',
		description: l === 'de' ? 'Beschreibung' : 'Description',
		thumbnail: 't.jpg',
		keyvisual: 'kv.webp',
		groups: [
			{ ref: { src: 'chapter1/scene-1.json' } },
			{
				label: 'g',
				slices: [slice(l === 'de' ? 'Hallo' : 'Hello'), slice(l === 'de' ? ['a', 'b'] : 'ab')],
				events: [
					{ type: 'showMessage', params: { text: l === 'de' ? 'Na?' : 'Well?', character: 'catpope' } },
					{ type: 'addChoice', params: { label: l === 'de' ? 'Ja' : 'Yes' } }
				]
			}
		],
		...over
	}) as unknown as Chapter;
const kinds = (report: { kind: string; path: string }[]) => report.map((r) => `${r.kind} ${r.path}`);

test('chapter: texts become locale maps, language + status are dropped, the rest stays once', () => {
	const { doc, report, locales } = neutralizeChapter({ de: chapter('de'), en: chapter('en') }, 'de');
	assert.deepEqual(report, []);
	assert.deepEqual(locales, ['de', 'en']);
	assert.deepEqual(doc.title, { de: 'Eins', en: 'One' });
	assert.deepEqual(doc.release, { de: '2025-10-04', en: '2025-10-04' });
	assert.equal('language' in doc, false);
	assert.equal('status' in doc, false);
	assert.equal(doc.keyvisual, 'kv.webp', 'a legacy keyvisual string stays a string');
	const g = doc.groups[1];
	assert.deepEqual(g.slices![1].dialog![0].text, { de: ['a', 'b'], en: 'ab' }, 'string vs string[] per locale');
	assert.deepEqual((g.events![0] as any).params, { text: { de: 'Na?', en: 'Well?' }, character: 'catpope' });
	assert.deepEqual((g.events![1] as any).params.label, { de: 'Ja', en: 'Yes' });
	// and back: every locale's document again (with the season's status)
	for (const l of ['de', 'en']) assert.deepEqual(resolveChapter(doc, l, L, 'published' as never), chapter(l));
});

test('chapter: a missing locale, a per-locale image and text-box size (source wins, reported), no source', () => {
	const en = chapter('en');
	const g = en.groups[1];
	g.slices![0] = slice('Hello', { image: 'a-en.webp' }) as never;
	(g.slices![1].dialog![0] as any).size = 3.5;
	const { doc, report } = neutralizeChapter({ de: chapter('de'), en }, 'de');
	assert.deepEqual(kinds(report), ['differs groups.1.slices.0.image', 'differs groups.1.slices.1.dialog.0.size']);
	assert.deepEqual(report[0].values, { de: 'a.webp', en: 'a-en.webp' });
	assert.equal(doc.groups[1].slices![0].image, 'a.webp');
	const only = neutralizeChapter({ de: chapter('de') }, 'de');
	assert.deepEqual(only.report, []);
	assert.deepEqual(only.doc.title, { de: 'Eins' });
	const sourceless = neutralizeChapter({ en: chapter('en') }, 'de');
	assert.equal(sourceless.base, 'en');
	assert.deepEqual(kinds(sourceless.report), ['source-missing ']);
	assert.deepEqual(sourceless.doc.title, { en: 'One' });
});

test("chapter: structure mismatches – the source wins, the other locale's texts there are reported, never guessed", () => {
	const en = chapter('en');
	en.groups[1].slices!.push(slice('Extra') as never);
	(en.groups[1].events![1] as any).type = 'showMessage';
	(en.groups[1].events![1] as any).params = { text: 'Other' };
	const { doc, report } = neutralizeChapter({ de: chapter('de'), en }, 'de');
	assert.deepEqual(kinds(report), [
		'text-lost groups.1.slices.0.dialog.0.text',
		'text-lost groups.1.slices.1.dialog.0.text',
		'text-lost groups.1.slices.2.dialog.0.text',
		'structure groups.1.slices',
		'structure groups.1.events.1',
		'text-lost groups.1.events.1.params.text'
	]);
	assert.deepEqual(report[3].values, { de: 2, en: 3 });
	assert.deepEqual(report.filter((r) => r.kind === 'text-lost').map((r) => r.values.en), ['Hello', 'ab', 'Extra', 'Other']);
	assert.deepEqual(report[4].values, { de: 'addChoice', en: 'showMessage' });
	assert.deepEqual(doc.groups[1].slices![0].dialog![0].text, { de: 'Hallo' }, 'mismatched slices: no en texts taken');
	assert.deepEqual((doc.groups[1].events![1] as any).params.label, { de: 'Ja' });
	assert.deepEqual((doc.groups[1].events![0] as any).params.text, { de: 'Na?', en: 'Well?' }, 'the matching event keeps its en text');
});

test("chapter: blank and missing texts are reported (they show the source's text)", () => {
	const en = chapter('en', { description: '' });
	delete (en.groups[1].slices![0].dialog![0] as any).text;
	const { doc, report } = neutralizeChapter({ de: chapter('de'), en }, 'de');
	assert.deepEqual(kinds(report), ['text-missing description', 'text-missing groups.1.slices.0.dialog.0.text']);
	assert.deepEqual(report[0].values, { en: '' });
	assert.deepEqual(report[1].values, { en: null });
	assert.equal(resolveChapter(doc, 'en', L).description, 'Beschreibung');
	assert.equal(resolveChapter(doc, 'en', L).groups[1].slices![0].dialog![0].text, 'Hallo');
});

test('scene: a field only another locale has is dropped (source wins) and reported; accepted warnings are kept', () => {
	const de = { label: 'scene-14', renderer: { type: 'vertical' }, slices: [slice('BUMM')], accepted_warnings: ['a#1'] } as unknown as Group;
	const en = { ...de, interaction: 'webtoon', slices: [slice('BOOM')], accepted_warnings: ['b#2'] } as unknown as Group;
	const { doc, report } = neutralizeGroup({ de, en }, 'de');
	assert.deepEqual(kinds(report), ['differs interaction']);
	assert.deepEqual(report[0].missing, ['de']);
	assert.equal('interaction' in doc, false);
	assert.deepEqual(doc.accepted_warnings, ['a#1', 'b#2']);
	assert.deepEqual(resolveGroup(doc, 'en', L).slices![0].dialog![0].text, 'BOOM');
});

test('season: status per locale, neutral cover, per-locale cover difference reported', () => {
	const season = (l: string, status: string, cover = 's1/c.jpg'): Season =>
		({
			title: l,
			language: l,
			number: 1,
			release: '2025-10-04',
			description: `d ${l}`,
			cover: { square: cover },
			sequence: [
				{ type: 'chapter', number: 1, status: 'published' },
				{ type: 'chapter', number: 2, status }
			]
		}) as Season;
	const { doc, report } = neutralizeSeason({ de: season('de', 'published'), en: season('en', 'coming-soon', 's1/c-en.jpg') }, 'de');
	assert.deepEqual(kinds(report), ['differs cover.square']);
	assert.deepEqual(doc.sequence[1].status, { de: 'published', en: 'coming-soon' });
	assert.equal('language' in doc, false);
	assert.deepEqual(resolveSeason(doc, 'en', L), season('en', 'coming-soon'));
});
