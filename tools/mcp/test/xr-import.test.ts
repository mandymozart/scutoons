import assert from 'node:assert/strict';
import { test } from 'node:test';
import { planImport } from '../src/xr-import.ts';

const LIBRARY = 'https://cdn.example/assets/demo';
const VERSION = 'https://cdn.example/osct/staging/demo/abc123';

const version = () => ({
	version: { schema: '5.0', version: '4.4.0' },
	experience: { id: 'demo', title: 'The Demo Book', link: 'https://example.org/about' },
	initialScene: 'cover',
	scenes: [{ id: 'cover' }, { id: 'poster' }],
	entries: [
		{ id: 'key', scene: 'cover', target: { imageSrc: '/assets/content/cover/key.jpg', entity: { ref: 'beacon' } } },
		{ id: 'catpope', scene: 'poster', target: { imageSrc: `${LIBRARY}/e61d/v1.webp`, entity: { ref: 'overlay' } } }
	],
	entities: {
		beacon: { type: 'model', assets: [{ src: '/assets/content/entities/beacon/beacon.glb' }] },
		overlay: { type: 'group', children: [{ ref: 'part' }, { type: 'video', assets: [{ src: `${LIBRARY}/f00d/v2.mp4` }] }] },
		part: { type: 'model', assets: [{ src: `${LIBRARY}/7867/v1.glb` }] }
	},
	plugins: [
		{ id: 'tint', api: '1.6', src: 'https://cdn.example/demo/old/assets/content/plugins/tint/plugin.js' },
		{ id: 'toon', api: '3.1', src: '/assets/content/plugins/toon/plugin-a700.js' }
	],
	subtitles: [
		{ media: `${LIBRARY}/f00d/v2.mp4`, tracks: [{ lang: 'en', src: '/assets/content/subtitles/f00d.en.vtt' }] },
		{ media: '/assets/content/cover/intro.mp4', tracks: [{ lang: 'en', src: '/assets/content/subtitles/intro.en.vtt' }] }
	]
});

test('one scene: its entries, the shared entities they use, its files', () => {
	const plan = planImport(version(), `${VERSION}/`, 'poster');
	const c = plan.configuration as ReturnType<typeof version>;
	assert.deepEqual(c.scenes, [{ id: 'poster' }]);
	assert.equal(c.initialScene, 'poster');
	assert.deepEqual(c.entries.map((e) => e.id), ['catpope']);
	// `overlay` uses `part`; `beacon` is the other scene's
	assert.deepEqual(Object.keys(c.entities).sort(), ['overlay', 'part']);
	assert.equal(c.entries[0].target.imageSrc, '/assets/content/library/e61d-v1.webp');
	assert.equal(c.entities.part.assets[0].src, '/assets/content/library/7867-v1.glb');
	assert.deepEqual(plan.files, [
		{ from: `${LIBRARY}/e61d/v1.webp`, path: 'assets/content/library/e61d-v1.webp' },
		{ from: `${LIBRARY}/f00d/v2.mp4`, path: 'assets/content/library/f00d-v2.mp4' },
		{ from: `${LIBRARY}/7867/v1.glb`, path: 'assets/content/library/7867-v1.glb' },
		{ from: `${VERSION}/assets/content/subtitles/f00d.en.vtt`, path: 'assets/content/subtitles/f00d.en.vtt' }
	]);
});

test('subtitles stay only for a video that is still in', () => {
	const plan = planImport(version(), VERSION, 'poster');
	assert.deepEqual(plan.configuration.subtitles, [
		{ media: '/assets/content/library/f00d-v2.mp4', tracks: [{ lang: 'en', src: '/assets/content/subtitles/f00d.en.vtt' }] }
	]);
});

test('the plugins leave the configuration: the newest plugin API as code to fetch, older ones named', () => {
	const plan = planImport(version(), VERSION, 'poster');
	assert.equal('plugins' in plan.configuration, false);
	assert.deepEqual(plan.plugins, [{ id: 'toon', api: '3.1', from: `${VERSION}/assets/content/plugins/toon/plugin-a700.js` }]);
	assert.deepEqual(plan.skipped, ['tint (plugin API 1.6)']);
});

test('the whole experience without a scene; a link is no file; the given version is left as it was', () => {
	const given = version();
	const plan = planImport(given, VERSION);
	const c = plan.configuration as ReturnType<typeof version>;
	assert.equal(c.scenes.length, 2);
	assert.equal(c.initialScene, 'cover');
	assert.deepEqual(Object.keys(c.entities).sort(), ['beacon', 'overlay', 'part']);
	assert.equal(c.experience.link, 'https://example.org/about');
	// the intro video is in no scene, so its subtitles go
	assert.equal(plan.files.length, 6);
	assert.deepEqual(given, version());
});

test('two files that would get one name get two', () => {
	const given = version();
	given.entries[1].target.imageSrc = 'https://other.example/x/e61d/v1.webp';
	given.entities.part.assets[0].src = `${LIBRARY}/e61d/v1.webp`;
	const paths = planImport(given, VERSION, 'poster').files.map((f) => f.path);
	assert.ok(paths.includes('assets/content/library/e61d-v1.webp'));
	assert.ok(paths.includes('assets/content/library/e61d-v1-2.webp'));
});

test('a scene the version does not have, and something that is no configuration', () => {
	assert.throws(() => planImport(version(), VERSION, 'back'), /no scene "back" \(it has: cover, poster\)/);
	assert.throws(() => planImport({ scenes: 'x' }, VERSION), /no game configuration/);
});
