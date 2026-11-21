import assert from 'node:assert/strict';
import { test } from 'node:test';
import { openRequest, readStageEvent, readStageRequest, STAGE_LOADING, stageAfter, stageCsp, stageMessage } from '../src/lib/editor/xr-stage-frame.ts';

const doc = {
	id: 'poster-catpope',
	configuration: { scenes: [{ id: 'poster' }] },
	plugins: [{ id: 'toon', api: '3.1', code: 'export default {}' }, { id: 'broken', api: 3 }, null]
};

test('an experience becomes the request that opens it, with the plugins that are whole', () => {
	const request = openRequest(doc, 'poster', 'https://cdn.example/xr/poster-catpope/abc', 'de');
	assert.deepEqual(request, {
		kind: 'open',
		configuration: doc.configuration,
		plugins: [{ id: 'toon', api: '3.1', code: 'export default {}' }],
		scene: 'poster',
		assets: 'https://cdn.example/xr/poster-catpope/abc',
		language: 'de'
	});
	assert.equal(openRequest({ id: 'x' }, 'poster', 'https://cdn.example/x', 'en'), null);
	assert.equal(openRequest(doc, '', 'https://cdn.example/x', 'en'), null);
});

test('the frame reads only tagged requests of a known kind', () => {
	const request = openRequest(doc, 'poster', 'https://cdn.example/xr/poster-catpope/abc', 'de')!;
	assert.deepEqual(readStageRequest(stageMessage(request)), request);
	assert.deepEqual(readStageRequest(stageMessage({ kind: 'view', view: 'top' })), { kind: 'view', view: 'top' });
	assert.equal(readStageRequest(request), null);
	assert.equal(readStageRequest({ ...stageMessage(request), assets: 'javascript:alert(1)' }), null);
	assert.equal(readStageRequest({ ...stageMessage({ kind: 'view', view: 'top' }), view: 'inside' }), null);
	assert.equal(readStageRequest('open'), null);
});

test('the page reads only the frame’s known events, cut to size', () => {
	assert.deepEqual(readStageEvent(stageMessage({ kind: 'ready' })), { kind: 'ready' });
	assert.deepEqual(readStageEvent(stageMessage({ kind: 'status', status: 'running', message: null })), { kind: 'status', status: 'running', message: null });
	assert.equal(readStageEvent({ kind: 'ready' }), null);
	assert.equal(readStageEvent({ tag: 'scutoons-xr-stage', kind: 'status', status: 'publish' }), null);
	assert.equal(readStageEvent({ tag: 'scutoons-xr-stage', kind: 'found', targets: [1, 2] }), null);
	assert.equal(readStageEvent({ tag: 'scutoons-xr-stage', kind: 'save', content: {} }), null);
	const long = readStageEvent({ tag: 'scutoons-xr-stage', kind: 'status', status: 'error', message: 'x'.repeat(5000) });
	assert.equal(long?.kind === 'status' ? long.message?.length : 0, 300);
	const many = readStageEvent({ tag: 'scutoons-xr-stage', kind: 'problems', problems: Array.from({ length: 500 }, (_, i) => `problem ${i}`) });
	assert.equal(many?.kind === 'problems' ? many.problems.length : 0, 20);
});

test('the panel shows loading until the scene runs, and why it failed', () => {
	const running = stageAfter(STAGE_LOADING, { kind: 'status', status: 'running', message: null });
	assert.equal(stageAfter(STAGE_LOADING, { kind: 'status', status: 'loading', message: null }).phase, 'loading');
	assert.equal(running.phase, 'playing');
	assert.deepEqual(stageAfter(running, { kind: 'found', targets: ['catpope'] }).found, ['catpope']);
	assert.deepEqual(stageAfter(running, { kind: 'status', status: 'error', message: 'no WebGL' }), { ...running, phase: 'failed', problems: ['no WebGL'] });
	assert.deepEqual(stageAfter(STAGE_LOADING, { kind: 'problems', problems: ['scenes: missing'] }), { ...STAGE_LOADING, phase: 'failed', problems: ['scenes: missing'] });
	assert.equal(stageAfter(running, { kind: 'subtitle', text: 'Miau' }).subtitle, 'Miau');
});

test('the frame’s policy names the creator and the experience’s folder, nothing of the API', () => {
	const csp = stageCsp({ app: 'https://creator.example', assets: 'https://cdn.example/xr/poster-catpope/abc' });
	assert.match(csp, /^default-src 'none'; /);
	assert.match(csp, /script-src https:\/\/creator\.example blob: 'wasm-unsafe-eval';/);
	assert.match(csp, /connect-src https:\/\/creator\.example https:\/\/cdn\.example\/xr\/poster-catpope\/abc\/ blob: data:;/);
	assert.doesNotMatch(csp, /api|unsafe-eval'[^;]*unsafe-eval|\*/);
	assert.match(stageCsp({ app: 'http://localhost:5174', assets: 'http://localhost:9100/xr/a/b/', devSocket: 'ws://localhost:5174' }), /connect-src [^;]*\/xr\/a\/b\/ blob: data: ws:\/\/localhost:5174;/);
});
