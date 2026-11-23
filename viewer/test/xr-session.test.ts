import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { XrGameConfiguration } from '@scutoons/content';
import { failureOf, placeIn, viewOf } from '../src/lib/xr/xr-session.ts';

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

test("the page's state: read, asked for the camera, and what the engine reports", () => {
	assert.equal(viewOf('idle', 0, false, false), 'loading');
	// the engine stays idle until the camera is asked for
	assert.equal(viewOf('idle', 0, true, false), 'ready');
	assert.equal(viewOf('ready', 0, true, false), 'ready');
	assert.equal(viewOf('idle', 0, true, true), 'starting');
	assert.equal(viewOf('loading', 0, true, true), 'starting');
	assert.equal(viewOf('starting', 0, true, true), 'starting');
	assert.equal(viewOf('running', 0, true, true), 'scanning');
	assert.equal(viewOf('running', 2, true, true), 'found');
	assert.equal(viewOf('paused', 1, true, true), 'paused');
	assert.equal(viewOf('error', 1, true, true), 'failed');
	assert.equal(viewOf('error', 0, false, false), 'failed');
});

test('what a failure is to the reader', () => {
	assert.equal(failureOf('ar-start-failed'), 'camera');
	assert.equal(failureOf('camera-not-responding'), 'camera');
	assert.equal(failureOf('ar-load-failed'), 'content');
	assert.equal(failureOf('ar-failed'), 'other');
	assert.equal(failureOf(), 'other');
});
