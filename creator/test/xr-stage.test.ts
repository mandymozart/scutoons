import assert from 'node:assert/strict';
import { test } from 'node:test';
import { STAGE_LOADING, stageAfter, stageInput } from '../src/lib/editor/xr-stage.ts';

test('an experience becomes what the stage opens, with the plugins that are whole', () => {
	const doc = { configuration: { scenes: [{ id: 'poster' }] }, plugins: [{ id: 'toon', api: '3.1', code: 'export default {}' }, { id: 'broken', api: 3 }, null] };
	assert.deepEqual(stageInput(doc), { configuration: doc.configuration, plugins: [{ id: 'toon', api: '3.1', code: 'export default {}' }] });
	assert.equal(stageInput({ id: 'x' }), null);
	assert.equal(stageInput(null), null);
});

test('the panel shows loading until the scene runs, and why it failed', () => {
	const running = stageAfter(STAGE_LOADING, 'running', null);
	assert.equal(stageAfter(STAGE_LOADING, 'loading', null).phase, 'loading');
	assert.equal(running.phase, 'playing');
	assert.deepEqual(stageAfter(running, 'error', 'no WebGL'), { ...running, phase: 'failed', problems: ['no WebGL'] });
	assert.deepEqual(stageAfter(running, 'error', null).problems, []);
});
