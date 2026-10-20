// node --experimental-transform-types --no-warnings --test server/tools/storyboard-import/import-sql.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { chapterDoc, chapterOfFolder, insertStatement, migrationSql, sqlString } from './import-sql.ts';

const scene = (n: number, extra: Record<string, unknown> = {}) => ({ storyboard: 2, toon: 'catpope', chapter: 3, source: { scene: n }, groups: [], ...extra });

test('chapter folders', () => {
	assert.equal(chapterOfFolder('s1c3'), 3);
	assert.equal(chapterOfFolder('s2c0'), 0);
	assert.equal(chapterOfFolder('s1c03'), null);
	assert.equal(chapterOfFolder('style'), null);
});

test('scenes in scene order, the files as they are', () => {
	const doc = chapterDoc('catpope', 3, [
		{ path: 'b', content: scene(26, { note: 'x' }) },
		{ path: 'a', content: scene(25) }
	]);
	assert.deepEqual(doc, { storyboard: 2, toon: 'catpope', chapter: 3, scenes: [scene(25), scene(26, { note: 'x' })] });
});

test('refuses files of another toon / chapter, doubled scenes, other formats', () => {
	assert.throws(() => chapterDoc('catpope', 4, [{ path: 'a', content: scene(25) }]), /chapter 3, expected 4/);
	assert.throws(() => chapterDoc('other', 3, [{ path: 'a', content: scene(25) }]), /toon catpope/);
	assert.throws(() => chapterDoc('catpope', 3, [{ path: 'a', content: scene(25) }, { path: 'b', content: scene(25) }]), /twice/);
	assert.throws(() => chapterDoc('catpope', 3, [{ path: 'a', content: { ...scene(25), storyboard: 1 } }]), /storyboard 2/);
	assert.throws(() => chapterDoc('catpope', 3, [{ path: 'a', content: [] }]), /JSON object/);
});

test('SQL string literals stay on one line', () => {
	assert.equal(sqlString("it's\n\\ok"), "'it\\'s\\n\\\\ok'");
});

test('a guarded insert per chapter, chapters in order', () => {
	const doc = chapterDoc('catpope', 3, [{ path: 'a', content: scene(25, { text: 'Schrödinger’s "cat"; done' }) }]);
	const sql = insertStatement(doc, 'm', '2026-10-20 00:00:00');
	assert.match(sql, /^INSERT INTO `entity_versions`/);
	assert.match(sql, /WHERE NOT EXISTS \(SELECT 1 FROM `entity_versions` v WHERE v.`entity_key` = 'storyboard:catpope:3'\)/);
	assert.match(sql, /AND EXISTS \(SELECT 1 FROM `entity_versions` t WHERE t.`entity_key` = 'toon:catpope'\);\n$/);
	assert.ok(sql.includes('Schrödinger’s \\\\"cat\\\\"; done'), 'unicode kept, JSON quotes escaped once more for SQL');
	assert.equal(sql.split('\n').length, 5, 'four lines + the end');
	const four = { ...doc, chapter: 4, scenes: [] };
	const all = migrationSql([four, doc], 'm', 'x', '-- h\n');
	assert.ok(all.startsWith('-- h\n') && all.indexOf("'storyboard:catpope:3'") < all.indexOf("'storyboard:catpope:4'"));
});
