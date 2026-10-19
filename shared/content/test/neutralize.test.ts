// Per-locale documents → one language-neutral document (agents/NEUTRAL.md "Migration"), and back: resolving the
// neutral document gives each locale's document again where the locales agreed.
//   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	foldToonName,
	hasToonTexts,
	neutralizeChapter,
	neutralizeSeason,
	neutralizeToon,
	planToonConversion,
	resolveChapter,
	resolveSeason,
	resolveToon,
	toonLanguages,
	toonName,
	visibleDifferences,
	type Chapter,
	type Season,
	type ToonIndex,
	type ToonSettings
} from '../src/index.ts';

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
				{ number: 1, status: 'published' },
				{ number: 2, status }
			]
		}) as Season;
	const { doc, report } = neutralizeSeason({ de: season('de', 'published'), en: season('en', 'coming-soon', 's1/c-en.jpg') }, 'de');
	assert.deepEqual(kinds(report), ['differs cover.square']);
	assert.deepEqual(doc.sequence[1].status, { de: 'published', en: 'coming-soon' });
	assert.equal('language' in doc, false);
	assert.deepEqual(resolveSeason(doc, 'en', L), season('en', 'coming-soon'));
});

// ── Toon settings → the toon entry (agents/NEUTRAL.md step 4) ──────────────────────────────────────────────────────

const entry = (over: object = {}): ToonIndex =>
	({ id: 't', uri: 't', status: 'published', sourceLanguage: 'de', languages: ['de', 'en'], i18n: { de: { name: 'Katzenpapst', description: '' }, en: { name: 'Cat Pope' } }, avatar: 'a.webp', authors: ['A'], ...over }) as ToonIndex;
const settings = (l: string, over: object = {}): ToonSettings =>
	({ id: 't', title: l === 'de' ? 'Katzenpapst' : 'Cat Pope', language: l, release: l === 'de' ? '2025-10-04' : '2025-12-01', status: 'published', description: `d ${l}`, ...over }) as ToonSettings;

test('toon: the settings become the entry\'s locale maps, resolving gives each language\'s settings back', () => {
	const { doc, report } = neutralizeToon(entry(), { de: settings('de'), en: settings('en', { status: 'draft' }) }, 'de');
	assert.deepEqual(report, []);
	assert.deepEqual(doc.title, { de: 'Katzenpapst', en: 'Cat Pope' });
	assert.deepEqual(doc.release, { de: '2025-10-04', en: '2025-12-01' });
	assert.deepEqual(doc.languageStatus, { de: 'published', en: 'draft' });
	assert.equal('language' in doc, false);
	assert.equal('i18n' in doc, false, 'the old name is folded into title (decision 4)');
	assert.equal(doc.status, 'published', "the whole toon's status is the entry's own");
	assert.deepEqual(doc.authors, ['A']);
	assert.equal(hasToonTexts(doc), true);
	for (const l of ['de', 'en']) assert.deepEqual(resolveToon(doc, l, L), settings(l, l === 'en' ? { status: 'draft' } : {}));
});

test('toon: leftovers – neutral ones the entry has are dropped (invisible), others moved; cover / download / id reported', () => {
	const de = settings('de', { id: 'x', authors: ['old'], thumbnail: 't.webp', cover: { square: 'c.webp' }, download: { music: 'm.zip' }, accepted_warnings: ['title#1'] });
	const { doc, report } = neutralizeToon(entry({ accepted_warnings: ['map#2'] }), { de, en: settings('en', { thumbnail: 't-en.webp' }) }, 'de');
	const fields = ['id', 'authors', 'thumbnail', 'cover', 'download'];
	// first the languages against each other (the source wins), then what the entry does with the source's leftovers
	assert.deepEqual(kinds(report), [...fields, ...fields].map((f) => `differs ${f}`));
	const by = (path: string) => report.filter((r) => r.path === path)[1];
	assert.deepEqual(by('id').values, { entry: 't', de: 'x' });
	assert.deepEqual(by('authors').values, { entry: ['A'], de: ['old'] });
	assert.equal(by('authors').invisible, true, "the reader always took the entry's");
	assert.equal(by('thumbnail').invisible, true, 'moved into the entry: the reader keeps it');
	assert.deepEqual(by('thumbnail').missing, ['entry']);
	assert.equal(doc.thumbnail, 't.webp');
	assert.deepEqual(doc.authors, ['A']);
	assert.equal(by('cover').invisible, undefined, 'the reader showed the deprecated cover');
	assert.equal(by('download').invisible, true);
	assert.equal('cover' in doc, false);
	assert.deepEqual(doc.accepted_warnings, ['map#2', 'title#1']);
	assert.deepEqual(visibleDifferences(report).map((r) => r.path), ['id', 'thumbnail', 'cover', 'id', 'cover'], "not authors (the entry's), not download (ignored)");
});

test('toon: the old name i18n.<l>.name – fills a language without a settings title, else reported (the title wins); i18n dropped', () => {
	const { doc, report } = neutralizeToon(
		entry({ i18n: { de: { name: 'Katzenpapst (alt)', description: 'old' }, en: { name: 'Cat Pope' }, ko: { name: '고양이' } } }),
		{ de: settings('de'), en: settings('en', { title: ' ' }) },
		'de'
	);
	assert.deepEqual(doc.title, { de: 'Katzenpapst', en: 'Cat Pope', ko: '고양이' });
	assert.equal('i18n' in doc, false);
	const names = report.filter((r) => r.path.startsWith('i18n.'));
	assert.deepEqual(names.map((r) => [r.path, r.values, r.invisible ?? false]), [
		['i18n.de.name', { entry: 'Katzenpapst (alt)', de: 'Katzenpapst' }, false],
		['i18n.de.description', { entry: 'old' }, true]
	]);
	assert.deepEqual(foldToonName(entry({ i18n: undefined })).report, []);
	const plain = { ...entry() } as Partial<ToonIndex>;
	delete plain.i18n;
	assert.deepEqual(foldToonName(plain as ToonIndex).doc, plain, 'no i18n: unchanged');
});

test('toonName: the title (locale → source → any), older entries their i18n name, else the id; the languages from title keys', () => {
	assert.equal(toonName({ id: 't', title: { de: 'Katzenpapst', en: 'Cat Pope' }, sourceLanguage: 'de' }, 'en'), 'Cat Pope');
	assert.equal(toonName({ id: 't', title: { de: 'Katzenpapst', en: ' ' }, sourceLanguage: 'de' }, 'en'), 'Katzenpapst');
	assert.equal(toonName({ id: 't', title: { en: 'Cat Pope' }, sourceLanguage: 'de' }, 'fr'), 'Cat Pope');
	assert.equal(toonName({ id: 't', title: { de: 'Katzenpapst' }, i18n: { en: { name: 'Old' } } }, 'en'), 'Katzenpapst', 'title wins');
	assert.equal(toonName({ id: 't', i18n: { de: { name: 'Katzenpapst' }, en: { name: 'Cat Pope' } } }, 'en'), 'Cat Pope');
	assert.equal(toonName({ id: 't' }, 'de'), 't');
	assert.equal(toonName(null, 'de'), '');
	assert.deepEqual(toonLanguages({ title: { en: 'Cat Pope', de: 'Katzenpapst' } }), { source: 'en', languages: ['en', 'de'] });
	assert.deepEqual(toonLanguages({ i18n: { de: {}, en: {} } }), { source: 'de', languages: ['de', 'en'] });
});

test('toon: an entry with texts that still stores i18n – only the name is folded (published + newer draft); idempotent', () => {
	const v = (id: number, content: unknown) => ({ id, content });
	const converted = entry({ title: { de: 'Katzenpapst' }, languageStatus: { de: 'published' } });
	const input = { source: 'de', languages: ['de', 'en'], entryPublished: v(1, converted), entryNewest: v(2, { ...converted, announce: true }), published: {}, newest: {} };
	const steps = planToonConversion(input);
	assert.deepEqual(steps.map((s) => [s.status, s.versions, s.nameOnly]), [['published', 'toon#1', true], ['draft', 'toon#2', true]]);
	assert.deepEqual(steps[0].doc.title, { de: 'Katzenpapst', en: 'Cat Pope' });
	assert.equal('i18n' in steps[1].doc, false);
	assert.equal(steps[1].doc.announce, true);
	assert.deepEqual(planToonConversion({ ...input, entryPublished: v(3, steps[0].doc), entryNewest: v(4, steps[1].doc) }), [], 'folded: nothing');
	assert.deepEqual(planToonConversion({ ...input, entryNewest: v(1, converted) }).map((s) => s.status), ['published']);
	const unconverted = entry();
	assert.deepEqual(planToonConversion({ ...input, entryPublished: v(1, unconverted), entryNewest: v(1, unconverted) }), [], 'no texts and no settings: never folded (the name alone would open the pages)');
});

test('toon: the plan – published entry + settings, a draft after it; nothing once the entry has texts or without an entry', () => {
	const v = (id: number, content: unknown) => ({ id, content });
	const input = {
		source: 'de',
		languages: ['de', 'en'],
		entryPublished: v(1, entry()),
		entryNewest: v(5, entry({ announce: true })),
		published: { de: v(2, settings('de')), en: v(3, settings('en')) },
		newest: { de: v(2, settings('de')), en: v(4, settings('en', { title: 'Cat Pope!' })) }
	};
	const steps = planToonConversion(input);
	assert.deepEqual(steps.map((s) => [s.status, s.versions]), [['published', 'toon#1 de#2 en#3'], ['draft', 'toon#5 de#2 en#4']]);
	assert.equal(steps[0].doc.announce, undefined);
	assert.deepEqual(steps[1].doc.title, { de: 'Katzenpapst', en: 'Cat Pope!' });
	assert.equal(steps[1].doc.announce, true);
	assert.deepEqual(planToonConversion({ ...input, entryNewest: v(1, entry()), newest: input.published }).map((s) => s.status), ['published']);
	assert.deepEqual(planToonConversion({ ...input, entryPublished: undefined }).map((s) => s.status), ['draft'], 'an entry never published: a draft only');
	assert.deepEqual(planToonConversion({ ...input, entryNewest: v(6, steps[1].doc) }), [], 'converted');
	assert.deepEqual(planToonConversion({ ...input, entryPublished: undefined, entryNewest: undefined }), [], 'no entry: never created here');
});
