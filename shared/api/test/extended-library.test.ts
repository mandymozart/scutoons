// The extended Library's reader (#864) against a fake fetch: the path it asks, the 304, the pages.
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createApiClient } from '../src/client.ts';
import { library, libraryEntries, libraryPath } from '../src/extended-library.ts';
import { FILE_KINDS, THING_TYPES, type LibraryEntry, type LibraryList } from '../src/library-types.ts';

type Call = { url: string; headers: Record<string, string> };

function fake(answer: (url: string, headers: Record<string, string>) => Response) {
	const calls: Call[] = [];
	const client = createApiClient({
		baseUrl: 'https://api.test/api',
		getToken: () => 'scd_deck',
		fetch: (async (url: string, init: RequestInit) => {
			const headers = init.headers as Record<string, string>;
			calls.push({ url, headers });
			return answer(url, headers);
		}) as typeof fetch
	});
	return { client, calls };
}

const entry = (address: string): LibraryEntry => ({ address, type: 'character', name: address, picture: null, toons: ['catpope'], usedIn: 0, updated: null });
const page = (entries: LibraryEntry[], next: string | null): LibraryList => ({ library: 1, version: 'v1', toon: 'catpope', place: 'toon', locale: 'en', entries, total: 3, next, counts: { character: 3 } });
const json = (body: unknown) => new Response(JSON.stringify(body), { status: 200, headers: { 'Content-Type': 'application/json', ETag: '"v1"' } });

test('the path: only what is set, lists joined, the address and the places escaped', () => {
	assert.equal(libraryPath(), '/library');
	assert.equal(libraryPath({ toon: 'catpope', type: ['character', 'scene'], kind: 'image', limit: 50, cursor: null }), '/library?toon=catpope&type=character%2Cscene&kind=image&limit=50');
	assert.equal(libraryPath({ toon: 'catpope', place: 'cast:1' }), '/library?toon=catpope&place=cast%3A1');
	assert.equal(libraryPath({ at: 'image:catpope/s1/a b.webp', locale: 'de' }), '/library?at=image%3Acatpope%2Fs1%2Fa+b.webp&locale=de');
	assert.equal(libraryPath({ place: 'item:character:catpope', from: 'generated', subtype: 'recipe' }), '/library?place=item%3Acharacter%3Acatpope&subtype=recipe&from=generated');
});

test('a page, and null when the held version is still current (304)', async () => {
	const { client, calls } = fake((_, headers) => (headers['If-None-Match'] === '"v1"' ? new Response(null, { status: 304 }) : json(page([entry('character:boy')], null))));
	const first = await library(client, { toon: 'catpope' });
	assert.deepEqual(first?.entries.map((e) => e.address), ['character:boy']);
	assert.equal(calls[0].url, 'https://api.test/api/library?toon=catpope');
	assert.equal(calls[0].headers.Authorization, 'Bearer scd_deck', 'a deck reads with its token');
	assert.equal(await library(client, { toon: 'catpope' }, 'v1'), null);
	assert.equal(calls[1].headers['If-None-Match'], '"v1"');
});

test('a thing stepped into', async () => {
	const step = { library: 1, version: 'v2', locale: 'en', at: entry('character:catpope'), levels: [{ address: 'character:catpope.regular', level: 'persona', name: 'regular', picture: null, parent: 'character:catpope' }], files: [], uses: [], usedIn: [{ address: 'season:catpope:1', name: 'Season one', as: 'cast' }] };
	const { client, calls } = fake(() => json(step));
	const answer = await library(client, { at: 'character:catpope' });
	assert.equal(answer?.levels[0].level, 'persona');
	assert.equal(answer?.usedIn[0].as, 'cast');
	assert.equal(calls[0].url, 'https://api.test/api/library?at=character%3Acatpope');
});

test('every entry of a place, page after page', async () => {
	const pages: Record<string, LibraryList> = { '': page([entry('a'), entry('b')], 'c1'), c1: page([entry('c')], null) };
	const { client, calls } = fake((url) => json(pages[new URL(url).searchParams.get('cursor') ?? '']));
	assert.deepEqual((await libraryEntries(client, { toon: 'catpope', limit: 2 })).map((e) => e.address), ['a', 'b', 'c']);
	assert.deepEqual(calls.map((c) => new URL(c.url).search), ['?toon=catpope&limit=2', '?toon=catpope&cursor=c1&limit=2']);
});

test('an error stays an error', async () => {
	const { client } = fake(() => new Response(JSON.stringify({ status: 'error', error: 'invalid_place' }), { status: 400 }));
	await assert.rejects(library(client, { toon: 'catpope', place: 'recent' }), (e: { status: number }) => e.status === 400);
});

test('the types cover the deck\'s 13 kinds (PLAN P11) and the new ones', () => {
	const deck = ['module', 'instrument', 'rig', 'scene', 'character', 'item', 'place', 'image', 'audio', 'video', 'model', 'animation', 'file'];
	const ours: readonly string[] = [...THING_TYPES, ...FILE_KINDS];
	for (const k of deck) assert.ok(ours.includes(k), k);
	for (const k of ['toon', 'show', 'generator', 'kanon', 'person']) assert.ok(ours.includes(k), k);
	const vectors = JSON.parse(readFileSync(new URL('../../../server/schema/api/library.vectors.json', import.meta.url), 'utf8')) as { cases: { expect: { kind: string } }[] };
	for (const c of vectors.cases) assert.ok((FILE_KINDS as readonly string[]).includes(c.expect.kind), c.expect.kind);
});
