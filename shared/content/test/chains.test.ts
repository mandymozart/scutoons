// Linked groups ("chains", owner 2026-10-06; agents/DATA.md "Linked groups").   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { chainOf, chainsOf, chainSlices, groupEnd, hasVisualNovel, linksToNext, nextStep, scriptFlow, scriptJumps, unreachable, unreachableEvents, validateFile } from '../src/index.ts';

const slices = (n: number) => Array.from({ length: n }, () => ({ image: 'images/p.webp', width: 390, height: 625 }));

test('chainOf: a run of groups where all but the last continue', () => {
	const groups = [{ continues: true }, { continues: true }, {}, {}, { continues: true }, {}];
	assert.deepEqual(chainOf(groups, 0), { start: 0, end: 2 });
	assert.deepEqual(chainOf(groups, 1), { start: 0, end: 2 });
	assert.deepEqual(chainOf(groups, 2), { start: 0, end: 2 });
	assert.deepEqual(chainOf(groups, 3), { start: 3, end: 3 });
	assert.deepEqual(chainOf(groups, 5), { start: 4, end: 5 });
	assert.deepEqual(chainsOf(groups), [{ start: 0, end: 2 }, { start: 3, end: 3 }, { start: 4, end: 5 }]);
	// the chapter's last group can't continue (nothing follows); only `true` links (old strings don't)
	assert.deepEqual(chainOf([{}, { continues: true }], 1), { start: 1, end: 1 });
	assert.deepEqual(chainOf([{ continues: 'true' }, {}], 0), { start: 0, end: 0 });
	assert.deepEqual(chainOf([], 0), { start: 0, end: 0 });
});

test('chainOf: VN groups (scripts, slice events, even jumping scripts) link like any other group (owner 2026-10-06)', () => {
	assert.equal(hasVisualNovel({ events: [] }), true);
	assert.equal(hasVisualNovel({ slices: [{ events: [{ type: 'showMessage' }] }] }), true);
	assert.equal(hasVisualNovel({ slices: [{ events: [] }, {}] }), false);
	const jump = { type: 'jumpTo', params: { target: { type: 'chapter', value: '2' } } };
	const groups = [{ continues: true, events: [{ type: 'showMessage' }] }, { continues: true, events: [jump] }, { continues: true, slices: [{ events: [{ type: 'showMessage' }] }] }, {}];
	assert.equal(linksToNext(groups, 0), true);
	assert.equal(linksToNext(groups, 1), true);
	assert.deepEqual(chainOf(groups, 0), { start: 0, end: 3 });
});

test('scriptFlow / scriptJumps: where a script can go', () => {
	const jump = { type: 'jumpTo', params: { target: { type: 'chapter', value: '2' } } };
	const msg = { type: 'showMessage', params: { text: 'x' } };
	assert.equal(scriptJumps({ events: [msg] }), false);
	assert.equal(scriptJumps({ events: [msg, jump] }), true);
	assert.equal(scriptJumps({ events: [{ type: 'addChoice', params: { label: 'a', onSelect: [{ jumpTo: { target: { type: 'chapter', value: '2' } } }] } }, { type: 'showChoices' }] }), true);
	// always leaves: no run ends
	let flow = scriptFlow([msg, jump, msg]);
	assert.deepEqual([flow.ends, flow.jumps, [...flow.reachable]], [false, true, [0, 1]]);
	// a conditional jump may be skipped
	flow = scriptFlow([{ ...jump, if: { variable: 'v', op: 'set' } }, msg]);
	assert.deepEqual([flow.ends, flow.jumps], [true, true]);
	// a choice that stays + one that jumps
	flow = scriptFlow([
		{ type: 'addChoice', params: { label: 'stay' } },
		{ type: 'addChoice', params: { label: 'go', onSelect: { jumpTo: { target: { type: 'chapter', value: '2' } } } } },
		{ type: 'showChoices' }
	]);
	assert.deepEqual([flow.ends, flow.jumps], [true, true]);
	// a divert skips events; a divert loop never ends
	flow = scriptFlow([{ type: 'divert', params: { to: 'b' } }, msg, { type: 'label', params: { name: 'b' } }, msg]);
	assert.deepEqual([flow.ends, [...flow.reachable].sort()], [true, [0, 2, 3]]);
	flow = scriptFlow([{ type: 'label', params: { name: 'a' } }, { type: 'divert', params: { to: 'a' } }]);
	assert.deepEqual([flow.ends, flow.jumps], [false, false]);
});

test('unreachable: groups / slices after content that always leaves, events no run reaches', () => {
	const jump = { type: 'jumpTo', params: { target: { type: 'chapter', value: '2' } } };
	const g = (o: object = {}) => ({ slices: [{}, {}, {}], ...o });
	// a always jumps: b (no label) is unreachable, c (label) may be jumped to
	assert.deepEqual(unreachable([g({ events: [jump], continues: true }), g({ continues: true }), g({ label: 'c' })]).groups, [1]);
	// not continuing: nothing follows on the page
	assert.deepEqual(unreachable([g({ events: [jump] }), g()]).groups, []);
	// a conditional jump makes nothing unreachable
	assert.deepEqual(unreachable([g({ events: [{ ...jump, if: { variable: 'v', op: 'set' } }], continues: true }), g()]).groups, []);
	// a scroll trigger on slice 2 that always jumps: slice 3 and the next group
	const exit = g({ continues: true, slices: [{}, { events: [{ ...jump, trigger: { type: 'intersect' } }] }, {}] });
	const lost = unreachable([exit, g()]);
	assert.deepEqual([lost.groups, [...lost.slices]], [[1], [[0, [2]]]]);
	// a click trigger may never be clicked
	assert.deepEqual(unreachable([g({ continues: true, slices: [{ events: [{ ...jump, trigger: { type: 'click' } }] }, {}] }), g()]).groups, []);
	// events: after an unconditional jump / divert, unless a slice event diverts to a point there
	const msg = { type: 'showMessage', params: { text: 'x' } };
	const script = [msg, jump, msg, { type: 'label', params: { name: 'p' } }, msg];
	assert.deepEqual(unreachableEvents({ events: script }), [2, 3, 4]);
	assert.deepEqual(unreachableEvents({ events: script, slices: [{ events: [{ type: 'divert', params: { to: 'p' }, trigger: { type: 'click' } }] }] }), [2]);
});

test('chainSlices: page slice indices run on across the chain', () => {
	const groups = [{ slices: slices(2) }, { slices: slices(3) }, {}, { slices: slices(1) }];
	assert.deepEqual(chainSlices(groups, { start: 0, end: 3 }), { offsets: [0, 2, 5, 5], count: 6 });
	assert.deepEqual(chainSlices(groups, { start: 1, end: 1 }), { offsets: [0], count: 3 });
});

test('groupEnd: a usable target only', () => {
	assert.equal(groupEnd({}), null);
	assert.equal(groupEnd({ end: { target: { type: 'nope', value: '1' } } }), null);
	assert.equal(groupEnd({ end: { target: { type: 'chapter', value: '' } } }), null);
	assert.deepEqual(groupEnd({ end: { target: { type: 'chapter', value: '3', label: 'x' } } }), { target: { type: 'chapter', value: '3', label: 'x' }, auto: false });
	assert.deepEqual(groupEnd({ end: { target: { type: 'toon', value: 'other' }, auto: true } }), { target: { type: 'toon', value: 'other' }, auto: true });
});

test("nextStep: after the chain's last group; an authored end wins", () => {
	const sequence = [{ number: 1, status: 'published', isPreview: true }, { number: 2, status: 'published', isPreview: true }];
	const groups = [
		{ label: 'a', continues: true },
		{ label: 'b' },
		{ label: 'c', continues: true },
		{ end: { target: { type: 'chapter', value: '2', label: 'x' }, auto: true } },
		{ label: 'e' }
	];
	// any group of the chain a–b: the group after it
	assert.deepEqual(nextStep({ groups, group: 0, chapter: 1, sequence, member: false }), { kind: 'group', label: 'c' });
	assert.deepEqual(nextStep({ groups, group: 1, chapter: 1, sequence, member: false }), { kind: 'group', label: 'c' });
	// the chain c–d ends with an authored end
	const jump = { kind: 'jump', target: { type: 'chapter', value: '2', label: 'x' }, auto: true };
	assert.deepEqual(nextStep({ groups, group: 2, chapter: 1, sequence, member: false }), jump);
	assert.deepEqual(nextStep({ groups, group: 3, chapter: 1, sequence, member: false }), jump);
	// an end in the middle of a chain is never read
	const middle = [{ label: 'a', continues: true, end: { target: { type: 'toon', value: 't' } } }, { label: 'b' }, { label: 'c' }];
	assert.deepEqual(nextStep({ groups: middle, group: 0, chapter: 1, sequence, member: false }), { kind: 'group', label: 'c' });
	// a chain to the chapter's end → the next chapter
	assert.deepEqual(nextStep({ groups: [{ label: 'a', continues: true }, { label: 'b' }], group: 0, chapter: 1, sequence, member: false }), { kind: 'chapter', chapter: 2 });
});

test("validate: continues / end on groups and the chapter's chains", () => {
	const chapter = (groups: object[]) => ({
		title: 'T',
		author: 'A',
		description: 'D',
		thumbnail: 't.webp',
		number: 1,
		language: 'de',
		release: '2025-01-01',
		groups: groups.map((g, i) => ({ label: `g${i}`, renderer: { type: 'vertical' }, slices: slices(1), ...g }))
	});
	const list = (data: object, ctx: object = {}) =>
		validateFile('chapter', data, 'de', { file: 'x', ...ctx } as never)
			.filter((p) => /continues|end|label/.test(p.path.join('.')))
			.map((p) => `${p.level} ${p.path.join('.')}`);
	const problems = (groups: object[]) => list(chapter(groups));
	assert.deepEqual(problems([{ continues: true }, { continues: false, end: { target: { type: 'chapter', value: '2' }, auto: true } }]), []);
	// the last group can't continue
	assert.deepEqual(problems([{}, { continues: true }]), ['error groups.1.continues']);
	// not a boolean; end not an object, a bad target, auto not a boolean
	assert.deepEqual(problems([{ continues: 'true' }, {}]), ['error groups.0.continues']);
	assert.deepEqual(problems([{ end: 'x' }]), ['error groups.0.end']);
	assert.deepEqual(problems([{ end: { target: { type: 'nope', value: '' }, auto: 'yes' } }]), [
		'error groups.0.end.target.type',
		'error groups.0.end.target.value',
		'error groups.0.end.auto'
	]);
	// end on a group that continues: ignored (warning)
	assert.deepEqual(problems([{ continues: true, end: { target: { type: 'chapter', value: '2' } } }, {}]), ['warning groups.0.end']);
	// VN groups link like any other (owner 2026-10-06)
	assert.deepEqual(problems([{ continues: true, events: [] }, {}]), []);
	assert.deepEqual(problems([{ continues: true }, { events: [] }]), []);
	// a script that can jump away and continues: the next group needs a label (a warning); with one it is fine
	const jump = { type: 'jumpTo', params: { target: { type: 'chapter', value: '2' } }, if: { variable: 'v', op: 'set' } };
	assert.deepEqual(problems([{ continues: true, events: [jump] }, { label: undefined }]), ['warning groups.1.label', 'warning groups.1.label']);
	assert.deepEqual(problems([{ continues: true, events: [jump] }, {}]), []);
	// a chain that isn't the chapter's start needs a label on its first group (the plain "no label" warnings come first)
	assert.deepEqual(problems([{}, { label: undefined, continues: true }, { label: undefined }]), [
		'warning groups.1.label',
		'warning groups.2.label',
		'warning groups.1.label'
	]);
	// ref groups: the scene's continues counts (the creator resolves them)
	const scenes: Record<string, object> = { 'chapter1/s.json': { label: 's', continues: true, renderer: { type: 'vertical' }, slices: slices(1) } };
	const resolveGroup = (g: { ref?: { src: string } }) => (g.ref ? scenes[g.ref.src] : g);
	const withRef = { ...chapter([{}]), groups: [{ label: 'a', renderer: { type: 'vertical' }, slices: slices(1) }, { ref: { src: 'chapter1/s.json' } }] };
	assert.deepEqual(list(withRef, { resolveGroup }), ['error groups.1.continues']);
});

test('validate: text box ids unique in the chapter (scenes too); unreachable content warns', () => {
	const box = (id: string) => ({ id, rect: { x: 0, y: 0, width: 10 }, text: 'x' });
	const chapter = (groups: object[]) => ({
		title: 'T',
		author: 'A',
		description: 'D',
		thumbnail: 't.webp',
		number: 1,
		language: 'de',
		release: '2025-01-01',
		groups
	});
	const group = (label: string, o: object) => ({ label, renderer: { type: 'vertical' }, ...o });
	const run = (data: object, ctx: object = {}, re = /dialog\.\d+\.id|unreachable/) =>
		validateFile('chapter', data, 'de', { file: 'x', ...ctx } as never)
			.filter((p) => re.test(p.path.join('.')) || re.test(p.message))
			.map((p) => `${p.level} ${p.path.join('.')}`);
	const slice = (dialog: object[] = []) => ({ ...slices(1)[0], dialog });
	assert.deepEqual(run(chapter([group('a', { slices: [slice([box('d1')])] }), group('b', { slices: [slice([box('d1'), box('d2')])] })])), ['error groups.1.slices.0.dialog.0.id']);
	// a ref group's scene counts
	const scenes: Record<string, object> = { 'chapter1/s.json': group('s', { slices: [slice([box('d2')])] }) };
	const resolveGroup = (g: { ref?: { src: string } }) => (g.ref ? scenes[g.ref.src] : g);
	assert.deepEqual(run(chapter([group('a', { slices: [slice([box('d2')])] }), { ref: { src: 'chapter1/s.json' } }]), { resolveGroup }), ['error groups.1.slices.0.dialog.0.id']);
	// unreachable: events after an unconditional jump, the unlabelled group after it
	const jump = { type: 'jumpTo', params: { target: { type: 'chapter', value: '2' } } };
	const data = chapter([group('a', { continues: true, slices: [slice()], events: [jump, { type: 'hideMessage' }] }), { renderer: { type: 'vertical' }, slices: [slice()] }]);
	assert.deepEqual(run(data, {}, /unreachable/), ['warning groups.0.events.1', 'warning groups.1']);
});
