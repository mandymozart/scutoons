// Story variables across every toon (owner 2026-10-06: one save game): facts / dead things / never-true / similar names
// judge a variable over all toons (the server's flow index `variables` → `variablesElsewhere`).
//   node --experimental-transform-types --no-warnings --test test/global-variables.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { chapterReach, deadThings, similarVariable, toonFacts, validateFile, variablesElsewhere } from '../src/index.ts';

const set = (variable: string) => ({ type: 'setVariable', params: { variable, value: 1, op: 'add' } });
const msg = (text: string, cond?: object) => ({ type: 'showMessage', params: { text }, ...(cond ? { if: cond } : {}) });

const index = {
	variables: {
		mama_tips: { set: ['catpope', 'toptoy'], read: ['yellowmouse'] },
		visits_hypercity: { set: ['toptoy'], read: [] },
		only_here: { set: ['catpope'], read: ['catpope'] }
	}
};

test('variablesElsewhere: what the OTHER toons set / read', () => {
	assert.deepEqual(variablesElsewhere(index, 'catpope'), { sets: ['mama_tips', 'visits_hypercity'], reads: ['mama_tips'] });
	assert.deepEqual(variablesElsewhere(index, 'yellowmouse'), { sets: ['mama_tips', 'only_here', 'visits_hypercity'], reads: ['only_here'] });
	assert.deepEqual(variablesElsewhere(null, 'x'), { sets: [], reads: [] });
});

test('dead things across toons: set here + read in another toon = used; read here + set in another toon = set', () => {
	// catpope sets mama_tips (read only in yellowmouse) and reads visits_hypercity (set only in toptoy)
	const groups = [{ label: 'a', slices: [{}], events: [set('mama_tips'), msg('{visits_hypercity} mal'), set('lonely')] }];
	const alone = deadThings([{ number: 1, groups }], toonFacts([], { toon: 'catpope' }));
	assert.deepEqual(alone.unread.map((d) => d.name), ['lonely', 'mama_tips']);
	assert.deepEqual(alone.unset.map((d) => d.name), ['visits_hypercity']);
	const facts = toonFacts([], { toon: 'catpope', elsewhere: variablesElsewhere(index, 'catpope') });
	const dead = deadThings([{ number: 1, groups }], facts);
	assert.deepEqual(dead.unread.map((d) => d.name), ['lonely'], 'mama_tips is read in yellowmouse');
	assert.deepEqual(dead.unset.map((d) => d.name), [], 'visits_hypercity is set in toptoy');
});

test('never true: a test of a variable another toon sets is no "never true"', () => {
	const groups = [{ slices: [{}], events: [msg('reunion', { variable: 'mama_tips', op: '>=', value: 2 })] }];
	assert.equal(chapterReach(groups, toonFacts([], { toon: 'yellowmouse' })).neverTrue.length, 1);
	assert.equal(chapterReach(groups, toonFacts([], { toon: 'yellowmouse', elsewhere: variablesElsewhere(index, 'yellowmouse') })).neverTrue.length, 0);
});

test('similarVariable: a slip that makes a second variable', () => {
	assert.equal(similarVariable('visits_harbor', ['visits_harbour', 'visits_market']), 'visits_harbour');
	assert.equal(similarVariable('Mama_tips', ['mama_tips']), 'mama_tips', 'case only');
	assert.equal(similarVariable('step1', ['step2']), undefined, 'digits only: meant to differ');
	assert.equal(similarVariable('tips', ['tipz']), undefined, 'too short to judge');
	assert.equal(similarVariable('tip_hat', ['tip_road', 'tips']), undefined);
	assert.equal(similarVariable('visits_harbour', ['visits_harbour']), undefined, 'itself');
});

test('validation warns at the place a chapter sets / tests a look-alike name', () => {
	const data = { number: 1, groups: [{ label: 'a', slices: [{ image: 'x.webp' }], events: [set('visits_harbor')] }] };
	const facts = toonFacts([], { toon: 'catpope', elsewhere: { sets: ['visits_harbour'], reads: [] } });
	const problems = validateFile('chapter', data, 'de', { file: 'c', facts } as never).filter((p) => p.message.includes('looks like'));
	assert.equal(problems.length, 1);
	assert.deepEqual(problems[0].path, ['groups', 0, 'events', 0, 'params', 'variable']);
	assert.match(problems[0].message, /"visits_harbor" looks like "visits_harbour"/);
	assert.equal(validateFile('chapter', data, 'de', { file: 'c' } as never).filter((p) => p.message.includes('looks like')).length, 0, 'without facts nothing is judged');
});
