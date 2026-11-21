import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { XrGameConfiguration } from '@scutoons/content';
import { failureOf, placeIn, viewOf, withPlugins } from '../src/lib/xr/xr-session.ts';

const configuration: XrGameConfiguration = {
	version: { schema: '5.0' },
	initialScene: 'cover',
	scenes: [{ id: 'cover' }, { id: 'poster' }],
	entries: [
		{ id: 'catpope', scene: 'poster', target: { id: 'catpope' } },
		{ id: 'key', scene: 'cover', target: {} },
		{ id: 'note', scene: 'cover' },
		{ id: 'lost', scene: 'gone', target: { id: 'lost' } }
	]
};

test('an experience opens at its initial scene, else its first', () => {
	assert.deepEqual(placeIn(configuration), { scene: 'cover' });
	assert.deepEqual(placeIn({ ...configuration, initialScene: 'gone' }), { scene: 'cover' });
	assert.deepEqual(placeIn({ ...configuration, initialScene: undefined }), { scene: 'cover' });
	assert.equal(placeIn({ ...configuration, scenes: [] }), null);
	assert.equal(placeIn({ ...configuration, scenes: ['cover', null, { id: 7 }] }), null);
});

test('the second segment is a scene or a target', () => {
	assert.deepEqual(placeIn(configuration, 'poster'), { scene: 'poster' });
	assert.deepEqual(placeIn(configuration, 'catpope'), { scene: 'poster', target: 'catpope' });
	// a target's id defaults to its entry's
	assert.deepEqual(placeIn(configuration, 'key'), { scene: 'cover', target: 'key' });
});

test('an entry without a target, a target of a missing scene and an unknown name are not places', () => {
	assert.equal(placeIn(configuration, 'note'), null);
	assert.equal(placeIn(configuration, 'lost'), null);
	assert.equal(placeIn(configuration, 'nowhere'), null);
});

test("the document's plugins become the configuration's modules", () => {
	const made = withPlugins(configuration, [{ id: 'toon', api: '3.1', code: 'export {}' }], (code) => `blob:${code.length}`);
	assert.deepEqual(made.plugins, [{ id: 'toon', api: '3.1', src: 'blob:9' }]);
	assert.equal(made.scenes, configuration.scenes);
	assert.equal(configuration.plugins, undefined);
});

test("the engine's status as the page's state", () => {
	assert.equal(viewOf('idle', 0), 'loading');
	assert.equal(viewOf('loading', 0), 'loading');
	assert.equal(viewOf('ready', 0), 'ready');
	assert.equal(viewOf('starting', 0), 'starting');
	assert.equal(viewOf('running', 0), 'scanning');
	assert.equal(viewOf('running', 2), 'found');
	assert.equal(viewOf('paused', 1), 'paused');
	assert.equal(viewOf('error', 1), 'failed');
});

test('what a failure is to the reader', () => {
	assert.equal(failureOf('ar-start-failed'), 'camera');
	assert.equal(failureOf('camera-not-responding'), 'camera');
	assert.equal(failureOf('ar-load-failed'), 'content');
	assert.equal(failureOf('ar-failed'), 'other');
	assert.equal(failureOf(), 'other');
});
