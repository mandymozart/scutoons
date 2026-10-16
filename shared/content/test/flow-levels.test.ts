// The flow on every level (owner 2026-10-06, agents/DESIGN.md "Flow panel"): group, slice, season / toon (+ toonReach),
// SCUTOONS, dead things.   node --experimental-transform-types --no-warnings --test test/flow-levels.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { chapterExits, chapterFlow, chapterPictures, chapterReach, deadThings, groupFlow, groupLayout, rendererOf, scriptFaces, sliceFlow, sliceImage, targetPictures, toonFacts, toonFlow, toonReach, universeFlow, type ChapterFlow, type FlowChapter } from '../src/index.ts';

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
	assert.equal(f.edges[0].label, 'Hyper City · again', 'the merged ways keep their texts');
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

test('step 3: expression conditions, askText, repeat rules (once for good), the dead things read expressions', () => {
	const ask = (variable: string | undefined, scope?: string) => ({ type: 'askText', params: { text: 'What’s your name?', ...(variable ? { variable } : {}), ...(scope ? { scope } : {}) } });
	const groups = [
		{
			label: 'a',
			slices: [
				{
					events: [
						{ ...msg('first time'), id: 'e1', trigger: { type: 'click', repeat: { mode: 'once' } } },
						{ ...jump('3'), id: 'e2', trigger: { type: 'click', repeat: { mode: 'once' } } },
						// no repeat and `every` are one rule – one run
						{ ...jump('4'), id: 'e3', trigger: { type: 'hover' } },
						{ ...msg('again'), id: 'e4', trigger: { type: 'hover', repeat: { mode: 'every' } } }
					],
					items: [{ item: 'key', repeat: { mode: 'once' }, events: [jump('5')] }]
				}
			],
			events: [
				ask('hero'),
				ask(undefined, 'reader'),
				{ ...jump('2'), if: { expr: 'coins >= 3 && met' } },
				{ ...msg('{hero}, hi {reader.name}'), if: { expr: '  ' } },
				msg('{gold > 2: rich|poor}')
			]
		}
	];
	// (1) an expression is a condition (dashed, its text on the edge), a blank one isn't; never "never true"
	const facts = toonFacts([groups]);
	assert.ok(facts.variables.has('hero'), 'askText (toon scope) sets its variable');
	assert.ok(!facts.variables.has('reader.name'));
	const reach = chapterReach(groups, facts);
	assert.deepEqual(reach.neverTrue, []);
	const f = chapterFlow(groups, { chapter: 2, facts, reach });
	assert.ok(f.edges.some((e) => e.from === 'g0e2' && e.condition === 'coins >= 3 && met'), JSON.stringify(out(f, 'g0e2')));
	assert.ok(f.edges.some((e) => e.from === 'g0e2' && e.branch === 'else'), 'a conditional jump goes on when its test fails');
	// (5) (6) once for good: marked on the trigger / item tap; no repeat = every = one run
	const triggers = f.nodes.filter((n) => n.kind === 'trigger');
	assert.deepEqual(triggers.map((n) => [n.trigger, n.repeat ?? 'every']), [['click', 'once'], ['hover', 'every'], ['item', 'once']]);
	assert.deepEqual(reach.once, [['groups', 0, 'slices', 0, 'events', 0], ['groups', 0, 'slices', 0, 'items', 0, 'events', 0]]);
	const hover = groupFlow(groups, 0, { chapter: 2, facts, reach }).nodes.find((n) => n.kind === 'trigger' && n.trigger === 'hover')!;
	assert.equal(hover.path?.join('.'), 'groups.0.slices.0.events.2');
	// (4) askText: a step of its own on the group level – the question and where the answer goes
	const g = groupFlow(groups, 0, { chapter: 2, facts, reach });
	assert.deepEqual([node(g, 'g0e0')?.kind, node(g, 'g0e0')?.event, node(g, 'g0e0')?.asks, node(g, 'g0e0')?.first], ['step', 'askText', 'hero', 'What’s your name?']);
	assert.equal(node(g, 'g0e1')?.asks, 'reader.name');
	assert.equal(node(g, 'g0e1')?.condition, undefined);
	// a blank expression is no condition: the message joins the run after the jump
	assert.equal(node(g, 'g0e3')?.kind, 'run');
	// (3) dead things: expression conditions and texts' {…} read (names of one part), askText sets
	const dead = deadThings([{ number: 2, groups }], facts);
	assert.deepEqual(dead.unread, [], JSON.stringify(dead.unread));
	assert.deepEqual(dead.unset.map((d) => d.name), ['coins', 'gold', 'met']);
});

test('pictures: every level names its small picture as data (slices low, thumbnails, avatars, items, faces)', () => {
	const s = (n: number, variants = false) => ({ image: `s1/c2/slice-${n}.webp`, width: 390, height: 625, ...(variants ? { imageVariants: { low: `s1/c2/slice-${n}.webp`, medium: `s1/c2/slice-${n}.2x.webp` } } : {}) });
	const groups = [
		{
			label: 'start',
			slices: [s(1, true), { ...s(2), items: [{ id: 'cucumber-1', item: 'cucumber', rect: { x: 0, y: 0, width: 10 }, events: [msg('crunch')] }] }],
			events: [
				join('cat'), // legacy id → catpope · cat
				{ type: 'showMessage', params: { text: 'Mama?', character: 'cat', mood: 'sad' } },
				msg('Not your mama', 'boy'),
				choice('harbour', { jumpTo: { target: { type: 'chapter', value: '2', label: 'harbour' } } }),
				choice('forest', { jumpTo: { target: { type: 'chapter', value: '3', label: 'forest' } } }),
				choice('hypercity', { jumpTo: { target: { type: 'chapter', value: '1', toon: 'toptoy', label: 'hyper' } } }),
				{ type: 'showChoices' },
				jump('2', 'harbour')
			]
		},
		{ label: 'harbour', renderer: { type: 'horizontal' }, slices: [s(5)] },
		{ label: 'band', renderer: { type: 'horizontal', params: { endless: true } }, slices: [] }
	];
	const facts = toonFacts([groups], { toon: 'catpope' });
	const f = chapterFlow(groups, { chapter: 2, toon: 'catpope', facts, nextChapter: 3 });
	// a group: its first slice (the low variant), everyone acting in order (persona / mood as named), its layout, entry
	assert.deepEqual(node(f, 'g0')?.image, { kind: 'slice', path: 's1/c2/slice-1.webp', slice: 0, toon: 'catpope', group: 0 });
	assert.deepEqual(node(f, 'g0')?.faces, [{ character: 'catpope', persona: 'cat' }, { character: 'boy' }]);
	assert.deepEqual([node(f, 'g0')?.layout, node(f, 'g1')?.layout, node(f, 'g2')?.layout], ['vertical', 'strip', 'endless']);
	// the creator's Layout select writes the renderer back: groupLayout(rendererOf(x)) = x; endless off a horizontal group is vertical
	for (const layout of ['vertical', 'strip', 'endless'] as const) assert.equal(groupLayout({ renderer: rendererOf(layout) }), layout);
	assert.deepEqual([rendererOf('vertical'), rendererOf('strip')], [{ type: 'vertical' }, { type: 'horizontal' }]);
	assert.equal(groupLayout({ renderer: { type: 'vertical', params: { endless: true } } }), 'vertical');
	assert.equal(node(f, 'g2')?.image, undefined, 'no slice: no picture (the UI draws the dashed slot)');
	assert.deepEqual([node(f, 'g0')?.entry, node(f, 'g1')?.entry, node(f, 'g2')?.entry], [undefined, true, undefined], 'harbour: something jumps there');
	// the question: who asks (the last speaker before it)
	const q = f.nodes.find((n) => n.kind === 'choices')!;
	assert.deepEqual(q.faces, [{ character: 'boy' }]);
	// a jump in the chapter: where the label lands (its group's first slice); outside: the chapter (+ label), another toon's badge
	assert.deepEqual(node(f, 'g0e7')?.image, { kind: 'slice', path: 's1/c2/slice-5.webp', slice: 0, toon: 'catpope', group: 1 });
	const forest = f.nodes.find((n) => n.kind === 'exit' && n.target?.value === '3')!;
	assert.deepEqual([forest.image, forest.badge], [{ kind: 'chapter', chapter: 3, label: 'forest' }, undefined]);
	const toptoy = f.nodes.find((n) => n.kind === 'exit' && n.target?.toon === 'toptoy')!;
	assert.deepEqual([toptoy.image, toptoy.badge], [{ kind: 'chapter', chapter: 1, toon: 'toptoy', label: 'hyper' }, { kind: 'toon', toon: 'toptoy' }]);
	// the next chapter: its picture when the caller knows which one
	assert.deepEqual(node(f, 'x:next')?.image, { kind: 'chapter', chapter: 3 });
	// group level: a run's speakers' faces (mood as named), an item tap's item
	const g = groupFlow(groups, 0, { chapter: 2, toon: 'catpope', facts });
	assert.deepEqual(node(g, 'g0e0')?.faces, [{ character: 'catpope', persona: 'cat', mood: 'sad' }, { character: 'boy' }]);
	const tap = g.nodes.find((n) => n.kind === 'trigger' && n.trigger === 'item')!;
	assert.deepEqual([tap.item, tap.image], ['cucumber', { kind: 'item', item: 'cucumber' }]);
	// slice level: the slice's own image
	assert.deepEqual(node(sliceFlow(groups, 0, 1, { chapter: 2, toon: 'catpope' }), 'g0s1')?.image, { kind: 'slice', path: 's1/c2/slice-2.webp', slice: 1, toon: 'catpope', group: 0 });
	// season / toon: a chapter's thumbnail from its document
	const toon = toonFlow({ toon: 'catpope', seasons: [{ number: 1, chapters: [2, 3] }], chapters: [{ number: 2, groups, thumbnail: 's1/c2/thumb.jpg' }, { number: 3, groups: [{ label: 'forest', slices: [s(9)] }] }] });
	assert.deepEqual(node(toon, 'c2')?.image, { kind: 'thumbnail', path: 's1/c2/thumb.jpg', chapter: 2, toon: 'catpope' });
	assert.equal(node(toon, 'c3')?.image, undefined);
	// SCUTOONS: a toon's avatar; the flow index's chapter pictures (what the server lists)
	const u = universeFlow({ toons: [{ id: 'catpope', seasons: [] }], edges: [{ source: { toon: 'catpope', chapter: 2 }, target: { type: 'toon', value: 'nobody' }, kind: 'jump' }] });
	assert.deepEqual([node(u, 't:catpope')?.image, node(u, 'x:toon:nobody')?.image], [{ kind: 'toon', toon: 'catpope' }, { kind: 'toon', toon: 'nobody' }]);
	assert.deepEqual(chapterPictures(groups, 's1/c2/thumb.jpg'), { thumbnail: 's1/c2/thumb.jpg', labels: { start: 's1/c2/slice-1.webp', harbour: 's1/c2/slice-5.webp' }, first: 's1/c2/slice-1.webp' });
	assert.deepEqual(chapterPictures([]), {});
	assert.deepEqual(scriptFaces([join('plf'), msg('x', 'plf')]), [{ character: 'catpope', persona: 'human' }]);
	assert.deepEqual(sliceImage({ image: 'a.webp', imageVariants: { medium: 'a.2x.webp' } }, 0), { kind: 'slice', path: 'a.webp', slice: 0 });
	assert.deepEqual(targetPictures({ type: 'season', value: '2' }), {});
});
