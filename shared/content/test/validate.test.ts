// Season downloads (season.download, moved from the toon settings 2026-09-30).   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { validateFile } from '../src/index.ts';

const season = (extra: object) => ({ title: 'S', description: 'D', language: 'de', number: 1, sequence: [], ...extra });
const problems = (kind: string, data: object) => validateFile(kind, data, 'de', { file: 'x' } as never).map((p) => `${p.level} ${p.path.join('.')}`);

test('season.download: optional, music is a path, complete a path or null', () => {
	assert.deepEqual(problems('season', season({})), []);
	assert.deepEqual(problems('season', season({ download: { music: 'audio/a.zip' } })), []);
	assert.deepEqual(problems('season', season({ download: { music: 'audio/a.zip', complete: null } })), []);
	assert.deepEqual(problems('season', season({ download: { music: 3 } })), ['error download.music']);
	assert.deepEqual(problems('season', season({ download: {} })), [], 'an emptied download field shows nothing');
	assert.deepEqual(problems('season', season({ download: 'x' })), ['error download']);
});

test('season.supporters: ordered { name, logo?, url? }', () => {
	assert.deepEqual(problems('season', season({ supporters: [] })), []);
	assert.deepEqual(problems('season', season({ supporters: [{ name: 'Stadt Wien', logo: 'images/supporters/wien.svg', url: 'https://wien.gv.at' }, { name: 'Friends' }] })), []);
	assert.deepEqual(problems('season', season({ supporters: [{ name: '' }, { name: 'X', url: 'wien.gv.at' }, { name: 'Y', logo: 3 }] })), ['error supporters.0.name', 'error supporters.1.url', 'error supporters.2.logo']);
	assert.deepEqual(problems('season', season({ supporters: {} })), ['error supporters']);
});
