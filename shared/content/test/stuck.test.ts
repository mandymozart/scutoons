// Where a reader could get stuck (owner 2026-10-06): stuck.ts – refused by validation (and the server's FlowCheck), marked
// in reach / the Flow graph.   node --experimental-transform-types --no-warnings --test test/stuck.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { chapterFlow, chapterReach, questionsOf, stuckPoints, validateFile } from '../src/index.ts';

const choice = (label: string, cond?: object) => ({ type: 'addChoice', params: { label }, ...(cond ? { if: cond } : {}) });
const show = { type: 'showChoices', params: {} };
const set = { variable: 'x', op: 'set' };

test('questionsOf: the choices before each showChoices; an unconditional divert / jumpTo ends them', () => {
	const qs = questionsOf([choice('a'), choice('b', set), show, choice('c'), { type: 'divert', params: { to: 'p' } }, choice('d', set), show], ['events']);
	assert.deepEqual(
		qs.map((q) => q.choices.map((c) => `${c.path.join('.')}${c.conditional ? '?' : ''}`)),
		[['events.0', 'events.1?'], ['events.5?']]
	);
});

test('stuckPoints: a question whose every choice is conditional – in scripts, slice triggers and item taps', () => {
	const groups = [
		{ label: 'a', slices: [{ events: [choice('x', set), show], items: [{ id: 'i', events: [choice('y', { expr: 'x > 1' }), show] }] }], events: [choice('ok'), choice('maybe', set), show, choice('z', set), show] }
	];
	assert.deepEqual(
		stuckPoints(groups).map((p) => p.path.join('.')),
		['groups.0.events.4', 'groups.0.slices.0.events.1', 'groups.0.slices.0.items.0.events.1']
	);
	assert.deepEqual(stuckPoints([{ events: [show] }]), [], 'no choice at all: nothing hidden (the reader goes on)');
});

test('validation refuses it (an error at the showChoices); reach + the Flow graph mark it "stuck"', () => {
	const data = { number: 1, groups: [{ label: 'a', renderer: { type: 'vertical' }, slices: [{ image: 'x.webp' }], events: [choice('A', set), choice('B', { expr: 'y' }), show] }] };
	const problems = validateFile('chapter', data, 'de', { file: 'c' } as never).filter((p) => p.message.includes('every choice'));
	assert.equal(problems.length, 1);
	assert.equal(problems[0].level, 'error');
	assert.deepEqual(problems[0].path, ['groups', 0, 'events', 2]);
	assert.deepEqual(chapterReach(data.groups).stuck, [['groups', 0, 'events', 2]]);
	const flow = chapterFlow(data.groups);
	assert.ok(flow.nodes.some((n) => n.kind === 'choices' && n.status === 'stuck'), JSON.stringify(flow.nodes.map((n) => [n.kind, n.status])));
	// one unconditional choice: fine
	data.groups[0].events.splice(2, 0, choice('Weiter'));
	assert.equal(validateFile('chapter', data, 'de', { file: 'c' } as never).filter((p) => p.message.includes('every choice')).length, 0);
	assert.ok(!chapterFlow(data.groups).nodes.some((n) => n.status === 'stuck'));
});
