// Jumps into another toon's chapter + label (owner 2026-10-06, "at label"): `JumpToTarget.toon` on chapter targets –
// reach / facts / flow on every level, validation.   node --experimental-transform-types --no-warnings --test test/toon-jumps.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { chapterFlow, chapterReach, deadThings, incomingJumps, intoOtherToon, targetToon, toonFacts, toonFlow, toonReach, universeFlow, validateFile, type FlowEdge } from '../src/index.ts';

const choice = (label: string, onSelect?: object) => ({ type: 'addChoice', params: { label, ...(onSelect ? { onSelect } : {}) } });
const into = (toon: string, value: string, label?: string, extra: object = {}) => ({ type: 'chapter', value, toon, ...(label ? { label } : {}), ...extra });
const out = (f: { edges: FlowEdge[] }, from: string) => f.edges.filter((e) => e.from === from).map((e) => `${e.kind}:${e.to}${e.label ? `[${e.label}]` : ''}`);

// catpope 3: choices into toptoy 1 at `hyper` and at toptoy's `middle`; catpope's own `middle` follows a linked group
const catpope = [
	{
		label: 'forest',
		continues: true,
		slices: [{}],
		events: [choice('Hyper City', { jumpTo: { target: into('toptoy', '1', 'hyper') } }), choice('Mitte', { jumpTo: { target: into('toptoy', '1', 'middle') } }), { type: 'showChoices' }]
	},
	{ label: 'middle', slices: [{}] }
];
// toptoy 1: `city` ends into catpope 3; `hyper` (after the end) and `middle` (linked) only catpope's jumps reach; `lost`
// (linked) no jump names
const toptoy = [
	{ label: 'city', slices: [{}], end: { target: into('catpope', '3', 'forest') } },
	{ label: 'hyper', continues: true, slices: [{}] },
	{ label: 'middle', continues: true, slices: [{}] },
	{ label: 'lost', slices: [{}] }
];
// the server's flow index (GET /api/creator/flow-index) lists these ways
const index = {
	edges: [
		{ source: { toon: 'catpope', chapter: 3 }, target: into('toptoy', '1', 'hyper') as any, kind: 'choice' as const },
		{ source: { toon: 'catpope', chapter: 3 }, target: into('toptoy', '1', undefined, { anyOf: ['middle', 'hyper'] }) as any, kind: 'jump' as const },
		{ source: { toon: 'toptoy', chapter: 1 }, target: into('catpope', '3', 'forest') as any, kind: 'end' as const },
		{ source: { toon: 'toptoy', chapter: 1 }, target: { type: 'chapter', value: '1', label: 'lost' } as any, kind: 'end' as const }
	]
};

test('targetToon / intoOtherToon: where a target leads', () => {
	assert.equal(targetToon(into('toptoy', '1'), 'catpope'), 'toptoy');
	assert.equal(targetToon({ type: 'chapter', value: '1' }, 'catpope'), 'catpope');
	assert.equal(targetToon({ type: 'toon', value: 'yellowmouse' }, 'catpope'), 'yellowmouse');
	assert.equal(intoOtherToon(into('toptoy', '1'), 'catpope'), true);
	assert.equal(intoOtherToon(into('catpope', '1'), 'catpope'), false, 'its own toon named');
	assert.equal(intoOtherToon(into('catpope', '1')), true, 'an unknown toon: any named toon is another one');
	assert.equal(intoOtherToon({ type: 'toon', value: 'toptoy' }, 'catpope'), false, "a toon's page is no chapter target");
});

test('toonFacts: a target into another toon is no entry here; incoming labels are', () => {
	assert.deepEqual([...toonFacts([catpope], { toon: 'catpope' }).labels], []);
	assert.deepEqual([...toonFacts([catpope]).labels], []);
	assert.deepEqual([...toonFacts([into('catpope', '3', 'mine')], { toon: 'catpope' }).labels], ['mine'], 'its own toon named: counts');
	assert.equal(chapterReach(catpope, toonFacts([catpope], { toon: 'catpope' })).groups[1], 'unreachable');
	const incoming = incomingJumps(index, 'toptoy');
	assert.deepEqual(incoming, { chapters: [1], labels: ['hyper', 'middle'] });
	assert.deepEqual(incomingJumps(index, 'catpope'), { chapters: [3], labels: ['forest'] });
	assert.deepEqual(incomingJumps(null, 'catpope'), { chapters: [], labels: [] });
	const facts = toonFacts([toptoy], { toon: 'toptoy', incoming: incoming.labels });
	assert.deepEqual(chapterReach(toptoy, toonFacts([toptoy], { toon: 'toptoy' })).groups, ['reachable', 'unreachable', 'unreachable', 'unreachable']);
	assert.deepEqual(chapterReach(toptoy, facts).groups, ['reachable', 'reachable', 'reachable', 'reachable']);
	// dead labels: without the incoming jumps `hyper` / `middle` look dead (the PHP flow index agrees: FlowIndexApiTest)
	assert.deepEqual(deadThings([{ number: 1, groups: toptoy }], toonFacts([toptoy], { toon: 'toptoy' })).labels.map((x) => x.name), ['hyper', 'middle', 'lost']);
	assert.deepEqual(deadThings([{ number: 1, groups: toptoy }], facts).labels.map((x) => x.name), ['lost']);
	assert.deepEqual(deadThings([{ number: 3, groups: catpope }], toonFacts([catpope], { toon: 'catpope' })).labels.map((x) => x.name), ['middle']);
});

test('flow: chapter / toon level exits into the other toon, toon reach entries, SCUTOONS toon → toon', () => {
	// chapter level: an exit (catpope has a chapter 1 too – never its groups)
	const f = chapterFlow(catpope, { chapter: 1, toon: 'catpope' });
	assert.deepEqual(
		f.nodes.filter((n) => n.kind === 'exit' && n.target).map((n) => `${n.id} ${JSON.stringify(n.target)}`),
		['x:chapter:toptoy/1:hyper: {"type":"chapter","value":"1","toon":"toptoy","label":"hyper"}', 'x:chapter:toptoy/1:middle: {"type":"chapter","value":"1","toon":"toptoy","label":"middle"}']
	);
	// toon reach: a chapter other toons jump into is an entry (like a visible map place)
	const facts = toonFacts([toptoy], { toon: 'toptoy', incoming: incomingJumps(index, 'toptoy').labels });
	const input = { toon: 'toptoy', chapters: [{ number: 1, groups: toptoy }, { number: 3, groups: [{ slices: [{}] }] }], seasons: [{ number: 1, chapters: [3] }], facts };
	assert.equal(toonReach(input).get(1)?.reach, 'unreachable');
	assert.equal(toonReach({ ...input, incoming: incomingJumps(index, 'toptoy').chapters }).get(1)?.reach, 'reachable');
	// toon level: the end into catpope 3 is an exit, never toptoy's own chapter 3
	const tf = toonFlow({ ...input, incoming: [1] });
	assert.deepEqual(out(tf, 'c1'), ['end:x:chapter:catpope/3[forest]']);
	assert.deepEqual(tf.nodes.find((n) => n.id === 'x:chapter:catpope/3')?.target, { type: 'chapter', value: '3', toon: 'catpope' });
	// SCUTOONS level: toon → toon, merged with jumps to the toon's page; jumps inside a toon stay inside
	const u = universeFlow({ toons: [{ id: 'catpope', seasons: [] }, { id: 'toptoy', seasons: [] }], edges: [...index.edges, { source: { toon: 'catpope', chapter: 3 }, target: { type: 'toon', value: 'toptoy' } as any, kind: 'jump' }] });
	assert.deepEqual(u.edges.map((e) => `${e.from}>${e.to}:${e.kind}:${e.count}`), ['t:catpope>t:toptoy:choice:1', 't:catpope>t:toptoy:jump:2', 't:toptoy>t:catpope:end:1']);
});

test('validation: toon only on chapter targets; toon / chapter / label checked where the caller knows them – a known missing place is an error (owner 2026-10-06)', () => {
	const chapter = (target: object) => ({ number: 3, groups: [{ label: 'a', slices: [{ image: 'x.webp' }], end: { target } }] });
	const run = (target: object, targets?: object) =>
		validateFile('chapter', chapter(target), 'de', { file: 'x', ...(targets ? { targets } : {}) } as never)
			.filter((p) => p.path.includes('end'))
			.map((p) => `${p.level} ${p.path.slice(3).join('.')}: ${p.message}`);
	assert.deepEqual(run(into('toptoy', '1', 'hyper')), [], 'no context: nothing to check');
	assert.deepEqual(run({ type: 'toon', value: 'toptoy', toon: 'toptoy' }), ['error target.toon: toon belongs to a chapter target (a chapter of another toon) – a toon page is type "toon"']);
	assert.deepEqual(run(into('', '1')), ['error target.toon: toon must be a toon id (absent = this toon)']);
	const known = {
		toon: 'catpope',
		toons: ['catpope', 'toptoy', 'yellowmouse'],
		chapters: (t: string) => (t === 'catpope' ? [1, 2, 3] : t === 'toptoy' ? [1] : undefined),
		labels: (t: string, n: number) => (t === 'catpope' && n === 2 ? ['mama-crossroads'] : t === 'toptoy' && n === 1 ? ['hyper'] : undefined)
	};
	assert.deepEqual(run(into('toptoy', '1', 'hyper'), known), []);
	assert.deepEqual(run({ type: 'chapter', value: '2', label: 'mama-crossroads' }, known), []);
	assert.deepEqual(run(into('nobody', '1'), known), ['error target.toon: there is no toon "nobody"']);
	assert.deepEqual(run({ type: 'toon', value: 'nobody' }, known), ['error target.value: there is no toon "nobody"']);
	assert.deepEqual(run(into('toptoy', '7'), known), ['error target.value: toptoy has no chapter 7']);
	assert.deepEqual(run({ type: 'chapter', value: '8' }, known), ['error target.value: the toon has no chapter 8']);
	assert.deepEqual(run(into('toptoy', '1', undefined, { anyOf: ['hyper', 'gone'] }), known), ['error target.anyOf: toptoy chapter 1 has no group labelled "gone"']);
	assert.deepEqual(run({ type: 'chapter', value: '2', label: 'nope' }, known), ['error target.label: chapter 2 has no group labelled "nope"']);
	// this toon's chapter not loaded: no check; another toon the creator can't see: its labels can't be checked
	assert.deepEqual(run({ type: 'chapter', value: '1', label: 'x' }, known), []);
	assert.deepEqual(run(into('yellowmouse', '1', 'mama-boss'), known), [
		`warning target.label: can't check yellowmouse chapter 1: its groups aren't loaded (no access to yellowmouse?) – make sure "mama-boss" exist there`
	]);
});
