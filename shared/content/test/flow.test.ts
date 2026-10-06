// The chapter's story flow (owner 2026-10-06, the creator's Flow panel).   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { chapterFlow, chapterReach, conditionText, toonFacts, type ChapterFlow } from '../src/index.ts';

const g = (o: object = {}) => ({ slices: [{}, {}], ...o });
const msg = { type: 'showMessage', params: { text: 'x' } };
const jump = (value: string, label?: string) => ({ type: 'jumpTo', params: { target: { type: 'chapter', value, ...(label ? { label } : {}) } } });
const node = (f: ChapterFlow, id: string) => f.nodes.find((n) => n.id === id);
const out = (f: ChapterFlow, from: string) => f.edges.filter((e) => e.from === from).map((e) => `${e.kind}:${e.to}${e.label ? `[${e.label}]` : ''}${e.branch ? `(${e.branch})` : ''}${e.condition ? `{${e.condition}}` : ''}${e.never ? '!' : ''}`);

test('conditionText: one line per test', () => {
	assert.equal(conditionText({ variable: 'coins', op: '>=', value: 3 }), 'coins >= 3');
	assert.equal(conditionText({ variable: 'met', op: 'set' }), 'met set');
	assert.equal(conditionText(null), '');
});

test('chapterFlow: reading order – linked groups, the follow-up bar, an authored end, the next chapter', () => {
	const f = chapterFlow([g({ label: 'a', continues: true }), g(), g({ label: 'c', end: { target: { type: 'chapter', value: '5', label: 'x' }, auto: true } }), g({ label: 'd' })], { chapter: 2 });
	assert.deepEqual(out(f, 'g0'), ['continues:g1']);
	// the chain a+b is through, c has a label: the follow-up bar leads there
	assert.deepEqual(out(f, 'g1'), ['follow:g2']);
	assert.deepEqual(out(f, 'g2'), ['end:x:chapter:5:x:(auto)']);
	// the last group: the next chapter
	assert.deepEqual(out(f, 'g3'), ['follow:x:next']);
	assert.deepEqual(node(f, 'x:chapter:5:x:')?.target, { type: 'chapter', value: '5', label: 'x' });
	assert.equal(node(f, 'x:next')?.status, 'reachable');
	// an unlabelled group after a chain: the bar goes to the next chapter, the group is unreachable (as reach says)
	const h = chapterFlow([g(), g()], { chapter: 2, facts: toonFacts([]) });
	assert.deepEqual(out(h, 'g0'), ['follow:x:next']);
	assert.deepEqual(h.nodes.filter((n) => n.kind === 'group').map((n) => n.status), ['reachable', 'unreachable']);
});

test('chapterFlow: a script – points, questions, diverts, jumps (in and out of the chapter)', () => {
	const events = [
		msg,
		{ type: 'label', params: { name: 'menu' } },
		{ type: 'addChoice', params: { label: { de: 'Nochmal', en: 'Again' }, onSelect: { divert: { to: 'menu' } } } },
		{ type: 'addChoice', params: { label: 'Leave', onSelect: [{ set: { variable: 'left', value: 1 } }, { jumpTo: { target: { type: 'chapter', value: '3' } } }] } },
		{ type: 'addChoice', params: { label: 'Stay' }, if: { variable: 'coins', op: '>=', value: 2 } },
		{ type: 'showChoices' },
		{ ...jump('2', 'b'), if: { variable: 'left', op: 'set' } },
		msg
	];
	const f = chapterFlow([g({ label: 'a', events, continues: true }), g({ label: 'b' })], { chapter: 2, text: (v) => (typeof v === 'string' ? v : (v as any)?.en ?? '') });
	assert.deepEqual(out(f, 'g0'), ['then:g0e1']);
	assert.deepEqual(out(f, 'g0e1'), ['then:g0e5']);
	assert.equal(node(f, 'g0e5')?.kind, 'choices');
	assert.equal(node(f, 'g0e5')?.count, 3);
	assert.deepEqual(out(f, 'g0e5'), ['choice:g0e1[Again]', 'jump:x:chapter:3::[Leave]', 'choice:g0e6[Stay]{coins >= 2}']);
	// a jump inside the chapter goes to the labelled group; failing its test the script goes on to the next group
	assert.deepEqual(out(f, 'g0e6'), ['jump:g1{left set}', 'continues:g1(else)']);
	assert.deepEqual(f.nodes.map((n) => n.kind), ['group', 'group', 'label', 'choices', 'exit', 'jump', 'exit']);
});

test('chapterFlow: statuses come from chapterReach – unreachable, never true, endless loop', () => {
	const facts = toonFacts([]);
	const groups = [
		g({ events: [{ type: 'divert', params: { to: 'p' }, if: { variable: 'v', op: 'set' } }, jump('3'), { type: 'label', params: { name: 'p' } }, msg], continues: true }),
		g({ events: [{ type: 'label', params: { name: 'a' } }, msg, { type: 'divert', params: { to: 'a' } }] })
	];
	const reach = chapterReach(groups, facts);
	const f = chapterFlow(groups, { chapter: 2, facts, reach });
	assert.equal(f.reach, reach, 'the caller\'s reach is used as is');
	assert.equal(node(f, 'g0e0')?.status, 'never');
	assert.deepEqual(out(f, 'g0e0'), ['divert:g0e2{v set}!', 'then:g0e1(else)']);
	assert.equal(node(f, 'g0e2')?.status, 'unreachable');
	// the second group: nothing leads there (the first always jumps away)
	assert.equal(node(f, 'g1')?.status, 'unreachable');
	assert.deepEqual(
		f.nodes.filter((n) => n.group !== undefined).map((n) => [n.id, n.status]),
		[['g0', 'reachable'], ['g1', 'unreachable'], ['g0e2', 'unreachable'], ['g0e0', 'never'], ['g0e1', 'reachable'], ['g1e0', 'unreachable'], ['g1e2', 'unreachable']]
	);
	// reachable loop
	const loop = chapterFlow([g({ events: [{ type: 'label', params: { name: 'a' } }, msg, { type: 'divert', params: { to: 'a' } }] })]);
	assert.equal(node(loop, 'g0e2')?.status, 'loop');
	assert.deepEqual(out(loop, 'g0e2'), ['divert:g0e0']);
	// a group's slices that can't be reached while the group can
	const dead = chapterFlow([g({ slices: [{}, { events: [{ ...jump('3'), trigger: { type: 'intersect' } }] }, {}] })]);
	assert.deepEqual(node(dead, 'g0')?.deadSlices, [2]);
	assert.deepEqual(out(dead, 'g0'), ['trigger:g0s1t0', 'follow:x:next']);
	assert.deepEqual(node(dead, 'g0s1t0'), { id: 'g0s1t0', kind: 'trigger', status: 'reachable', group: 0, path: ['groups', 0, 'slices', 1, 'events', 0], slice: 1, trigger: 'intersect' });
});

test('chapterFlow: slice triggers and item taps – a lone divert enters the script, random lists, exits', () => {
	const groups = [
		g({
			label: 'a',
			events: [msg, { type: 'label', params: { name: 'p' } }, msg],
			slices: [{ events: [{ type: 'divert', params: { to: 'p' }, trigger: { type: 'click' } }], items: [{ item: 'key', events: [{ type: 'jumpTo', params: { target: { type: 'chapter', value: '2', anyOf: ['b', 'zz'] } } }] }] }, {}]
		}),
		g({ label: 'b' })
	];
	const f = chapterFlow(groups, { chapter: 2 });
	assert.deepEqual(out(f, 'g0s0t0'), ['divert:g0e1']);
	assert.equal(node(f, 'g0s0t1')?.trigger, 'item');
	assert.deepEqual(out(f, 'g0s0t1'), ['then:g0s0t1e0']);
	assert.deepEqual(out(f, 'g0s0t1e0'), ['random:g1', 'random:x:chapter:2:zz']);
	assert.deepEqual(node(f, 'g0s0t1e0')?.path, ['groups', 0, 'slices', 0, 'items', 0, 'events', 0]);
	// a cyclic big chapter stays fast
	const big = Array.from({ length: 300 }, (_, i) => g({ label: `g${i}`, continues: i < 299, events: [{ type: 'label', params: { name: 'l' } }, { type: 'divert', params: { to: 'l' }, if: { variable: 'v', op: 'set' } }, jump('2', `g${(i + 7) % 300}`)] }));
	const t = Date.now();
	assert.equal(chapterFlow(big, { chapter: 2 }).nodes.filter((n) => n.kind === 'group').length, 300);
	assert.ok(Date.now() - t < 2000);
});
