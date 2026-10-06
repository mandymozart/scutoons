// Linked groups ("chains", owner 2026-10-06; agents/DATA.md "Linked groups").   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { chainOf, chainsOf, chainSlices, groupEnd, hasVisualNovel, linksToNext, nextStep, validateFile } from '../src/index.ts';

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

test('chainOf: a VN group (script or slice events) ends a chain – its continues is ignored', () => {
	assert.equal(hasVisualNovel({ events: [] }), true);
	assert.equal(hasVisualNovel({ slices: [{ events: [{ type: 'showMessage' }] }] }), true);
	assert.equal(hasVisualNovel({ slices: [{ events: [] }, {}] }), false);
	const groups = [{ continues: true }, { continues: true, events: [] }, { continues: true }, {}];
	assert.equal(linksToNext(groups, 1), false);
	assert.deepEqual(chainOf(groups, 0), { start: 0, end: 1 }, "the VN group is the chain's last");
	assert.deepEqual(chainOf(groups, 2), { start: 2, end: 3 });
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
	// a VN group (script or slice events) only ends a chain
	assert.deepEqual(problems([{ continues: true, events: [] }, {}]), ['error groups.0.continues']);
	assert.deepEqual(problems([{ continues: true }, { events: [] }]), []);
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
