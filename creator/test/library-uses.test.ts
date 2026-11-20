// The open item's uses of files for the Library: read from the document, the part in words, the jump.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { usesIn } from '../src/lib/editor/library-uses.ts';

test('a chapter: its slices by group and slice, relative to the toon, a jump to the slice', () => {
	const chapter = {
		thumbnail: 's1/c3/thumb.webp',
		keyvisual: { image: 's1/c3/kv.webp', video: 's1/c3/kv.720p.mp4' },
		groups: [{ slices: [{ image: 's1/c3/s1c3-slice-1.webp', width: 390 }] }, { slices: [{ image: 's1/c3/a.webp' }, { image: '../characters/catpope/catpope_avatar-happy.webp', link: 'https://x.test/a.webp' }] }],
		title: 'not a file'
	};
	const uses = usesIn('chapter:catpope:3', chapter, 'catpope');
	assert.deepEqual(
		uses.map((u) => [u.path, u.part, u.at]),
		[
			['catpope/s1/c3/thumb.webp', 'thumbnail', ''],
			['catpope/s1/c3/kv.webp', 'keyvisual · image', ''],
			['catpope/s1/c3/kv.720p.mp4', 'keyvisual · video', ''],
			['catpope/s1/c3/s1c3-slice-1.webp', 'group 1 · slice 1 · image', 'group=1&slice=1'],
			['catpope/s1/c3/a.webp', 'group 2 · slice 1 · image', 'group=2&slice=1'],
			['characters/catpope/catpope_avatar-happy.webp', 'group 2 · slice 2 · image', 'group=2&slice=2']
		]
	);
	assert.ok(uses.every((u) => u.item === 'chapter:catpope:3'));
});

test('a character: personas and looks by their ids, paths from the CDN root', () => {
	const character = { id: 'catpope', personas: [{ id: 'regular', looks: [{ id: 'tourist', moods: { happy: { portrait: 'characters/catpope/catpope_tourist-happy.webp' } } }] }] };
	const [use] = usesIn('character:catpope', character);
	assert.equal(use.path, 'characters/catpope/catpope_tourist-happy.webp');
	assert.equal(use.part, 'persona regular · look tourist · moods · happy · portrait');
	assert.equal(use.at, 'persona=regular&look=tourist');
});
