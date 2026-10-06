// Linked groups ("chains", owner 2026-10-06; agents/DATA.md "Linked groups").   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { chainOf, chainsOf, chainSlices, groupEnd, hasVisualNovel, linksToNext, nextStep, chapterReach, neverTrue, scriptJumps, toonFacts, validateFile } from '../src/index.ts';

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

test('scriptJumps: a jumpTo event or a choice that jumps', () => {
	const jump = { type: 'jumpTo', params: { target: { type: 'chapter', value: '2' } } };
	assert.equal(scriptJumps({ events: [{ type: 'showMessage' }] }), false);
	assert.equal(scriptJumps({ events: [{ type: 'showMessage' }, { ...jump, if: { variable: 'v', op: 'set' } }] }), true);
	assert.equal(scriptJumps({ events: [{ type: 'addChoice', params: { label: 'a', onSelect: [{ jumpTo: { target: { type: 'chapter', value: '2' } } }] } }, { type: 'showChoices' }] }), true);
});

test('toonFacts / neverTrue: variables set somewhere, labels something jumps to', () => {
	const facts = toonFacts([
		{ groups: [{ events: [{ type: 'setVariable', params: { variable: 'clock', value: 1 } }, { type: 'addChoice', params: { onSelect: [{ add: { variable: 'coins', value: 1 } }, { jumpTo: { target: { type: 'chapter', value: '2', label: 'there', anyOf: ['x', 'y'] } } }] } }] }] },
		{ places: [{ unlock: { type: 'chapter', value: '1', label: 'map-label' } }] },
		{ slices: [{ items: [{ item: 'onion', action: 'pickup' }] }] }
	]);
	assert.deepEqual([...facts.variables].sort(), ['clock', 'coins', 'onion']);
	assert.deepEqual([...facts.labels].sort(), ['map-label', 'there', 'x', 'y']);
	assert.equal(neverTrue({ variable: 'nope', op: 'set' }, facts), true);
	assert.equal(neverTrue({ variable: 'nope', op: '==', value: 'a' }, facts), true);
	assert.equal(neverTrue({ variable: 'nope', op: '==', value: '' }, facts), false, 'unset == "" holds');
	assert.equal(neverTrue({ variable: 'nope', op: 'unset' }, facts), false);
	assert.equal(neverTrue({ variable: 'nope', op: '!=', value: 'a' }, facts), false);
	assert.equal(neverTrue({ variable: 'clock', op: '>', value: 3 }, facts), false);
	assert.equal(neverTrue({ variable: 'picked:x#1', op: 'set' }, facts), false, 'internal keys are not judged');
	assert.equal(neverTrue({ variable: 'nope', op: 'set' }, undefined), false, 'without the toon nothing is judged');
});

test('chapterReach: groups / slices / events – reachable, only through a condition, unreachable', () => {
	const jump = { type: 'jumpTo', params: { target: { type: 'chapter', value: '2' } } };
	const msg = { type: 'showMessage', params: { text: 'x' } };
	const g = (o: object = {}) => ({ slices: [{}, {}, {}], ...o });
	const none = toonFacts([]);
	// a always jumps: b (no label) can't be reached; c's label is unknown to the toon → unreachable too (with facts)
	let r = chapterReach([g({ label: 'a', events: [jump], continues: true }), g({ continues: true }), g({ label: 'c' })], none);
	assert.deepEqual(r.groups, ['reachable', 'unreachable', 'unreachable']);
	// without facts every labelled group may be jumped to
	r = chapterReach([g({ label: 'a', events: [jump], continues: true }), g({ continues: true }), g({ label: 'c' })]);
	assert.deepEqual(r.groups, ['reachable', 'unreachable', 'reachable']);
	// a conditional jump: the rest only through a condition (the test must fail)
	r = chapterReach([g({ events: [{ ...jump, if: { variable: 'v', op: 'set' } }, msg], continues: true }), g()]);
	assert.deepEqual([r.groups, r.events[0]], [['reachable', 'conditional'], ['reachable', 'conditional']]);
	// a conditional message is no branch: what follows is reached either way
	r = chapterReach([g({ events: [{ ...msg, if: { variable: 'v', op: 'set' } }, msg], continues: true }), g()]);
	assert.deepEqual([r.groups, r.events[0]], [['reachable', 'reachable'], ['reachable', 'reachable']]);
	// a scroll trigger on slice 2 that always jumps: slice 3 and the next group; a click trigger may never be clicked
	r = chapterReach([g({ continues: true, slices: [{}, { events: [{ ...jump, trigger: { type: 'intersect' } }] }, {}] }), g()]);
	assert.deepEqual([r.slices[0], r.groups[1]], [['reachable', 'reachable', 'unreachable'], 'unreachable']);
	r = chapterReach([g({ continues: true, slices: [{ events: [{ ...jump, trigger: { type: 'click' } }] }, {}] }), g()]);
	assert.deepEqual(r.groups, ['reachable', 'reachable']);
	// the follow-up bar leads to a labelled next chain – not after an authored end, not to an unlabelled group
	r = chapterReach([g(), g({ label: 'b' })], none);
	assert.deepEqual(r.groups, ['reachable', 'reachable']);
	r = chapterReach([g({ end: { target: { type: 'chapter', value: '3' } } }), g({ label: 'b' })], none);
	assert.deepEqual(r.groups, ['reachable', 'unreachable']);
	// events after an unconditional jump / divert; a slice event's divert enters the script at its point
	const script = [msg, jump, msg, { type: 'label', params: { name: 'p' } }, msg];
	assert.deepEqual(chapterReach([g({ events: script })]).events[0], ['reachable', 'reachable', 'unreachable', 'unreachable', 'unreachable']);
	r = chapterReach([g({ events: script, slices: [{ events: [{ type: 'divert', params: { to: 'p' }, trigger: { type: 'click' } }] }] })]);
	assert.deepEqual(r.events[0], ['reachable', 'reachable', 'unreachable', 'reachable', 'reachable']);
	// a choice: one that stays, one that jumps – both ways
	r = chapterReach([
		g({
			continues: true,
			events: [{ type: 'addChoice', params: { label: 'stay' } }, { type: 'addChoice', params: { label: 'go', onSelect: { jumpTo: { target: { type: 'chapter', value: '2' } } } } }, { type: 'showChoices' }, msg]
		}),
		g()
	]);
	assert.deepEqual([r.groups[1], r.events[0][3]], ['reachable', 'reachable']);
});

test('chapterReach: never-true conditions cut their branch; endless loops; big cyclic scripts stay fast', () => {
	const jump = { type: 'jumpTo', params: { target: { type: 'chapter', value: '2' } } };
	const msg = { type: 'showMessage', params: { text: 'x' } };
	const facts = toonFacts([]);
	// "if v is set → divert to p" with v set nowhere: p is unreachable, the test is reported
	const r = chapterReach([{ slices: [{}], events: [{ type: 'divert', params: { to: 'p' }, if: { variable: 'v', op: 'set' } }, jump, { type: 'label', params: { name: 'p' } }, msg] }], facts);
	assert.deepEqual(r.events[0], ['reachable', 'reachable', 'unreachable', 'unreachable']);
	assert.deepEqual(r.neverTrue, [['groups', 0, 'events', 0, 'if']]);
	// a slice event's condition too, at its own index
	const r2 = chapterReach([{ slices: [{ events: [{ ...msg, trigger: { type: 'click' } }, { ...msg, trigger: { type: 'intersect' }, if: { variable: 'w', op: '>', value: 1 } }] }] }], facts);
	assert.deepEqual(r2.neverTrue, [['groups', 0, 'slices', 0, 'events', 1, 'if']]);
	// a loop with no way out; a menu loop with a way out is fine
	const loop = chapterReach([{ slices: [{}], events: [{ type: 'label', params: { name: 'a' } }, msg, { type: 'divert', params: { to: 'a' } }] }]);
	assert.deepEqual(loop.loops, [['groups', 0, 'events', 2]]);
	const menu = chapterReach([
		{
			slices: [{}],
			events: [{ type: 'label', params: { name: 'm' } }, { type: 'addChoice', params: { label: 'again', onSelect: { divert: { to: 'm' } } } }, { type: 'addChoice', params: { label: 'leave', onSelect: { jumpTo: { target: { type: 'chapter', value: '2' } } } } }, { type: 'showChoices' }]
		}
	]);
	assert.deepEqual(menu.loops, []);
	// 3000 events diverting around in a cycle, 200 groups: linear, no recursion
	const big = Array.from({ length: 3000 }, (_, i) => (i % 3 === 0 ? { type: 'label', params: { name: `l${i}` } } : i % 3 === 1 ? { type: 'divert', params: { to: `l${(i + 2) % 3000}` }, if: { variable: 'v', op: 'set' } } : msg));
	const t = Date.now();
	const many = chapterReach(Array.from({ length: 200 }, (_, i) => ({ label: `g${i}`, continues: i < 199, slices: [{}, {}], events: big })));
	assert.equal(many.groups.length, 200);
	assert.ok(Date.now() - t < 3000, `took ${Date.now() - t} ms`);
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
	assert.deepEqual(run(data, {}, /unreachable/), ['warning groups.1', 'warning groups.0.events.1']);
});
