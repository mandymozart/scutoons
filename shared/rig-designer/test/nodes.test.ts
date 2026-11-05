// The Nodes tab's layout (#282) on the core's own wiring of catpope's rig (shared/live-deck/test/fixtures/deck.json, written
// by `cargo test -p deck-space deck_fixture`): only placement is the tab's.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { NODE, cableText, cableTexts, layoutGraph, sameTexts, type GraphIn } from '../src/nodes.ts';

const DECK = JSON.parse(readFileSync(new URL('../../live-deck/test/fixtures/deck.json', import.meta.url), 'utf8')) as { catpope: { wiring: GraphIn }; own: { wiring: GraphIn } };
const CATPOPE = DECK.catpope.wiring;
const byId = (l: ReturnType<typeof layoutGraph>, id: string) => l.nodes.find((n) => n.id === id)!;

test('a column per depth, the nodes of a column one under the other without overlapping', () => {
	const l = layoutGraph(CATPOPE);
	assert.equal(l.nodes.length, CATPOPE.nodes.length);
	CATPOPE.nodes.forEach((n, i) => assert.equal(l.nodes[i].x, NODE.margin + n.depth * (NODE.width + NODE.gapX), n.id));
	assert.ok(byId(l, 'voice_melody').x < byId(l, 'voice_voice').x && byId(l, 'voice_voice').x < byId(l, 'mix1').x);
	for (const a of l.nodes)
		for (const b of l.nodes) if (a !== b && a.x === b.x) assert.ok(a.y + a.h <= b.y || b.y + b.h <= a.y, `${a.id} / ${b.id}`);
	assert.ok(l.nodes.every((n) => n.x + n.w <= l.width && n.y + n.h <= l.height));
});

test("a box has a row per port, the inputs on its left edge and the outputs on its right; a cable's ends are marked", () => {
	const l = layoutGraph(CATPOPE);
	const fm = byId(l, 'voice_voice');
	assert.deepEqual(fm.inputs.map((p) => p.name), ['cv', 'freq', 'ratio', 'index', 'gain']);
	assert.ok(fm.inputs.every((p) => p.x === fm.x) && fm.outputs.every((p) => p.x === fm.x + fm.w));
	assert.equal(fm.h, NODE.head + 5 * NODE.row);
	assert.deepEqual(fm.inputs.filter((p) => p.wired).map((p) => p.name), ['cv'], 'the melody plays it');
	assert.equal(byId(l, 'out').outputs.length, 0, 'the outlet has no output');
});

test('a cable runs from its output to its input, its label in the middle; every link is drawn', () => {
	const l = layoutGraph(CATPOPE);
	assert.equal(l.cables.length, CATPOPE.links.length);
	const i = CATPOPE.links.findIndex((k) => k.value === 'voice_melody.cv');
	const c = l.cables.find((k) => k.index === i)!;
	const melody = byId(l, 'voice_melody').outputs[0];
	const cv = byId(l, 'voice_voice').inputs[0];
	assert.ok(c.d.startsWith(`M ${melody.x} ${melody.y} C`) && c.d.endsWith(`${cv.x} ${cv.y}`), c.d);
	assert.deepEqual([c.labelX, c.labelY], [(melody.x + cv.x) / 2, (melody.y + cv.y) / 2]);
	assert.ok(l.cables.every((k) => !k.back), 'catpope has no loop');
});

test('a cable back against the columns loops out; a link to a port a node lacks is not drawn', () => {
	const g: GraphIn = {
		nodes: [
			{ id: 'a', kind: 'k', track: '', depth: 0, inputs: [{ name: 'in', type: 'audio' }], outputs: [{ name: 'out', type: 'audio' }], played: true },
			{ id: 'b', kind: 'k', track: '', depth: 1, inputs: [{ name: 'in', type: 'audio' }], outputs: [{ name: 'out', type: 'audio' }], played: true }
		],
		links: [
			{ from: 0, out: 'out', to: 1, input: 'in', type: 'audio', value: 'a.out' },
			{ from: 1, out: 'out', to: 0, input: 'in', type: 'audio', value: 'b.out' },
			{ from: 1, out: 'nope', to: 0, input: 'in', type: 'audio', value: 'b.nope' }
		]
	};
	const l = layoutGraph(g);
	assert.deepEqual(l.cables.map((c) => [c.index, c.back]), [[0, false], [1, true]]);
	assert.deepEqual(layoutGraph({ nodes: [], links: [] }), { nodes: [], cables: [], width: NODE.margin, height: NODE.margin - NODE.gapY });
});

test("the deck's own scene lays out too, its unplayed nodes kept", () => {
	const l = layoutGraph(DECK.own.wiring);
	assert.equal(l.nodes.length, DECK.own.wiring.nodes.length);
	assert.ok(l.nodes.some((n) => !n.played));
});

test("a cable's value as text: none, a peak, a pitch, a control value", () => {
	assert.equal(cableText(NaN), '–');
	assert.equal(cableText(0.4567), '0.46');
	assert.equal(cableText(147), '147');
	assert.equal(cableText(12.34), '12.3');
	assert.equal(cableText(-0.5), '-0.50');
	const read = (p: string) => (p === 'fft.level' ? 0.25 : NaN);
	const texts = cableTexts(CATPOPE.links, read);
	assert.equal(texts.length, CATPOPE.links.length);
	assert.equal(texts[CATPOPE.links.findIndex((k) => k.value === 'fft.level')], '0.25');
	assert.ok(sameTexts(texts, [...texts]) && !sameTexts(texts, texts.slice(1)));
});
