// npm run deploy:status (deploy/live.mjs liveState, issue 167): a target's state next to main, judged by the planner's
// relevantFiles – the same filter the Deploy workflow uses (changed.mjs). git is injected: no clone needed here.
//   node --test deploy/test/*.test.mjs
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { config, liveState } from '../live.mjs';

const target = (name) => ({ name, ...config.targets[name] });
const main = 'a'.repeat(40);
const older = 'b'.repeat(40);
const current = JSON.stringify(config);
const state = (name, live, files, { known = () => true, oldConfig = () => current } = {}) =>
	liveState(target(name), live, main, { diff: () => files, known, oldConfig });

test('no live commit, the same commit, an abbreviated one, a commit this clone lacks', () => {
	assert.equal(state('qr', null, []), '✗ unknown (not reachable / no version.json)');
	assert.equal(state('qr', main, ['qr/src/Landing.php']), '✓ = main');
	assert.equal(state('qr', main.slice(0, 7), ['qr/src/Landing.php']), '✓ = main');
	assert.equal(state('qr', older, ['qr/src/Landing.php'], { known: () => false }), '✗ commit not in this clone');
});

test('behind main only for a change the target watches', () => {
	assert.equal(state('qr', older, ['qr/src/Landing.php', 'viewer/src/app.html']), '✗ behind main (1 changed files, e.g. qr/src/Landing.php)');
	assert.equal(state('qr', older, ['viewer/src/app.html', 'qr/tests/LandingTest.php', 'agents/RULES.md']), '✓ no relevant change since');
	// a live commit without deploy/targets.json counts the file as changed – the planner does the same
	assert.equal(state('qr', older, ['deploy/targets.json'], { oldConfig: () => null }), '✗ behind main (1 changed files, e.g. deploy/targets.json)');
	assert.equal(state('qr', older, ['deploy/targets.json']), '✓ no relevant change since');
});
