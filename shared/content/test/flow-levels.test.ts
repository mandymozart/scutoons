// The flow on every level (owner 2026-10-06, agents/DESIGN.md "Flow panel"): group, slice, season / toon (+ toonReach),
// SCUTOONS, dead things.   node --experimental-transform-types --no-warnings --test test/flow-levels.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { chapterExits, chapterFlow, deadThings, groupFlow, sliceFlow, toonFacts, toonFlow, toonReach, universeFlow, type ChapterFlow, type FlowChapter } from '../src/index.ts';

const msg = (text: string, character?: string) => ({ type: 'showMessage', params: { text, ...(character ? { character } : {}) } });
const join = (character: string, persona?: string) => ({ type: 'joinScene', params: { characters: [{ character, ...(persona ? { persona } : {}), variant: 'avatar' }] } });
const jump = (value: string, label?: string, type = 'chapter') => ({ type: 'jumpTo', params: { target: { type, value, ...(label ? { label } : {}) } } });
const choice = (label: string, onSelect?: object, extra: object = {}) => ({ type: 'addChoice', params: { label, ...(onSelect ? { onSelect } : {}) }, ...extra });
const node = (f: { nodes: ChapterFlow['nodes'] }, id: string) => f.nodes.find((n) => n.id === id);
const out = (f: { edges: ChapterFlow['edges'] }, from: string) =>
	f.edges.filter((e) => e.from === from).map((e) => `${e.kind}:${e.to}${e.label ? `[${e.label}]` : ''}${e.branch ? `(${e.branch})` : ''}${e.condition ? `{${e.condition}}` : ''}${e.never ? '!' : ''}`);

test('groupFlow: every event – plain ones in runs between the key points, steps under a test, where the reader comes from', () => {
	const groups = [
		{ label: 'a', slices: [{}], events: [jump('2', 'b')] },
		{
			label: 'b',
			slices: [{ events: [{ ...msg('tap'), trigger: { type: 'click' } }] }, {}],
			events: [
				join('catpope', 'cat'),
				msg('Mama?', 'catpope'),
				{ type: 'setVariable', params: { variable: 'visits', value: 1, op: 'add' } },
				{ ...msg('again'), if: { variable: 'visits', op: '>=', value: 2 } },
				{ type: 'label', params: { name: 'ask' } },
				choice('stay', { divert: { to: 'ask' } }),
				choice('go', { jumpTo: { target: { type: 'toon', value: 'toptoy' } } }),
				{ type: 'showChoices' },
				{ type: 'showTextBox', params: { dialog: 'x' } }
			]
		}
	];
	const facts = toonFacts([groups]);
	const f = groupFlow(groups, 1, { chapter: 2, facts });
	// the group, the one it comes from (a stub), its script
	assert.equal(node(f, 'g1')?.stub, undefined);
	assert.equal(node(f, 'g0')?.stub, true);
	assert.deepEqual(out(f, 'g0'), ['jump:g1']);
	assert.deepEqual(out(f, 'g1'), ['then:g1e0', 'trigger:g1s0t0']);
	const run = node(f, 'g1e0')!;
	assert.equal(run.kind, 'run');
	assert.deepEqual([run.events, run.messages, run.joins, run.speakers, run.sets, run.first], [[0, 2], 1, ['catpope·cat'], ['catpope'], ['visits'], 'Mama?']);
	// a plain event under a test: a step in line, the test on the edge into it
	assert.equal(node(f, 'g1e3')?.kind, 'step');
	assert.equal(node(f, 'g1e3')?.condition, 'visits >= 2');
	assert.deepEqual(out(f, 'g1e0'), ['then:g1e3{visits >= 2}']);
	assert.deepEqual(out(f, 'g1e3'), ['then:g1e4']);
	assert.deepEqual(out(f, 'g1e7'), ['choice:g1e4[stay]', 'jump:x:toon:toptoy::[go]']);
	// after a question that always diverts or jumps: the text box is drawn, unreachable, nothing leads there
	assert.equal(node(f, 'g1e8')?.status, 'unreachable');
	assert.ok(!f.edges.some((e) => e.to === 'g1e8'));
	// the trigger's own events (every run on this level, not only the branching ones)
	assert.equal(node(f, 'g1s0t0')?.trigger, 'click');
	assert.deepEqual(out(f, 'g1s0t0'), ['then:g1s0t0e0']);
	assert.equal(node(f, 'g1s0t0e0')?.messages, 1);
	// the chapter level is unchanged: no runs, no non-branching triggers
	const chapter = chapterFlow(groups, { chapter: 2, facts });
	assert.ok(!chapter.nodes.some((n) => n.kind === 'run' || n.kind === 'step' || n.kind === 'trigger'));
	// an unknown group: empty
	assert.deepEqual(groupFlow(groups, 7).nodes, []);
});

test('groupFlow: after the group – linked, the follow-up bar, an authored end (auto), the next chapter', () => {
	const groups = [
		{ label: 'a', continues: true, slices: [{}], events: [msg('1')] },
		{ slices: [{}], end: { target: { type: 'chapter', value: '0' }, auto: true } },
		{ label: 'c', slices: [{}] }
	];
	assert.deepEqual(out(groupFlow(groups, 0, { chapter: 2 }), 'g0e0'), ['continues:g1']);
	assert.deepEqual(out(groupFlow(groups, 1, { chapter: 2 }), 'g1'), ['end:x:chapter:0::(auto)']);
	assert.deepEqual(out(groupFlow(groups, 2, { chapter: 2 }), 'g2'), ['follow:x:next']);
});

test('sliceFlow: the slice, its triggers and item taps, the script points they divert into, the events that reveal its boxes', () => {
	const groups = [
		{
			label: 'a',
			slices: [
				{},
				{
					dialog: [{ id: 'box-1' }],
					events: [{ type: 'divert', params: { to: 'miau' }, trigger: { type: 'click' } }],
					items: [{ item: 'cucumber', events: [msg('A cucumber!'), jump('3')] }]
				}
			],
			events: [msg('x'), { type: 'showTextBox', params: { dialog: 'box-1' } }, { type: 'label', params: { name: 'miau' } }, msg('MIAU')]
		}
	];
	const f = sliceFlow(groups, 0, 1, { chapter: 2 });
	assert.equal(node(f, 'g0')?.stub, true);
	assert.equal(node(f, 'g0s1')?.kind, 'slice');
	assert.deepEqual(out(f, 'g0'), ['then:g0s1', 'then:g0e1']);
	// the script event that reveals the slice's box
	assert.deepEqual(node(f, 'g0e1')?.boxes, ['box-1']);
	assert.deepEqual(out(f, 'g0e1'), ['reveal:g0s1']);
	// the click diverts into the group's script: its point as a stub
	assert.deepEqual(out(f, 'g0s1t0'), ['divert:g0e2']);
	assert.deepEqual([node(f, 'g0e2')?.kind, node(f, 'g0e2')?.name, node(f, 'g0e2')?.stub], ['label', 'miau', true]);
	// the item tap: its message run, then the jump out of the chapter
	assert.equal(node(f, 'g0s1t1')?.trigger, 'item');
	assert.deepEqual(out(f, 'g0s1t1e0'), ['then:g0s1t1e1']);
	assert.deepEqual(out(f, 'g0s1t1e1'), ['jump:x:chapter:3::']);
	assert.equal(node(f, 'x:chapter:3::')?.status, 'reachable');
	assert.deepEqual(sliceFlow(groups, 0, 9).nodes, []);
});

// a toon: season 1 = chapters 1, 2, 3, 4; chapter 2 always jumps to chapter 3's label (no follow-up bar), chapter 3
// jumps to another toon or (only with coins) back to 1; chapter 4 is a draft only the order would reach; chapter 9 is in
// no season but a jump of chapter 1 (through a condition) leads there; a visible map place opens chapter 4
const toon = (places = true) => {
	const chapters: FlowChapter[] = [
		{ number: 1, status: 'published', title: 'One', groups: [{ slices: [{}], events: [{ ...jump('9'), if: { variable: 'coins', op: 'set' } }] }] },
		{ number: 2, status: 'draft', groups: [{ slices: [{}], events: [jump('3', 'forest')] }] },
		{
			number: 3,
			status: 'draft',
			groups: [
				{ slices: [{}] },
				{ label: 'forest', slices: [{}], events: [choice('Top Toy', { jumpTo: { target: { type: 'toon', value: 'toptoy' } } }), choice('back', { jumpTo: { target: { type: 'chapter', value: '1' } } }, { if: { variable: 'coins', op: 'set' } }), { type: 'showChoices' }] }
			]
		},
		{ number: 4, status: 'draft', groups: [{ slices: [{}] }, { slices: [{}] }] },
		{ number: 9, groups: [{ slices: [{}] }] }
	];
	const facts = toonFacts([...chapters.map((c) => c.groups), { type: 'setVariable', params: { variable: 'coins', value: 1 } }]);
	return { toon: 'catpope', chapters, seasons: [{ number: 1, chapters: [1, 2, 3, 4] }], facts, places: places ? [{ id: 'well', name: 'Well', chapter: 4 }] : [] };
};

test('chapterExits: the ways out of a chapter with the best status', () => {
	const t = toon();
	const exits = chapterExits(chapterFlow(t.chapters[2].groups, { chapter: 3, facts: t.facts }));
	assert.deepEqual(
		exits.map((x) => `${x.kind}:${x.next ? 'next' : `${x.target?.type}:${x.target?.value}`}:${x.status}${x.label ? `[${x.label}]` : ''}`),
		['jump:toon:toptoy:reachable[Top Toy]', 'jump:chapter:1:conditional[back]']
	);
	assert.deepEqual(chapterExits(chapterFlow(t.chapters[0].groups, { chapter: 1, facts: t.facts })).map((x) => `${x.kind}:${x.next ? 'next' : x.target?.value}:${x.status}`), ['jump:9:conditional', 'order:next:reachable']);
});

test('toonReach: entries (season start, published, visible places), the season order, jumps; conditions', () => {
	const r = toonReach(toon(false));
	assert.deepEqual([...r].map(([n, x]) => `${n}:${x.reach}`), ['1:reachable', '2:reachable', '3:reachable', '4:unreachable', '9:conditional']);
	// chapter 2 never reaches its follow-up bar: chapter 4 only through a visible map place
	assert.equal(toonReach(toon()).get(4)?.reach, 'reachable');
	// dead groups per chapter (chapterReach): chapter 3's first group can't get to its labelled second one … but it is the entry
	assert.equal(r.get(3)?.dead, 0);
	assert.equal(r.get(4)?.dead, 1, 'an unlabelled group after a group: the bar leads to the next chapter');
});

test('toonFlow: season level (its chapters, exits elsewhere) and toon level (every chapter, map places)', () => {
	const season = toonFlow(toon(), { season: 1 });
	assert.deepEqual(season.nodes.filter((n) => n.kind === 'chapter').map((n) => `${n.id}:${n.status}:${n.state}`), ['c1:reachable:published', 'c2:reachable:draft', 'c3:reachable:draft', 'c4:reachable:draft']);
	assert.deepEqual(out(season, 'c1'), ['jump:x:chapter:9{}'.replace('{}', ''), 'order:c2']);
	assert.deepEqual(out(season, 'c2'), ['jump:c3[forest]']);
	assert.deepEqual(out(season, 'c3'), ['jump:x:toon:toptoy[Top Toy]', 'jump:c1[back]']);
	assert.equal(season.edges.find((e) => e.from === 'c3' && e.to === 'c1')?.status, 'conditional');
	assert.equal(node(season, 'x:chapter:9')?.status, 'conditional');
	assert.ok(!season.nodes.some((n) => n.kind === 'place'), 'no places on the season level');
	const all = toonFlow(toon());
	assert.deepEqual(all.nodes.filter((n) => n.kind === 'chapter').map((n) => n.id), ['c1', 'c2', 'c3', 'c4', 'c9']);
	assert.deepEqual(out(all, 'c1'), ['jump:c9', 'order:c2']);
	assert.deepEqual(node(all, 'p4')?.places, ['Well']);
	assert.deepEqual(out(all, 'p4'), ['place:c4']);
	// parallel ways between two chapters are merged and counted
	const twice = toonFlow({ ...toon(), chapters: [{ number: 1, groups: [{ slices: [{}], events: [choice('a', { jumpTo: { target: { type: 'chapter', value: '2' } } }), choice('b', { jumpTo: { target: { type: 'chapter', value: '2' } } }), { type: 'showChoices' }] }] }, { number: 2, groups: [{ slices: [{}] }] }], seasons: [{ number: 1, chapters: [1, 2] }], places: [] });
	assert.deepEqual(twice.edges.map((e) => `${e.from}>${e.to}:${e.kind}:${e.label}:${e.count ?? 1}`), ['c1>c2:jump:a · b:2']);
});

test('universeFlow: one box per toon, jumps between toons merged, products / XR as exits', () => {
	const f = universeFlow({
		toons: [
			{ id: 'catpope', title: { de: 'Katzenpapst' }, status: 'published', seasons: [{ number: 1, chapters: [{ number: 0 }, { number: 1 }] }] },
			{ id: 'toptoy', title: 'Top Toy', status: 'draft', seasons: [{ number: 1, chapters: [{ number: 1 }] }] }
		],
		edges: [
			{ source: { toon: 'catpope', chapter: 3 }, target: { type: 'toon', value: 'toptoy' } as any, kind: 'choice', label: 'Hyper City' },
			{ source: { toon: 'catpope', chapter: 3, group: 'x' }, target: { type: 'toon', value: 'toptoy' } as any, kind: 'choice', label: 'again' },
			{ source: { toon: 'toptoy', chapter: 1 }, target: { type: 'toon', value: 'catpope' } as any, kind: 'jump' },
			{ source: { toon: 'toptoy', chapter: 1 }, target: { type: 'chapter', value: '2' } as any, kind: 'jump' },
			{ source: { toon: 'toptoy', chapter: 1 }, target: { type: 'product', value: 'vinyl' } as any, kind: 'end' }
		]
	});
	assert.deepEqual(f.nodes.map((n) => `${n.id}:${n.kind}:${n.name ?? ''}:${n.count ?? ''}`), ['t:catpope:toon:Katzenpapst:2', 't:toptoy:toon:Top Toy:1', 'x:product:vinyl:exit::']);
	assert.deepEqual(f.edges.map((e) => `${e.from}>${e.to}:${e.kind}:${e.count}`), ['t:catpope>t:toptoy:choice:2', 't:toptoy>t:catpope:jump:1', 't:toptoy>x:product:vinyl:end:1']);
});

test('deadThings: variables set but never read, read but never set, points and labels nothing jumps to', () => {
	const chapters = [
		{
			number: 2,
			groups: [
				{
					label: 'start',
					continues: true,
					slices: [{ dialog: [{ id: 'b', text: { de: 'Runde {laps}' } }], events: [{ type: 'divert', params: { to: 'tap' }, trigger: { type: 'click' } }] }],
					events: [
						{ type: 'setVariable', params: { variable: 'searching', value: 'yes' } },
						{ type: 'setVariable', params: { variable: 'laps', value: 1, op: 'add' } },
						{ ...msg('rocket'), if: { variable: 'rocket', op: '==', value: 'yes' } },
						{ ...msg('{shouts} times') },
						{ type: 'label', params: { name: 'tap' } },
						{ type: 'label', params: { name: 'lonely' } },
						choice('x', { add: { variable: 'shouts', value: 1 } }),
						{ type: 'showChoices' }
					]
				},
				{ label: 'middle', slices: [{}] },
				{ label: 'after-end', slices: [{}], end: { target: { type: 'chapter', value: '3' } } },
				{ label: 'attic', slices: [{}] },
				{ label: 'used', slices: [{}] }
			]
		}
	];
	const facts = toonFacts([chapters[0].groups, { jumpTo: { target: { type: 'chapter', value: '2', label: 'used' } } }]);
	const dead = deadThings(chapters, facts);
	assert.deepEqual(dead.unread.map((x) => x.name), ['searching']);
	assert.deepEqual(dead.unset.map((x) => `${x.name}@${x.at[0].chapter}.${x.at[0].group}:${x.at[0].label}`), ['rocket@2.0:start']);
	assert.deepEqual(dead.points.map((x) => x.name), ['lonely']);
	// `middle` follows a linked group (nothing needs its label), `attic` follows an authored end; `after-end` gets the
	// follow-up bar, `used` a jump
	assert.deepEqual(dead.labels.map((x) => x.name), ['middle', 'attic']);
	assert.deepEqual(deadThings(chapters).labels, [], 'without the facts no label is judged');
});
